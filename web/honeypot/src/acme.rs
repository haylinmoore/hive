use crate::names;
use crate::shared::Shared;
use crate::store::{self, now_ms};
use instant_acme::{
    Account, AccountCredentials, AuthorizationStatus, ChallengeType, Identifier, NewAccount,
    NewOrder, OrderStatus, RetryPolicy,
};
use rusqlite::{OptionalExtension, params};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::sync::Arc;
use std::time::Duration;
use x509_parser::extensions::ParsedExtension;

type Error = Box<dyn std::error::Error + Send + Sync>;

const RENEW_BEFORE_MS: i64 = 30 * 86_400_000;

fn write_private(path: &std::path::Path, data: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&tmp)?
        .write_all(data.as_bytes())?;
    std::fs::rename(tmp, path)
}

async fn account(shared: &Shared) -> Result<Account, Error> {
    let path = shared.cfg.state.join("acme-account.json");
    if let Ok(text) = std::fs::read_to_string(&path) {
        let creds: AccountCredentials = serde_json::from_str(&text)?;
        return Ok(Account::builder()?.from_credentials(creds).await?);
    }
    let (account, creds) = Account::builder()?
        .create(
            &NewAccount {
                contact: &[],
                terms_of_service_agreed: true,
                only_return_existing: false,
            },
            shared.cfg.acme_url.clone(),
            None,
        )
        .await?;
    write_private(&path, &serde_json::to_string(&creds)?)?;
    Ok(account)
}

async fn order(
    shared: &Shared,
    account: &Account,
    names: &[String],
    profile: Option<&str>,
    records: &mut Vec<(String, String)>,
) -> Result<(String, String), Error> {
    let ids: Vec<Identifier> = names.iter().map(|n| Identifier::Dns(n.clone())).collect();
    let mut new = NewOrder::new(&ids);
    if let Some(p) = profile {
        new = new.profile(p);
    }
    let mut order = account.new_order(&new).await?;
    let mut authzs = order.authorizations();
    while let Some(authz) = authzs.next().await {
        let mut authz = authz?;
        if authz.status == AuthorizationStatus::Valid {
            continue;
        }
        let mut challenge = authz
            .challenge(ChallengeType::Dns01)
            .ok_or("no dns-01 challenge offered")?;
        let Identifier::Dns(name) = challenge.identifier().identifier else {
            return Err("not a dns identifier".into());
        };
        let record = format!("_acme-challenge.{}", name.to_ascii_lowercase());
        let value = challenge.key_authorization().dns_value();
        shared
            .challenges
            .write()
            .unwrap()
            .entry(record.clone())
            .or_default()
            .push(value.clone());
        records.push((record, value));
        challenge.set_ready().await?;
    }
    let retry = RetryPolicy::new().timeout(Duration::from_secs(180));
    let status = order.poll_ready(&retry).await?;
    if status != OrderStatus::Ready {
        return Err(format!("order ended up {status:?}").into());
    }
    let key = order.finalize().await?;
    let chain = order.poll_certificate(&retry).await?;
    Ok((chain, key))
}

async fn issue(
    shared: &Shared,
    account: &Account,
    names: &[String],
    profile: Option<&str>,
) -> Result<(String, String), Error> {
    let mut records = Vec::new();
    let result = order(shared, account, names, profile, &mut records).await;
    let mut challenges = shared.challenges.write().unwrap();
    for (record, value) in records {
        if let Some(values) = challenges.get_mut(&record) {
            values.retain(|v| *v != value);
            if values.is_empty() {
                challenges.remove(&record);
            }
        }
    }
    result
}

struct Info {
    serial: String,
    sha256: String,
    not_before: i64,
    not_after: i64,
    scts: serde_json::Value,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn info(chain: &str) -> Result<Info, Error> {
    let (_, pem) = x509_parser::pem::parse_x509_pem(chain.as_bytes())?;
    let cert = pem.parse_x509()?;
    let mut scts = Vec::new();
    for ext in cert.extensions() {
        if let ParsedExtension::SCT(list) = ext.parsed_extension() {
            for sct in list {
                scts.push(json!({"log": hex(sct.id.key_id), "at_ms": sct.timestamp}));
            }
        }
    }
    Ok(Info {
        serial: hex(cert.raw_serial()),
        sha256: hex(&Sha256::digest(&pem.contents)),
        not_before: cert.validity().not_before.timestamp(),
        not_after: cert.validity().not_after.timestamp(),
        scts: serde_json::Value::Array(scts),
    })
}

fn tls_config(chain: &str, key: &str) -> Result<Arc<rustls::ServerConfig>, Error> {
    let certs = rustls_pemfile::certs(&mut chain.as_bytes()).collect::<Result<Vec<_>, _>>()?;
    let key = rustls_pemfile::private_key(&mut key.as_bytes())?.ok_or("no private key")?;
    let mut cfg = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS12, &rustls::version::TLS13])?
    .with_no_client_auth()
    .with_single_cert(certs, key)?;
    cfg.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Arc::new(cfg))
}

