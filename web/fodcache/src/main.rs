use std::{
    collections::HashMap,
    path::PathBuf,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    body::Body,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, put},
};
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use futures_util::StreamExt;
use serde::Deserialize;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    sync::Semaphore,
};

const NIX32: &[u8] = b"0123456789abcdfghijklmnpqrsvwxyz";
const SYSTEMS: &[&str] = &[
    "x86_64-linux",
    "aarch64-linux",
    "x86_64-darwin",
    "aarch64-darwin",
];

struct App {
    cache: PathBuf,
    tmp: PathBuf,
    nixpkgs: String,
    max_size: u64,
    eval_timeout: Duration,
    eval_lock: Semaphore,
    upload_slots: Semaphore,
    compress_lock: Semaphore,
    counter: AtomicU64,
    files: AtomicU64,
    bytes: AtomicU64,
}

type Res = Result<(StatusCode, String), (StatusCode, String)>;

fn err(code: StatusCode, msg: impl Into<String>) -> (StatusCode, String) {
    let mut m = msg.into();
    m.push('\n');
    (code, m)
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Algo {
    Sha1,
    Sha256,
    Sha512,
}

impl Algo {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "sha1" => Some(Self::Sha1),
            "sha256" => Some(Self::Sha256),
            "sha512" => Some(Self::Sha512),
            _ => None,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Sha1 => "sha1",
            Self::Sha256 => "sha256",
            Self::Sha512 => "sha512",
        }
    }
    fn size(self) -> usize {
        match self {
            Self::Sha1 => 20,
            Self::Sha256 => 32,
            Self::Sha512 => 64,
        }
    }
}

fn nix32_len(n: usize) -> usize {
    (n * 8 - 1) / 5 + 1
}

fn nix32_encode(h: &[u8]) -> String {
    let len = nix32_len(h.len());
    let mut s = String::with_capacity(len);
    for n in (0..len).rev() {
        let b = n * 5;
        let (i, j) = (b / 8, b % 8);
        let lo = (h[i] as u16) >> j;
        let hi = if i + 1 < h.len() {
            (h[i + 1] as u16) << (8 - j)
        } else {
            0
        };
        s.push(NIX32[((lo | hi) & 0x1f) as usize] as char);
    }
    s
}

fn nix32_decode(s: &str, size: usize) -> Option<Vec<u8>> {
    let mut h = vec![0u8; size];
    for (n, c) in s.bytes().rev().enumerate() {
        let d = NIX32.iter().position(|&x| x == c)? as u16;
        let b = n * 5;
        let (i, j) = (b / 8, b % 8);
        h[i] |= (d << j) as u8;
        let carry = d >> (8 - j);
        if i + 1 < size {
            h[i + 1] |= carry as u8;
        } else if carry != 0 {
            return None;
        }
    }
    Some(h)
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

fn hex_encode(h: &[u8]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

fn parse_hash(hash: &str, algo: &str) -> Option<(Algo, Vec<u8>)> {
    if let Some((a, b64)) = hash.split_once('-')
        && let Some(a) = Algo::parse(a)
    {
        let d = B64.decode(b64).ok()?;
        return (d.len() == a.size()).then_some((a, d));
    }
    let (algo, hash) = match hash.split_once(':') {
        Some((a, h)) => (a, h),
        None => (algo, hash),
    };
    let a = Algo::parse(algo)?;
    let d = if hash.len() == a.size() * 2 {
        hex_decode(hash)?
    } else if hash.len() == nix32_len(a.size()) {
        nix32_decode(hash, a.size())?
    } else {
        B64.decode(hash).ok()?
    };
    (d.len() == a.size()).then_some((a, d))
}

#[derive(Debug)]
struct Candidate {
    name: String,
    out: String,
    algo: Algo,
    digest: Vec<u8>,
}

impl Candidate {
    fn hash_part(&self) -> &str {
        &self.out["/nix/store/".len()..][..32]
    }
}

fn valid_attr(a: &str) -> bool {
    !a.is_empty()
        && a.len() <= 200
        && a.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && a.chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-.+'".contains(c))
        && !a.contains("..")
}

fn valid_out(out: &str, name: &str) -> bool {
    out.strip_prefix("/nix/store/").is_some_and(|rest| {
        rest.len() > 33
            && rest.as_bytes()[32] == b'-'
            && rest[..32].bytes().all(|c| NIX32.contains(&c))
            && &rest[33..] == name
    })
}

async fn run(cmd: &mut Command, timeout: Duration) -> Result<String, String> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let child = cmd.spawn().map_err(|e| format!("spawn: {e}"))?;
    let out = tokio::time::timeout(timeout, child.wait_with_output())
        .await
        .map_err(|_| "evaluation timed out".to_string())?
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let tail: Vec<&str> = stderr.lines().rev().take(15).collect();
        return Err(tail.into_iter().rev().collect::<Vec<_>>().join("\n"));
    }
    String::from_utf8(out.stdout).map_err(|e| e.to_string())
}

