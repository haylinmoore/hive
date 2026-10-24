use crate::agent::DeploymentView;
use crate::config::Config;
use crate::hub::{HubState, NodeView};
use crate::state::State;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

const CSS: &str = r#"
:root {
  --bg: #fafaf9; --fg: #1c1917; --muted: #78716c; --line: #e7e5e4; --panel: #f5f5f4;
  --accent: #c2410c;
  color-scheme: light;
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg: #161514; --fg: #e7e5e4; --muted: #a8a29e; --line: #2e2b29; --panel: #1f1d1b;
    --accent: #fb923c;
    color-scheme: dark;
  }
}
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--fg);
  font: 14px/1.5 ui-monospace, "Berkeley Mono", "JetBrains Mono", Menlo, monospace; }
a { color: inherit; }
header { display: flex; gap: 20px; align-items: baseline; padding: 14px 16px;
  border-bottom: 1px solid var(--line); flex-wrap: wrap; }
header .brand { font-weight: 700; text-decoration: none; }
header nav { display: flex; gap: 14px; }
header .served { margin-left: auto; color: var(--muted); }
main { padding: 16px; max-width: 1200px; }
h1 { font-size: 16px; margin: 0 0 12px; }
h2 { font-size: 14px; margin: 28px 0 8px; color: var(--muted); font-weight: 400; }
.muted { color: var(--muted); }
.need { color: var(--accent); font-weight: 700; }
.banner { background: var(--accent); color: var(--bg); padding: 10px 16px; font-weight: 700; }
.scroll { overflow-x: auto; }
table { border-collapse: collapse; width: 100%; }
th, td { text-align: left; padding: 6px 14px 6px 0; border-bottom: 1px solid var(--line);
  white-space: nowrap; vertical-align: top; }
th { color: var(--muted); font-weight: 400; }
td.wrap { white-space: normal; }
form { display: inline; }
button { font: inherit; background: none; color: var(--accent); border: 1px solid var(--accent);
  padding: 1px 10px; cursor: pointer; }
button:hover { background: var(--accent); color: var(--bg); }
pre { background: var(--panel); border: 1px solid var(--line); padding: 12px; overflow-x: auto;
  white-space: pre-wrap; word-break: break-word; margin: 0; }
dl { display: grid; grid-template-columns: max-content 1fr; gap: 4px 20px; margin: 0; }
dt { color: var(--muted); }
dd { margin: 0; }
"#;

const LOG_JS: &str = r#"
const pre = document.getElementById('log');
let offset = 0;
async function tick() {
  try {
    const r = await fetch(pre.dataset.src + '?offset=' + offset);
    if (r.ok) {
      const text = await r.text();
      const stick = innerHeight + scrollY >= document.body.scrollHeight - 40;
      if (text) pre.append(text);
      offset = Number(r.headers.get('x-hived-next-offset') || offset);
      if (stick) scrollTo(0, document.body.scrollHeight);
      if (r.headers.get('x-hived-eof') !== 'true') return setTimeout(tick, 50);
    }
  } catch (e) {}
  if (pre.dataset.live === '1') setTimeout(tick, 1000);
}
tick();
"#;

pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

pub fn ago(now: i64, then: i64) -> String {
    let d = (now - then).max(0);
    match d {
        0..60 => format!("{d}s ago"),
        60..3600 => format!("{}m ago", d / 60),
        3600..86400 => format!("{}h ago", d / 3600),
        _ => format!("{}d ago", d / 86400),
    }
}

fn duration(secs: f64) -> String {
    let s = secs.round() as i64;
    if s < 60 {
        format!("{s}s")
    } else {
        format!("{}m{:02}s", s / 60, s % 60)
    }
}

fn short(rev: &str) -> &str {
    &rev[..rev.len().min(7)]
}

fn commit(cfg: &Config, rev: &str) -> String {
    format!(
        r#"<a href="{}">{}</a>"#,
        esc(&cfg.commit_url(rev)),
        esc(short(rev))
    )
}

