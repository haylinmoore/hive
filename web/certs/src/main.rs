mod pages;
mod record;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use std::path::PathBuf;
use std::sync::Arc;

struct App {
    dir: PathBuf,
    zones: Vec<String>,
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

async fn index(State(app): State<Arc<App>>) -> Html<String> {
    Html(pages::index(&record::load(&app.dir), &app.zones, now()).into_string())
}

async fn zone(State(app): State<Arc<App>>, Path(zone): Path<String>) -> Response {
    if !app.zones.contains(&zone) {
        return (StatusCode::NOT_FOUND, "not a zone watched here").into_response();
    }
    Html(pages::zone(&record::load(&app.dir), &zone, now()).into_string()).into_response()
}

async fn cert(State(app): State<Arc<App>>, Path(sha): Path<String>) -> Response {
    match record::load(&app.dir)
        .into_iter()
        .find(|r| r.cert_sha256 == sha)
    {
        Some(r) => Html(pages::cert(&r, now()).into_string()).into_response(),
        None => (StatusCode::NOT_FOUND, "no such certificate").into_response(),
    }
}

fn serve() {
    let bind = env("CERTS_BIND").unwrap_or_else(|| "127.0.0.1:15700".into());
    let app = Arc::new(App {
        dir: PathBuf::from(
            env("CERTS_DIR").unwrap_or_else(|| "/var/lib/certspotter/records".into()),
        ),
        zones: env("CERTS_ZONES")
            .map(|v| v.split(',').map(str::to_string).collect())
            .unwrap_or_default(),
    });
    let router = Router::new()
        .route("/", get(index))
        .route("/zone/{zone}", get(zone))
        .route("/cert/{sha}", get(cert))
        .with_state(app);

    tokio::runtime::Runtime::new()
        .expect("tokio runtime")
        .block_on(async {
            let listener = tokio::net::TcpListener::bind(&bind)
                .await
                .unwrap_or_else(|e| panic!("cannot bind {bind}: {e}"));
            axum::serve(listener, router).await.expect("serve");
        });
}

fn record() {
    let dir =
        PathBuf::from(env("CERTS_DIR").unwrap_or_else(|| "/var/lib/certspotter/records".into()));
    if let Some(r) = record::from_env(env, now())
        && let Err(e) = record::save(&dir, &r)
    {
        eprintln!("cannot record {}: {e}", r.cert_sha256);
        std::process::exit(1);
    }
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("serve") => serve(),
        Some("record") => record(),
        _ => {
            eprintln!("usage: certs serve | certs record");
            std::process::exit(2);
        }
    }
}