async fn find_candidates(app: &App, attr: &str, system: &str) -> Result<Vec<Candidate>, String> {
    let expr = format!(
        r#"{{ system }}: import (builtins.fetchTarball "{}") {{
          inherit system;
          overlays = [ ];
          config = {{
            allowUnfree = true;
            allowBroken = true;
            allowUnsupportedSystem = true;
            allowInsecurePredicate = _: true;
          }};
        }}"#,
        app.nixpkgs
    );
    let out = run(
        Command::new("nix-instantiate")
            .args(["--option", "allow-import-from-derivation", "false"])
            .args(["--option", "tarball-ttl", "3600"])
            .args(["--argstr", "system", system])
            .args(["--expr", &expr])
            .args(["-A", attr]),
        app.eval_timeout,
    )
    .await?;
    let drvs: Vec<&str> = out
        .lines()
        .map(|l| l.split('!').next().unwrap_or(l))
        .filter(|l| l.starts_with("/nix/store/") && l.ends_with(".drv"))
        .collect();
    if drvs.is_empty() {
        return Err(format!("{attr} did not evaluate to a derivation"));
    }
    if drvs.len() > 16 {
        return Err(format!("{attr} evaluates to too many derivations"));
    }
    let json = run(
        Command::new("nix")
            .args(["--extra-experimental-features", "nix-command"])
            .args(["derivation", "show", "--recursive"])
            .args(&drvs),
        app.eval_timeout,
    )
    .await?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let drvmap = v.get("derivations").unwrap_or(&v);
    let Some(drvmap) = drvmap.as_object() else {
        return Err("unexpected derivation show output".into());
    };
    let mut found = Vec::new();
    for drv in drvmap.values() {
        let Some(env) = drv.get("env").and_then(|e| e.as_object()) else {
            continue;
        };
        let get = |k: &str| env.get(k).and_then(|v| v.as_str()).unwrap_or("");
        let marker = |s: &str| s.ends_with("-restrict-message");
        let args = drv.get("args").and_then(|a| a.as_array());
        if !marker(get("builder"))
            && !args.is_some_and(|a| a.iter().any(|x| x.as_str().is_some_and(marker)))
        {
            continue;
        }
        if !matches!(get("outputHashMode"), "" | "flat") {
            continue;
        }
        let (name, out) = (get("name").to_string(), get("out").to_string());
        if !valid_out(&out, &name) {
            continue;
        }
        if let Some((algo, digest)) = parse_hash(get("outputHash"), get("outputHashAlgo")) {
            found.push(Candidate {
                name,
                out,
                algo,
                digest,
            });
        }
    }
    Ok(found)
}

fn nar_str(buf: &mut Vec<u8>, s: &[u8]) {
    buf.extend_from_slice(&(s.len() as u64).to_le_bytes());
    buf.extend_from_slice(s);
    buf.resize(buf.len() + (8 - s.len() % 8) % 8, 0);
}

fn nar_prefix(size: u64) -> Vec<u8> {
    let mut b = Vec::new();
    for s in ["nix-archive-1", "(", "type", "regular", "contents"] {
        nar_str(&mut b, s.as_bytes());
    }
    b.extend_from_slice(&size.to_le_bytes());
    b
}

fn nar_suffix(size: u64) -> Vec<u8> {
    let mut b = vec![0u8; ((8 - size % 8) % 8) as usize];
    nar_str(&mut b, b")");
    b
}

#[derive(Deserialize)]
struct UploadQuery {
    attr: String,
    system: Option<String>,
}

