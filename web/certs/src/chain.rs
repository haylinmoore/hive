use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use x509_parser::pem::Pem;

#[derive(Debug, Clone)]
pub struct Link {
    pub sha256: String,
    pub subject: String,
    pub subject_cn: String,
    pub issuer: String,
    pub not_before: i64,
    pub not_after: i64,
    pub pem: String,
}

impl Link {
    pub fn self_signed(&self) -> bool {
        self.subject == self.issuer
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn cn(name: &x509_parser::x509::X509Name) -> String {
    name.iter_common_name()
        .next()
        .and_then(|c| c.as_str().ok())
        .unwrap_or("")
        .to_string()
}

fn armor(der: &[u8]) -> String {
    use std::fmt::Write;
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut b64 = String::new();
    for chunk in der.chunks(3) {
        let n = chunk.iter().fold(0u32, |a, &b| (a << 8) | b as u32) << (8 * (3 - chunk.len()));
        for i in 0..4 {
            if i <= chunk.len() {
                b64.push(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                b64.push('=');
            }
        }
    }
    let mut out = String::from("-----BEGIN CERTIFICATE-----\n");
    for line in b64.as_bytes().chunks(64) {
        let _ = writeln!(out, "{}", String::from_utf8_lossy(line));
    }
    out.push_str("-----END CERTIFICATE-----\n");
    out
}

pub fn parse(pem: &str) -> Vec<Link> {
    Pem::iter_from_buffer(pem.as_bytes())
        .flatten()
        .filter_map(|block| {
            let cert = block.parse_x509().ok()?;
            Some(Link {
                sha256: hex(&Sha256::digest(&block.contents)),
                subject: cert.subject().to_string(),
                subject_cn: cn(cert.subject()),
                issuer: cert.issuer().to_string(),
                not_before: cert.validity().not_before.timestamp(),
                not_after: cert.validity().not_after.timestamp(),
                pem: armor(&block.contents),
            })
        })
        .collect()
}

#[derive(Default)]
pub struct Store {
    by_sha: HashMap<String, Link>,
    by_subject: HashMap<String, Link>,
}

impl Store {
    pub fn add(&mut self, link: Link) {
        self.by_subject
            .entry(link.subject.clone())
            .or_insert_with(|| link.clone());
        self.by_sha.entry(link.sha256.clone()).or_insert(link);
    }

    pub fn load(dir: &Path) -> Store {
        let mut store = Store::default();
        let mut paths: Vec<_> = std::fs::read_dir(dir.join("issuers"))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "pem"))
            .collect();
        paths.sort();
        for path in paths {
            if let Ok(pem) = std::fs::read_to_string(&path) {
                parse(&pem).into_iter().for_each(|l| store.add(l));
            }
        }
        store
    }

    pub fn full_chain(&self, leaf: &str, logged: &[String]) -> Vec<Link> {
        let mut links: Vec<Link> = parse(leaf).into_iter().take(1).collect();
        if links.is_empty() {
            return links;
        }
        links.extend(logged.iter().map_while(|sha| self.by_sha.get(sha).cloned()));
        while let Some(last) = links.last() {
            if last.self_signed() || links.len() > 8 {
                break;
            }
            match self.by_subject.get(&last.issuer) {
                Some(next) if !links.iter().any(|l| l.sha256 == next.sha256) => {
                    links.push(next.clone())
                }
                _ => break,
            }
        }
        links
    }
}

pub fn split(dir: &Path, pem: &str) -> std::io::Result<Option<(String, Vec<String>)>> {
    let mut links = parse(pem).into_iter();
    let Some(leaf) = links.next() else {
        return Ok(None);
    };
    let issuers = dir.join("issuers");
    std::fs::create_dir_all(&issuers)?;
    let mut chain = Vec::new();
    for link in links {
        let path = issuers.join(format!("{}.pem", link.sha256));
        if !path.exists() {
            let tmp = issuers.join(format!(".{}.pem", link.sha256));
            std::fs::write(&tmp, &link.pem)?;
            std::fs::rename(tmp, path)?;
        }
        chain.push(link.sha256);
    }
    Ok(Some((leaf.pem, chain)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_known_values() {
        assert!(armor(b"Man").contains("\nTWFu\n"));
        assert!(armor(b"Ma").contains("\nTWE=\n"));
        assert!(armor(b"M").contains("\nTQ==\n"));
    }

    #[test]
    fn a_real_lets_encrypt_chain_parses_in_order() {
        let chain = parse(include_str!("../tests/chain.pem"));
        let names: Vec<&str> = chain.iter().map(|l| l.subject_cn.as_str()).collect();
        assert_eq!(
            names,
            vec!["", "YE2", "Root YE", "ISRG Root X2", "ISRG Root X1"]
        );
        for pair in chain.windows(2) {
            assert_eq!(pair[0].issuer, pair[1].subject);
        }
        assert_eq!(chain[0].sha256.len(), 64);
        assert!(
            parse(&chain[1].pem)[0].sha256 == chain[1].sha256,
            "re-armoured pem is the same cert"
        );
    }

    fn tmp(name: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("certs-chain-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn shas(links: &[Link]) -> Vec<&str> {
        links.iter().map(|l| l.sha256.as_str()).collect()
    }

    #[test]
    fn a_split_chain_is_stored_once_and_rebuilt_exactly() {
        let dir = tmp("split");
        let full = include_str!("../tests/chain.pem");
        let links = parse(full);
        let (leaf, logged) = split(&dir, full).unwrap().unwrap();
        assert_eq!(parse(&leaf).len(), 1);
        assert_eq!(logged.len(), 4);
        split(&dir, full).unwrap();
        assert_eq!(std::fs::read_dir(dir.join("issuers")).unwrap().count(), 4);
        let store = Store::load(&dir);
        assert_eq!(shas(&store.full_chain(&leaf, &logged)), shas(&links));
    }

    #[test]
    fn a_bare_leaf_gets_the_chain_its_issuer_is_known_by() {
        let dir = tmp("bare");
        let full = include_str!("../tests/chain.pem");
        let links = parse(full);
        split(&dir, full).unwrap();
        let store = Store::load(&dir);
        assert_eq!(shas(&store.full_chain(&links[0].pem, &[])), shas(&links));
        assert_eq!(Store::default().full_chain(&links[0].pem, &[]).len(), 1);
    }

    #[test]
    fn garbage_is_no_chain() {
        assert!(parse("not a certificate").is_empty());
    }
}