fn name_of(cfg: &Config, key: &str) -> String {
    match cfg.name_for_key(key) {
        Some(n) => n.to_string(),
        None => format!("? {}", &key[..key.len().min(16)]),
    }
}

/// Keys seen anywhere on the mesh that the registry does not know. Any of
/// these means something outside the hive has peered with us.
pub fn unknown_keys(cfg: &Config, st: &HubState) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for view in st.nodes.values() {
        let Some(mesh) = view.status.as_ref().and_then(|s| s.mesh.as_ref()) else {
            continue;
        };
        let keys = mesh
            .tree
            .iter()
            .map(|t| t.key.as_str())
            .chain(mesh.peers.iter().map(|p| p.key.as_str()));
        for key in keys {
            if !key.is_empty() && cfg.name_for_key(key).is_none() {
                out.insert(key.to_string());
            }
        }
    }
    out
}

fn layout(cfg: &Config, st: &HubState, title: &str, refresh: bool, body: &str) -> String {
    let unknown = unknown_keys(cfg, st);
    let banner = if unknown.is_empty() {
        String::new()
    } else {
        format!(
            r#"<div class="banner">unknown keys on the mesh: {}</div>"#,
            unknown
                .iter()
                .map(|k| esc(&k[..k.len().min(16)]))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let refresh = if refresh {
        r#"<meta http-equiv="refresh" content="5">"#
    } else {
        ""
    };
    format!(
        r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} · hive</title>{refresh}<style>{CSS}</style></head>
<body><header><a class="brand" href="/">hive</a>
<nav><a href="/">fleet</a><a href="/mesh">mesh</a></nav>
<span class="served">served by {host}</span></header>
{banner}<main>{body}</main></body></html>"#,
        title = esc(title),
        host = esc(&cfg.host),
    )
}

/// What a node is doing, as one line of the fleet table.
#[derive(Debug, PartialEq, Eq)]
pub enum Shown {
    Unreachable,
    Deploying(String),
    Ready(String),
    Failed(String, State),
    InSync,
    Behind,
    Unknown,
}

pub fn shown(view: &NodeView, main: Option<&str>) -> Shown {
    let Some(s) = &view.status else {
        return Shown::Unreachable;
    };
    if view.error.is_some() {
        return Shown::Unreachable;
    }
    if let Some(r) = &s.running {
        let phase = r.deployment.phase.map(|p| p.label()).unwrap_or("starting");
        return Shown::Deploying(phase.to_string());
    }
    let deployed = s.deployed.as_ref().map(|d| d.deployment.rev.as_str());
    if let Some(rev) = &s.available
        && deployed != Some(rev.as_str())
    {
        return Shown::Ready(rev.clone());
    }
    let Some(main) = main else {
        return Shown::Unknown;
    };
    if let Some(latest) = &s.latest
        && latest.deployment.rev == main
        && matches!(
            latest.deployment.state,
            State::Failed | State::Rejected | State::Degraded
        )
    {
        return Shown::Failed(main.to_string(), latest.deployment.state);
    }
    if deployed == Some(main) {
        Shown::InSync
    } else {
        Shown::Behind
    }
}

fn approve_button(name: &str, rev: &str, label: &str) -> String {
    format!(
        r#"<form method="post" action="/node/{name}/approve"><input type="hidden" name="rev" value="{rev}"><button>{label}</button></form>"#,
        name = esc(name),
        rev = esc(rev),
        label = esc(label),
    )
}

fn state_cell(view: &NodeView, main: Option<&str>) -> (String, String) {
    match shown(view, main) {
        Shown::Unreachable => (
            r#"<span class="need">unreachable</span>"#.into(),
            String::new(),
        ),
        Shown::Deploying(phase) => (format!("deploying, {}", esc(&phase)), String::new()),
        Shown::Ready(rev) => (
            r#"<span class="need">update ready</span>"#.into(),
            approve_button(&view.name, &rev, "deploy"),
        ),
        Shown::Failed(rev, state) => (
            format!(r#"<span class="need">{}</span>"#, state.label()),
            approve_button(&view.name, &rev, "retry"),
        ),
        Shown::InSync => ("in sync".into(), String::new()),
        Shown::Behind => ("behind".into(), String::new()),
        Shown::Unknown => (
            r#"<span class="muted">waiting for main</span>"#.into(),
            String::new(),
        ),
    }
}

fn health_cell(view: &NodeView) -> String {
    let Some(s) = &view.status else {
        return String::new();
    };
    let mut parts = Vec::new();
    if !s.failed_units.is_empty() {
        parts.push(format!(
            r#"<span class="need">{} failed</span>"#,
            s.failed_units.len()
        ));
    }
    if s.drifted {
        parts.push("drifted".into());
    }
    if parts.is_empty() {
        "ok".into()
    } else {
        parts.join(", ")
    }
}

fn peers_cell(view: &NodeView) -> String {
    let Some(mesh) = view.status.as_ref().and_then(|s| s.mesh.as_ref()) else {
        return r#"<span class="muted">no mesh</span>"#.into();
    };
    let up: BTreeSet<_> = mesh
        .peers
        .iter()
        .filter(|p| p.up)
        .map(|p| p.key.as_str())
        .collect();
    let lan: BTreeSet<_> = mesh
        .peers
        .iter()
        .filter(|p| p.up && p.lan_interface().is_some())
        .map(|p| p.key.as_str())
        .collect();
    if lan.is_empty() {
        up.len().to_string()
    } else {
        format!("{} ({} lan)", up.len(), lan.len())
    }
}

pub fn fleet(cfg: &Config, st: &HubState, now: i64) -> String {
    let mut body = String::new();
    let main = st.main.as_deref();
    match main {
        Some(rev) => {
            let _ = write!(body, "<h1>main at {}</h1>", commit(cfg, rev));
        }
        None => body.push_str(r#"<h1 class="muted">main not known yet</h1>"#),
    }
    if let Some(e) = &st.main_error {
        let _ = write!(
            body,
            r#"<p class="need">could not read main: {}</p>"#,
            esc(e)
        );
    }

    body.push_str(
        r#"<div class="scroll"><table><tr><th>node</th><th>state</th><th>running</th><th>last deploy</th><th>health</th><th>peers</th><th></th></tr>"#,
    );
    for view in st.nodes.values() {
        let (state, action) = state_cell(view, main);
        let s = view.status.as_ref();
        let running = s
            .and_then(|s| s.deployed.as_ref())
            .map(|d| commit(cfg, &d.deployment.rev))
            .unwrap_or_else(|| r#"<span class="muted">none</span>"#.into());
        let last = s
            .and_then(|s| s.latest.as_ref())
            .and_then(|d| d.deployment.finished_at.or(d.deployment.started_at))
            .map(|t| ago(now, t))
            .unwrap_or_default();
        let _ = write!(
            body,
            r#"<tr><td><a href="/node/{name}">{name}</a></td><td>{state}</td><td>{running}</td><td>{last}</td><td>{health}</td><td>{peers}</td><td>{action}</td></tr>"#,
            name = esc(&view.name),
            health = health_cell(view),
            peers = peers_cell(view),
        );
    }
    body.push_str("</table></div>");
    layout(cfg, st, "fleet", true, &body)
}

fn deployment_rows(cfg: &Config, name: &str, list: &[DeploymentView], now: i64) -> String {
    let mut out = String::from(
        r#"<div class="scroll"><table><tr><th>#</th><th>commit</th><th>state</th><th>when</th><th>took</th><th class="wrap">subject</th></tr>"#,
    );
    for d in list {
        let dep = &d.deployment;
        let state = match dep.state {
            State::Failed | State::Rejected | State::Degraded | State::Interrupted => {
                format!(r#"<span class="need">{}</span>"#, dep.state.label())
            }
            State::Running => format!(
                "running, {}",
                dep.phase.map(|p| p.label()).unwrap_or("starting")
            ),
            s => s.label().to_string(),
        };
        let _ = write!(
            out,
            r#"<tr><td><a href="/node/{name}/deploy/{id}">{id}</a></td><td>{commit}</td><td>{state}</td><td>{when}</td><td>{took}</td><td class="wrap">{subject}</td></tr>"#,
            name = esc(name),
            id = dep.id,
            commit = commit(cfg, &dep.rev),
            when = ago(now, dep.queued_at),
            took = d.total.map(duration).unwrap_or_default(),
            subject = esc(dep.subject.as_deref().unwrap_or("")),
        );
    }
    out.push_str("</table></div>");
    out
}

pub fn node(
    cfg: &Config,
    st: &HubState,
    view: &NodeView,
    deployments: &Result<Vec<DeploymentView>, String>,
    now: i64,
) -> String {
    let main = st.main.as_deref();
    let (state, action) = state_cell(view, main);
    let mut body = format!("<h1>{} {state} {action}</h1>", esc(&view.name));

    body.push_str("<dl>");
    let _ = write!(body, "<dt>address</dt><dd>{}</dd>", view.address);
    if let Some(e) = &view.error {
        let _ = write!(body, r#"<dt>error</dt><dd class="need">{}</dd>"#, esc(e));
    }
    if let Some(t) = view.seen {
        let _ = write!(body, "<dt>last seen</dt><dd>{}</dd>", ago(now, t));
    }
    if let Some(s) = &view.status {
        let _ = write!(
            body,
            "<dt>deploys</dt><dd>{}</dd>",
            if s.auto_deploy {
                "on their own"
            } else {
                "when approved"
            }
        );
        if let Some(sys) = &s.current_system {
            let _ = write!(body, "<dt>system</dt><dd>{}</dd>", esc(sys));
        }
        if !s.failed_units.is_empty() {
            let _ = write!(
                body,
                r#"<dt>failed units</dt><dd class="need">{}</dd>"#,
                esc(&s.failed_units.join(", "))
            );
        }
    }
    body.push_str("</dl>");

    if let Some(mesh) = view.status.as_ref().and_then(|s| s.mesh.as_ref()) {
        body.push_str("<h2>peers</h2>");
        body.push_str(&peer_table(cfg, mesh));
    }

    body.push_str("<h2>deployments</h2>");
    match deployments {
        Ok(list) if list.is_empty() => body.push_str(r#"<p class="muted">none yet</p>"#),
        Ok(list) => body.push_str(&deployment_rows(cfg, &view.name, list, now)),
        Err(e) => {
            let _ = write!(body, r#"<p class="need">{}</p>"#, esc(e));
        }
    }
    layout(cfg, st, &view.name, true, &body)
}

fn peer_table(cfg: &Config, mesh: &crate::ygg::Mesh) -> String {
    let mut peers = mesh.peers.clone();
    peers.sort_by_key(|p| name_of(cfg, &p.key));
    let mut out = String::from(
        r#"<div class="scroll"><table><tr><th>peer</th><th>dir</th><th>via</th><th>state</th><th>rtt</th><th>cost</th></tr>"#,
    );
    for p in &peers {
        let via = match p.lan_interface() {
            Some(i) => format!("lan {i}"),
            None => p.host().to_string(),
        };
        let state = if p.up {
            "up".to_string()
        } else {
            format!(
                r#"<span class="need">down</span> {}"#,
                esc(p.last_error.as_deref().unwrap_or(""))
            )
        };
        let _ = write!(
            out,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{state}</td><td>{}ms</td><td>{}</td></tr>",
            esc(&name_of(cfg, &p.key)),
            if p.inbound { "in" } else { "out" },
            esc(&via),
            p.latency / 1_000_000,
            p.cost,
        );
    }
    out.push_str("</table></div>");
    out
}

pub fn deployment(
    cfg: &Config,
    st: &HubState,
    name: &str,
    id: u64,
    found: Option<&DeploymentView>,
    now: i64,
) -> String {
    let mut body = format!(
        r#"<h1><a href="/node/{n}">{n}</a> deploy {id}</h1>"#,
        n = esc(name)
    );
    let mut live = false;
    if let Some(d) = found {
        let dep = &d.deployment;
        live = !dep.state.is_terminal();
        body.push_str("<dl>");
        let _ = write!(body, "<dt>commit</dt><dd>{}</dd>", commit(cfg, &dep.rev));
        if let Some(s) = &dep.subject {
            let _ = write!(body, "<dt>subject</dt><dd>{}</dd>", esc(s));
        }
        let _ = write!(body, "<dt>state</dt><dd>{}</dd>", dep.state.label());
        let _ = write!(body, "<dt>queued</dt><dd>{}</dd>", ago(now, dep.queued_at));
        if !dep.durations.is_empty() {
            let phases: Vec<_> = dep
                .durations
                .iter()
                .map(|(p, s)| format!("{} {}", p.label(), duration(*s)))
                .collect();
            let _ = write!(body, "<dt>phases</dt><dd>{}</dd>", esc(&phases.join(", ")));
        }
        if !dep.new_failed_units.is_empty() {
            let _ = write!(
                body,
                r#"<dt>broke</dt><dd class="need">{}</dd>"#,
                esc(&dep.new_failed_units.join(", "))
            );
        }
        if let Some(e) = &dep.error {
            let _ = write!(
                body,
                r#"<dt>error</dt><dd class="need">{} failed</dd>"#,
                e.phase.label()
            );
        }
        body.push_str("</dl>");
        if let Some(e) = &dep.error
            && dep.log_reaped
        {
            let _ = write!(body, "<h2>error</h2><pre>{}</pre>", esc(&e.message));
        }
    }
    let _ = write!(
        body,
        r#"<h2>log</h2><pre id="log" data-src="/node/{n}/deploy/{id}/log" data-live="{live}"></pre><script>{LOG_JS}</script>"#,
        n = esc(name),
        live = if live { "1" } else { "0" },
    );
    layout(cfg, st, &format!("{name} deploy {id}"), false, &body)
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Link {
    pub a: String,
    pub b: String,
    pub lan: bool,
}

/// Each side of a peering reports it, once per direction it was dialled,
/// so the same link shows up to four times. Folded into one per pair and
/// kind, keeping the best latency either side measured.
pub fn links(cfg: &Config, st: &HubState) -> BTreeMap<Link, u64> {
    let mut out: BTreeMap<Link, u64> = BTreeMap::new();
    for view in st.nodes.values() {
        let Some(mesh) = view.status.as_ref().and_then(|s| s.mesh.as_ref()) else {
            continue;
        };
        for p in mesh.peers.iter().filter(|p| p.up) {
            let other = name_of(cfg, &p.key);
            let (a, b) = if view.name <= other {
                (view.name.clone(), other)
            } else {
                (other, view.name.clone())
            };
            let link = Link {
                a,
                b,
                lan: p.lan_interface().is_some(),
            };
            let rtt = p.latency / 1_000_000;
            out.entry(link)
                .and_modify(|r| *r = (*r).min(rtt))
                .or_insert(rtt);
        }
    }
    out
}

pub fn mesh(cfg: &Config, st: &HubState, _now: i64) -> String {
    let mut body = String::from("<h1>mesh</h1><h2>links</h2>");
    body.push_str(
        r#"<div class="scroll"><table><tr><th>between</th><th>via</th><th>rtt</th></tr>"#,
    );
    for (link, rtt) in links(cfg, st) {
        let _ = write!(
            body,
            "<tr><td>{} and {}</td><td>{}</td><td>{rtt}ms</td></tr>",
            esc(&link.a),
            esc(&link.b),
            if link.lan { "lan" } else { "internet" },
        );
    }
    body.push_str("</table></div>");

    let tree = st
        .nodes
        .get(&cfg.host)
        .and_then(|v| v.status.as_ref())
        .and_then(|s| s.mesh.as_ref())
        .filter(|m| !m.tree.is_empty())
        .or_else(|| {
            st.nodes
                .values()
                .filter_map(|v| v.status.as_ref().and_then(|s| s.mesh.as_ref()))
                .find(|m| !m.tree.is_empty())
        });
    body.push_str("<h2>spanning tree</h2>");
    match tree {
        Some(mesh) => {
            body.push_str(
                r#"<div class="scroll"><table><tr><th>node</th><th>parent</th><th>address</th></tr>"#,
            );
            let mut entries = mesh.tree.clone();
            entries.sort_by_key(|t| name_of(cfg, &t.key));
            for t in entries {
                let parent = if t.parent == t.key {
                    r#"<span class="muted">root</span>"#.to_string()
                } else {
                    esc(&name_of(cfg, &t.parent))
                };
                let _ = write!(
                    body,
                    "<tr><td>{}</td><td>{parent}</td><td>{}</td></tr>",
                    esc(&name_of(cfg, &t.key)),
                    esc(&t.address),
                );
            }
            body.push_str("</table></div>");
        }
        None => body.push_str(r#"<p class="muted">no node has reported a tree yet</p>"#),
    }
    layout(cfg, st, "mesh", true, &body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::NodeStatus;
    use crate::state::{Deployment, Phase};

    fn dep(rev: &str, state: State) -> DeploymentView {
        let raw = serde_json::json!({
            "id": 1, "rev": rev, "state": state, "queued_at": 0,
        });
        DeploymentView {
            deployment: serde_json::from_value::<Deployment>(raw).unwrap(),
            total: None,
            log_size: 0,
        }
    }

    fn status() -> NodeStatus {
        NodeStatus {
            host: "zoe".into(),
            epoch: "e".into(),
            auto_deploy: true,
            target: None,
            available: None,
            deployed: None,
            running: None,
            queued: None,
            latest: None,
            drifted: false,
            current_system: None,
            failed_units: vec![],
            mesh: None,
        }
    }

    fn view(status: Option<NodeStatus>) -> NodeView {
        NodeView {
            name: "zoe".into(),
            address: "200::1".parse().unwrap(),
            status,
            seen: None,
            error: None,
        }
    }

    #[test]
    fn a_node_on_main_is_in_sync() {
        let mut s = status();
        s.deployed = Some(dep("aaa", State::Succeeded));
        s.latest = Some(dep("aaa", State::Succeeded));
        assert_eq!(shown(&view(Some(s)), Some("aaa")), Shown::InSync);
    }

    #[test]
    fn a_failure_on_main_offers_a_retry() {
        let mut s = status();
        s.deployed = Some(dep("old", State::Succeeded));
        s.latest = Some(dep("aaa", State::Failed));
        assert_eq!(
            shown(&view(Some(s)), Some("aaa")),
            Shown::Failed("aaa".into(), State::Failed)
        );
    }

    #[test]
    fn an_update_waiting_on_a_laptop_is_ready() {
        let mut s = status();
        s.auto_deploy = false;
        s.deployed = Some(dep("old", State::Succeeded));
        s.available = Some("aaa".into());
        assert_eq!(
            shown(&view(Some(s)), Some("aaa")),
            Shown::Ready("aaa".into())
        );
    }

    #[test]
    fn a_running_deploy_wins_over_everything() {
        let mut s = status();
        let mut running = dep("aaa", State::Running);
        running.deployment.phase = Some(Phase::Build);
        s.running = Some(running);
        s.available = Some("aaa".into());
        assert_eq!(
            shown(&view(Some(s)), Some("aaa")),
            Shown::Deploying("build".into())
        );
    }

    #[test]
    fn a_node_that_stopped_answering_is_unreachable() {
        let mut v = view(Some(status()));
        v.error = Some("timed out".into());
        assert_eq!(shown(&v, Some("aaa")), Shown::Unreachable);
    }

    #[test]
    fn escaping_covers_attributes() {
        assert_eq!(esc(r#"<a href="x">'"#), "&lt;a href=&quot;x&quot;&gt;&#39;");
    }

    #[test]
    fn ago_reads_naturally() {
        assert_eq!(ago(100, 90), "10s ago");
        assert_eq!(ago(4000, 0), "1h ago");
        assert_eq!(ago(0, 10), "0s ago");
    }
}
