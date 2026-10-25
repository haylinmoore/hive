use crate::agent::DeploymentView;
use crate::config::Config;
use crate::hub::{HubState, NodeView};
use crate::state::{JobPhase, JobRun, JobState, Mode, State};
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
.graph { display: block; width: 100%; max-width: 760px; height: auto; margin: 4px 0 8px; }
.graph text { fill: var(--fg); font: 13px ui-monospace, "Berkeley Mono", Menlo, monospace;
  text-anchor: middle; }
.graph .small { fill: var(--muted); font-size: 11px; }
.graph .lbl { paint-order: stroke; stroke: var(--bg); stroke-width: 5px; stroke-linejoin: round; }
.graph .hull { fill: var(--panel); stroke: var(--line); }
.graph .pill { fill: var(--bg); stroke: var(--line); }
.graph .edge { stroke-width: 2.5; }
.graph .edge.lan { stroke: var(--fg); }
.graph .edge.net { stroke: var(--muted); stroke-dasharray: 7 6; }
.graph .node { fill: var(--bg); stroke: var(--fg); stroke-width: 2; }
.graph .node.off { stroke: var(--muted); stroke-dasharray: 3 3; }
.graph .node.unknown { stroke: var(--accent); stroke-width: 3; }
.graph .ring { fill: none; stroke: var(--muted); stroke-width: 1.5; }
.graph .hit { fill: transparent; }
.legend .key { display: inline-block; width: 26px; height: 0; vertical-align: middle;
  margin: 0 6px 0 14px; border-top: 2.5px solid var(--fg); }
.legend .key:first-child { margin-left: 0; }
.legend .key.net { border-top: 2.5px dashed var(--muted); }
.legend .key.ring { width: 12px; height: 12px; border: 1.5px solid var(--muted); border-radius: 50%; }
dl { display: grid; grid-template-columns: max-content 1fr; gap: 4px 20px; margin: 0; }
dt { color: var(--muted); }
dd { margin: 0; }
"#;

pub const LIVE_END: &str = "<!--live-->";

const LIVE_JS: &str = r#"
const live = document.getElementById('live');
let held = null;
const selecting = () => {
  const s = getSelection();
  return s && !s.isCollapsed && live.contains(s.anchorNode);
};
const apply = (html) => {
  if (selecting()) { held = html; return; }
  held = null;
  live.innerHTML = html;
};
document.addEventListener('selectionchange', () => {
  if (held !== null && !selecting()) apply(held);
});
new EventSource('/live?path=' + encodeURIComponent(location.pathname)).onmessage = (e) => apply(e.data);
"#;

