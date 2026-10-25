use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Cert {
    pub name: String,
    pub domains: Vec<String>,
    pub issuer: Option<String>,
    pub not_before: Option<i64>,
    pub not_after: Option<i64>,
    pub fingerprint: Option<String>,
    pub renew_from: Option<i64>,
    pub next_check: Option<i64>,
    pub last_check: Option<i64>,
    pub last_ok: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventKind {
    Issued,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertEvent {
    pub at: i64,
    pub name: String,
    pub kind: EventKind,
    pub detail: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CertFile {
    pub checked_at: i64,
    pub certs: Vec<Cert>,
    pub history: Vec<CertEvent>,
}

#[derive(Debug, Default, PartialEq)]
pub struct X509 {
    pub domains: Vec<String>,
    pub issuer: Option<String>,
    pub not_before: Option<i64>,
    pub not_after: Option<i64>,
    pub fingerprint: Option<String>,
}

fn openssl_time(v: &str) -> Option<i64> {
    let v = v.trim().trim_end_matches(" GMT");
    chrono::NaiveDateTime::parse_from_str(v, "%b %e %H:%M:%S %Y")
        .ok()
        .map(|t| t.and_utc().timestamp())
}

pub fn parse_x509(text: &str) -> X509 {
    let mut out = X509::default();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("issuer=") {
            out.issuer = v
                .split(',')
                .map(|part| part.replace(' ', ""))
                .find_map(|part| part.strip_prefix("CN=").map(str::to_string))
                .or_else(|| Some(v.trim().to_string()));
        } else if let Some(v) = line.strip_prefix("notBefore=") {
            out.not_before = openssl_time(v);
        } else if let Some(v) = line.strip_prefix("notAfter=") {
            out.not_after = openssl_time(v);
        } else if let Some((_, v)) = line.split_once("Fingerprint=") {
            out.fingerprint = Some(v.trim().to_string());
        } else if line.starts_with("X509v3 Subject Alternative Name")
            && let Some(names) = lines.next()
        {
            out.domains = names
                .split(',')
                .filter_map(|n| n.trim().strip_prefix("DNS:").map(str::to_string))
                .collect();
        }
    }
    out
}

fn read_cert(path: &Path) -> Option<X509> {
    let out = Command::new("openssl")
        .args(["x509", "-noout", "-issuer", "-startdate", "-enddate"])
        .args(["-fingerprint", "-sha256", "-ext", "subjectAltName", "-in"])
        .arg(path)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| parse_x509(&String::from_utf8_lossy(&out.stdout)))
}

#[derive(Deserialize)]
struct Timer {
    unit: String,
    #[serde(default)]
    next: Option<u64>,
    #[serde(default)]
    last: Option<u64>,
}

fn usec(v: Option<u64>) -> Option<i64> {
    v.filter(|&u| u > 0).map(|u| (u / 1_000_000) as i64)
}

fn timers() -> Vec<(String, Option<i64>, Option<i64>)> {
    let Ok(out) = Command::new("systemctl")
        .args(["list-timers", "acme-renew-*", "--all", "-o", "json"])
        .output()
    else {
        return Vec::new();
    };
    let timers: Vec<Timer> = serde_json::from_slice(&out.stdout).unwrap_or_default();
    timers
        .into_iter()
        .filter_map(|t| {
            let name = t
                .unit
                .strip_prefix("acme-renew-")?
                .strip_suffix(".timer")?
                .to_string();
            Some((name, usec(t.next), usec(t.last)))
        })
        .collect()
}

fn last_run(name: &str) -> (bool, Option<i64>) {
    let unit = format!("acme-order-renew-{name}.service");
    let Ok(out) = Command::new("systemctl")
        .args([
            "show",
            "--timestamp=unix",
            "-p",
            "Result",
            "-p",
            "ExecMainExitTimestamp",
            &unit,
        ])
        .output()
    else {
        return (true, None);
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let field = |key: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(key))
            .map(str::trim)
            .unwrap_or("")
            .to_string()
    };
    let ok = matches!(field("Result=").as_str(), "success" | "");
    let exited = field("ExecMainExitTimestamp=")
        .trim_start_matches('@')
        .parse()
        .ok();
    (ok, exited)
}

pub fn renew_from(x509: &X509, min_days: Option<i64>) -> Option<i64> {
    let after = x509.not_after?;
    if let Some(days) = min_days {
        return Some(after - days * 86400);
    }
    let lifetime = after - x509.not_before?;
    let keep = if lifetime <= 10 * 86400 {
        lifetime / 2
    } else {
        lifetime / 3
    };
    Some(after - keep)
}

pub fn parse_renew_days(raw: &[String]) -> BTreeMap<String, i64> {
    raw.iter()
        .filter_map(|e| {
            let (name, days) = e.rsplit_once('=')?;
            Some((name.to_string(), days.parse().ok()?))
        })
        .collect()
}