fn load_wildcard(shared: &Shared) -> Option<i64> {
    let chain = std::fs::read_to_string(shared.cfg.state.join("wildcard.pem")).ok()?;
    let key = std::fs::read_to_string(shared.cfg.state.join("wildcard.key")).ok()?;
    let not_after = info(&chain).ok()?.not_after * 1000;
    match tls_config(&chain, &key) {
        Ok(cfg) => *shared.tls.write().unwrap() = Some(cfg),
        Err(e) => {
            eprintln!("acme: wildcard on disk is unusable: {e}");
            return None;
        }
    }
    Some(not_after)
}

async fn renew_wildcard(shared: &Shared, account: &Account) -> Result<(), Error> {
    let zone = shared.cfg.zone.clone();
    let names = [zone.clone(), format!("*.{zone}")];
    let (chain, key) = issue(shared, account, &names, None).await?;
    write_private(&shared.cfg.state.join("wildcard.key"), &key)?;
    write_private(&shared.cfg.state.join("wildcard.pem"), &chain)?;
    load_wildcard(shared).ok_or("renewed wildcard did not load")?;
    eprintln!("acme: wildcard renewed");
    Ok(())
}

async fn bait(shared: &Shared, account: &Account) -> Result<(), Error> {
    let now = now_ms();
    let day = names::day_of(now / 1000);
    let cfg = &shared.cfg;
    let offset = if day == cfg.launch_day {
        0
    } else {
        cfg.key.order_offset(day)
    };
    if day < cfg.launch_day || now < day * 86_400_000 + offset * 1000 {
        return Ok(());
    }
    let path = shared.store.path.clone();
    let row: Option<(i64, Option<i64>, Option<i64>)> = tokio::task::spawn_blocking(move || {
        store::open(&path)?
            .query_row(
                "SELECT ordered_at_ms, issued_at_ms, attempted_at_ms FROM certs WHERE day = ?1",
                [day],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
    })
    .await??;
    match row {
        Some((_, Some(_), _)) => return Ok(()),
        Some((_, None, Some(at))) if now - at < 3_600_000 => return Ok(()),
        _ => {}
    }
    let name = format!("{}.{}", cfg.key.name(day), cfg.zone);
    {
        let path = shared.store.path.clone();
        let name = name.clone();
        tokio::task::spawn_blocking(move || {
            store::open(&path)?.execute(
                "INSERT INTO certs (day, name, ordered_at_ms, attempted_at_ms) VALUES (?1, ?2, ?3, ?3)
                 ON CONFLICT (day) DO UPDATE SET attempted_at_ms = ?3",
                params![day, name, now],
            )
        })
        .await??;
    }
    shared.ordered.write().unwrap().entry(day).or_insert(now);
    shared.pending.write().unwrap().insert(name.clone());
    let result = issue(
        shared,
        account,
        std::slice::from_ref(&name),
        Some("shortlived"),
    )
    .await;
    shared.pending.write().unwrap().remove(&name);
    let path = shared.store.path.clone();
    match result {
        Ok((chain, key)) => {
            drop(key);
            let i = info(&chain)?;
            let issued = now_ms();
            eprintln!("acme: bait for day {day} issued, serial {}", i.serial);
            tokio::task::spawn_blocking(move || {
                store::open(&path)?.execute(
                    "UPDATE certs SET issued_at_ms = ?2, serial = ?3, sha256 = ?4, not_before = ?5,
                     not_after = ?6, scts = ?7, pem = ?8, error = NULL WHERE day = ?1",
                    params![
                        day,
                        issued,
                        i.serial,
                        i.sha256,
                        i.not_before,
                        i.not_after,
                        i.scts.to_string(),
                        chain
                    ],
                )
            })
            .await??;
            Ok(())
        }
        Err(e) => {
            let text = e.to_string();
            tokio::task::spawn_blocking(move || {
                store::open(&path)?.execute(
                    "UPDATE certs SET error = ?2 WHERE day = ?1",
                    params![day, text],
                )
            })
            .await??;
            Err(e)
        }
    }
}

pub async fn run(shared: Arc<Shared>) {
    let mut expires = load_wildcard(&shared);
    let account = loop {
        match account(&shared).await {
            Ok(a) => break a,
            Err(e) => {
                eprintln!("acme: account: {e}");
                tokio::time::sleep(Duration::from_secs(300)).await;
            }
        }
    };
    let mut wildcard_failed_at = 0;
    loop {
        let now = now_ms();
        let due = expires.is_none_or(|at| at - now < RENEW_BEFORE_MS);
        if due && now - wildcard_failed_at > 3_600_000 {
            match renew_wildcard(&shared, &account).await {
                Ok(()) => expires = load_wildcard(&shared),
                Err(e) => {
                    eprintln!("acme: wildcard: {e}");
                    wildcard_failed_at = now;
                }
            }
        }
        if let Err(e) = bait(&shared, &account).await {
            eprintln!("acme: bait: {e}");
        }
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn order_times_spread_over_the_day() {
        let key = crate::names::Key::new([7; 32]);
        let offsets: Vec<i64> = (20000..20100).map(|d| key.order_offset(d)).collect();
        assert!(offsets.iter().all(|o| (0..20 * 3600).contains(o)));
        assert!(offsets.iter().min().unwrap() < &3600);
        assert!(offsets.iter().max().unwrap() > &(19 * 3600));
    }
}