async fn upload(
    State(app): State<Arc<App>>,
    Query(q): Query<UploadQuery>,
    headers: HeaderMap,
    body: Body,
) -> Res {
    let system = q.system.as_deref().unwrap_or("x86_64-linux");
    if !valid_attr(&q.attr) {
        return Err(err(StatusCode::BAD_REQUEST, "invalid attr"));
    }
    if !SYSTEMS.contains(&system) {
        return Err(err(StatusCode::BAD_REQUEST, "unsupported system"));
    }
    let size: u64 = headers
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok()?.parse().ok())
        .ok_or_else(|| err(StatusCode::LENGTH_REQUIRED, "Content-Length required"))?;
    if size > app.max_size {
        return Err(err(StatusCode::PAYLOAD_TOO_LARGE, "file too large"));
    }

    let _slot = app.upload_slots.try_acquire().map_err(|_| {
        err(
            StatusCode::SERVICE_UNAVAILABLE,
            "too many uploads in progress",
        )
    })?;

    let candidates = {
        let _lock = app.eval_lock.acquire().await.unwrap();
        find_candidates(&app, &q.attr, system)
            .await
            .map_err(|e| err(StatusCode::UNPROCESSABLE_ENTITY, e))?
    };
    if candidates.is_empty() {
        return Err(err(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!(
                "{} has no requireFile dependencies on nixpkgs master",
                q.attr
            ),
        ));
    }
    let mut missing = Vec::new();
    for c in &candidates {
        if !fs::try_exists(app.cache.join(format!("{}.narinfo", c.hash_part())))
            .await
            .unwrap_or(false)
        {
            missing.push(c);
        }
    }
    if missing.is_empty() {
        let paths: Vec<&str> = candidates.iter().map(|c| c.out.as_str()).collect();
        return Ok((
            StatusCode::OK,
            format!("already cached:\n{}\n", paths.join("\n")),
        ));
    }

    let id = app.counter.fetch_add(1, Ordering::Relaxed);
    let tmp = app.tmp.join(format!("{}-{id}.nar", std::process::id()));
    let result = receive(&app, &tmp, size, body, &missing).await;
    let _ = fs::remove_file(&tmp).await;
    result
}

async fn receive(
    app: &Arc<App>,
    tmp: &PathBuf,
    size: u64,
    body: Body,
    missing: &[&Candidate],
) -> Res {
    let ioerr = |e: std::io::Error| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    let mut f = fs::File::create(tmp).await.map_err(ioerr)?;
    let (mut s1, mut s256, mut s512, mut nar) =
        (Sha1::new(), Sha256::new(), Sha512::new(), Sha256::new());
    let want = |a: Algo| missing.iter().any(|c| c.algo == a);
    let (w1, w256, w512) = (want(Algo::Sha1), want(Algo::Sha256), want(Algo::Sha512));

    let prefix = nar_prefix(size);
    nar.update(&prefix);
    f.write_all(&prefix).await.map_err(ioerr)?;

    let mut got: u64 = 0;
    let mut stream = body.into_data_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| err(StatusCode::BAD_REQUEST, e.to_string()))?;
        got += chunk.len() as u64;
        if got > size {
            return Err(err(StatusCode::BAD_REQUEST, "body exceeds Content-Length"));
        }
        if w1 {
            s1.update(&chunk);
        }
        if w256 {
            s256.update(&chunk);
        }
        if w512 {
            s512.update(&chunk);
        }
        nar.update(&chunk);
        f.write_all(&chunk).await.map_err(ioerr)?;
    }
    if got != size {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "body shorter than Content-Length",
        ));
    }
    let suffix = nar_suffix(size);
    nar.update(&suffix);
    f.write_all(&suffix).await.map_err(ioerr)?;
    f.sync_all().await.map_err(ioerr)?;
    drop(f);

    let digests: HashMap<&str, Vec<u8>> = [
        ("sha1", s1.finalize().to_vec()),
        ("sha256", s256.finalize().to_vec()),
        ("sha512", s512.finalize().to_vec()),
    ]
    .into();
    let Some(c) = missing.iter().find(|c| digests[c.algo.name()] == c.digest) else {
        let expected: Vec<String> = missing
            .iter()
            .map(|c| format!("  {} {}:{}", c.name, c.algo.name(), hex_encode(&c.digest)))
            .collect();
        return Err(err(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!(
                "upload does not match any uncached requireFile; expected one of:\n{}",
                expected.join("\n")
            ),
        ));
    };

    let nar_hash = nix32_encode(&nar.finalize());
    let nar_size = size + prefix.len() as u64 + suffix.len() as u64;
    let nar_rel = format!("nar/{nar_hash}.nar");
    fs::rename(tmp, app.cache.join(&nar_rel))
        .await
        .map_err(ioerr)?;

    let narinfo = format!(
        "StorePath: {out}\nURL: {nar_rel}\nCompression: none\nFileHash: sha256:{nar_hash}\nFileSize: {nar_size}\nNarHash: sha256:{nar_hash}\nNarSize: {nar_size}\nReferences: \nCA: fixed:{algo}:{ca}\n",
        out = c.out,
        algo = c.algo.name(),
        ca = nix32_encode(&c.digest),
    );
    let info_tmp = tmp.with_extension("narinfo");
    let info_path = app.cache.join(format!("{}.narinfo", c.hash_part()));
    fs::write(&info_tmp, narinfo).await.map_err(ioerr)?;
    fs::rename(&info_tmp, &info_path).await.map_err(ioerr)?;

    let (app2, hash_part) = (app.clone(), c.hash_part().to_string());
    tokio::spawn(async move {
        if let Err(e) = compress(&app2, &hash_part, &info_path, &nar_rel, nar_size).await {
            eprintln!("compress {hash_part}: {e}");
        }
    });

    app.files.fetch_add(1, Ordering::Relaxed);
    app.bytes.fetch_add(nar_size, Ordering::Relaxed);
    eprintln!("cached {}", c.out);
    Ok((StatusCode::CREATED, format!("cached {}\n", c.out)))
}

