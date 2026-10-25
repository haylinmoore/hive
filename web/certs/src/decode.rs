use x509_parser::der_parser::oid::Oid;
use x509_parser::extensions::{DistributionPointName, GeneralName, ParsedExtension};
use x509_parser::objects::{oid_registry, oid2sn};
use x509_parser::pem::Pem;
use x509_parser::prelude::{FromDer, X509Certificate};
use x509_parser::public_key::PublicKey;

pub type Field = (String, Vec<String>);

fn colon_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

fn oid_name(oid: &Oid) -> String {
    match oid.to_id_string().as_str() {
        "2.23.140.1.2.1" => "domain validated".into(),
        "2.23.140.1.2.2" => "organization validated".into(),
        "2.23.140.1.2.3" => "individual validated".into(),
        "2.23.140.1.1" => "extended validation".into(),
        "1.3.6.1.4.1.11129.2.4.3" => "precertificate poison".into(),
        "1.3.6.1.5.5.7.48.1" => "ocsp".into(),
        "1.3.6.1.5.5.7.48.2" => "ca issuers".into(),
        id => oid2sn(oid, oid_registry())
            .map(str::to_string)
            .unwrap_or_else(|_| id.to_string()),
    }
}

fn names(list: &[GeneralName]) -> Vec<String> {
    list.iter()
        .map(|n| match n {
            GeneralName::DNSName(d) => d.to_string(),
            GeneralName::URI(u) => u.to_string(),
            GeneralName::IPAddress(ip) => match ip.len() {
                4 => std::net::Ipv4Addr::from(<[u8; 4]>::try_from(*ip).unwrap()).to_string(),
                16 => std::net::Ipv6Addr::from(<[u8; 16]>::try_from(*ip).unwrap()).to_string(),
                _ => colon_hex(ip),
            },
            other => other.to_string(),
        })
        .collect()
}

fn key(cert: &X509Certificate) -> String {
    let spki = cert.public_key();
    match spki.parsed() {
        Ok(PublicKey::RSA(k)) => format!("RSA {} bits", k.key_size()),
        Ok(PublicKey::EC(k)) => {
            let curve = spki
                .algorithm
                .parameters
                .as_ref()
                .and_then(|p| p.as_oid().ok())
                .map(|o| oid_name(&o))
                .unwrap_or_default();
            format!("EC {curve} {} bits", k.key_size())
        }
        _ => oid_name(&spki.algorithm.algorithm),
    }
}

fn extension(parsed: &ParsedExtension) -> Option<(&'static str, Vec<String>)> {
    Some(match parsed {
        ParsedExtension::SubjectAlternativeName(san) => {
            ("subject alt names", names(&san.general_names))
        }
        ParsedExtension::KeyUsage(ku) => ("key usage", vec![ku.to_string()]),
        ParsedExtension::ExtendedKeyUsage(eku) => {
            let mut v: Vec<String> = [
                (eku.any, "any"),
                (eku.server_auth, "server auth"),
                (eku.client_auth, "client auth"),
                (eku.code_signing, "code signing"),
                (eku.email_protection, "email protection"),
                (eku.time_stamping, "time stamping"),
                (eku.ocsp_signing, "ocsp signing"),
            ]
            .iter()
            .filter(|(on, _)| *on)
            .map(|(_, name)| name.to_string())
            .collect();
            v.extend(eku.other.iter().map(oid_name));
            ("extended key usage", v)
        }
        ParsedExtension::BasicConstraints(bc) => (
            "basic constraints",
            vec![match (bc.ca, bc.path_len_constraint) {
                (true, Some(n)) => format!("CA, path length {n}"),
                (true, None) => "CA".into(),
                (false, _) => "not a CA".into(),
            }],
        ),
        ParsedExtension::SubjectKeyIdentifier(id) => ("subject key id", vec![colon_hex(id.0)]),
        ParsedExtension::AuthorityKeyIdentifier(aki) => (
            "authority key id",
            vec![aki.key_identifier.as_ref().map(|k| colon_hex(k.0))?],
        ),
        ParsedExtension::AuthorityInfoAccess(aia) => (
            "authority info access",
            aia.accessdescs
                .iter()
                .map(|d| {
                    format!(
                        "{}: {}",
                        oid_name(&d.access_method),
                        names(std::slice::from_ref(&d.access_location)).join("")
                    )
                })
                .collect(),
        ),
        ParsedExtension::CRLDistributionPoints(crl) => (
            "crl distribution points",
            crl.points
                .iter()
                .flat_map(|p| match &p.distribution_point {
                    Some(DistributionPointName::FullName(list)) => names(list),
                    _ => vec![],
                })
                .collect(),
        ),
        ParsedExtension::CertificatePolicies(policies) => (
            "policies",
            policies.iter().map(|p| oid_name(&p.policy_id)).collect(),
        ),
        ParsedExtension::SCT(scts) => (
            "signed certificate timestamps",
            scts.iter()
                .map(|s| {
                    let at = chrono::DateTime::from_timestamp_millis(s.timestamp as i64)
                        .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                        .unwrap_or_default();
                    format!("{at}, log {}", crate::chain::base64(s.id.key_id))
                })
                .collect(),
        ),
        _ => return None,
    })
}

