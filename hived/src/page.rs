use crate::api::{Shared, status_json};
use crate::state::{Deployment, Phase, State, Store};
use axum::extract::{Path, State as AxumState};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use maud::{DOCTYPE, Markup, PreEscaped, html};

/// Deliberately plain: headings, one table, and enough CSS to line the numbers
/// up. No JavaScript, nothing to build, and it stays legible with CSS off.
const STYLE: &str = r#"
:root { --pg:#fcfbf6; --ink:#17160f; --dim:#6f6e60; --rule:#d5d3c4; --zebra:#f4f2e9;
        --ok:#2e6b34; --warn:#8a6503; --bad:#a52a21; --link:#0645ad; }
@media (prefers-color-scheme: dark) {
  :root { --pg:#16181a; --ink:#e6e6df; --dim:#8b8d82; --rule:#333a3d; --zebra:#1d2124;
          --ok:#63b183; --warn:#c9a24a; --bad:#de8686; --link:#79b8ff; }
}
* { box-sizing: border-box; }
body { background:var(--pg); color:var(--ink); margin:0; padding:0 0 32px;
       font:13px/1.55 ui-monospace,SFMono-Regular,"DejaVu Sans Mono","Courier New",monospace; }
a { color:var(--link); }
.head { display:flex; align-items:baseline; gap:10px; flex-wrap:wrap;
        border-bottom:2px solid var(--ink); padding:14px 18px 7px; margin-bottom:14px; }
h1 { font-size:16px; margin:0; letter-spacing:.06em; text-transform:lowercase; }
.host { background:var(--ink); color:var(--pg); padding:1px 7px; font-size:11.5px; letter-spacing:.1em; }
.clock { margin-left:auto; color:var(--dim); font-size:11.5px; }
dl { display:grid; grid-template-columns:max-content 1fr; gap:3px 16px; margin:0 18px; }
dt { color:var(--dim); font-size:10.5px; letter-spacing:.12em; text-transform:uppercase; padding-top:2px; }
dd { margin:0; }
h2 { font-size:10.5px; color:var(--dim); letter-spacing:.16em; text-transform:uppercase;
     margin:20px 18px 7px; font-weight:700; }
.scroll { overflow-x:auto; padding:0 18px; }
table { border-collapse:collapse; font-size:12.5px; }
th { text-align:left; color:var(--dim); font-size:10.5px; letter-spacing:.12em; text-transform:uppercase;
     font-weight:400; border-bottom:1px solid var(--ink); padding:0 11px 5px 0; white-space:nowrap; }
td { padding:4px 11px 4px 0; white-space:nowrap; vertical-align:top; }
tbody tr:nth-child(odd) td { background:var(--zebra); }
th:first-child, td:first-child { padding-left:8px; }
td.r, th.r { text-align:right; font-variant-numeric:tabular-nums; }
.ok { color:var(--ok); } .warn { color:var(--warn); } .bad { color:var(--bad); } .dim { color:var(--dim); }
pre { border-left:3px solid var(--bad); background:var(--zebra); margin:3px 0 6px; padding:7px 11px;
      font:11.5px/1.5 inherit; white-space:pre-wrap; overflow-x:auto; }
.note td { color:var(--dim); font-size:11.5px; padding-top:0; }
"#;

fn short(rev: &str) -> String {
    rev.chars().take(7).collect()
}

fn secs(v: f64) -> String {
    let total = v.round() as i64;
    if total >= 60 {
        format!("{}m{:02}s", total / 60, total % 60)
    } else {
        format!("{total}s")
    }
}

fn stamp(t: i64) -> String {
    chrono::DateTime::from_timestamp(t, 0)
        .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

fn state_class(state: State) -> &'static str {
    match state {
        State::Succeeded => "ok",
        State::Degraded => "warn",
        State::Failed | State::Rejected | State::Interrupted => "bad",
        State::Superseded | State::Cancelled => "dim",
        State::Queued | State::Running => "",
    }
}

fn state_text(d: &Deployment) -> String {
    match (d.state, d.phase) {
        (State::Running, Some(p)) => format!("running {}", p.label()),
        (State::Failed, _) => match &d.error {
            Some(e) => format!("failed {}", e.phase.label()),
            None => "failed".into(),
        },
        (s, _) => s.label().to_string(),
    }
}

fn shell(title: &str, refresh: u64, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                meta http-equiv="refresh" content=(refresh.to_string());
                title { (title) }
                style { (PreEscaped(STYLE)) }
            }
            body { (body) }
        }
    }
}

fn head_bar(host: &str) -> Markup {
    html! {
        div .head {
            h1 { "hived" }
            span .host { (host.to_uppercase()) }
            span .clock { (stamp(crate::api::now())) }
        }
    }
}

