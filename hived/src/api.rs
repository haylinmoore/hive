use crate::auth::{self, Jwks};
use crate::config::Config;
use crate::health;
use crate::runner;
use crate::state::{Admission, Deployment, State, Store};
use crate::store::StateDir;
use axum::extract::{Path, Query, State as AxumState};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

pub struct App {
    pub cfg: Config,
    pub dir: StateDir,
    pub jwks: Jwks,
}

pub type Shared = Arc<App>;

/// The status page is served from GitHub Pages on a different origin, and
/// everything here is public anyway, so reads are open to any origin.
fn cors() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    h
}

fn err(status: u16, message: &str) -> Response {
    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        cors(),
        Json(json!({ "error": message })),
    )
        .into_response()
}

#[derive(Serialize)]
pub struct DeploymentView<'a> {
    #[serde(flatten)]
    pub deployment: &'a Deployment,
    pub epoch: &'a str,
    pub total: Option<f64>,
    pub log_size: u64,
}

pub fn view<'a>(dir: &StateDir, store: &'a Store, d: &'a Deployment) -> DeploymentView<'a> {
    DeploymentView {
        deployment: d,
        epoch: &store.epoch,
        total: d.total_duration(),
        log_size: dir.log_size(d.id),
    }
}

pub fn is_unit_active(cfg: &Config, id: u64) -> bool {
    Command::new("systemctl")
        .args(["is-active", "--quiet", &cfg.run_unit(id)])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn start_unit(cfg: &Config, id: u64) -> std::io::Result<()> {
    let status = Command::new("systemctl")
        .args(["start", "--no-block", &cfg.run_unit(id)])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "failed to start {}",
            cfg.run_unit(id)
        )))
    }
}

pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

#[derive(Deserialize)]
pub struct CreateBody {
    pub rev: String,
}

async fn create(
    AxumState(app): AxumState<Shared>,
    headers: HeaderMap,
    Json(body): Json<CreateBody>,
) -> Response {
    let token = match headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        Some(t) => t.trim().to_string(),
        None => return err(401, "missing bearer token"),
    };

    let rev = body.rev.trim().to_lowercase();
    if !auth::is_hex_sha(&rev) {
        return err(400, "rev must be a full 40 character sha");
    }

    let claims = match app
        .jwks
        .verify(&token, &app.cfg.claims, &app.cfg.issuer)
        .await
    {
        Ok(c) => c,
        Err(e) => return err(e.status(), e.message()),
    };

    // A token may only deploy the commit whose push minted it.
    if let Err(e) = auth::check_rev_matches(&claims, &rev) {
        return err(e.status(), e.message());
    }

    eprintln!(
        "deploy request for {rev} from {} (run {})",
        claims.sub,
        claims.run_id.as_deref().unwrap_or("?")
    );

    let admission = match app.dir.update(|s| s.admit(&rev, now())) {
        Ok(a) => a,
        Err(e) => return err(500, &format!("state write failed: {e}")),
    };

    if let Admission::Started(id) = admission
        && let Err(e) = start_unit(&app.cfg, id)
    {
        let _ = app.dir.update(|s| s.finish(id, State::Failed, now()));
        return err(500, &format!("could not start runner: {e}"));
    }

    let status = match admission {
        Admission::Existing(_) => StatusCode::OK,
        _ => StatusCode::ACCEPTED,
    };

    let store = match app.dir.load() {
        Ok(s) => s,
        Err(e) => return err(500, &format!("state read failed: {e}")),
    };
    let Some(d) = store.get(admission.id()) else {
        return err(500, "record vanished");
    };

    (
        status,
        cors(),
        Json(json!({
            "id": d.id,
            "epoch": store.epoch,
            "rev": d.rev,
            "state": d.state,
            "poll_after_ms": 2000,
        })),
    )
        .into_response()
}

#[derive(Deserialize)]
struct WaitQuery {
    #[serde(default)]
    wait: Option<u64>,
}

