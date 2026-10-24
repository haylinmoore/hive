use crate::config::Config;
use crate::health;
use crate::runner;
use crate::state::{Admission, Deployment, Offer, State, Store, is_hex_sha};
use crate::store::StateDir;
use crate::ygg::{self, Mesh};
use axum::extract::{ConnectInfo, Path, Query, State as AxumState};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::{IpAddr, SocketAddr};
use std::process::Command;
use std::sync::Arc;

pub struct App {
    pub cfg: Config,
    pub dir: StateDir,
}

pub type Shared = Arc<App>;

pub fn err(status: u16, message: &str) -> Response {
    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        Json(json!({ "error": message })),
    )
        .into_response()
}

/// Whether the runner for `id` is still alive.
///
/// `systemctl is-active` only succeeds for ActiveState=active, and a
/// Type=oneshot unit is "activating" for the whole of its ExecStart. Asking
/// is-active would call a running deploy dead a second after starting it.
pub fn is_unit_active(cfg: &Config, id: u64) -> bool {
    let out = Command::new("systemctl")
        .args(["show", "-p", "ActiveState", "--value", &cfg.run_unit(id)])
        .output();
    match out {
        Ok(o) => matches!(
            String::from_utf8_lossy(&o.stdout).trim(),
            "active" | "activating" | "reloading" | "deactivating"
        ),
        // If we cannot ask, assume alive rather than kill a live deploy.
        Err(_) => true,
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentView {
    #[serde(flatten)]
    pub deployment: Deployment,
    pub total: Option<f64>,
    pub log_size: u64,
}

fn view(dir: &StateDir, d: &Deployment) -> DeploymentView {
    DeploymentView {
        deployment: d.clone(),
        total: d.total_duration(),
        log_size: dir.log_size(d.id),
    }
}

/// Everything a hub needs to know about one node, in one request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub host: String,
    pub epoch: String,
    pub auto_deploy: bool,
    pub target: Option<String>,
    pub available: Option<String>,
    pub deployed: Option<DeploymentView>,
    pub running: Option<DeploymentView>,
    pub queued: Option<DeploymentView>,
    pub latest: Option<DeploymentView>,
    pub drifted: bool,
    pub current_system: Option<String>,
    pub failed_units: Vec<String>,
    pub mesh: Option<Mesh>,
}

pub fn status(app: &App) -> NodeStatus {
    let store = app.dir.load().unwrap_or_else(|_| Store::new(String::new()));
    let snapshot = health::snapshot().unwrap_or_default();
    let running_system = runner::current_system();
    let deployed = store.last_switched();

    // A deploy from a laptop leaves the node on something no record
    // describes. Say so rather than showing a stale commit as current.
    //
    // Not while a deploy is running, though: activate runs before check, so
    // between the two the node has legitimately moved off the last terminal
    // record and every deploy would flash "drifted" on its way through.
    let drifted = store.running().is_none()
        && match (deployed.and_then(|d| d.toplevel.as_ref()), &running_system) {
            (Some(recorded), Some(actual)) => recorded != actual,
            _ => false,
        };

    NodeStatus {
        host: app.cfg.host.clone(),
        epoch: store.epoch.clone(),
        auto_deploy: app.cfg.auto_deploy,
        target: store.target.clone(),
        available: store.available.clone(),
        deployed: deployed.map(|d| view(&app.dir, d)),
        running: store.running().map(|d| view(&app.dir, d)),
        queued: store.queued().map(|d| view(&app.dir, d)),
        latest: store.latest().map(|d| view(&app.dir, d)),
        drifted,
        current_system: running_system,
        failed_units: health::failed_units(&snapshot)
            .into_iter()
            .filter(|u| !u.starts_with(&app.cfg.run_unit_prefix))
            .collect(),
        mesh: ygg::snapshot(&app.cfg.ygg_socket).ok(),
    }
}

async fn get_status(AxumState(app): AxumState<Shared>) -> Response {
    match tokio::task::spawn_blocking(move || status(&app)).await {
        Ok(s) => Json(s).into_response(),
        Err(e) => err(500, &format!("status failed: {e}")),
    }
}

/// Mesh addresses are derived from keys, so a source address on the mesh is
/// proof of which node sent the request. Only hubs may steer an agent.
fn from_hub(app: &App, peer: SocketAddr) -> bool {
    match peer.ip() {
        IpAddr::V6(ip) => app.cfg.hubs.contains(&ip) || ip.is_loopback(),
        IpAddr::V4(ip) => ip.is_loopback(),
    }
}

#[derive(Deserialize)]
pub struct RevBody {
    pub rev: String,
}

fn admitted(app: &App, admission: &Admission) -> Result<(), String> {
    if let Admission::Started(id) = admission
        && let Err(e) = start_unit(&app.cfg, *id)
    {
        let _ = app.dir.update(|s| s.finish(*id, State::Failed, now()));
        return Err(format!("could not start runner: {e}"));
    }
    Ok(())
}

async fn target(
    AxumState(app): AxumState<Shared>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(body): Json<RevBody>,
) -> Response {
    if !from_hub(&app, peer) {
        return err(403, "only a hub may set the target");
    }
    let rev = body.rev.trim().to_lowercase();
    if !is_hex_sha(&rev) {
        return err(400, "rev must be a full 40 character sha");
    }

    let offer = match app
        .dir
        .update(|s| s.offer(&rev, app.cfg.auto_deploy, now()))
    {
        Ok(o) => o,
        Err(e) => return err(500, &format!("state write failed: {e}")),
    };
    if let Offer::Admitted(admission) = &offer {
        eprintln!("{peer} moved the target to {rev}");
        if let Err(e) = admitted(&app, admission) {
            return err(500, &e);
        }
    }
    Json(json!({ "rev": rev, "offer": format!("{offer:?}") })).into_response()
}

async fn approve(
    AxumState(app): AxumState<Shared>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(body): Json<RevBody>,
) -> Response {
    if !from_hub(&app, peer) {
        return err(403, "approvals come through a hub");
    }
    let rev = body.rev.trim().to_lowercase();
    if !is_hex_sha(&rev) {
        return err(400, "rev must be a full 40 character sha");
    }

    let admission = match app.dir.update(|s| s.approve(&rev, now())) {
        Ok(a) => a,
        Err(e) => return err(500, &format!("state write failed: {e}")),
    };
    eprintln!("{peer} approved {rev}");
    if let Err(e) = admitted(&app, &admission) {
        return err(500, &e);
    }
    Json(json!({ "rev": rev, "id": admission.id() })).into_response()
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
        .map(|d| view(&app.dir, d))
        .collect();
    Json(items).into_response()
}

#[derive(Deserialize)]
pub struct LogQuery {
    #[serde(default)]
    pub offset: Option<u64>,
}

async fn logs(
    AxumState(app): AxumState<Shared>,
    Path(id): Path<u64>,
    Query(q): Query<LogQuery>,
) -> Response {
    let offset = q.offset.unwrap_or(0);
    match app.dir.read_log(id, offset, 256 * 1024) {
        Ok((chunk, next, eof)) => {
            let mut resp = chunk.into_response();
            let h = resp.headers_mut();
            h.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            );
            h.insert("x-hived-next-offset", HeaderValue::from(next));
            h.insert(
                "x-hived-eof",
                HeaderValue::from_static(if eof { "true" } else { "false" }),
            );
            resp
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            err(404, "no log for that deployment")
        }
        Err(e) => err(500, &format!("log read failed: {e}")),
    }
}

async fn healthz() -> &'static str {
    "ok"
}

pub fn router(app: Shared) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/status", get(get_status))
        .route("/v1/target", post(target))
        .route("/v1/approve", post(approve))
        .route("/v1/deployments", get(list))
        .route("/v1/deployments/{id}/logs", get(logs))
        .with_state(app)
}
