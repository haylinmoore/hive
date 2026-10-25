use crate::agent::{DeploymentView, LogQuery, NodeStatus, RevBody, Shared, err, now};
use crate::dash;
use axum::extract::{ConnectInfo, Path, Query, State as AxumState};
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Notify, RwLock};

#[derive(Debug, Clone)]
pub struct NodeView {
    pub name: String,
    pub address: Ipv6Addr,
    pub status: Option<NodeStatus>,
    pub seen: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct HubState {
    pub main: Option<String>,
    pub main_checked: Option<i64>,
    pub main_error: Option<String>,
    pub nodes: BTreeMap<String, NodeView>,
}

pub struct Hub {
    pub app: Shared,
    client: reqwest::Client,
    pub state: RwLock<HubState>,
    poke: Notify,
    wake: Notify,
}

pub type SharedHub = Arc<Hub>;

impl Hub {
    pub fn new(app: Shared) -> SharedHub {
        let nodes = app
            .cfg
            .nodes
            .iter()
            .map(|n| {
                (
                    n.name.clone(),
                    NodeView {
                        name: n.name.clone(),
                        address: n.address,
                        status: None,
                        seen: None,
                        error: None,
                    },
                )
            })
            .collect();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .no_proxy()
            .build()
            .expect("http client");
        Arc::new(Hub {
            app,
            client,
            state: RwLock::new(HubState {
                nodes,
                ..Default::default()
            }),
            poke: Notify::new(),
            wake: Notify::new(),
        })
    }

    fn agent_url(&self, address: Ipv6Addr, path: &str) -> String {
        format!("http://[{address}]:{}{path}", self.app.cfg.agent_port)
    }

    fn address_of(&self, name: &str) -> Option<Ipv6Addr> {
        self.app
            .cfg
            .nodes
            .iter()
            .find(|n| n.name == name)
            .map(|n| n.address)
    }
}

async fn ls_remote(hub: &Hub) -> Result<String, String> {
    let cfg = &hub.app.cfg;
    let out = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::process::Command::new("git")
            .args([
                "ls-remote",
                &cfg.repo_url,
                &format!("refs/heads/{}", cfg.branch),
            ])
            .output(),
    )
    .await
    .map_err(|_| "git ls-remote timed out".to_string())?
    .map_err(|e| format!("git ls-remote: {e}"))?;

    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let rev = stdout
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_lowercase();
    if crate::state::is_hex_sha(&rev) {
        Ok(rev)
    } else {
        Err(format!("unexpected ls-remote output: {stdout}"))
    }
}

async fn refresh_main(hub: &Hub) {
    let result = ls_remote(hub).await;
    let mut st = hub.state.write().await;
    st.main_checked = Some(now());
    match result {
        Ok(rev) => {
            st.main_error = None;
            if st.main.as_deref() != Some(&rev) {
                eprintln!("main is now {rev}");
                st.main = Some(rev);
                hub.wake.notify_one();
            }
        }
        Err(e) => {
            eprintln!("could not read main: {e}");
            st.main_error = Some(e);
        }
    }
}

async fn main_loop(hub: SharedHub) {
    loop {
        refresh_main(&hub).await;
        tokio::select! {
            _ = hub.poke.notified() => {}
            _ = tokio::time::sleep(hub.app.cfg.main_every) => {}
        }
    }
}

async fn poll_one(hub: SharedHub, name: String, address: Ipv6Addr, main: Option<String>) {
    let fetched = async {
        let status: NodeStatus = hub
            .client
            .get(hub.agent_url(address, "/v1/status"))
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        Ok::<_, String>(status)
    }
    .await;

    if let (Ok(status), Some(main)) = (&fetched, &main)
        && status.target.as_deref() != Some(main.as_str())
    {
        let sent = hub
            .client
            .post(hub.agent_url(address, "/v1/target"))
            .json(&serde_json::json!({ "rev": main }))
            .send()
            .await
            .and_then(|r| r.error_for_status());
        if let Err(e) = sent {
            eprintln!("could not tell {name} about {main}: {e}");
        }
    }

    let mut st = hub.state.write().await;
    if let Some(view) = st.nodes.get_mut(&name) {
        match fetched {
            Ok(status) => {
                view.status = Some(status);
                view.seen = Some(now());
                view.error = None;
            }
            Err(e) => view.error = Some(e),
        }
    }
}

