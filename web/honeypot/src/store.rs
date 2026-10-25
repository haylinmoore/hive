use crate::config::{Config, Target};
use rusqlite::{Connection, params};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{SyncSender, sync_channel};

pub struct Event {
    pub at_ms: i64,
    pub transport: &'static str,
    pub proto: String,
    pub src: SocketAddr,
    pub dst: SocketAddr,
    pub name: Option<String>,
    pub summary: String,
    pub detail: serde_json::Value,
    pub raw: Option<Vec<u8>>,
    pub acme: bool,
}

impl Event {
    pub fn new(transport: &'static str, proto: &str, src: SocketAddr, dst: SocketAddr) -> Event {
        Event {
            at_ms: now_ms(),
            transport,
            proto: proto.to_string(),
            src,
            dst,
            name: None,
            summary: String::new(),
            detail: serde_json::Value::Null,
            raw: None,
            acme: false,
        }
    }
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

const SCHEMA: &str = "
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY,
    at_ms INTEGER NOT NULL,
    transport TEXT NOT NULL,
    proto TEXT NOT NULL,
    src TEXT NOT NULL,
    src_port INTEGER NOT NULL,
    dst TEXT NOT NULL,
    dst_port INTEGER NOT NULL,
    name TEXT,
    via TEXT NOT NULL,
    label TEXT,
    day INTEGER,
    kind TEXT,
    addr_via TEXT NOT NULL,
    summary TEXT NOT NULL,
    detail TEXT,
    raw BLOB,
    acme INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS events_at ON events (at_ms);
CREATE INDEX IF NOT EXISTS events_day ON events (day, kind);
CREATE INDEX IF NOT EXISTS events_src ON events (src);
CREATE TABLE IF NOT EXISTS certs (
    day INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    ordered_at_ms INTEGER NOT NULL,
    attempted_at_ms INTEGER,
    issued_at_ms INTEGER,
    serial TEXT,
    sha256 TEXT,
    not_before INTEGER,
    not_after INTEGER,
    scts TEXT,
    pem TEXT,
    error TEXT
);
";

const COLUMNS: [&str; 3] = ["ua", "path", "ja4"];

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.execute_batch(SCHEMA)?;
    let have: Vec<String> = conn
        .prepare("SELECT name FROM pragma_table_info('events')")?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    for column in COLUMNS {
        if !have.iter().any(|h| h == column) {
            conn.execute_batch(&format!("ALTER TABLE events ADD COLUMN {column} TEXT"))?;
        }
    }
    if filled(&conn)? {
        indexes(&conn)?;
    }
    Ok(conn)
}

fn filled(conn: &Connection) -> rusqlite::Result<bool> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))? >= 1)
}

fn indexes(conn: &Connection) -> rusqlite::Result<()> {
    for column in COLUMNS {
        conn.execute_batch(&format!(
            "CREATE INDEX IF NOT EXISTS events_{column} ON events (proto, {column}, src)"
        ))?;
    }
    Ok(())
}