pub fn serial(pem: &str) -> Option<String> {
    let block = Pem::iter_from_buffer(pem.as_bytes()).next()?.ok()?;
    let (_, cert) = X509Certificate::from_der(&block.contents).ok()?;
    Some(
        cert.raw_serial()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    )
}

pub fn fields(pem: &str) -> Vec<Field> {
    let Some(Ok(block)) = Pem::iter_from_buffer(pem.as_bytes()).next() else {
        return vec![];
    };
    let Ok((_, cert)) = X509Certificate::from_der(&block.contents) else {
        return vec![];
    };
    let time = |t: x509_parser::time::ASN1Time| {
        chrono::DateTime::from_timestamp(t.timestamp(), 0)
            .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_default()
    };
    let mut out: Vec<Field> = vec![
        ("version".into(), vec![format!("{}", cert.version().0 + 1)]),
        ("serial".into(), vec![colon_hex(cert.raw_serial())]),
        (
            "signature".into(),
            vec![oid_name(&cert.signature_algorithm.algorithm)],
        ),
        ("issuer".into(), vec![cert.issuer().to_string()]),
        (
            "subject".into(),
            vec![match cert.subject().to_string() {
                s if s.is_empty() => "(empty)".into(),
                s => s,
            }],
        ),
        ("not before".into(), vec![time(cert.validity().not_before)]),
        ("not after".into(), vec![time(cert.validity().not_after)]),
        ("public key".into(), vec![key(&cert)]),
    ];
    for ext in cert.extensions() {
        let (label, values) = match extension(ext.parsed_extension()) {
            Some((label, values)) => (label.to_string(), values),
            None => (oid_name(&ext.oid), vec![]),
        };
        let label = if ext.critical {
            format!("{label} (critical)")
        } else {
            label
        };
        out.push((label, values));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_real_leaf_decodes() {
        let links = crate::chain::parse(include_str!("../tests/chain.pem"));
        let f = fields(&links[0].pem);
        let get = |label: &str| {
            f.iter()
                .find(|(l, _)| l == label)
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| panic!("no {label} in {f:?}"))
        };
        assert_eq!(get("version"), vec!["3"]);
        assert!(get("public key")[0].starts_with("EC "));
        assert!(get("subject alt names (critical)").contains(&"estrogen.coffee".to_string()));
        assert_eq!(get("extended key usage"), vec!["server auth"]);
        assert_eq!(get("basic constraints (critical)"), vec!["not a CA"]);
        assert_eq!(get("policies"), vec!["domain validated"]);
        assert!(get("signed certificate timestamps").len() >= 2);
        assert_eq!(
            get("authority info access"),
            vec!["ca issuers: http://ye2.i.lencr.org/"]
        );
        assert_eq!(get("subject"), vec!["(empty)"]);
        assert!(fields("garbage").is_empty());
        assert_eq!(
            serial(&links[0].pem).as_deref(),
            Some("06b057d1275a95305026c3901d37e37bb20f")
        );
    }
}