async fn poll_loop(hub: SharedHub) {
    loop {
        let (main, nodes) = {
            let st = hub.state.read().await;
            let nodes: Vec<_> = st
                .nodes
                .values()
                .map(|n| (n.name.clone(), n.address))
                .collect();
            (st.main.clone(), nodes)
        };

        let mut set = tokio::task::JoinSet::new();
        for (name, address) in nodes {
            set.spawn(poll_one(hub.clone(), name, address, main.clone()));
        }
        while set.join_next().await.is_some() {}

        tokio::select! {
            _ = hub.wake.notified() => {}
            _ = tokio::time::sleep(hub.app.cfg.poll_every) => {}
        }
    }
}

pub fn spawn(hub: SharedHub) {
    tokio::spawn(main_loop(hub.clone()));
    tokio::spawn(poll_loop(hub));
}

async fn poke(AxumState(hub): AxumState<SharedHub>) -> Response {
    hub.poke.notify_one();
    (StatusCode::ACCEPTED, "checking\n").into_response()
}

async fn healthz() -> &'static str {
    "ok"
}

pub fn public_router(hub: SharedHub) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/poke", post(poke))
        .with_state(hub)
}

async fn render_path(hub: &Hub, path: &str) -> Option<String> {
    let st = hub.state.read().await.clone();
    let cfg = &hub.app.cfg;
    let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
    match parts.as_slice() {
        [""] => Some(dash::fleet(cfg, &st, now())),
        ["mesh"] => Some(dash::mesh(cfg, &st, now())),
        ["certs"] => Some(dash::certs(cfg, &st, now())),
        ["node", name] => {
            let view = st.nodes.get(*name)?;
            let deployments = history(hub, name).await;
            Some(dash::node(cfg, &st, view, &deployments, now()))
        }
        ["job", name] => Some(job_html(hub, &st, name).await),
        _ => None,
    }
}

async fn page(AxumState(hub): AxumState<SharedHub>, uri: Uri) -> Response {
    match render_path(&hub, uri.path()).await {
        Some(html) => Html(html).into_response(),
        None => (StatusCode::NOT_FOUND, "no such page").into_response(),
    }
}

#[derive(serde::Deserialize)]
struct LiveQuery {
    path: String,
}