/// Long poll: return as soon as state or phase moves, so a caller can follow a
/// twenty minute build without hammering the box.
async fn show(
    AxumState(app): AxumState<Shared>,
    Path(id): Path<u64>,
    Query(q): Query<WaitQuery>,
) -> Response {
    let wait = q.wait.unwrap_or(0).min(60);
    let deadline = std::time::Instant::now() + Duration::from_secs(wait);

    let initial = match app.dir.load() {
        Ok(s) => s.get(id).map(|d| (d.state, d.phase)),
        Err(e) => return err(500, &format!("state read failed: {e}")),
    };
    if initial.is_none() {
        return err(404, "no such deployment");
    }

    loop {
        let store = match app.dir.load() {
            Ok(s) => s,
            Err(e) => return err(500, &format!("state read failed: {e}")),
        };
        let Some(d) = store.get(id) else {
            return err(404, "no such deployment");
        };
        let changed = Some((d.state, d.phase)) != initial;
        if changed || std::time::Instant::now() >= deadline {
            return (cors(), Json(json!(view(&app.dir, &store, d)))).into_response();
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

async fn cancel(AxumState(app): AxumState<Shared>, Path(id): Path<u64>) -> Response {
    match app.dir.update(|s| s.cancel(id, now())) {
        Ok(true) => (cors(), Json(json!({ "id": id, "state": "cancelled" }))).into_response(),
        Ok(false) => err(409, "only a queued deployment can be cancelled"),
        Err(e) => err(500, &format!("state write failed: {e}")),
    }
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(default)]
    limit: Option<usize>,
}

async fn list(AxumState(app): AxumState<Shared>, Query(q): Query<ListQuery>) -> Response {
    let store = match app.dir.load() {
        Ok(s) => s,
        Err(e) => return err(500, &format!("state read failed: {e}")),
    };
    let limit = q.limit.unwrap_or(50).min(500);
    let items: Vec<_> = store
        .deployments
        .iter()
        .take(limit)
        .map(|d| view(&app.dir, &store, d))
        .collect();
    (
        cors(),
        Json(json!({ "host": app.cfg.host, "deployments": items })),
    )
        .into_response()
}

async fn logs(
    AxumState(app): AxumState<Shared>,
    Path(id): Path<u64>,
    Query(q): Query<LogQuery>,
) -> Response {
    let offset = q.offset.unwrap_or(0);
    match app.dir.read_log(id, offset, 256 * 1024) {
        Ok((chunk, next, eof)) => {
            let mut headers = cors();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            );
            headers.insert("x-hived-next-offset", HeaderValue::from(next));
            headers.insert(
                "x-hived-eof",
                HeaderValue::from_static(if eof { "true" } else { "false" }),
            );
            (headers, chunk).into_response()
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            err(404, "no log for that deployment")
        }
        Err(e) => err(500, &format!("log read failed: {e}")),
    }
}

#[derive(Deserialize)]
struct LogQuery {
    #[serde(default)]
    offset: Option<u64>,
}

pub fn status_json(app: &App) -> serde_json::Value {
    let store = app.dir.load().unwrap_or_else(|_| Store::new(String::new()));
    let snapshot = health::snapshot().unwrap_or_default();
    let running_system = runner::current_system();

    let deployed = store.last_switched();
    // A colmena deploy from a laptop leaves the node on something no record
    // describes. Say so rather than showing a stale commit as current.
    let drifted = match (deployed.and_then(|d| d.toplevel.as_ref()), &running_system) {
        (Some(recorded), Some(actual)) => recorded != actual,
        // Nothing deployed yet is not drift, it is just an empty history.
        _ => false,
    };

    json!({
        "host": app.cfg.host,
        "epoch": store.epoch,
        "generation": deployed.and_then(|d| d.generation),
        "drifted": drifted,
        "current_system": running_system,
        "system_failed_units": health::failed_units(&snapshot).into_iter().filter(|u| !u.starts_with(&app.cfg.run_unit_prefix)).collect::<Vec<_>>(),
        "deployed": deployed.map(|d| view(&app.dir, &store, d)),
        "running": store.running().map(|d| view(&app.dir, &store, d)),
        "queued": store.queued().map(|d| view(&app.dir, &store, d)),
    })
}

async fn status(AxumState(app): AxumState<Shared>) -> Response {
    (cors(), Json(status_json(&app))).into_response()
}

async fn healthz() -> Response {
    (cors(), "ok").into_response()
}

async fn preflight() -> Response {
    let mut h = cors();
    h.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, DELETE, OPTIONS"),
    );
    h.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("authorization, content-type"),
    );
    (StatusCode::NO_CONTENT, h).into_response()
}

pub fn router(app: Shared) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/status", get(status))
        .route("/v1/deployments", get(list).post(create).options(preflight))
        .route(
            "/v1/deployments/{id}",
            get(show).delete(cancel).options(preflight),
        )
        .route("/v1/deployments/{id}/logs", get(logs))
        .merge(crate::page::routes())
        .with_state(app)
}