fn backfill(conn: &Connection) -> rusqlite::Result<()> {
    if filled(conn)? {
        return Ok(());
    }
    let max: i64 = conn.query_row("SELECT COALESCE(MAX(id), 0) FROM events", [], |r| r.get(0))?;
    let started = std::time::Instant::now();
    for from in (0..=max).step_by(50_000) {
        conn.execute(
            "UPDATE events SET ua = json_extract(detail, '$.ua'), path = json_extract(detail, '$.path'), ja4 = json_extract(detail, '$.ja4')
             WHERE id > ?1 AND id <= ?2 AND detail IS NOT NULL",
            params![from, from + 50_000],
        )?;
    }
    conn.execute_batch("PRAGMA user_version = 1")?;
    indexes(conn)?;
    eprintln!(
        "store: filled ua, path and ja4 for {max} rows in {:.1}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

#[derive(Clone)]
pub struct Store {
    tx: SyncSender<Event>,
    pub path: PathBuf,
}

impl Store {
    pub fn start(cfg: Config, path: PathBuf) -> rusqlite::Result<Store> {
        let conn = open(&path)?;
        let (tx, rx) = sync_channel::<Event>(100_000);
        std::thread::spawn(move || {
            if let Err(e) = backfill(&conn) {
                eprintln!("store: backfill: {e}");
            }
            while let Ok(first) = rx.recv() {
                let batch: Vec<Event> = std::iter::once(first)
                    .chain(rx.try_iter().take(999))
                    .collect();
                if let Err(e) = write(&conn, &cfg, &batch) {
                    eprintln!("store: {e}");
                }
            }
        });
        Ok(Store { tx, path })
    }

    pub fn record(&self, event: Event) {
        let _ = self.tx.try_send(event);
    }
}

fn split(target: &Target) -> (Option<i64>, Option<&'static str>, Option<String>) {
    match target {
        Target::Daily(day) => (Some(*day), Some("bait"), None),
        Target::Other(label) => (None, None, Some(label.clone())),
        _ => (None, None, None),
    }
}

fn write(conn: &Connection, cfg: &Config, batch: &[Event]) -> rusqlite::Result<()> {
    let tx = conn.unchecked_transaction()?;
    {
        let mut insert = tx.prepare_cached(
            "INSERT INTO events (at_ms, transport, proto, src, src_port, dst, dst_port, name, via, label, day, kind, addr_via, summary, detail, raw, acme, ua, path, ja4)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
        )?;
        for e in batch {
            let today = crate::names::day_of(e.at_ms / 1000);
            let by_addr = cfg.by_address(e.dst.ip(), today);
            let by_name = e.name.as_deref().map(|n| cfg.by_name(n));
            let target = match &by_name {
                Some(t @ (Target::Daily(_) | Target::Other(_) | Target::Apex | Target::Ns(_))) => {
                    t.clone()
                }
                _ => by_addr.clone(),
            };
            let (day, kind, label) = split(&target);
            let via = match (&by_name, &target) {
                (Some(Target::Foreign(_)), _) => "foreign",
                (_, t) => t.via(),
            };
            let detail = (!e.detail.is_null()).then(|| e.detail.to_string());
            insert.execute(params![
                e.at_ms,
                e.transport,
                e.proto,
                e.src.ip().to_canonical().to_string(),
                e.src.port(),
                e.dst.ip().to_canonical().to_string(),
                e.dst.port(),
                e.name,
                via,
                label,
                day,
                kind,
                by_addr.via(),
                e.summary,
                detail,
                e.raw,
                e.acme,
                e.detail["ua"].as_str(),
                e.detail["path"].as_str(),
                e.detail["ja4"].as_str(),
            ])?;
        }
    }
    tx.commit()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ua_path_and_ja4_become_columns_old_rows_included() {
        let cfg = crate::config::test();
        let dir = std::env::temp_dir().join(format!("honeypot-cols-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let conn = open(&dir.join("db")).unwrap();
        conn.execute(
            r#"INSERT INTO events (at_ms, transport, proto, src, src_port, dst, dst_port, via, addr_via, summary, detail)
             VALUES (0, 'tcp', 'https', 'a', 1, 'b', 443, 'x', 'x', '', '{"ua":"old","path":"/x","ja4":"t13d"}')"#,
            [],
        )
        .unwrap();
        backfill(&conn).unwrap();
        let src: SocketAddr = "198.51.100.7:4444".parse().unwrap();
        let mut e = Event::new("tcp", "https", src, SocketAddr::new(cfg.apex4.into(), 443));
        e.detail = serde_json::json!({"ua": "new", "path": "/y", "ja4": "t12d"});
        write(&conn, &cfg, &[e]).unwrap();
        let rows: Vec<(String, String, String)> = conn
            .prepare("SELECT ua, path, ja4 FROM events ORDER BY id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(rows[0], ("old".into(), "/x".into(), "t13d".into()));
        assert_eq!(rows[1], ("new".into(), "/y".into(), "t12d".into()));
        assert!(filled(&conn).unwrap());
    }

    #[test]
    fn events_are_attributed_by_name_then_address() {
        let cfg = crate::config::test();
        let dir = std::env::temp_dir().join(format!("honeypot-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let conn = open(&dir.join("db")).unwrap();
        let src: SocketAddr = "198.51.100.7:4444".parse().unwrap();
        let mut by_name = Event::new("tcp", "http", src, SocketAddr::new(cfg.apex4.into(), 80));
        by_name.at_ms = 20751 * 86_400_000;
        by_name.name = Some(format!("{}.startguess.ing", cfg.key.name(20751)));
        let mut guessed = Event::new("tcp", "http", src, SocketAddr::new(cfg.apex4.into(), 80));
        guessed.name = Some("www.startguess.ing".into());
        let mut raw = Event::new(
            "tcp",
            "raw",
            src,
            SocketAddr::new(cfg.v6(20751).into(), 3306),
        );
        raw.at_ms = 20751 * 86_400_000;
        write(&conn, &cfg, &[by_name, guessed, raw]).unwrap();
        let rows: Vec<(String, Option<String>, Option<i64>, Option<String>, String)> = conn
            .prepare("SELECT via, label, day, kind, addr_via FROM events ORDER BY id")
            .unwrap()
            .query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            rows[0],
            (
                "bait".into(),
                None,
                Some(20751),
                Some("bait".into()),
                "ns".into()
            )
        );
        assert_eq!(
            rows[1],
            (
                "unknown".into(),
                Some("www".into()),
                None,
                None,
                "ns".into()
            )
        );
        assert_eq!(
            rows[2],
            (
                "bait".into(),
                None,
                Some(20751),
                Some("bait".into()),
                "bait".into()
            )
        );
    }
}