async fn live(AxumState(hub): AxumState<SharedHub>, Query(q): Query<LiveQuery>) -> Response {
    let seed = render_path(&hub, &q.path)
        .await
        .and_then(|p| dash::live_region(&p).map(str::to_string));
    let Some(seed) = seed else {
        return (StatusCode::NOT_FOUND, "no such page").into_response();
    };

    let stream =
        futures_util::stream::unfold((hub, q.path, seed), |(hub, path, last)| async move {
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let page = render_path(&hub, &path).await?;
                let region = dash::live_region(&page)?.to_string();
                if region != last {
                    let event = Event::default().data(region.clone());
                    return Some((Ok::<_, Infallible>(event), (hub, path, region)));
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

async fn history(hub: &Hub, name: &str) -> Result<Vec<DeploymentView>, String> {
    let address = hub
        .address_of(name)
        .ok_or_else(|| format!("no node called {name}"))?;
    hub.client
        .get(hub.agent_url(address, "/v1/deployments?limit=40"))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

async fn deployment(
    AxumState(hub): AxumState<SharedHub>,
    Path((name, id)): Path<(String, u64)>,
) -> Response {
    let st = hub.state.read().await.clone();
    if !st.nodes.contains_key(&name) {
        return (StatusCode::NOT_FOUND, "no such node").into_response();
    }
    let found = history(&hub, &name)
        .await
        .ok()
        .and_then(|list| list.into_iter().find(|d| d.deployment.id == id));
    Html(dash::deployment(
        &hub.app.cfg,
        &st,
        &name,
        id,
        found.as_ref(),
        now(),
    ))
    .into_response()
}

async fn log_proxy(
    AxumState(hub): AxumState<SharedHub>,
    Path((name, id)): Path<(String, u64)>,
    Query(q): Query<LogQuery>,
) -> Response {
    let Some(address) = hub.address_of(&name) else {
        return err(404, "no such node");
    };
    let path = format!("/v1/deployments/{id}/logs?offset={}", q.offset.unwrap_or(0));
    forward_log(&hub, address, &path).await
}

async fn forward_log(hub: &Hub, address: Ipv6Addr, path: &str) -> Response {
    let resp = match hub.client.get(hub.agent_url(address, path)).send().await {
        Ok(r) => r,
        Err(e) => return err(502, &e.to_string()),
    };

    let status = resp.status().as_u16();
    let mut headers = HeaderMap::new();
    for key in ["content-type", "x-hived-next-offset", "x-hived-eof"] {
        if let Some(v) = resp.headers().get(key)
            && let Ok(v) = HeaderValue::from_bytes(v.as_bytes())
        {
            headers.insert(key, v);
        }
    }
    let body = resp.bytes().await.unwrap_or_default();
    let body = crate::job::strip_ansi(&String::from_utf8_lossy(&body));
    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),
        headers,
        body,
    )
        .into_response()
}

// Only trusted from loopback, where nginx is.
fn client_ip(peer: SocketAddr, headers: &HeaderMap) -> IpAddr {
    if peer.ip().is_loopback()
        && let Some(ip) = headers
            .get("x-real-ip")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
    {
        return ip;
    }
    peer.ip()
}

fn may_approve(hub: &Hub, peer: SocketAddr, headers: &HeaderMap) -> Result<(), &'static str> {
    let operator = match client_ip(peer, headers) {
        IpAddr::V6(ip) => hub.app.cfg.operators.contains(&ip),
        IpAddr::V4(_) => false,
    };
    if !operator {
        return Err("approvals are only taken from an operator's device");
    }

    // The address check alone would let any site open in that browser post here.
    let get = |h: header::HeaderName| headers.get(h).and_then(|v| v.to_str().ok());
    let host = get(header::HOST).ok_or("missing host")?;
    let origin = get(header::ORIGIN).ok_or("missing origin")?;
    let same = origin
        .split_once("://")
        .map(|(_, o)| o == host)
        .unwrap_or(false);
    if !same {
        return Err("that request did not come from this dashboard");
    }
    if let Some(site) = headers.get("sec-fetch-site").and_then(|v| v.to_str().ok())
        && site != "same-origin"
    {
        return Err("that request did not come from this dashboard");
    }
    Ok(())
}

async fn approve(
    AxumState(hub): AxumState<SharedHub>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path(name): Path<String>,
    headers: HeaderMap,
    Form(body): Form<RevBody>,
) -> Response {
    if let Err(why) = may_approve(&hub, peer, &headers) {
        return (StatusCode::FORBIDDEN, why).into_response();
    }
    let Some(address) = hub.address_of(&name) else {
        return (StatusCode::NOT_FOUND, "no such node").into_response();
    };

    let sent = hub
        .client
        .post(hub.agent_url(address, "/v1/approve"))
        .json(&serde_json::json!({ "rev": body.rev }))
        .send()
        .await
        .and_then(|r| r.error_for_status());
    if let Err(e) = sent {
        return (StatusCode::BAD_GATEWAY, format!("{name} refused: {e}")).into_response();
    }
    eprintln!(
        "{} approved {} on {name}",
        client_ip(peer, &headers),
        body.rev
    );
    hub.wake.notify_one();
    Redirect::to(&format!("/node/{name}")).into_response()
}

fn job_owner(st: &HubState, job: &str) -> Option<(String, Ipv6Addr)> {
    st.nodes.values().find_map(|v| {
        let s = v.status.as_ref()?;
        s.jobs
            .iter()
            .any(|j| j == job)
            .then(|| (v.name.clone(), v.address))
    })
}

async fn preview_text(hub: &Hub, address: Ipv6Addr, id: u64) -> Option<String> {
    let text = hub
        .client
        .get(hub.agent_url(address, &format!("/v1/jobs/{id}/logs")))
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .text()
        .await
        .ok()?;
    let text = crate::job::strip_ansi(&text);
    let after = text
        .split_once("=== preview ===\n")
        .map(|(_, rest)| rest)
        .unwrap_or(&text);
    Some(after.trim().to_string())
}

async fn job_html(hub: &Hub, st: &HubState, name: &str) -> String {
    let owner = job_owner(st, name);
    let runs: Vec<_> = owner
        .as_ref()
        .and_then(|(n, _)| st.nodes.get(n))
        .and_then(|v| v.status.as_ref())
        .map(|s| {
            s.job_runs
                .iter()
                .filter(|r| r.job == name)
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let pending = runs
        .iter()
        .find(|r| r.state == crate::state::JobState::Pending);
    let preview = match (pending, &owner) {
        (Some(run), Some((_, address))) => preview_text(hub, *address, run.id).await,
        _ => None,
    };
    dash::job(
        &hub.app.cfg,
        st,
        name,
        owner.as_ref().map(|(n, _)| n.as_str()),
        &runs,
        preview.as_deref(),
        now(),
    )
}

async fn job_run_page(
    AxumState(hub): AxumState<SharedHub>,
    Path((name, id)): Path<(String, u64)>,
) -> Response {
    let st = hub.state.read().await.clone();
    let run = job_owner(&st, &name)
        .and_then(|(n, _)| st.nodes.get(&n).cloned())
        .and_then(|v| v.status)
        .and_then(|s| s.job_runs.into_iter().find(|r| r.id == id && r.job == name));
    Html(dash::job_run(
        &hub.app.cfg,
        &st,
        &name,
        id,
        run.as_ref(),
        now(),
    ))
    .into_response()
}

async fn job_log(
    AxumState(hub): AxumState<SharedHub>,
    Path((name, id)): Path<(String, u64)>,
    Query(q): Query<LogQuery>,
) -> Response {
    let st = hub.state.read().await.clone();
    let Some((_, address)) = job_owner(&st, &name) else {
        return err(404, "nothing runs that job");
    };
    let path = format!("/v1/jobs/{id}/logs?offset={}", q.offset.unwrap_or(0));
    forward_log(&hub, address, &path).await
}

async fn job_action(
    hub: &Hub,
    peer: SocketAddr,
    headers: &HeaderMap,
    name: &str,
    path: &str,
) -> Response {
    if let Err(why) = may_approve(hub, peer, headers) {
        return (StatusCode::FORBIDDEN, why).into_response();
    }
    let st = hub.state.read().await.clone();
    let Some((owner, address)) = job_owner(&st, name) else {
        return (StatusCode::NOT_FOUND, "nothing runs that job").into_response();
    };
    let sent = hub.client.post(hub.agent_url(address, path)).send().await;
    match sent {
        Ok(r) if r.status().is_success() => {
            eprintln!("{} asked {owner} for {path}", client_ip(peer, headers));
            hub.wake.notify_one();
            Redirect::to(&format!("/job/{name}")).into_response()
        }
        Ok(r) => {
            let status = r.status();
            let body = r.text().await.unwrap_or_default();
            (status, format!("{owner} refused: {body}")).into_response()
        }
        Err(e) => (StatusCode::BAD_GATEWAY, format!("{owner}: {e}")).into_response(),
    }
}

async fn approve_job(
    AxumState(hub): AxumState<SharedHub>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path((name, id)): Path<(String, u64)>,
    headers: HeaderMap,
) -> Response {
    job_action(
        &hub,
        peer,
        &headers,
        &name,
        &format!("/v1/jobs/{id}/approve"),
    )
    .await
}

async fn dismiss_job(
    AxumState(hub): AxumState<SharedHub>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path((name, id)): Path<(String, u64)>,
    headers: HeaderMap,
) -> Response {
    job_action(
        &hub,
        peer,
        &headers,
        &name,
        &format!("/v1/jobs/{id}/dismiss"),
    )
    .await
}

async fn preview_job(
    AxumState(hub): AxumState<SharedHub>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> Response {
    job_action(
        &hub,
        peer,
        &headers,
        &name,
        &format!("/v1/job/{name}/preview"),
    )
    .await
}

pub fn dash_router(hub: SharedHub) -> Router {
    Router::new()
        .route("/", get(page))
        .route("/mesh", get(page))
        .route("/certs", get(page))
        .route("/live", get(live))
        .route("/node/{name}", get(page))
        .route("/node/{name}/approve", post(approve))
        .route("/node/{name}/deploy/{id}", get(deployment))
        .route("/node/{name}/deploy/{id}/log", get(log_proxy))
        .route("/job/{name}", get(page))
        .route("/job/{name}/preview", post(preview_job))
        .route("/job/{name}/{id}", get(job_run_page))
        .route("/job/{name}/{id}/log", get(job_log))
        .route("/job/{name}/{id}/approve", post(approve_job))
        .route("/job/{name}/{id}/dismiss", post(dismiss_job))
        .route("/healthz", get(healthz))
        .route(
            "/robots.txt",
            get(|| async { "User-agent: *\nDisallow: /\n" }),
        )
        .with_state(hub)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn forwarded(ip: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("x-real-ip", HeaderValue::from_str(ip).unwrap());
        h
    }

    #[test]
    fn the_forwarded_address_is_believed_from_nginx() {
        let peer: SocketAddr = "[::1]:4000".parse().unwrap();
        let ip = client_ip(peer, &forwarded("200:9b5f:5c96:46d8:3709:f4b9:c41f:e628"));
        assert_eq!(
            ip,
            "200:9b5f:5c96:46d8:3709:f4b9:c41f:e628"
                .parse::<IpAddr>()
                .unwrap()
        );
    }

    #[test]
    fn the_forwarded_address_is_ignored_from_anywhere_else() {
        let peer: SocketAddr = "[203:caa6::1]:4000".parse().unwrap();
        let ip = client_ip(peer, &forwarded("200:9b5f:5c96:46d8:3709:f4b9:c41f:e628"));
        assert_eq!(ip, peer.ip());
    }
}
