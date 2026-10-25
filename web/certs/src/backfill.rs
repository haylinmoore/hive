use crate::record::{self, Record};
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;

#[derive(Deserialize)]
struct Issuer {
    #[serde(default)]
    name: Option<String>,
}

#[derive(Deserialize)]
struct Issuance {
    id: String,
    tbs_sha256: String,
    cert_sha256: String,
    #[serde(default)]
    dns_names: Vec<String>,
    not_before: String,
    not_after: String,
    #[serde(default)]
    issuer: Option<Issuer>,
    #[serde(default)]
    cert_der: Option<String>,
}

#[derive(Deserialize)]
struct Logged {
    id: u64,
    issuer_name: String,
    name_value: String,
    not_before: String,
    not_after: String,
    serial_number: String,
}

fn rfc3339(v: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(v)
        .ok()
        .map(|t| t.timestamp())
}

fn naive(v: &str) -> Option<i64> {
    chrono::NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S")
        .ok()
        .map(|t| t.and_utc().timestamp())
}

fn pem(der_base64: &str) -> String {
    let body: Vec<String> = der_base64
        .as_bytes()
        .chunks(64)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect();
    format!(
        "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
        body.join("\n")
    )
}

fn from_sslmate(zone: &str, i: Issuance) -> Record {
    Record {
        tbs_sha256: i.tbs_sha256,
        cert_sha256: i.cert_sha256,
        zone: zone.to_string(),
        log_uri: String::new(),
        entry_index: 0,
        issuer: i.issuer.and_then(|x| x.name),
        serial: None,
        not_before: rfc3339(&i.not_before),
        not_after: rfc3339(&i.not_after),
        dns_names: i.dns_names,
        discovered_at: rfc3339(&i.not_before).unwrap_or(0),
        pem: i.cert_der.as_deref().map(pem),
        source: Some("sslmate".into()),
    }
}

fn from_crtsh(zone: &str, l: Logged) -> Record {
    let mut names: Vec<String> = l.name_value.lines().map(str::to_string).collect();
    names.sort();
    names.dedup();
    Record {
        tbs_sha256: String::new(),
        cert_sha256: format!("crtsh-{}", l.id),
        zone: zone.to_string(),
        log_uri: String::new(),
        entry_index: 0,
        issuer: Some(l.issuer_name),
        serial: Some(l.serial_number.to_lowercase()),
        not_before: naive(&l.not_before),
        not_after: naive(&l.not_after),
        dns_names: names,
        discovered_at: naive(&l.not_before).unwrap_or(0),
        pem: None,
        source: Some("crt.sh".into()),
    }
}

fn same_cert(a: &Record, b: &Record) -> bool {
    if !a.tbs_sha256.is_empty() && a.tbs_sha256 == b.tbs_sha256 {
        return true;
    }
    if a.serial.is_some() && a.serial == b.serial {
        return true;
    }
    let mut x = a.dns_names.clone();
    let mut y = b.dns_names.clone();
    x.sort();
    y.sort();
    a.not_before == b.not_before && a.not_after == b.not_after && x == y
}

pub fn merge_new(existing: &[Record], found: Vec<Record>, since: i64) -> Vec<Record> {
    let mut out: Vec<Record> = Vec::new();
    for r in found {
        if r.issued() < since {
            continue;
        }
        if existing.iter().chain(out.iter()).any(|e| same_cert(e, &r)) {
            continue;
        }
        out.push(r);
    }
    out
}

pub enum Fetch {
    Limited(Duration),
    Failed(String),
}

pub fn retry_after(headers: &reqwest::header::HeaderMap) -> Duration {
    let secs = headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(60);
    Duration::from_secs(secs.clamp(1, 3600))
}

async fn sslmate(
    client: &reqwest::Client,
    key: Option<&str>,
    zone: &str,
) -> Result<Vec<Record>, Fetch> {
    let mut out = Vec::new();
    let mut after: Option<String> = None;
    for _ in 0..20 {
        let mut url = format!(
            "https://api.certspotter.com/v1/issuances?domain={zone}&include_subdomains=true&expand=dns_names&expand=issuer&expand=cert_der"
        );
        if let Some(a) = &after {
            url.push_str(&format!("&after={a}"));
        }
        let mut req = client.get(&url);
        if let Some(key) = key {
            req = req.bearer_auth(key);
        }
        let page: Vec<Issuance> = match req.send().await {
            Ok(r) if r.status().is_success() => {
                r.json().await.map_err(|e| Fetch::Failed(e.to_string()))?
            }
            Ok(r) if r.status() == reqwest::StatusCode::TOO_MANY_REQUESTS => {
                return Err(Fetch::Limited(retry_after(r.headers())));
            }
            Ok(r) => return Err(Fetch::Failed(r.status().to_string())),
            Err(e) => return Err(Fetch::Failed(e.to_string())),
        };
        let Some(last) = page.last().map(|i| i.id.clone()) else {
            break;
        };
        out.extend(page.into_iter().map(|i| from_sslmate(zone, i)));
        after = Some(last);
    }
    Ok(out)
}