pub fn live_region(page: &str) -> Option<&str> {
    let start = page.find(r#"<div id="live">"#)? + r#"<div id="live">"#.len();
    let end = page.rfind(LIVE_END)?.checked_sub("</div>".len())?;
    page.get(start..end)
}

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

fn layout(cfg: &Config, st: &HubState, title: &str, live: bool, body: &str) -> String {
    let tabs: String = job_tabs(st)
        .into_iter()
        .map(|(job, waiting)| {
            if waiting > 0 {
                format!(
                    r#"<a href="/job/{j}">{j} <span class="need">{waiting}</span></a>"#,
                    j = esc(&job)
                )
            } else {
                format!(r#"<a href="/job/{j}">{j}</a>"#, j = esc(&job))
            }
        })
        .collect();
    let troubled = cert_rows(st)
        .iter()
        .filter(|(_, c)| cert_trouble(c, crate::agent::now()))
        .count();
    let certs_tab = if troubled > 0 {
        format!(r#"<a href="/certs">certs <span class="need">{troubled}</span></a>"#)
    } else {
        r#"<a href="/certs">certs</a>"#.to_string()
    };
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
    let script = if live {
        format!("<script>{LIVE_JS}</script>")
    } else {
        String::new()
    };
    format!(
        r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} · hive</title><style>{CSS}</style></head>
<body><div id="live"><header><a class="brand" href="/">hive</a>
<nav><a href="/">fleet</a>{tabs}{certs_tab}<a href="/mesh">mesh</a></nav>
<span class="served">served by {host}</span></header>
{banner}<main>{body}</main></div>{LIVE_END}{script}</body></html>"#,
        title = esc(title),
        host = esc(&cfg.host),
    )
}

#[derive(Debug, PartialEq, Eq)]
pub enum Shown {
    Unreachable,
    Deploying(String),
    Ready(String, bool),
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
        let what = if r.deployment.mode == Mode::Build {
            "building ahead"
        } else {
            "deploying"
        };
        return Shown::Deploying(format!("{what}, {phase}"));
    }
    if let (Some(main), Some(latest)) = (main, &s.latest)
        && latest.deployment.rev == main
        && matches!(
            latest.deployment.state,
            State::Failed | State::Rejected | State::Degraded
        )
    {
        return Shown::Failed(main.to_string(), latest.deployment.state);
    }
    let deployed = s.deployed.as_ref().map(|d| d.deployment.rev.as_str());
    if let Some(rev) = &s.available
        && deployed != Some(rev.as_str())
    {
        let built = s
            .latest
            .as_ref()
            .is_some_and(|d| &d.deployment.rev == rev && d.deployment.state == State::Built);
        return Shown::Ready(rev.clone(), built);
    }
    let Some(main) = main else {
        return Shown::Unknown;
    };
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
        Shown::Deploying(what) => (esc(&what), String::new()),
        Shown::Ready(rev, built) => (
            format!(
                r#"<span class="need">{}</span>"#,
                if built {
                    "ready to switch"
                } else {
                    "update ready"
                }
            ),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub lan: bool,
    pub cost: u64,
    pub rtt: u64,
}

type Pair = (String, String);

fn pair(a: &str, b: &str) -> Pair {
    if a <= b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

fn peerings(cfg: &Config, st: &HubState) -> Vec<(Pair, Edge)> {
    let mut out = Vec::new();
    for view in st.nodes.values() {
        let Some(mesh) = view.status.as_ref().and_then(|s| s.mesh.as_ref()) else {
            continue;
        };
        for p in mesh.peers.iter().filter(|p| p.up) {
            out.push((
                pair(&view.name, &name_of(cfg, &p.key)),
                Edge {
                    lan: p.lan_interface().is_some(),
                    cost: p.cost,
                    rtt: p.latency / 1_000_000,
                },
            ));
        }
    }
    out
}

pub fn best_links(cfg: &Config, st: &HubState) -> BTreeMap<Pair, Edge> {
    let peerings = peerings(cfg, st);
    let mut out: BTreeMap<Pair, Edge> = BTreeMap::new();
    for (pair, edge) in &peerings {
        let better = match out.get(pair) {
            Some(old) => (edge.cost, edge.rtt) < (old.cost, old.rtt),
            None => true,
        };
        if better {
            out.insert(pair.clone(), edge.clone());
        }
    }
    for (pair, edge) in &peerings {
        if edge.lan
            && let Some(best) = out.get_mut(pair)
        {
            best.lan = true;
        }
    }
    out
}

pub fn lan_groups(cfg: &Config, st: &HubState, names: &[String]) -> Vec<Vec<String>> {
    let mut group: BTreeMap<String, usize> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.clone(), i))
        .collect();
    for ((a, b), edge) in peerings(cfg, st) {
        if !edge.lan {
            continue;
        }
        let (Some(&ga), Some(&gb)) = (group.get(&a), group.get(&b)) else {
            continue;
        };
        if ga != gb {
            for g in group.values_mut() {
                if *g == gb {
                    *g = ga;
                }
            }
        }
    }
    let mut out: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for name in names {
        out.entry(group[name]).or_default().push(name.clone());
    }
    let mut groups: Vec<_> = out.into_values().collect();
    groups.sort_by_key(|g| std::cmp::Reverse(g.len()));
    groups
}

fn graph(cfg: &Config, st: &HubState) -> String {
    let links = best_links(cfg, st);
    let mut names: Vec<String> = st.nodes.keys().cloned().collect();
    for (a, b) in links.keys() {
        for n in [a, b] {
            if !names.contains(n) {
                names.push(n.clone());
            }
        }
    }
    let groups = lan_groups(cfg, st, &names);

    use std::f64::consts::{FRAC_PI_2, PI, TAU};
    let spread = if groups.len() > 1 { 190.0 } else { 0.0 };

    let mut pos: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    let mut hulls = String::new();
    let mut bounds = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    let mut grow = |x: f64, y: f64, r: f64| {
        bounds.0 = bounds.0.min(x - r);
        bounds.1 = bounds.1.min(y - r);
        bounds.2 = bounds.2.max(x + r);
        bounds.3 = bounds.3.max(y + r);
    };
    for (gi, members) in groups.iter().enumerate() {
        let angle = PI + TAU * gi as f64 / groups.len() as f64;
        let (gx, gy) = (spread * angle.cos(), spread * 0.7 * angle.sin());
        let radius = if members.len() > 1 {
            38.0 + 14.0 * members.len() as f64
        } else {
            0.0
        };
        let start = if groups.len() > 1 {
            angle + PI / members.len() as f64
        } else {
            -FRAC_PI_2
        };
        for (i, name) in members.iter().enumerate() {
            let a = start + TAU * i as f64 / members.len() as f64;
            let (x, y) = (gx + radius * a.cos(), gy + radius * a.sin());
            pos.insert(name.clone(), (x, y));
            grow(x, y, 50.0);
        }
        if members.len() > 1 {
            let r = radius + 46.0;
            grow(gx, gy, r);
            let _ = write!(
                hulls,
                r#"<circle class="hull" cx="{gx:.0}" cy="{gy:.0}" r="{r:.0}"/><text class="small" x="{gx:.0}" y="{:.0}">same lan</text>"#,
                gy - r + 16.0,
            );
        }
    }
    let pad = 16.0;
    let (x0, y0) = (bounds.0 - pad, bounds.1 - pad);
    let (w, h) = (
        bounds.2 - bounds.0 + 2.0 * pad,
        bounds.3 - bounds.1 + 2.0 * pad,
    );

    let mut edges = String::new();
    let mut labels = String::new();
    for ((a, b), edge) in &links {
        let (Some(&(x1, y1)), Some(&(x2, y2))) = (pos.get(a), pos.get(b)) else {
            continue;
        };
        let _ = write!(
            edges,
            r#"<line class="edge {}" x1="{x1:.0}" y1="{y1:.0}" x2="{x2:.0}" y2="{y2:.0}"/>"#,
            if edge.lan { "lan" } else { "net" },
        );
        let text = format!("{} · {}ms", edge.cost, edge.rtt);
        let (mx, my) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
        let width = text.chars().count() as f64 * 6.7 + 12.0;
        let _ = write!(
            labels,
            r#"<rect class="pill" x="{:.0}" y="{:.0}" width="{width:.0}" height="17" rx="8.5"/><text class="small" x="{mx:.0}" y="{:.0}">{text}</text>"#,
            mx - width / 2.0,
            my - 9.0,
            my + 4.0,
        );
    }

    let mut nodes = String::new();
    for name in &names {
        let (x, y) = pos[name];
        let view = st.nodes.get(name);
        let hub = view.map(|v| cfg.hubs.contains(&v.address)).unwrap_or(false);
        let class = match view {
            None => "node unknown",
            Some(v) if v.status.is_none() || v.error.is_some() => "node off",
            Some(_) => "node",
        };
        if hub {
            let _ = write!(
                nodes,
                r#"<circle class="ring" cx="{x:.0}" cy="{y:.0}" r="27"/>"#
            );
        }
        let _ = write!(
            nodes,
            r#"<circle class="{class}" cx="{x:.0}" cy="{y:.0}" r="21"/><text class="lbl" x="{x:.0}" y="{:.0}">{}</text>"#,
            y + 44.0,
            esc(name),
        );
        if view.is_some() {
            let _ = write!(
                nodes,
                r#"<a href="/node/{n}"><circle class="hit" cx="{x:.0}" cy="{y:.0}" r="27"/></a>"#,
                n = esc(name),
            );
        }
    }

    format!(
        r#"<svg class="graph" viewBox="{x0:.0} {y0:.0} {w:.0} {h:.0}" role="img" aria-label="mesh links">{hulls}{edges}{labels}{nodes}</svg>
<p class="muted legend"><span class="key lan"></span>same lan, found by multicast <span class="key net"></span>over the internet, through a hub <span class="key ring"></span>hub. Each line is the cheapest link between two nodes, labelled with its cost and round trip.</p>"#
    )
}

pub fn mesh(cfg: &Config, st: &HubState, _now: i64) -> String {
    let mut body = String::from("<h1>mesh</h1>");
    body.push_str(&graph(cfg, st));

    body.push_str("<h2>links</h2>");
    body.push_str(
        r#"<div class="scroll"><table><tr><th>between</th><th>via</th><th>cost</th><th>rtt</th></tr>"#,
    );
    for ((a, b), edge) in best_links(cfg, st) {
        let _ = write!(
            body,
            "<tr><td>{} and {}</td><td>{}</td><td>{}</td><td>{}ms</td></tr>",
            esc(&a),
            esc(&b),
            if edge.lan { "lan" } else { "internet" },
            edge.cost,
            edge.rtt,
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

pub fn job_tabs(st: &HubState) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for s in st.nodes.values().filter_map(|v| v.status.as_ref()) {
        for job in &s.jobs {
            let waiting = s
                .job_runs
                .iter()
                .filter(|r| &r.job == job && r.state == JobState::Pending)
                .count();
            *out.entry(job.clone()).or_insert(0) += waiting;
        }
    }
    out
}

fn job_state_cell(run: &JobRun) -> String {
    let label = match (run.phase, run.state) {
        (JobPhase::Push, JobState::Queued) => "push queued",
        (JobPhase::Push, JobState::Running) => "pushing",
        (JobPhase::Preview, JobState::Running) => "previewing",
        (_, s) => s.label(),
    };
    match run.state {
        JobState::Pending | JobState::Failed => {
            format!(r#"<span class="need">{label}</span>"#)
        }
        _ => label.to_string(),
    }
}

fn job_button(job: &str, action: &str, label: &str) -> String {
    format!(
        r#"<form method="post" action="/job/{j}/{action}"><button>{label}</button></form>"#,
        j = esc(job),
        label = esc(label),
    )
}

pub fn job(
    cfg: &Config,
    st: &HubState,
    name: &str,
    owner: Option<&str>,
    runs: &[JobRun],
    preview: Option<&str>,
    now: i64,
) -> String {
    let mut body = format!("<h1>{}", esc(name));
    match owner {
        Some(o) => {
            let _ = write!(
                body,
                r#" <span class="muted">runs on <a href="/node/{o}">{o}</a></span> {}"#,
                job_button(name, "preview", "preview main"),
                o = esc(o),
            );
        }
        None => body.push_str(r#" <span class="need">no node reports running this</span>"#),
    }
    body.push_str("</h1>");

    if let Some(run) = runs.iter().find(|r| r.state == JobState::Pending) {
        let _ = write!(
            body,
            r#"<h2>waiting for approval</h2><p>{} would make {} at {}, previewed {}. {} {}</p>"#,
            esc(name),
            match run.changes {
                Some(1) => "1 change".to_string(),
                Some(n) => format!("{n} changes"),
                None => "changes".to_string(),
            },
            commit(cfg, &run.rev),
            run.finished_at.map(|t| ago(now, t)).unwrap_or_default(),
            job_button(name, &format!("{}/approve", run.id), "approve and push"),
            job_button(name, &format!("{}/dismiss", run.id), "dismiss"),
        );
        let _ = write!(body, "<pre>{}</pre>", esc(preview.unwrap_or("")));
    }

    body.push_str("<h2>history</h2>");
    if runs.is_empty() {
        body.push_str(r#"<p class="muted">nothing yet. Runs start when main moves, or with preview main.</p>"#);
    } else {
        body.push_str(
            r#"<div class="scroll"><table><tr><th>#</th><th>commit</th><th>state</th><th>changes</th><th>when</th><th class="wrap">error</th></tr>"#,
        );
        for run in runs {
            let _ = write!(
                body,
                r#"<tr><td><a href="/job/{j}/{id}">{id}</a></td><td>{commit}</td><td>{state}</td><td>{changes}</td><td>{when}</td><td class="wrap">{error}</td></tr>"#,
                j = esc(name),
                id = run.id,
                commit = commit(cfg, &run.rev),
                state = job_state_cell(run),
                changes = run.changes.map(|n| n.to_string()).unwrap_or_default(),
                when = ago(
                    now,
                    run.finished_at.or(run.started_at).unwrap_or(run.queued_at)
                ),
                error = esc(run.error.as_deref().unwrap_or("")),
            );
        }
        body.push_str("</table></div>");
    }
    layout(cfg, st, name, true, &body)
}

pub fn job_run(
    cfg: &Config,
    st: &HubState,
    name: &str,
    id: u64,
    run: Option<&JobRun>,
    now: i64,
) -> String {
    let mut body = format!(
        r#"<h1><a href="/job/{j}">{j}</a> run {id}</h1>"#,
        j = esc(name)
    );
    let mut live = false;
    if let Some(run) = run {
        live = run.state.is_active();
        body.push_str("<dl>");
        let _ = write!(body, "<dt>commit</dt><dd>{}</dd>", commit(cfg, &run.rev));
        let _ = write!(body, "<dt>state</dt><dd>{}</dd>", job_state_cell(run));
        if let Some(n) = run.changes {
            let _ = write!(body, "<dt>changes</dt><dd>{n}</dd>");
        }
        let _ = write!(body, "<dt>queued</dt><dd>{}</dd>", ago(now, run.queued_at));
        if let Some(e) = &run.error {
            let _ = write!(body, r#"<dt>error</dt><dd class="need">{}</dd>"#, esc(e));
        }
        body.push_str("</dl>");
    }
    let _ = write!(
        body,
        r#"<h2>log</h2><pre id="log" data-src="/job/{j}/{id}/log" data-live="{live}"></pre><script>{LOG_JS}</script>"#,
        j = esc(name),
        live = if live { "1" } else { "0" },
    );
    layout(cfg, st, &format!("{name} run {id}"), false, &body)
}

pub fn until(now: i64, then: i64) -> String {
    let d = then - now;
    match d {
        ..=0 => "now".to_string(),
        1..60 => format!("in {d}s"),
        60..3600 => format!("in {}m", d / 60),
        3600..86400 => format!("in {}h", d / 3600),
        _ => format!("in {}d", d / 86400),
    }
}

fn date(t: i64) -> String {
    chrono::DateTime::from_timestamp(t, 0)
        .map(|d| d.format("%b %-d %Y").to_string())
        .unwrap_or_default()
}

pub fn crt_sh(e: &crate::certs::CertEvent) -> String {
    match &e.serial {
        Some(serial) => format!("https://crt.sh/?serial={serial}"),
        None => format!("https://crt.sh/?sha256={}", e.detail.replace(':', "")),
    }
}

const RENEW_GRACE: i64 = 26 * 3600;

fn overdue(c: &crate::certs::Cert, now: i64) -> bool {
    c.renew_from.is_some_and(|t| now > t + RENEW_GRACE) || c.not_after.is_some_and(|t| now >= t)
}

fn cert_rows(st: &HubState) -> Vec<(String, crate::certs::Cert)> {
    let mut rows: Vec<_> = st
        .nodes
        .values()
        .filter_map(|v| Some((v.name.clone(), v.status.as_ref()?.certs.as_ref()?)))
        .flat_map(|(node, file)| file.certs.iter().map(move |c| (node.clone(), c.clone())))
        .collect();
    rows.sort_by_key(|(node, c)| (c.not_after.unwrap_or(i64::MIN), node.clone()));
    rows
}

fn cert_trouble(c: &crate::certs::Cert, now: i64) -> bool {
    !c.last_ok || c.not_after.is_none() || overdue(c, now)
}

pub fn certs(cfg: &Config, st: &HubState, now: i64) -> String {
    let rows = cert_rows(st);
    let mut body = String::from("<h1>certs</h1>");
    if rows.is_empty() {
        body.push_str(r#"<p class="muted">no node has reported its certificates yet</p>"#);
        return layout(cfg, st, "certs", true, &body);
    }

    body.push_str(
        r#"<div class="scroll"><table><tr><th>cert</th><th>node</th><th>issuer</th><th>expires</th><th>renews from</th><th>next check</th><th>last check</th><th class="wrap">names</th></tr>"#,
    );
    for (node, c) in &rows {
        let expires = match c.not_after {
            Some(t) if overdue(c, now) => {
                format!(
                    r#"<span class="need">{} ({})</span>"#,
                    until(now, t),
                    date(t)
                )
            }
            Some(t) => format!("{} ({})", until(now, t), date(t)),
            None => r#"<span class="need">no cert</span>"#.to_string(),
        };
        let last = match (c.last_check, c.last_ok) {
            (Some(t), true) => ago(now, t),
            (Some(t), false) => format!(r#"<span class="need">failed</span> {}"#, ago(now, t)),
            (None, true) => r#"<span class="muted">not yet</span>"#.to_string(),
            (None, false) => r#"<span class="need">failed</span>"#.to_string(),
        };
        let _ = write!(
            body,
            r#"<tr><td>{name}</td><td><a href="/node/{node}">{node}</a></td><td>{issuer}</td><td>{expires}</td><td>{renew}</td><td>{next}</td><td>{last}</td><td class="wrap">{domains}</td></tr>"#,
            name = esc(&c.name),
            node = esc(node),
            issuer = esc(c.issuer.as_deref().unwrap_or("")),
            renew = match c.renew_from {
                Some(_) if overdue(c, now) => r#"<span class="need">overdue</span>"#.to_string(),
                Some(t) => until(now, t),
                None => String::new(),
            },
            next = c.next_check.map(|t| until(now, t)).unwrap_or_default(),
            domains = esc(&c.domains.join(" ")),
        );
    }
    body.push_str("</table></div>");

    let mut events: Vec<_> = st
        .nodes
        .values()
        .filter_map(|v| Some((v.name.clone(), v.status.as_ref()?.certs.as_ref()?)))
        .flat_map(|(node, file)| file.history.iter().map(move |e| (node.clone(), e.clone())))
        .collect();
    events.sort_by_key(|(_, e)| std::cmp::Reverse(e.at));
    events.truncate(80);

    body.push_str("<h2>history</h2>");
    body.push_str(
        r#"<div class="scroll"><table><tr><th>when</th><th>cert</th><th>node</th><th>event</th><th class="wrap">fingerprint</th></tr>"#,
    );
    for (node, e) in &events {
        let (event, detail) = match e.kind {
            crate::certs::EventKind::Issued => (
                "issued".to_string(),
                format!(
                    r#"<a href="{}">{}</a>"#,
                    esc(&crt_sh(e)),
                    esc(&e.detail.chars().take(23).collect::<String>()),
                ),
            ),
            crate::certs::EventKind::Failed => (
                r#"<span class="need">renewal failed</span>"#.to_string(),
                String::new(),
            ),
        };
        let _ = write!(
            body,
            r#"<tr><td>{when} <span class="muted">{date}</span></td><td>{name}</td><td>{node}</td><td>{event}</td><td class="wrap muted">{detail}</td></tr>"#,
            when = ago(now, e.at),
            date = date(e.at),
            name = esc(&e.name),
            node = esc(node),
            detail = detail,
        );
    }
    body.push_str("</table></div>");
    layout(cfg, st, "certs", true, &body)
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
            jobs: vec![],
            job_runs: vec![],
            certs: None,
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
            Shown::Ready("aaa".into(), false)
        );
    }

    #[test]
    fn a_built_update_is_ready_to_switch() {
        let mut s = status();
        s.auto_deploy = false;
        s.deployed = Some(dep("old", State::Succeeded));
        s.latest = Some(dep("aaa", State::Built));
        s.available = Some("aaa".into());
        assert_eq!(
            shown(&view(Some(s)), Some("aaa")),
            Shown::Ready("aaa".into(), true)
        );
    }

    #[test]
    fn an_unchanged_build_is_in_sync() {
        let mut s = status();
        s.deployed = Some(dep("aaa", State::Unchanged));
        s.latest = Some(dep("aaa", State::Unchanged));
        assert_eq!(shown(&view(Some(s)), Some("aaa")), Shown::InSync);
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
            Shown::Deploying("deploying, build".into())
        );
    }

    #[test]
    fn a_node_that_stopped_answering_is_unreachable() {
        let mut v = view(Some(status()));
        v.error = Some("timed out".into());
        assert_eq!(shown(&v, Some("aaa")), Shown::Unreachable);
    }

    #[test]
    fn the_live_region_is_everything_that_updates() {
        let (cfg, st) = hive();
        let page = fleet(&cfg, &st, 0);
        let region = live_region(&page).unwrap();
        assert!(region.starts_with("<header>"), "{region}");
        assert!(region.ends_with("</main>"), "{region}");
        assert!(!region.contains("<script>"));
    }

    #[test]
    fn a_short_lived_cert_is_only_trouble_once_its_renewal_is_overdue() {
        let day = 86400;
        let c = crate::certs::Cert {
            not_before: Some(0),
            not_after: Some(6 * day),
            renew_from: Some(3 * day),
            last_ok: true,
            ..Default::default()
        };
        assert!(
            !cert_trouble(&c, 4 * day),
            "two days left is normal for these"
        );
        assert!(
            cert_trouble(&c, 4 * day + 3 * 3600),
            "a day past renewal and still old"
        );
        let failed = crate::certs::Cert {
            last_ok: false,
            ..c
        };
        assert!(cert_trouble(&failed, day));
    }

    #[test]
    fn crt_sh_links_by_serial_so_the_precertificate_matches() {
        let mut e = crate::certs::CertEvent {
            at: 0,
            name: "zoe".into(),
            kind: crate::certs::EventKind::Issued,
            detail: "24:E1:4D".into(),
            serial: Some("0681ab".into()),
        };
        assert_eq!(crt_sh(&e), "https://crt.sh/?serial=0681ab");
        e.serial = None;
        assert_eq!(crt_sh(&e), "https://crt.sh/?sha256=24E14D");
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

    fn hive() -> (Config, HubState) {
        let mut cfg = Config::from_env();
        let mut st = HubState::default();
        for (i, name) in ["athena", "astrid", "pepper", "zoe"].iter().enumerate() {
            let address: std::net::Ipv6Addr = format!("200::{}", i + 1).parse().unwrap();
            cfg.nodes.push(crate::config::Node {
                name: name.to_string(),
                address,
                key: format!("{name}-key"),
            });
            let mut s = status();
            s.mesh = Some(crate::ygg::Mesh::default());
            st.nodes.insert(
                name.to_string(),
                NodeView {
                    name: name.to_string(),
                    address,
                    status: Some(s),
                    seen: None,
                    error: None,
                },
            );
        }
        (cfg, st)
    }

    fn peer(st: &mut HubState, from: &str, to: &str, remote: &str, cost: u64) {
        let mesh = st
            .nodes
            .get_mut(from)
            .unwrap()
            .status
            .as_mut()
            .unwrap()
            .mesh
            .as_mut()
            .unwrap();
        mesh.peers.push(crate::ygg::Peer {
            key: format!("{to}-key"),
            address: String::new(),
            remote: remote.into(),
            up: true,
            inbound: false,
            uptime: 0.0,
            latency: 5_000_000,
            cost,
            last_error: None,
        });
    }

    const LAN: &str = "tls://[fe80::1%25wlp0s20f3]:9445";

    #[test]
    fn the_cheapest_link_between_a_pair_wins() {
        let (cfg, mut st) = hive();
        peer(&mut st, "pepper", "zoe", "tls://zoe.infra.hayl.in:9444", 12);
        peer(&mut st, "pepper", "zoe", LAN, 4);
        peer(&mut st, "zoe", "pepper", LAN, 3);

        let links = best_links(&cfg, &st);
        let edge = &links[&("pepper".to_string(), "zoe".to_string())];
        assert!(edge.lan);
        assert_eq!(edge.cost, 3);
        assert_eq!(links.len(), 1);
    }

    #[test]
    fn dialling_a_hub_on_the_same_lan_is_still_lan() {
        let (cfg, mut st) = hive();
        peer(&mut st, "astrid", "zoe", "tls://zoe.infra.hayl.in:9444", 2);
        peer(&mut st, "astrid", "zoe", LAN, 3);

        let edge = &best_links(&cfg, &st)[&("astrid".to_string(), "zoe".to_string())];
        assert_eq!(edge.cost, 2);
        assert!(edge.lan);
    }

    #[test]
    fn multicast_peers_are_grouped_and_hubs_over_the_internet_are_not() {
        let (cfg, mut st) = hive();
        peer(&mut st, "pepper", "zoe", LAN, 3);
        peer(&mut st, "astrid", "pepper", LAN, 3);
        peer(
            &mut st,
            "pepper",
            "athena",
            "tls://athena.infra.hayl.in:9444",
            25,
        );

        let names: Vec<String> = st.nodes.keys().cloned().collect();
        let groups = lan_groups(&cfg, &st, &names);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0], vec!["astrid", "pepper", "zoe"]);
        assert_eq!(groups[1], vec!["athena"]);
    }

    #[test]
    fn a_stranger_on_the_mesh_shows_up_in_the_graph() {
        let (cfg, mut st) = hive();
        peer(&mut st, "pepper", "evil", LAN, 3);
        let svg = graph(&cfg, &st);
        assert!(svg.contains("node unknown"), "{svg}");
    }
}