async fn compress(
    app: &App,
    hash_part: &str,
    info_path: &PathBuf,
    nar_rel: &str,
    nar_size: u64,
) -> std::io::Result<()> {
    let _lock = app.compress_lock.acquire().await.unwrap();
    let src = app.cache.join(nar_rel);
    let tmp = app.tmp.join(format!("{hash_part}.nar.xz"));
    let out = std::fs::File::create(&tmp)?;
    let status = Command::new("nice")
        .args(["-n", "19", "xz", "-6", "-T0", "-c"])
        .arg(&src)
        .stdout(out)
        .kill_on_drop(true)
        .status()
        .await?;
    if !status.success() {
        let _ = fs::remove_file(&tmp).await;
        return Err(std::io::Error::other(format!("xz exited {status}")));
    }
    let comp_size = fs::metadata(&tmp).await?.len();
    if comp_size * 10 > nar_size * 9 {
        fs::remove_file(&tmp).await?;
        return Ok(());
    }

    let mut f = fs::File::open(&tmp).await?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    let file_hash = nix32_encode(&h.finalize());
    let comp_rel = format!("nar/{file_hash}.nar.xz");
    fs::rename(&tmp, app.cache.join(&comp_rel)).await?;

    let info = fs::read_to_string(info_path).await?;
    let info: String = info
        .lines()
        .map(|l| match l.split_once(": ").map(|(k, _)| k) {
            Some("URL") => format!("URL: {comp_rel}\n"),
            Some("Compression") => "Compression: xz\n".to_string(),
            Some("FileHash") => format!("FileHash: sha256:{file_hash}\n"),
            Some("FileSize") => format!("FileSize: {comp_size}\n"),
            _ => format!("{l}\n"),
        })
        .collect();
    let info_tmp = app.tmp.join(format!("{hash_part}.narinfo"));
    fs::write(&info_tmp, info).await?;
    fs::rename(&info_tmp, info_path).await?;
    eprintln!("compressed {hash_part}: {nar_size} -> {comp_size}");

    tokio::time::sleep(Duration::from_secs(60)).await;
    fs::remove_file(&src).await
}

fn human(b: u64) -> String {
    let units = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < units.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{b} B")
    } else {
        format!("{v:.0} {}", units[i])
    }
}

