use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub tbs_sha256: String,
    pub cert_sha256: String,
    pub zone: String,
    pub log_uri: String,
    pub entry_index: u64,
    #[serde(default)]
    pub issuer: Option<String>,
    #[serde(default)]
    pub serial: Option<String>,
    #[serde(default)]
    pub not_before: Option<i64>,
    #[serde(default)]
    pub not_after: Option<i64>,
    #[serde(default)]
    pub dns_names: Vec<String>,
    pub discovered_at: i64,
    #[serde(default)]
    pub pem: Option<String>,
}

impl Record {
    pub fn issuer_cn(&self) -> &str {
        let dn = self.issuer.as_deref().unwrap_or("");
        dn.split(',')
            .find_map(|p| p.trim().strip_prefix("CN="))
            .unwrap_or(dn)
    }

    pub fn crt_sh(&self) -> String {
        match &self.serial {
            Some(serial) => format!("https://crt.sh/?serial={serial}"),
            None => format!("https://crt.sh/?sha256={}", self.cert_sha256),
        }
    }
}

#[derive(Deserialize)]
struct CertspotterJson {
    #[serde(default)]
    dns_names: Vec<String>,
}

pub fn from_env(env: impl Fn(&str) -> Option<String>, now: i64) -> Option<Record> {
    if env("EVENT").as_deref() != Some("discovered_cert") {
        return None;
    }
    let time = |k: &str| env(k).and_then(|v| v.parse::<i64>().ok());
    let read = |k: &str| env(k).and_then(|p| std::fs::read_to_string(p).ok());
    Some(Record {
        tbs_sha256: env("TBS_SHA256")?,
        cert_sha256: env("CERT_SHA256")?,
        zone: env("WATCH_ITEM")?.trim_start_matches('.').to_string(),
        log_uri: env("LOG_URI").unwrap_or_default(),
        entry_index: env("ENTRY_INDEX").and_then(|v| v.parse().ok()).unwrap_or(0),
        issuer: env("ISSUER_DN"),
        serial: env("SERIAL").map(|s| s.to_lowercase()),
        not_before: time("NOT_BEFORE_UNIXTIME"),
        not_after: time("NOT_AFTER_UNIXTIME"),
        dns_names: read("JSON_FILENAME")
            .and_then(|raw| serde_json::from_str::<CertspotterJson>(&raw).ok())
            .map(|j| j.dns_names)
            .unwrap_or_default(),
        discovered_at: now,
        pem: read("CERT_FILENAME"),
    })
}

pub fn save(dir: &Path, record: &Record) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("{}.json", record.cert_sha256));
    let tmp = dir.join(format!(".{}.json", record.cert_sha256));
    std::fs::write(
        &tmp,
        serde_json::to_vec(record).map_err(std::io::Error::other)?,
    )?;
    std::fs::rename(tmp, path)
}

pub fn load(dir: &Path) -> Vec<Record> {
    let mut out: Vec<Record> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            name.ends_with(".json") && !name.starts_with('.')
        })
        .filter_map(|e| serde_json::from_slice(&std::fs::read(e.path()).ok()?).ok())
        .collect();
    out.sort_by_key(|r| std::cmp::Reverse(r.discovered_at));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn tmp(name: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("certs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn a_discovered_cert_becomes_a_record() {
        let dir = tmp("env");
        let json = dir.join("c.v1.json");
        std::fs::write(&json, r#"{"dns_names":["dash.hive.hayl.in"]}"#).unwrap();
        let pem = dir.join("c.pem");
        std::fs::write(&pem, "-----BEGIN CERTIFICATE-----").unwrap();

        let vars: HashMap<&str, String> = HashMap::from([
            ("EVENT", "discovered_cert".into()),
            ("TBS_SHA256", "ab".into()),
            ("CERT_SHA256", "cd".into()),
            ("WATCH_ITEM", ".hayl.in".into()),
            ("LOG_URI", "https://log.example/".into()),
            ("ENTRY_INDEX", "7".into()),
            ("ISSUER_DN", "C=US, O=Let's Encrypt, CN=YE2".into()),
            ("SERIAL", "0681AB".into()),
            ("NOT_BEFORE_UNIXTIME", "100".into()),
            ("JSON_FILENAME", json.to_string_lossy().into()),
            ("CERT_FILENAME", pem.to_string_lossy().into()),
        ]);
        let r = from_env(|k| vars.get(k).cloned(), 5).unwrap();
        assert_eq!(r.zone, "hayl.in");
        assert_eq!(r.issuer_cn(), "YE2");
        assert_eq!(r.serial.as_deref(), Some("0681ab"));
        assert_eq!(r.dns_names, vec!["dash.hive.hayl.in"]);
        assert_eq!(r.not_before, Some(100));
        assert!(r.pem.as_deref().unwrap().starts_with("-----BEGIN"));
        assert_eq!(r.crt_sh(), "https://crt.sh/?serial=0681ab");
    }

    #[test]
    fn other_events_are_ignored() {
        assert!(from_env(|k| (k == "EVENT").then(|| "error".to_string()), 0).is_none());
    }

    #[test]
    fn records_round_trip_newest_first() {
        let dir = tmp("store");
        for (sha, at) in [("a", 5), ("b", 9)] {
            let r = Record {
                tbs_sha256: sha.into(),
                cert_sha256: sha.into(),
                zone: "hayl.in".into(),
                log_uri: String::new(),
                entry_index: 0,
                issuer: None,
                serial: None,
                not_before: None,
                not_after: None,
                dns_names: vec![],
                discovered_at: at,
                pem: None,
            };
            save(&dir, &r).unwrap();
        }
        std::fs::write(dir.join(".half-written.json"), "{").unwrap();
        let all = load(&dir);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].cert_sha256, "b");
    }
}
