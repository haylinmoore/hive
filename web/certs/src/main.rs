mod backfill;
mod chain;
mod pages;
mod record;
mod tree;

use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

struct App {
    dir: PathBuf,
    hive: Vec<String>,
}

#[derive(serde::Deserialize, Default)]
struct Filter {
    #[serde(default)]
    only: Option<String>,
    #[serde(default)]
    page: Option<usize>,
}

impl Filter {
    fn hive(&self) -> bool {
        self.only.as_deref() == Some("hive")
    }

    fn page(&self) -> usize {
        self.page.unwrap_or(1)
    }
}

fn records(app: &App, filter: &Filter) -> Vec<record::Record> {
    let mut all = record::load(&app.dir);
    if filter.hive() {
        all.retain(|r| app.hive.contains(&r.zone));
    }
    all
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

fn dir() -> PathBuf {
    PathBuf::from(env("CERTS_DIR").unwrap_or_else(|| "/var/lib/certspotter/records".into()))
}

async fn index(State(app): State<Arc<App>>, Query(filter): Query<Filter>) -> Html<String> {
    Html(pages::index(&records(&app, &filter), filter.hive(), filter.page(), now()).into_string())
}

async fn live(State(app): State<Arc<App>>, Query(filter): Query<Filter>) -> Response {
    let filter = Arc::new(filter);
    let render = move |app: &App, f: &Filter| {
        pages::tables(&records(app, f), f.hive(), f.page(), now()).into_string()
    };
    let seed = render(&app, &filter);
    let stream =
        futures_util::stream::unfold((app, filter, seed), move |(app, filter, last)| async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                let html = render(&app, &filter);
                if html != last {
                    let event = Event::default().data(html.clone());
                    return Some((Ok::<_, Infallible>(event), (app, filter, html)));
                }
            }
        });
    let mut resp = Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response();
    resp.headers_mut()
        .insert("x-accel-buffering", HeaderValue::from_static("no"));
    resp
}

fn chains(records: &[record::Record]) -> Vec<Vec<chain::Link>> {
    let pool = chain::pool(records.iter().filter_map(|r| r.pem.as_deref()));
    records
        .iter()
        .map(|r| {
            r.pem
                .as_deref()
                .map(|p| chain::full_chain(p, &pool))
                .unwrap_or_default()
        })
        .collect()
}

async fn cert(State(app): State<Arc<App>>, Path(sha): Path<String>) -> Response {
    let records = record::load(&app.dir);
    let chains = chains(&records);
    match records
        .iter()
        .zip(&chains)
        .find(|(r, _)| r.cert_sha256 == sha)
    {
        Some((r, links)) => Html(pages::cert(r, links, now()).into_string()).into_response(),
        None => (StatusCode::NOT_FOUND, "no such certificate").into_response(),
    }
}

async fn ca(State(app): State<Arc<App>>, Path(sha): Path<String>) -> Response {
    let records = record::load(&app.dir);
    let chains: Vec<(&record::Record, Vec<chain::Link>)> =
        records.iter().zip(chains(&records)).collect();
    let found = chains.iter().find_map(|(_, links)| {
        let at = links.iter().skip(1).position(|l| l.sha256 == sha)? + 1;
        Some((links[at].clone(), links[at + 1..].to_vec()))
    });
    let Some((link, above)) = found else {
        return (StatusCode::NOT_FOUND, "no such issuer").into_response();
    };
    let issued: Vec<&record::Record> = chains
        .iter()
        .filter(|(_, links)| links.get(1).is_some_and(|l| l.sha256 == sha))
        .map(|(r, _)| *r)
        .collect();
    Html(pages::ca(&link, &above, &issued, now()).into_string()).into_response()
}

#[derive(serde::Deserialize, Default)]
struct TreeQuery {
    #[serde(default)]
    live: Option<String>,
}

async fn tree_page(State(app): State<Arc<App>>, Query(q): Query<TreeQuery>) -> Html<String> {
    let live_only = q.live.is_some();
    let mut records = record::load(&app.dir);
    if live_only {
        let now = now();
        records.retain(|r| r.active(now));
    }
    Html(pages::tree(&tree::build(&records), live_only).into_string())
}

async fn domain(State(app): State<Arc<App>>, Path(name): Path<String>) -> Response {
    let name = name.to_lowercase();
    let records = record::load(&app.dir);
    let root = tree::build(&records);
    let node = tree::find(&root, &name);
    if node.is_none() {
        return (StatusCode::NOT_FOUND, "no certificates name that").into_response();
    }
    let mine: Vec<record::Record> = records
        .into_iter()
        .filter(|r| r.dns_names.iter().any(|n| tree::under(n, &name)))
        .collect();
    Html(pages::domain(&name, node, &mine, now()).into_string()).into_response()
}

fn serve() {
    let bind = env("CERTS_BIND").unwrap_or_else(|| "127.0.0.1:15700".into());
    let app = Arc::new(App {
        dir: dir(),
        hive: env("CERTS_HIVE")
            .map(|v| v.split(',').map(str::to_string).collect())
            .unwrap_or_default(),
    });
    let router = Router::new()
        .route("/", get(index))
        .route("/live", get(live))
        .route("/cert/{sha}", get(cert))
        .route("/ca/{sha}", get(ca))
        .route("/tree", get(tree_page))
        .route("/d/{name}", get(domain))
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
    if let Some(r) = record::from_env(env, now())
        && let Err(e) = record::save(&dir(), &r)
    {
        eprintln!("cannot record {}: {e}", r.cert_sha256);
        std::process::exit(1);
    }
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("serve") => serve(),
        Some("record") => record(),
        Some("backfill") => {
            let zones: Vec<String> = env("CERTS_ZONES")
                .map(|v| v.split(',').map(str::to_string).collect())
                .unwrap_or_default();
            let days: i64 = env("CERTS_BACKFILL_DAYS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(90);
            backfill::run(
                &dir(),
                &zones,
                now() - days * 86400,
                env("CERTS_SSLMATE_KEY"),
            );
        }
        _ => {
            eprintln!("usage: certs serve | certs record | certs backfill");
            std::process::exit(2);
        }
    }
}