async fn index(State(app): State<Arc<App>>, headers: HeaderMap) -> Response {
    let files = app.files.load(Ordering::Relaxed);
    let size = human(app.bytes.load(Ordering::Relaxed));
    let html = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("text/html"));
    let noun = if files == 1 { "file" } else { "files" };
    if html {
        Html(
            include_str!("index.html")
                .replace("{{count}}", &files.to_string())
                .replace("{{noun}}", noun)
                .replace("{{size}}", &size),
        )
        .into_response()
    } else {
        format!(
            "fods.lol: a binary cache for nixpkgs requireFile sources\n\n\
             {files} {noun} cached, {size}\n\n\
             upload:  curl -T <file> 'https://fods.lol/upload?attr=<nixpkgs attr>&system=x86_64-linux'\n\
             use:     extra-substituters = https://fods.lol\n\n\
             the attr must depend on a requireFile on nixpkgs master whose hash matches the file.\n"
        )
        .into_response()
    }
}

async fn scan(cache: &PathBuf) -> (u64, u64) {
    let (mut files, mut bytes) = (0, 0);
    let Ok(mut rd) = fs::read_dir(cache).await else {
        return (0, 0);
    };
    while let Ok(Some(e)) = rd.next_entry().await {
        if e.path().extension().is_none_or(|x| x != "narinfo") {
            continue;
        }
        let Ok(info) = fs::read_to_string(e.path()).await else {
            continue;
        };
        files += 1;
        bytes += info
            .lines()
            .find_map(|l| l.strip_prefix("NarSize: ")?.parse::<u64>().ok())
            .unwrap_or(0);
    }
    (files, bytes)
}

#[tokio::main]
async fn main() {
    let env = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_string());
    let state = PathBuf::from(env("FODCACHE_STATE", "/var/lib/fodcache"));
    let app = Arc::new(App {
        cache: state.join("cache"),
        tmp: state.join("tmp"),
        nixpkgs: env(
            "FODCACHE_NIXPKGS",
            "https://github.com/NixOS/nixpkgs/archive/master.tar.gz",
        ),
        max_size: env("FODCACHE_MAX_SIZE", "34359738368").parse().unwrap(),
        eval_timeout: Duration::from_secs(env("FODCACHE_EVAL_TIMEOUT", "300").parse().unwrap()),
        eval_lock: Semaphore::new(1),
        upload_slots: Semaphore::new(env("FODCACHE_UPLOAD_SLOTS", "4").parse().unwrap()),
        compress_lock: Semaphore::new(1),
        counter: AtomicU64::new(0),
        files: AtomicU64::new(0),
        bytes: AtomicU64::new(0),
    });

    for d in [app.cache.join("nar"), app.tmp.clone()] {
        fs::create_dir_all(&d).await.unwrap();
    }
    let mut rd = fs::read_dir(&app.tmp).await.unwrap();
    while let Ok(Some(e)) = rd.next_entry().await {
        let _ = fs::remove_file(e.path()).await;
    }
    fs::write(
        app.cache.join("nix-cache-info"),
        "StoreDir: /nix/store\nWantMassQuery: 1\nPriority: 60\n",
    )
    .await
    .unwrap();

    let (files, bytes) = scan(&app.cache).await;
    app.files.store(files, Ordering::Relaxed);
    app.bytes.store(bytes, Ordering::Relaxed);

    let router = Router::new()
        .route("/", get(index))
        .route("/upload", put(upload))
        .layer(axum::extract::DefaultBodyLimit::disable())
        .with_state(app);

    let bind = env("FODCACHE_BIND", "127.0.0.1:15650");
    let listener = tokio::net::TcpListener::bind(&bind).await.unwrap();
    eprintln!("listening on {bind}");
    axum::serve(listener, router).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nix32_roundtrip() {
        let h = Sha256::digest(b"hello");
        let s = nix32_encode(&h);
        assert_eq!(s, "094qif9n4cq4fdg459qzbhg1c6wywawwaaivx0k0x8xhbyx4vwic");
        assert_eq!(nix32_decode(&s, 32).unwrap(), h.to_vec());
    }

    #[test]
    fn nar_matches_nix() {
        let data = b"hello";
        let mut nar = nar_prefix(5);
        nar.extend_from_slice(data);
        nar.extend_from_slice(&nar_suffix(5));
        assert_eq!(
            nix32_encode(&Sha256::digest(&nar)),
            "0sg9f58l1jj88w6pdrfdpj5x9b1zrwszk84j81zvby36q9whhhqa"
        );
    }
}
