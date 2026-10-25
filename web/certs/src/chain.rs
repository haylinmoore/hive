use sha2::{Digest, Sha256};
use x509_parser::pem::Pem;

#[derive(Debug, Clone)]
pub struct Link {
    pub sha256: String,
    pub subject: String,
    pub subject_cn: String,
    pub issuer: String,
    pub issuer_cn: String,
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
                issuer_cn: cn(cert.issuer()),
                not_before: cert.validity().not_before.timestamp(),
                not_after: cert.validity().not_after.timestamp(),
                pem: armor(&block.contents),
            })
        })
        .collect()
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

    #[test]
    fn garbage_is_no_chain() {
        assert!(parse("not a certificate").is_empty());
    }
}