async fn crtsh(client: &reqwest::Client, zone: &str) -> Option<Vec<Record>> {
    let url = format!("https://crt.sh/?q={zone}&output=json");
    for attempt in 0..2 {
        match client.get(&url).send().await {
            Ok(r) if r.status().is_success() => {
                let rows: Vec<Logged> = r.json().await.unwrap_or_default();
                return Some(rows.into_iter().map(|l| from_crtsh(zone, l)).collect());
            }
            Ok(r) => eprintln!("crt.sh {zone}: {}", r.status()),
            Err(e) => eprintln!("crt.sh {zone}: {e}"),
        }
        tokio::time::sleep(Duration::from_secs(10 * (attempt + 1))).await;
    }
    None
}

pub fn run(dir: &Path, zones: &[String], since: i64, key: Option<String>) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .user_agent("certs.hayl.in backfill")
        .build()
        .expect("http client");
    tokio::runtime::Runtime::new()
        .expect("tokio runtime")
        .block_on(async {
            let mut crtsh_up = true;
            'zones: for zone in zones {
                let done = dir.join(format!(".backfilled-{zone}"));
                if done.exists() {
                    continue;
                }
                let mut found = loop {
                    match sslmate(&client, key.as_deref(), zone).await {
                        Ok(found) => break found,
                        Err(Fetch::Limited(wait)) => {
                            eprintln!("sslmate {zone}: rate limited, waiting {}s", wait.as_secs());
                            tokio::time::sleep(wait).await;
                        }
                        Err(Fetch::Failed(e)) => {
                            eprintln!("sslmate {zone}: {e}, trying it again next run");
                            continue 'zones;
                        }
                    }
                };
                if crtsh_up {
                    match crtsh(&client, zone).await {
                        Some(more) => found.extend(more),
                        None => {
                            eprintln!("crt.sh is down, skipping it for the rest of this run");
                            crtsh_up = false;
                        }
                    }
                }
                let existing = record::load(dir);
                let fresh = merge_new(&existing, found, since);
                for r in &fresh {
                    if let Err(e) = record::save(dir, r) {
                        eprintln!("cannot save {}: {e}", r.cert_sha256);
                    }
                }
                eprintln!("{zone}: {} added", fresh.len());
                let _ = std::fs::write(&done, "");
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(tbs: &str, serial: Option<&str>, nb: i64, names: &[&str]) -> Record {
        Record {
            tbs_sha256: tbs.into(),
            cert_sha256: format!("{tbs}{nb}"),
            zone: "hayl.in".into(),
            log_uri: String::new(),
            entry_index: 0,
            issuer: None,
            serial: serial.map(str::to_string),
            not_before: Some(nb),
            not_after: Some(nb + 100),
            dns_names: names.iter().map(|s| s.to_string()).collect(),
            discovered_at: nb,
            pem: None,
            source: None,
        }
    }

    #[test]
    fn a_cert_already_known_by_any_key_is_skipped() {
        let live = vec![rec("t1", None, 500, &["a.hayl.in"])];
        let found = vec![
            rec("t1", None, 500, &["a.hayl.in"]),
            rec("", Some("06"), 600, &["b.hayl.in"]),
            rec("", Some("06"), 600, &["b.hayl.in"]),
            rec("", Some("07"), 500, &["a.hayl.in"]),
            rec("", Some("08"), 50, &["old.hayl.in"]),
        ];
        let fresh = merge_new(&live, found, 100);
        assert_eq!(fresh.len(), 1);
        assert_eq!(fresh[0].serial.as_deref(), Some("06"));
    }

    #[test]
    fn crtsh_rows_become_records() {
        let l = Logged {
            id: 42,
            issuer_name: "C=US, O=Let's Encrypt, CN=YE2".into(),
            name_value: "*.estrogen.coffee\nestrogen.coffee".into(),
            not_before: "2026-08-17T01:47:28".into(),
            not_after: "2026-11-15T01:47:27".into(),
            serial_number: "06E684A5".into(),
        };
        let r = from_crtsh("estrogen.coffee", l);
        assert_eq!(r.cert_sha256, "crtsh-42");
        assert_eq!(r.serial.as_deref(), Some("06e684a5"));
        assert_eq!(r.issuer_name(), "Let's Encrypt YE2");
        assert_eq!(r.not_after.unwrap() - r.not_before.unwrap(), 90 * 86400 - 1);
    }

    #[test]
    fn retry_after_is_honoured_within_reason() {
        let mut h = reqwest::header::HeaderMap::new();
        assert_eq!(retry_after(&h), Duration::from_secs(60));
        h.insert(reqwest::header::RETRY_AFTER, "120".parse().unwrap());
        assert_eq!(retry_after(&h), Duration::from_secs(120));
        h.insert(reqwest::header::RETRY_AFTER, "99999".parse().unwrap());
        assert_eq!(retry_after(&h), Duration::from_secs(3600));
    }

    #[test]
    fn der_is_wrapped_as_pem() {
        let p = pem(&"A".repeat(70));
        assert!(p.starts_with("-----BEGIN CERTIFICATE-----\n"));
        assert_eq!(p.lines().nth(1).unwrap().len(), 64);
        assert!(p.ends_with("-----END CERTIFICATE-----\n"));
    }
}