pub fn merge(
    previous: &CertFile,
    certs: Vec<Cert>,
    failures: Vec<(String, i64)>,
    now: i64,
) -> CertFile {
    let mut history = previous.history.clone();
    for cert in &certs {
        let Some(fp) = &cert.fingerprint else {
            continue;
        };
        let known = history
            .iter()
            .any(|e| e.kind == EventKind::Issued && e.name == cert.name && &e.detail == fp);
        if !known {
            history.push(CertEvent {
                at: cert.not_before.unwrap_or(now),
                name: cert.name.clone(),
                kind: EventKind::Issued,
                detail: fp.clone(),
            });
        }
    }
    for (name, at) in failures {
        let known = history
            .iter()
            .any(|e| e.kind == EventKind::Failed && e.name == name && e.at == at);
        if !known {
            history.push(CertEvent {
                at,
                name,
                kind: EventKind::Failed,
                detail: "renewal failed".into(),
            });
        }
    }
    history.sort_by_key(|e| std::cmp::Reverse(e.at));
    history.truncate(300);
    CertFile {
        checked_at: now,
        certs,
        history,
    }
}

pub fn collect(cfg: &Config, previous: &CertFile, now: i64) -> CertFile {
    let renew_days = parse_renew_days(&cfg.cert_renew_days);
    let mut certs = Vec::new();
    let mut failures = Vec::new();
    for (name, next, last) in timers() {
        let x509 =
            read_cert(&Path::new("/var/lib/acme").join(&name).join("cert.pem")).unwrap_or_default();
        let (ok, exited) = last_run(&name);
        if !ok && let Some(at) = exited {
            failures.push((name.clone(), at));
        }
        certs.push(Cert {
            renew_from: renew_from(&x509, renew_days.get(&name).copied()),
            name,
            domains: x509.domains,
            issuer: x509.issuer,
            not_before: x509.not_before,
            not_after: x509.not_after,
            fingerprint: x509.fingerprint,
            next_check: next,
            last_check: last,
            last_ok: ok,
        });
    }
    certs.sort_by(|a, b| a.name.cmp(&b.name));
    merge(previous, certs, failures, now)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPENSSL: &str = "issuer=C=US, O=Let's Encrypt, CN=R11
notBefore=Oct 10 12:00:00 2026 GMT
notAfter=Jan  8 12:00:00 2027 GMT
sha256 Fingerprint=AB:CD:EF
X509v3 Subject Alternative Name:
    DNS:zoe.infra.hayl.in, DNS:*.zoe.infra.hayl.in
";

    #[test]
    fn openssl_output_parses() {
        let x = parse_x509(OPENSSL);
        assert_eq!(x.issuer.as_deref(), Some("R11"));
        assert_eq!(x.domains, vec!["zoe.infra.hayl.in", "*.zoe.infra.hayl.in"]);
        assert_eq!(x.fingerprint.as_deref(), Some("AB:CD:EF"));
        assert_eq!(x.not_after, Some(1799409600));
        assert_eq!(x.not_after.unwrap() - x.not_before.unwrap(), 90 * 86400);
    }

    #[test]
    fn spaced_issuer_fields_parse_too() {
        let x = parse_x509("issuer=C = US, O = Let's Encrypt, CN = E6\n");
        assert_eq!(x.issuer.as_deref(), Some("E6"));
    }

    fn cert(name: &str, fp: &str, issued: i64) -> Cert {
        Cert {
            name: name.into(),
            fingerprint: Some(fp.into()),
            not_before: Some(issued),
            ..Default::default()
        }
    }

    #[test]
    fn a_new_fingerprint_is_an_issue_and_the_same_one_is_not() {
        let first = merge(
            &CertFile::default(),
            vec![cert("zoe", "aa", 100)],
            vec![],
            1000,
        );
        assert_eq!(first.history.len(), 1);
        assert_eq!(first.history[0].at, 100);

        let again = merge(&first, vec![cert("zoe", "aa", 100)], vec![], 2000);
        assert_eq!(again.history.len(), 1);

        let renewed = merge(&again, vec![cert("zoe", "bb", 5000)], vec![], 6000);
        assert_eq!(renewed.history.len(), 2);
        assert_eq!(renewed.history[0].detail, "bb");
    }

    #[test]
    fn each_failed_run_is_recorded_once() {
        let one = merge(&CertFile::default(), vec![], vec![("zoe".into(), 50)], 100);
        let same = merge(&one, vec![], vec![("zoe".into(), 50)], 200);
        assert_eq!(same.history.len(), 1);
        let next = merge(&same, vec![], vec![("zoe".into(), 900)], 1000);
        assert_eq!(next.history.len(), 2);
        assert_eq!(next.history[0].kind, EventKind::Failed);
    }

    #[test]
    fn renewal_follows_lego_when_no_minimum_is_set() {
        let six_day = X509 {
            not_before: Some(0),
            not_after: Some(6 * 86400),
            ..Default::default()
        };
        assert_eq!(renew_from(&six_day, None), Some(3 * 86400));
        assert_eq!(renew_from(&six_day, Some(1)), Some(5 * 86400));

        let ninety_day = X509 {
            not_before: Some(0),
            not_after: Some(90 * 86400),
            ..Default::default()
        };
        assert_eq!(renew_from(&ninety_day, None), Some(60 * 86400));
    }

    #[test]
    fn renew_days_come_from_the_module() {
        let raw = vec!["dash.hive.hayl.in=30".to_string(), "x=bad".to_string()];
        let days = parse_renew_days(&raw);
        assert_eq!(days.get("dash.hive.hayl.in"), Some(&30));
        assert_eq!(days.len(), 1);
    }
}