/// One row per deploy, one column per phase. Numbers you can compare down a
/// column beat a bar chart, and the longest phase is bold so the dominant
/// cost reads at a glance.
fn history(store: &Store) -> Markup {
    html! {
        h2 { "history" }
        div .scroll {
            table {
                thead {
                    tr {
                        th { "rev" } th { "commit" } th { "state" }
                        @for p in Phase::all() { th .r { (p.label()) } }
                        th .r { "total" } th { "" }
                    }
                }
                tbody {
                    @for d in &store.deployments {
                        tr {
                            td { a href={ "/d/" (d.id) } { (short(&d.rev)) } }
                            td { (d.subject.clone().unwrap_or_default()) }
                            td .(state_class(d.state)) { (state_text(d)) }
                            @let longest = d.longest_phase();
                            @for p in Phase::all() {
                                td .r {
                                    @match d.durations.get(&p) {
                                        Some(v) if Some(p) == longest => b { (secs(*v)) },
                                        Some(v) => (secs(*v)),
                                        None => "–",
                                    }
                                }
                            }
                            td .r { @match d.total_duration() { Some(t) => (secs(t)), None => "–" } }
                            td { @if !d.log_reaped { a href={ "/d/" (d.id) } { "log" } } }
                        }
                        @if let Some(e) = &d.error {
                            tr .note { td colspan="10" { pre { (e.message) } } }
                        }
                        @if !d.new_failed_units.is_empty() {
                            tr .note {
                                td colspan="10" { "newly failed units: " (d.new_failed_units.join(", ")) }
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn index(AxumState(app): AxumState<Shared>) -> Response {
    let store = match app.dir.load() {
        Ok(s) => s,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let status = status_json(&app);
    let running = store.running().cloned();
    let deployed = store.last_switched().cloned();
    let drifted = status["drifted"].as_bool().unwrap_or(false);
    let failed = status["system_failed_units"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    let refresh = if running.is_some() { 5 } else { 60 };

    let body = html! {
        (head_bar(&app.cfg.host))
        dl {
            dt { "deployed" }
            dd {
                @match &deployed {
                    Some(d) => {
                        a href={ "/d/" (d.id) } { (short(&d.rev)) }
                        " " (d.subject.clone().unwrap_or_default())
                    },
                    None => span .dim { "nothing deployed by hived yet" },
                }
            }
            @if let Some(d) = &deployed {
                dt { "generation" }
                dd {
                    @match d.generation { Some(g) => (g.to_string()), None => "?" }
                    ", switched " (d.finished_at.map(stamp).unwrap_or_default())
                    @if let Some(t) = d.total_duration() { ", took " (secs(t)) }
                }
            }
            dt { "system" }
            dd {
                @if drifted {
                    span .warn { "drifted" }
                    " – running system does not match the last hived deploy"
                } @else {
                    span .ok { "in sync" }
                }
                @if !failed.is_empty() { ", failing: " (failed) }
            }
            @if let Some(d) = &running {
                dt { "in progress" }
                dd {
                    a href={ "/d/" (d.id) } { (short(&d.rev)) }
                    " " (d.subject.clone().unwrap_or_default())
                    ", " (state_text(d))
                    @if let Some(started) = d.started_at {
                        ", " (secs((crate::api::now() - started) as f64)) " elapsed"
                    }
                }
            }
            @if let Some(d) = store.queued() {
                dt { "waiting" }
                dd { (short(&d.rev)) " " (d.subject.clone().unwrap_or_default()) }
            }
        }
        (history(&store))
    };

    Html(shell(&format!("hived {}", app.cfg.host), refresh, body).into_string()).into_response()
}

async fn detail(AxumState(app): AxumState<Shared>, Path(id): Path<u64>) -> Response {
    let store = match app.dir.load() {
        Ok(s) => s,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let Some(d) = store.get(id) else {
        return (StatusCode::NOT_FOUND, "no such deployment").into_response();
    };

    let log = app
        .dir
        .read_log(id, 0, 2 * 1024 * 1024)
        .map(|(chunk, _, _)| String::from_utf8_lossy(&chunk).into_owned())
        .unwrap_or_default();

    let refresh = if d.state == State::Running { 5 } else { 3600 };
    let body = html! {
        (head_bar(&app.cfg.host))
        dl {
            dt { "deployment" } dd { "#" (d.id) }
            dt { "rev" } dd { (d.rev) }
            dt { "commit" } dd { (d.subject.clone().unwrap_or_default()) }
            dt { "state" } dd .(state_class(d.state)) { (state_text(d)) }
            dt { "queued" } dd { (stamp(d.queued_at)) }
            @if let Some(t) = d.total_duration() { dt { "took" } dd { (secs(t)) } }
            @if !d.preexisting_failed_units.is_empty() {
                dt { "already failing" } dd .dim { (d.preexisting_failed_units.join(", ")) }
            }
            @if !d.new_failed_units.is_empty() {
                dt { "newly failed" } dd .warn { (d.new_failed_units.join(", ")) }
            }
        }
        @if let Some(e) = &d.error {
            h2 { "error" }
            div .scroll { pre { (e.message) } }
        }
        h2 { "log" }
        div .scroll {
            @if d.log_reaped {
                p .dim { "log was reaped, the error above is what was kept" }
            } @else {
                pre style="border-left-color:var(--rule)" { (log) }
            }
        }
        p style="margin:18px" { a href="/" { "back" } }
    };

    Html(shell(&format!("hived {} #{id}", app.cfg.host), refresh, body).into_string())
        .into_response()
}

/// The GitHub Pages dashboard fetches from both nodes, so it needs the origin
/// header here too.
async fn robots() -> Response {
    (
        [(header::CONTENT_TYPE, "text/plain")],
        "User-agent: *\nDisallow: /\n",
    )
        .into_response()
}

pub fn routes() -> axum::Router<Shared> {
    axum::Router::new()
        .route("/", axum::routing::get(index))
        .route("/d/{id}", axum::routing::get(detail))
        .route("/robots.txt", axum::routing::get(robots))
}
