use crate::config::Config;
use crate::store::Store;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

pub struct Shared {
    pub cfg: Config,
    pub store: Store,
    pub ordered: RwLock<HashMap<i64, i64>>,
    pub challenges: RwLock<HashMap<String, Vec<String>>>,
    pub pending: RwLock<HashSet<String>>,
    pub tls: RwLock<Option<Arc<rustls::ServerConfig>>>,
}

impl Shared {
    pub fn new(cfg: Config, store: Store) -> Arc<Shared> {
        let ordered = rusqlite::Connection::open(&store.path)
            .and_then(|c| {
                c.prepare("SELECT day, ordered_at_ms FROM certs")?
                    .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                    .collect::<rusqlite::Result<HashMap<i64, i64>>>()
            })
            .unwrap_or_default();
        Arc::new(Shared {
            cfg,
            store,
            ordered: RwLock::new(ordered),
            challenges: RwLock::new(HashMap::new()),
            pending: RwLock::new(HashSet::new()),
            tls: RwLock::new(None),
        })
    }

    pub fn ordered_at(&self, day: i64) -> Option<i64> {
        self.ordered.read().unwrap().get(&day).copied()
    }

    pub fn is_pending(&self, name: &str) -> bool {
        let name = name.trim_end_matches('.').to_ascii_lowercase();
        let name = name.strip_prefix("_acme-challenge.").unwrap_or(&name);
        self.pending.read().unwrap().contains(name)
    }
}

#[cfg(test)]
pub fn test() -> Arc<Shared> {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("honeypot-shared-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = crate::config::test();
    let store = Store::start(cfg.clone(), dir.join("db")).unwrap();
    Shared::new(cfg, store)
}
