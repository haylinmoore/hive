use crate::chain::{self, Store};
use crate::record::Record;
use std::collections::HashSet;
use std::path::Path;

async fn fetch(client: &reqwest::Client, url: &str) -> Result<Vec<chain::Link>, String> {
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.status().to_string());
    }
    let body = resp.bytes().await.map_err(|e| e.to_string())?;
    let links = chain::from_download(&body);
    if links.is_empty() {
        return Err("no certificate in the response".into());
    }
    Ok(links)
}

pub async fn complete(client: &reqwest::Client, dir: &Path, records: &[Record]) -> usize {
    let mut tried = HashSet::new();
    let mut added = 0;
    for _ in 0..6 {
        let store = Store::load(dir);
        let wanted: HashSet<String> = records
            .iter()
            .filter_map(|r| store.wanted(r.pem.as_deref()?, &r.chain))
            .filter(|url| !tried.contains(url))
            .collect();
        if wanted.is_empty() {
            break;
        }
        for url in wanted {
            match fetch(client, &url).await {
                Ok(links) => {
                    for link in links {
                        match chain::keep(dir, &link) {
                            Ok(true) => added += 1,
                            Ok(false) => {}
                            Err(e) => eprintln!("cannot keep {}: {e}", link.sha256),
                        }
                    }
                }
                Err(e) => eprintln!("issuer {url}: {e}"),
            }
            tried.insert(url);
        }
    }
    added
}
