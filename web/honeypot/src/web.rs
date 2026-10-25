use crate::asn;
use crate::chart;
use crate::config::Config;
use crate::names;
use crate::shared::Shared;
use crate::store::{self, now_ms};
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use maud::{DOCTYPE, Markup, PreEscaped, html};
use rusqlite::{Connection, OptionalExtension};
use std::collections::{BTreeMap, HashMap};
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const DAY_MS: i64 = 86_400_000;

const CSS: &str = "
:root { --bg: #fafaf9; --fg: #1c1917; --muted: #78716c; --line: #e7e5e4; --bar: #a8a29e; --accent: #c2410c; --mark: #78716c; --cell: #efedeb; --hot-ink: #ffffff; }
@media (prefers-color-scheme: dark) {
  :root { --bg: #161514; --fg: #e7e5e4; --muted: #a8a29e; --line: #2e2c2a; --bar: #57534e; --accent: #ea580c; --mark: #a8a29e; --cell: #23211f; --hot-ink: #161514; }
}
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--fg); font: 15px/1.5 ui-sans-serif, system-ui, sans-serif; }
main { max-width: 980px; margin: 0 auto; padding: 32px 16px 64px; }
h1 { font-size: 24px; margin: 0 0 8px; }
h2 { font-size: 17px; margin: 40px 0 8px; }
p { margin: 0 0 12px; color: var(--muted); max-width: 680px; }
.wrap { overflow-x: auto; }
table { border-collapse: collapse; width: 100%; font-size: 13px; }
th, td { text-align: left; padding: 4px 10px 4px 0; border-bottom: 1px solid var(--line); white-space: nowrap; vertical-align: top; }
th { color: var(--muted); font-weight: 500; }
td.n, th.n { text-align: right; font-variant-numeric: tabular-nums; }
td.text { white-space: normal; word-break: break-all; font-family: ui-monospace, monospace; font-size: 12px; }
.stats { display: flex; flex-wrap: wrap; gap: 24px; margin: 20px 0; }
.stat b { display: block; font-size: 22px; font-variant-numeric: tabular-nums; }
.stat span { color: var(--muted); font-size: 13px; }
.bar { display: inline-block; height: 9px; background: var(--bar); vertical-align: middle; }
.bar.a { background: var(--accent); }
.grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 0 32px; }
.muted { color: var(--muted); }
svg.chart { display: block; width: max(100%, 640px); max-width: none; height: auto; margin: 8px 0; }
.chart text { font: 12px ui-sans-serif, system-ui, sans-serif; fill: var(--muted); }
.chart text.title { fill: var(--fg); font-size: 13px; font-weight: 500; }
.chart .grid { stroke: var(--line); stroke-width: 1; }
.chart .axis { stroke: var(--bar); stroke-width: 1; }
.chart .col.a { fill: var(--accent); }
.chart .col.n { fill: var(--mark); }
.chart line.mark { stroke: var(--fg); stroke-opacity: 0.45; stroke-width: 1; }
.chart .hover { fill: var(--fg); fill-opacity: 0; }
.chart .hover:hover { fill-opacity: 0.06; }
.block { display: grid; grid-template-columns: repeat(8, minmax(0, 1fr)); gap: 2px; margin: 8px 0 12px; }
.cell { text-decoration: none; display: flex; flex-direction: column; min-height: 60px; padding: 6px; border-radius: 4px; font-size: 11px; line-height: 1.3; color: var(--fg); }
.cell b { font-size: 13px; font-weight: 500; }
.cell span { opacity: 0.8; }
.cell .n { margin-top: auto; opacity: 1; font-variant-numeric: tabular-nums; }
.cell.hot { color: var(--hot-ink); }
.cell.named { box-shadow: inset 0 0 0 2px var(--accent); }
@media (max-width: 600px) { .block { grid-template-columns: repeat(4, minmax(0, 1fr)); } }
a { color: var(--accent); }
details summary { cursor: pointer; }
form.search { margin: 0 0 12px; }
input[type=search] { max-width: 100%; font: inherit; padding: 4px 6px; }
pre { white-space: pre-wrap; word-break: break-all; font-size: 12px; margin: 6px 0; color: var(--muted); }
";

pub struct Web {
    shared: Arc<Shared>,
    asn: asn::Cache,
    cache: Mutex<Option<(Instant, String)>>,
}

fn name_label(day: i64) -> String {
    format!("[name {}]", date(day))
}

fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn date(day: i64) -> String {
    chrono::DateTime::from_timestamp(day * 86400, 0)
        .map(|t| t.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

fn time(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|t| t.format("%H:%M").to_string())
        .unwrap_or_default()
}

fn stamp(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

fn span(ms: i64) -> String {
    let s = ms.max(0) / 1000;
    match s {
        0..60 => format!("{s}s"),
        60..3600 => format!("{}m {}s", s / 60, s % 60),
        3600..86400 => format!("{}h {}m", s / 3600, s % 3600 / 60),
        _ => format!("{}d {}h", s / 86400, s % 86400 / 3600),
    }
}

fn names_in(cfg: &Config, run: &str, today: i64) -> String {
    let words: Vec<&str> = run.split('-').collect();
    if words.len() < 3 {
        return run.to_string();
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i < words.len() {
        if i + 3 <= words.len()
            && let Some(day) = cfg.key.decode_name(&words[i..i + 3].join("-"))
            && (cfg.launch_day..=today + 1).contains(&day)
        {
            out.push(name_label(day));
            i += 3;
            continue;
        }
        out.push(words[i].to_string());
        i += 1;
    }
    out.join("-")
}

pub fn scrub(cfg: &Config, s: &str) -> String {
    let lower = s.to_ascii_lowercase();
    let mut cut = String::new();
    let mut i = 0;
    while let Some(p) = lower[i..].find(cfg.zone.as_str()) {
        cut.push_str(&s[i..i + p]);
        cut.push_str("[domain]");
        i += p + cfg.zone.len();
    }
    cut.push_str(&s[i..]);
    let today = names::day_of(now_ms() / 1000);
    let mut out = String::new();
    let mut run = String::new();
    for c in cut.chars() {
        if c.is_ascii_alphabetic() || c == '-' {
            run.push(c);
        } else {
            out.push_str(&names_in(cfg, &run, today));
            run.clear();
            out.push(c);
        }
    }
    out.push_str(&names_in(cfg, &run, today));
    out
}

#[derive(Default)]
struct Day {
    ordered: Option<i64>,
    issued: Option<i64>,
    ct: Option<i64>,
    error: Option<String>,
    first_dns: Option<i64>,
    first_hit: Option<i64>,
    dns: i64,
    hits: i64,
}

struct Top {
    title: &'static str,
    link: Option<&'static str>,
    rows: Vec<(String, i64, i64)>,
}

struct Age {
    age: i64,
    names: i64,
    hits: i64,
    dns: i64,
}

struct Report {
    days: BTreeMap<i64, Day>,
    ages: Vec<Age>,
    tops: Vec<Top>,
    asn_all: Vec<(String, i64, i64)>,
    asn_bait: Vec<(String, i64, i64)>,
    roles: BTreeMap<i64, HashMap<String, i64>>,
    totals: (i64, i64, i64),
    timeline: chart::Timeline,
    block: Vec<chart::Cell>,
    v6: (i64, i64),
}

fn top(
    conn: &Connection,
    title: &'static str,
    link: Option<&'static str>,
    sql: &str,
) -> rusqlite::Result<Top> {
    let rows = conn
        .prepare(sql)?
        .query_map([], |r| {
            Ok((
                r.get::<_, Option<String>>(0)?.unwrap_or_default(),
                r.get(1)?,
                r.get(2)?,
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(Top { title, link, rows })
}

fn by_asn(table: &asn::Table, rows: Vec<(String, i64)>) -> Vec<(String, i64, i64)> {
    let mut agg: HashMap<String, (i64, i64)> = HashMap::new();
    for (src, n) in rows {
        let label = src
            .parse::<IpAddr>()
            .ok()
            .and_then(|ip| table.lookup(ip))
            .map(|a| format!("AS{} {} ({})", a.number, a.name, a.country))
            .unwrap_or_else(|| "unknown".into());
        let e = agg.entry(label).or_default();
        e.0 += n;
        e.1 += 1;
    }
    let mut out: Vec<_> = agg.into_iter().map(|(k, (n, s))| (k, n, s)).collect();
    out.sort_by_key(|r| std::cmp::Reverse(r.1));
    out.truncate(15);
    out
}

fn sources(conn: &Connection, filter: &str) -> rusqlite::Result<Vec<(String, i64)>> {
    conn.prepare(&format!(
        "SELECT src, COUNT(*) FROM events WHERE acme = 0 AND {filter} GROUP BY src"
    ))?
    .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
    .collect()
}

fn role_of(cfg: &Config, ip: std::net::Ipv4Addr, today: i64) -> String {
    match ip {
        ip if ip == cfg.apex4 => "ns1 and apex".to_string(),
        ip if ip == cfg.ns2_4 => "ns2".to_string(),
        ip if ip == cfg.baseline4 => "baseline".to_string(),
        ip => match names::decode_v4(cfg.first4, ip, today) {
            Some(d) if d == today => "today's name".to_string(),
            Some(d) if d >= cfg.launch_day => format!("name, {}", &date(d)[5..]),
            _ => "unused".to_string(),
        },
    }
}

fn report(conn: &Connection, cfg: &Config, table: &asn::Table) -> rusqlite::Result<Report> {
    let now = now_ms();
    let today = names::day_of(now / 1000);
    let mut days: BTreeMap<i64, Day> = BTreeMap::new();
    for d in cfg.launch_day.max(today - 60)..=today {
        days.insert(d, Day::default());
    }
    let mut certs =
        conn.prepare("SELECT day, ordered_at_ms, issued_at_ms, scts, error FROM certs")?;
    let rows = certs.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, Option<i64>>(2)?,
            r.get::<_, Option<String>>(3)?,
            r.get::<_, Option<String>>(4)?,
        ))
    })?;
    for row in rows {
        let (day, ordered, issued, scts, error) = row?;
        let d = days.entry(day).or_default();
        d.ordered = Some(ordered);
        d.issued = issued;
        d.error = error;
        d.ct = scts
            .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(&s).ok())
            .and_then(|v| v.iter().filter_map(|x| x["at_ms"].as_i64()).min());
    }
    let mut per = conn.prepare(
        "SELECT day, proto = 'dns', MIN(at_ms), COUNT(*) FROM events
         WHERE kind = 'bait' AND acme = 0 AND (name IS NOT NULL OR instr(dst, ':') > 0)
         GROUP BY day, proto = 'dns'",
    )?;
    let rows = per.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, bool>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, i64>(3)?,
        ))
    })?;
    for row in rows {
        let (day, dns, first, n) = row?;
        let d = days.entry(day).or_default();
        if dns {
            (d.first_dns, d.dns) = (Some(first), n);
        } else {
            (d.first_hit, d.hits) = (Some(first), n);
        }
    }
    let mut counts: BTreeMap<i64, [i64; 2]> = BTreeMap::new();
    let mut age = conn.prepare(
        "SELECT e.proto = 'dns', (e.at_ms - c.issued_at_ms) / 86400000, COUNT(*)
         FROM events e JOIN certs c ON c.day = e.day
         WHERE e.kind = 'bait' AND e.acme = 0 AND e.at_ms >= c.issued_at_ms
           AND (e.name IS NOT NULL OR instr(e.dst, ':') > 0)
         GROUP BY 1, 2",
    )?;
    let rows = age.query_map([], |r| {
        Ok((
            r.get::<_, bool>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })?;
    for row in rows {
        let (dns, age, n) = row?;
        counts.entry(age).or_default()[dns as usize] += n;
    }
    let mut names: BTreeMap<i64, i64> = BTreeMap::new();
    for issued in days.values().filter_map(|d| d.issued) {
        for k in 0..=(now - issued) / DAY_MS {
            *names.entry(k).or_default() += 1;
        }
    }
    let ages = names
        .into_iter()
        .map(|(age, names)| {
            let [hits, dns] = counts.get(&age).copied().unwrap_or_default();
            Age {
                age,
                names,
                hits,
                dns,
            }
        })
        .collect();
    let tops = vec![
        top(
            conn,
            "Names scanners tried",
            Some("/name/"),
            "SELECT CASE WHEN via = 'apex' THEN '@' ELSE label END, COUNT(*), COUNT(DISTINCT src) FROM events
             WHERE (label IS NOT NULL OR via = 'apex') AND acme = 0 GROUP BY 1 ORDER BY 2 DESC LIMIT 15",
        )?,
        top(
            conn,
            "Ports",
            None,
            "SELECT transport || '/' || dst_port, COUNT(*), COUNT(DISTINCT src) FROM events WHERE proto != 'dns' GROUP BY 1 ORDER BY 2 DESC LIMIT 15",
        )?,
        top(
            conn,
            "Protocols",
            None,
            "SELECT proto, COUNT(*), COUNT(DISTINCT src) FROM events WHERE acme = 0 GROUP BY 1 ORDER BY 2 DESC LIMIT 15",
        )?,
        top(
            conn,
            "TLS fingerprints (JA4)",
            None,
            "SELECT json_extract(detail, '$.ja4') j, COUNT(*), COUNT(DISTINCT src) FROM events WHERE proto = 'tls' AND j IS NOT NULL GROUP BY 1 ORDER BY 2 DESC LIMIT 15",
        )?,
        top(
            conn,
            "User agents",
            Some("/ua?q="),
            "SELECT json_extract(detail, '$.ua') u, COUNT(*), COUNT(DISTINCT src) FROM events WHERE proto IN ('http', 'https') AND u IS NOT NULL GROUP BY 1 ORDER BY 2 DESC LIMIT 15",
        )?,
        top(
            conn,
            "HTTP paths",
            None,
            "SELECT json_extract(detail, '$.path'), COUNT(*), COUNT(DISTINCT src) FROM events WHERE proto IN ('http', 'https') GROUP BY 1 ORDER BY 2 DESC LIMIT 15",
        )?,
    ];
    let month = format!("at_ms > {}", now - 30 * DAY_MS);
    let asn_all = by_asn(table, sources(conn, &month)?);
    let asn_bait = by_asn(
        table,
        sources(
            conn,
            "kind = 'bait' AND (name IS NOT NULL OR instr(dst, ':') > 0)",
        )?,
    );
    let mut roles: BTreeMap<i64, HashMap<String, i64>> = BTreeMap::new();
    let mut role = conn.prepare(
        "SELECT at_ms / 86400000, addr_via, COUNT(*) FROM events
         WHERE proto != 'dns' AND acme = 0 AND at_ms > ?1 GROUP BY 1, 2",
    )?;
    let rows = role.query_map([now - 14 * DAY_MS], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })?;
    for row in rows {
        let (day, via, n) = row?;
        *roles.entry(day).or_default().entry(via).or_default() += n;
    }
    let first: Option<i64> = conn.query_row("SELECT MIN(at_ms) FROM events", [], |r| r.get(0))?;
    let from = first.unwrap_or(now).max(now - 14 * DAY_MS);
    let bucket = chart::bucket_for(now - from);
    let start = from.div_euclid(bucket) * bucket;
    let mut rows = vec![[0i64; 3]; ((now - start) / bucket + 1) as usize];
    let signal = "COALESCE(kind, '') = 'bait' AND (name IS NOT NULL OR instr(dst, ':') > 0)";
    let mut tl = conn.prepare(&format!(
        "SELECT (at_ms - ?1) / ?2,
                SUM({signal} AND proto != 'dns'),
                SUM(COALESCE(kind, '') = 'bait' AND proto = 'dns'),
                SUM(proto != 'dns' AND NOT ({signal}))
         FROM events WHERE at_ms >= ?1 AND acme = 0 GROUP BY 1"
    ))?;
    let found = tl.query_map([start, bucket], |r| {
        Ok((r.get::<_, i64>(0)?, [r.get(1)?, r.get(2)?, r.get(3)?]))
    })?;
    for row in found {
        let (b, v) = row?;
        if let Some(slot) = rows.get_mut(b as usize) {
            *slot = v;
        }
    }
    let timeline = chart::Timeline {
        start,
        bucket,
        rows,
        issued: days.values().filter_map(|d| d.issued).collect(),
    };
    let mut per_dst: HashMap<String, (i64, i64)> = HashMap::new();
    let mut by_dst = conn.prepare(
        "SELECT dst, COUNT(*), COUNT(DISTINCT src) FROM events
         WHERE proto != 'dns' AND acme = 0 AND at_ms > ?1 GROUP BY dst",
    )?;
    let found = by_dst.query_map([now - DAY_MS], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })?;
    let mut v6 = (0, 0);
    for row in found {
        let (dst, n, s) = row?;
        if dst.contains(':') {
            v6 = (v6.0 + n, v6.1 + 1);
        } else {
            per_dst.insert(dst, (n, s));
        }
    }
    let base = u32::from(cfg.apex4);
    let block = (0..32u32)
        .map(|i| {
            let ip = std::net::Ipv4Addr::from(base + i);
            let role = role_of(cfg, ip, today);
            let (events, sources) = per_dst.get(&ip.to_string()).copied().unwrap_or_default();
            chart::Cell {
                href: format!("/addr/{ip}"),
                label: format!(".{}", ip.octets()[3]),
                named: role == "today's name",
                role,
                events,
                sources,
            }
        })
        .collect();
    let totals = conn.query_row(
        "SELECT COUNT(*), COUNT(DISTINCT src),
                COALESCE(SUM(kind = 'bait' AND proto != 'dns' AND (name IS NOT NULL OR instr(dst, ':') > 0)), 0)
         FROM events WHERE acme = 0",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    Ok(Report {
        days,
        ages,
        tops,
        asn_all,
        asn_bait,
        roles,
        totals,
        timeline,
        block,
        v6,
    })
}

fn after(base: Option<i64>, at: Option<i64>) -> String {
    match (base, at) {
        (Some(b), Some(a)) if a >= b => format!("+{}", span(a - b)),
        (_, Some(a)) => time(a),
        _ => String::new(),
    }
}

fn bar(n: i64, max: i64, accent: bool) -> Markup {
    let width = if max > 0 {
        (n as f64 / max as f64 * 160.0)
            .round()
            .max(if n > 0 { 1.0 } else { 0.0 })
    } else {
        0.0
    };
    html! { span class=(if accent { "bar a" } else { "bar" }) style=(format!("width: {width}px")) {} }
}

fn href(prefix: &str, key: &str) -> Option<String> {
    if prefix == "/asn/" {
        let number = key.strip_prefix("AS")?.split(' ').next()?;
        return Some(format!("/asn/{number}"));
    }
    Some(format!("{prefix}{}", encode(key)))
}

fn network(cfg: &Config, table: &asn::Table, src: &str) -> Markup {
    let Some(a) = src.parse::<IpAddr>().ok().and_then(|ip| table.lookup(ip)) else {
        return html! {};
    };
    html! { a href=(format!("/asn/{}", a.number)) { "AS" (a.number) " " (scrub(cfg, &a.name)) } }
}

fn top_table(cfg: &Config, title: &str, link: Option<&str>, rows: &[(String, i64, i64)]) -> Markup {
    html! {
        div {
            h2 { (title) }
            @if rows.is_empty() {
                p { "Nothing yet." }
            } @else {
                div.wrap { table {
                    tr { th { "" } th.n { "hits" } th.n { "sources" } }
                    @for (k, n, s) in rows {
                        tr {
                            td.text {
                                @if let Some(url) = link.filter(|_| scrub(cfg, k) == *k).and_then(|p| href(p, k)) {
                                    a href=(url) { (k) }
                                } @else {
                                    (scrub(cfg, k))
                                }
                            }
                            td.n { (n) } td.n { (s) }
                        }
                    }
                } }
            }
        }
    }
}

fn per_name(n: i64, names: i64) -> String {
    if names == 0 {
        return String::new();
    }
    format!("{:.1}", n as f64 / names as f64)
}

fn render(cfg: &Config, r: &Report) -> String {
    let max_rate = r
        .ages
        .iter()
        .map(|a| a.hits / a.names.max(1))
        .max()
        .unwrap_or(0);
    let roles = [
        ("bait", "daily name"),
        ("ns", "name servers"),
        ("baseline", "baseline"),
        ("foreign", "unused"),
    ];
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "CT honeypot" }
                style { (PreEscaped(CSS)) }
            }
            body { main {
                h1 { "Certificate Transparency honeypot" }
                p {
                    "Every day, at a random time, a brand new subdomain gets a TLS certificate. "
                    "Certificate Transparency logs publish every certificate within seconds, and that is the only place the name ever appears. "
                    "So anyone who looks it up or connects to it learned about it from those logs."
                }
                p {
                    "Each day's name is shown here by its date, never by the name itself. "
                    "Names that scanners guessed on their own, such as www, are shown as typed. All times are UTC."
                }
                div.stats {
                    div.stat { b { (chart::thousands(r.totals.2)) } span { "hits on daily names" } }
                    div.stat { b { (chart::thousands(r.totals.1)) } span { "distinct sources" } }
                    div.stat { b { (chart::thousands(r.totals.0)) } span { "events recorded" } }
                }
                h2 { "Traffic over time" }
                p {
                    "Events per " (chart::bucket_name(r.timeline.bucket)) ", since launch or over the last 14 days. "
                    "Orange is traffic that used the daily name, gray is everything else arriving on these addresses. "
                    "The vertical lines mark each certificate issuance. Hover a column for exact numbers."
                }
                div.wrap { (chart::timeline(&r.timeline)) }
                h2 { "The address block, last 24 hours" }
                p {
                    "Each square is one of the 32 IPv4 addresses, the stronger the shade, the more connections. Today's name is outlined. Click a square to see all of its traffic. "
                    "Only the name servers and the daily name are ever published in DNS. Everything else gets found by scanning the whole block."
                }
                (chart::block(&r.block))
                p { "IPv6: " (r.v6.0) " connections to " (r.v6.1) " addresses in the /64." }
                h2 { "Each day" }
                p {
                    "A lookup is a DNS query for the name. A hit is a connection that used it: a TLS handshake, an HTTP request, or anything else. "
                    "The logged, first lookup and first hit columns count from when the certificate was issued. "
                    "Click a date to see every HTTP request for that day's name."
                }
                div.wrap { table {
                    tr {
                        th { "date" } th { "ordered" } th { "issued" } th { "logged" }
                        th { "first lookup" } th { "first hit" } th.n { "lookups" } th.n { "hits" }
                    }
                    @for (day, d) in r.days.iter().rev() {
                        tr {
                            td { a href=(format!("/day/{}", date(*day))) { (date(*day)) } }
                            td { (d.ordered.map(time).unwrap_or_default()) }
                            td {
                                @if let Some(i) = d.issued { (time(i)) }
                                @else if d.error.is_some() { span.muted { "failed" } }
                            }
                            td { (after(d.issued, d.ct)) }
                            td { (after(d.issued, d.first_dns)) }
                            td { (after(d.issued, d.first_hit)) }
                            td.n { (d.dns) } td.n { (d.hits) }
                        }
                    }
                } }
                h2 { "Interest over time" }
                p {
                    "Average lookups and hits per name, by how many days old its certificate was at the time. "
                    "Each row only counts names that have reached that age, so rows stay comparable as more days are added. "
                    "Certificates expire after about six and a half days, but the names keep resolving and still answer over HTTPS."
                }
                div.wrap { table {
                    tr { th { "days since issue" } th.n { "names" } th.n { "lookups per name" } th.n { "hits per name" } th { "" } }
                    @for a in &r.ages {
                        tr {
                            td { (a.age) @if a.age >= 7 { " " span.muted { "expired" } } }
                            td.n { (a.names) }
                            td.n { (per_name(a.dns, a.names)) }
                            td.n { (per_name(a.hits, a.names)) }
                            td { (bar(a.hits / a.names.max(1), max_rate, true)) }
                        }
                    }
                } }
                div.grid {
                    (top_table(cfg, "Networks hitting daily names", Some("/asn/"), &r.asn_bait))
                    (top_table(cfg, "All networks, last 30 days", Some("/asn/"), &r.asn_all))
                    @for t in &r.tops { (top_table(cfg, t.title, t.link, &t.rows)) }
                }
                h2 { "Traffic by address, last 14 days" }
                p {
                    "Which kind of address each connection arrived on. "
                    "The baseline address has never been published anywhere, so it shows the background scanning every address on the internet gets. "
                    "Unused addresses are the rest of the range."
                }
                div.wrap { table {
                    tr { th { "date" } @for (_, label) in roles { th.n { (label) } } }
                    @for (day, m) in r.roles.iter().rev() {
                        tr {
                            td { (date(*day)) }
                            @for (key, _) in roles { td.n { (m.get(key).copied().unwrap_or(0)) } }
                        }
                    }
                } }
            } }
        }
    }
    .into_string()
}

struct Request {
    at: i64,
    proto: String,
    src: String,
    port: u16,
    summary: String,
    detail: serde_json::Value,
    body: Option<Vec<u8>>,
}

fn requests(
    conn: &Connection,
    filter: &str,
    param: impl rusqlite::ToSql,
) -> rusqlite::Result<Vec<Request>> {
    conn.prepare(&format!(
        "SELECT at_ms, proto, src, dst_port, summary, detail, raw FROM events
         WHERE {filter} AND proto IN ('http', 'https') AND acme = 0
         ORDER BY at_ms DESC LIMIT 2000"
    ))?
    .query_map([param], |r| {
        Ok(Request {
            at: r.get(0)?,
            proto: r.get(1)?,
            src: r.get(2)?,
            port: r.get(3)?,
            summary: r.get(4)?,
            detail: r
                .get::<_, Option<String>>(5)?
                .and_then(|d| serde_json::from_str(&d).ok())
                .unwrap_or_default(),
            body: r.get(6)?,
        })
    })?
    .collect()
}

fn request_table(
    cfg: &Config,
    table: &asn::Table,
    own: Option<i64>,
    base: Option<i64>,
    rows: &[Request],
) -> Markup {
    let own = own.map(name_label);
    let clean = |s: &str| {
        let s = scrub(cfg, s);
        match &own {
            Some(o) => s.replace(o.as_str(), "[name]"),
            None => s,
        }
    };
    let field = |r: &Request, k: &str| r.detail[k].as_str().map(clean).unwrap_or_default();
    html! {
        @if rows.is_empty() {
            p { "No requests yet." }
        } @else {
            div.wrap { table {
                tr {
                    th { "time" }
                    @if base.is_some() { th { "since issue" } }
                    th { "source" } th { "network" } th { "port" }
                    th { "host" } th { "request" } th { "user agent" }
                }
                @for r in rows {
                    tr {
                        td { (stamp(r.at)) }
                        @if base.is_some() { td { (after(base, Some(r.at))) } }
                        td { (r.src) }
                        td.text { (network(cfg, table, &r.src)) }
                        td { (r.proto) " " (r.port) }
                        td.text { (field(r, "host")) }
                        td.text {
                            details {
                                summary { (clean(&r.summary)) }
                                pre {
                                    @if let Some(headers) = r.detail["headers"].as_array() {
                                        @for h in headers {
                                            (clean(h[0].as_str().unwrap_or(""))) ": " (clean(h[1].as_str().unwrap_or(""))) "\n"
                                        }
                                    }
                                    @if let Some(ja4) = r.detail["ja4"].as_str() { "\nJA4: " (ja4) "\n" }
                                    @if let Some(body) = &r.body {
                                        "\n" (clean(&String::from_utf8_lossy(&body[..body.len().min(2048)])))
                                    }
                                }
                            }
                        }
                        td.text {
                            @if let Some(ua) = r.detail["ua"].as_str().filter(|ua| scrub(cfg, ua) == *ua) {
                                a href=(format!("/ua?q={}", encode(ua))) { (ua) }
                            } @else {
                                (field(r, "ua"))
                            }
                        }
                    }
                }
            } }
        }
    }
}

struct Lookup {
    at: i64,
    src: String,
    transport: String,
    name: String,
    acme: bool,
    detail: serde_json::Value,
}

fn lookups(
    conn: &Connection,
    filter: &str,
    param: impl rusqlite::ToSql,
) -> rusqlite::Result<Vec<Lookup>> {
    conn.prepare(&format!(
        "SELECT at_ms, src, transport, COALESCE(name, ''), acme, detail FROM events
         WHERE {filter} AND proto = 'dns'
         ORDER BY at_ms DESC LIMIT 2000"
    ))?
    .query_map([param], |r| {
        Ok(Lookup {
            at: r.get(0)?,
            src: r.get(1)?,
            transport: r.get(2)?,
            name: r.get(3)?,
            acme: r.get(4)?,
            detail: r
                .get::<_, Option<String>>(5)?
                .and_then(|d| serde_json::from_str(&d).ok())
                .unwrap_or_default(),
        })
    })?
    .collect()
}

fn lookup_table(
    cfg: &Config,
    table: &asn::Table,
    own: Option<i64>,
    base: Option<i64>,
    rows: &[Lookup],
) -> Markup {
    let own = own.map(name_label);
    let clean = |s: &str| {
        let s = scrub(cfg, s);
        match &own {
            Some(o) => s.replace(o.as_str(), "[name]"),
            None => s,
        }
    };
    html! {
        @if rows.is_empty() {
            p { "No lookups yet." }
        } @else {
            div.wrap { table {
                tr {
                    th { "time" }
                    @if base.is_some() { th { "since issue" } }
                    th { "resolver" } th { "network" } th { "type" } th { "name" }
                    th { "client subnet" } th { "" }
                }
                @for l in rows {
                    tr {
                        td { (stamp(l.at)) }
                        @if base.is_some() { td { (after(base, Some(l.at))) } }
                        td { (l.src) }
                        td.text { (network(cfg, table, &l.src)) }
                        td { (l.detail["qtype"].as_str().unwrap_or("")) " " span.muted { (l.transport) } }
                        td.text { (clean(&l.name)) }
                        td { (l.detail["ecs"].as_str().unwrap_or("")) }
                        td.muted { @if l.acme { "Let's Encrypt" } }
                    }
                }
            } }
        }
    }
}

fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

struct Contact {
    at: i64,
    src: String,
    transport: String,
    proto: String,
    port: u16,
    name: String,
    summary: String,
    detail: serde_json::Value,
    raw: Option<Vec<u8>>,
}

fn connections(
    conn: &Connection,
    filter: &str,
    param: impl rusqlite::ToSql,
) -> rusqlite::Result<Vec<Contact>> {
    conn.prepare(&format!(
        "SELECT at_ms, src, transport, proto, dst_port, COALESCE(name, ''), summary, detail, raw FROM events
         WHERE {filter} AND proto NOT IN ('dns', 'http', 'https') AND acme = 0
         ORDER BY at_ms DESC LIMIT 2000"
    ))?
    .query_map([param], |r| {
        Ok(Contact {
            at: r.get(0)?,
            src: r.get(1)?,
            transport: r.get(2)?,
            proto: r.get(3)?,
            port: r.get(4)?,
            name: r.get(5)?,
            summary: r.get(6)?,
            detail: r
                .get::<_, Option<String>>(7)?
                .and_then(|d| serde_json::from_str(&d).ok())
                .unwrap_or_default(),
            raw: r.get(8)?,
        })
    })?
    .collect()
}

fn printable(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &b in &bytes[..bytes.len().min(1024)] {
        match b {
            b'\n' => out.push_str("\\n\n"),
            b'\r' => out.push_str("\\r"),
            0x20..=0x7e => out.push(b as char),
            _ => out.push_str(&format!("\\x{b:02x}")),
        }
    }
    out
}

fn connection_table(
    cfg: &Config,
    table: &asn::Table,
    own: Option<i64>,
    base: Option<i64>,
    rows: &[Contact],
) -> Markup {
    let own = own.map(name_label);
    let clean = |s: &str| {
        let s = scrub(cfg, s);
        match &own {
            Some(o) => s.replace(o.as_str(), "[name]"),
            None => s,
        }
    };
    html! {
        @if rows.is_empty() {
            p { "No other connections yet." }
        } @else {
            div.wrap { table {
                tr {
                    th { "time" }
                    @if base.is_some() { th { "since issue" } }
                    th { "source" } th { "network" } th { "port" } th { "protocol" } th { "name" } th { "what happened" }
                }
                @for c in rows {
                    tr {
                        td { (stamp(c.at)) }
                        @if base.is_some() { td { (after(base, Some(c.at))) } }
                        td { (c.src) }
                        td.text { (network(cfg, table, &c.src)) }
                        td { (c.transport) " " (c.port) }
                        td { (c.proto) }
                        td.text { (clean(&c.name)) }
                        td.text {
                            @if c.raw.is_some() || c.detail["ja4"].is_string() {
                                details {
                                    summary { (clean(&c.summary)) }
                                    pre {
                                        @if let Some(ja4) = c.detail["ja4"].as_str() { "JA4: " (ja4) "\n" }
                                        @if let Some(alpn) = c.detail["alpn"].as_array().filter(|a| !a.is_empty()) {
                                            "ALPN: " (alpn.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", ")) "\n"
                                        }
                                        @if let Some(raw) = &c.raw { (clean(&printable(raw))) }
                                    }
                                }
                            } @else {
                                (clean(&c.summary))
                            }
                        }
                    }
                }
            } }
        }
    }
}

struct Log<'a> {
    title: &'a str,
    intro: Markup,
    own: Option<i64>,
    base: Option<i64>,
    lookups: Option<&'a [Lookup]>,
    requests: &'a [Request],
    others: Option<&'a [Contact]>,
    summary: Option<Markup>,
}

fn render_log(cfg: &Config, table: &asn::Table, log: Log) -> String {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (log.title) }
                style { (PreEscaped(CSS)) }
            }
            body { main {
                p { a href="/" { "Back to the overview" } }
                h1 { (log.title) }
                p { (log.intro) }
                @if let Some(s) = log.summary { (s) }
                p { "Newest first, up to 2,000 of each. Click a request to see its headers and body." }
                @if let Some(lookups) = log.lookups {
                    h2 { "DNS lookups (" (count(lookups.len(), "lookup", "lookups")) ")" }
                    p { "Let's Encrypt's own validation queries are labelled and left out of every count on the overview." }
                    (lookup_table(cfg, table, log.own, log.base, lookups))
                }
                h2 { "HTTP requests (" (count(log.requests.len(), "request", "requests")) ")" }
                (request_table(cfg, table, log.own, log.base, log.requests))
                @if let Some(others) = log.others {
                    h2 { "Other connections (" (count(others.len(), "connection", "connections")) ")" }
                    p { "TLS handshakes, SSH, telnet and anything else that was not an HTTP request. Click one to see the bytes it sent." }
                    (connection_table(cfg, table, log.own, log.base, others))
                }
            } }
        }
    }
    .into_string()
}

async fn day_page(
    State(web): State<Arc<Web>>,
    Path(text): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let day = chrono::NaiveDate::parse_from_str(&text, "%Y-%m-%d")
        .map(|d| names::day_of(d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp()))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let today = names::day_of(now_ms() / 1000);
    if !(web.shared.cfg.launch_day..=today).contains(&day) {
        return Err(StatusCode::NOT_FOUND);
    }
    let w = web.clone();
    tokio::task::spawn_blocking(move || {
        let cfg = &w.shared.cfg;
        let table = w.asn.get(&cfg.state.join("ip2asn-combined.tsv.gz"));
        let conn = store::open(&w.shared.store.path)?;
        let issued: Option<i64> = conn
            .query_row(
                "SELECT issued_at_ms FROM certs WHERE day = ?1",
                [day],
                |r| r.get(0),
            )
            .optional()?
            .flatten();
        let rows = requests(
            &conn,
            "day = ?1 AND (name IS NOT NULL OR instr(dst, ':') > 0)",
            day,
        )?;
        let dns = lookups(&conn, "day = ?1", day)?;
        let other = connections(
            &conn,
            "day = ?1 AND (name IS NOT NULL OR instr(dst, ':') > 0)",
            day,
        )?;
        let intro = html! {
            "Every DNS lookup and HTTP request that used the name for this day."
            @if let Some(i) = issued { " Its certificate was issued at " (time(i)) " UTC." }
        };
        let title = format!("Activity for {}", date(day));
        Ok::<_, rusqlite::Error>(render_log(
            cfg,
            &table,
            Log {
                title: &title,
                intro,
                own: Some(day),
                base: issued,
                lookups: Some(&dns),
                requests: &rows,
                others: Some(&other),
                summary: None,
            },
        ))
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map(Html)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn name_page(
    State(web): State<Arc<Web>>,
    Path(label): Path<String>,
) -> Result<Html<String>, StatusCode> {
    if scrub(&web.shared.cfg, &label) != label {
        return Err(StatusCode::NOT_FOUND);
    }
    let w = web.clone();
    tokio::task::spawn_blocking(move || {
        let cfg = &w.shared.cfg;
        let table = w.asn.get(&cfg.state.join("ip2asn-combined.tsv.gz"));
        let conn = store::open(&w.shared.store.path)?;
        let (filter, value, title, intro) = if label == "@" {
            (
                "via = ?1",
                "apex".to_string(),
                "Activity for the bare domain".to_string(),
                html! { "Every DNS lookup and HTTP request that used the bare domain, with no subdomain." },
            )
        } else {
            (
                "label = ?1",
                label.clone(),
                format!("Activity for {label}"),
                html! { "Every DNS lookup and HTTP request that used " code { (label) } ", a name that was never published." },
            )
        };
        let rows = requests(&conn, filter, &value)?;
        let dns = lookups(&conn, filter, &value)?;
        let other = connections(&conn, filter, &value)?;
        Ok::<_, rusqlite::Error>(render_log(
            cfg,
            &table,
            Log {
                title: &title,
                intro,
                own: None,
                base: None,
                lookups: Some(&dns),
                requests: &rows,
                others: Some(&other),
                summary: None,
            },
        ))
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map(Html)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn addr_page(
    State(web): State<Arc<Web>>,
    Path(text): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let ip: IpAddr = text.parse().map_err(|_| StatusCode::NOT_FOUND)?;
    let cfg = &web.shared.cfg;
    let today = names::day_of(now_ms() / 1000);
    let role = match ip {
        IpAddr::V4(a)
            if (u32::from(cfg.apex4)..u32::from(cfg.apex4) + 32).contains(&u32::from(a)) =>
        {
            role_of(cfg, a, today)
        }
        IpAddr::V6(a) if u128::from(a) >> 64 == u128::from(cfg.prefix6) >> 64 => "IPv6".to_string(),
        _ => return Err(StatusCode::NOT_FOUND),
    };
    let w = web.clone();
    tokio::task::spawn_blocking(move || {
        let cfg = &w.shared.cfg;
        let table = w.asn.get(&cfg.state.join("ip2asn-combined.tsv.gz"));
        let conn = store::open(&w.shared.store.path)?;
        let dst = ip.to_string();
        let rows = requests(&conn, "dst = ?1", &dst)?;
        let dns = lookups(&conn, "dst = ?1", &dst)?;
        let other = connections(&conn, "dst = ?1", &dst)?;
        let top: Vec<(String, i64, i64, String)> = conn
            .prepare(
                "SELECT src, COUNT(*), COUNT(DISTINCT dst_port), group_concat(DISTINCT proto) FROM events
                 WHERE dst = ?1 AND acme = 0 GROUP BY src ORDER BY 2 DESC LIMIT 15",
            )?
            .query_map([&dst], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let summary = html! {
            h2 { "Top sources" }
            div.wrap { table {
                tr { th { "source" } th { "network" } th.n { "events" } th.n { "ports" } th { "protocols" } }
                @for (src, n, ports, protos) in &top {
                    tr {
                        td { (src) }
                        td.text { (network(cfg, &table, src)) }
                        td.n { (n) } td.n { (ports) }
                        td.text { (protos.replace(',', ", ")) }
                    }
                }
            } }
        };
        let title = format!("Traffic to {dst}");
        let intro = html! {
            "Everything sent to " (dst) " (" (role) "), from anyone, on any name or port."
        };
        Ok::<_, rusqlite::Error>(render_log(
            cfg,
            &table,
            Log {
                title: &title,
                intro,
                own: None,
                base: None,
                lookups: Some(&dns),
                requests: &rows,
                others: Some(&other),
                summary: Some(summary),
            },
        ))
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map(Html)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn asn_page(
    State(web): State<Arc<Web>>,
    Path(number): Path<u32>,
) -> Result<Html<String>, StatusCode> {
    let w = web.clone();
    tokio::task::spawn_blocking(move || {
        let cfg = &w.shared.cfg;
        let table = w.asn.get(&cfg.state.join("ip2asn-combined.tsv.gz"));
        let conn = store::open(&w.shared.store.path)?;
        let mut owner = None;
        let mut ips = Vec::new();
        let srcs: Vec<String> = conn
            .prepare("SELECT DISTINCT src FROM events")?
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        for src in srcs {
            if let Some(a) = src
                .parse::<IpAddr>()
                .ok()
                .and_then(|ip| table.lookup(ip))
                .filter(|a| a.number == number)
            {
                owner.get_or_insert_with(|| a.clone());
                ips.push(src);
            }
        }
        let Some(owner) = owner else {
            return Ok(None);
        };
        let list = serde_json::to_string(&ips).unwrap_or_default();
        let filter = "src IN (SELECT value FROM json_each(?1))";
        let rows = requests(&conn, filter, &list)?;
        let dns = lookups(&conn, filter, &list)?;
        let other = connections(&conn, filter, &list)?;
        let title = format!("AS{} {}", owner.number, scrub(cfg, &owner.name));
        let intro = html! {
            "Everything sent from " (count(ips.len(), "address", "addresses"))
            " in AS" (owner.number) " (" (owner.country) "), on any name or address."
        };
        Ok::<_, rusqlite::Error>(Some(render_log(
            cfg,
            &table,
            Log {
                title: &title,
                intro,
                own: None,
                base: None,
                lookups: Some(&dns),
                requests: &rows,
                others: Some(&other),
                summary: None,
            },
        )))
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map(Html)
    .ok_or(StatusCode::NOT_FOUND)
}

async fn ua_page(
    State(web): State<Arc<Web>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Html<String>, StatusCode> {
    let q = params.get("q").cloned().unwrap_or_default();
    if scrub(&web.shared.cfg, &q) != q {
        return Err(StatusCode::NOT_FOUND);
    }
    let w = web.clone();
    tokio::task::spawn_blocking(move || {
        let cfg = &w.shared.cfg;
        let table = w.asn.get(&cfg.state.join("ip2asn-combined.tsv.gz"));
        let conn = store::open(&w.shared.store.path)?;
        let rows = if q.is_empty() {
            Vec::new()
        } else {
            requests(&conn, "instr(json_extract(detail, '$.ua'), ?1) > 0", &q)?
        };
        let intro = html! {
            form.search action="/ua" method="get" {
                input type="search" name="q" value=(q) placeholder="Part of a user agent" size="60";
                " "
                button type="submit" { "Search" }
            }
            "Every HTTP request whose user agent contains this text, on any name."
        };
        Ok::<_, rusqlite::Error>(render_log(
            cfg,
            &table,
            Log {
                title: "Requests by user agent",
                intro,
                own: None,
                base: None,
                lookups: None,
                requests: &rows,
                others: None,
                summary: None,
            },
        ))
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map(Html)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn page(State(web): State<Arc<Web>>) -> Response {
    match web.cache.lock().unwrap().as_ref() {
        Some((_, body)) => Html(body.clone()).into_response(),
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            [("retry-after", "30")],
            Html(r#"<!doctype html><meta http-equiv="refresh" content="30"><p>The report is still being built, this page reloads itself.</p>"#),
        )
            .into_response(),
    }
}

async fn refresh(web: Arc<Web>) {
    loop {
        let started = Instant::now();
        let w = web.clone();
        let body = tokio::task::spawn_blocking(move || {
            let cfg = &w.shared.cfg;
            let table = w.asn.get(&cfg.state.join("ip2asn-combined.tsv.gz"));
            let conn = store::open(&w.shared.store.path)?;
            Ok::<_, rusqlite::Error>(render(cfg, &report(&conn, cfg, &table)?))
        })
        .await;
        match body {
            Ok(Ok(body)) => {
                *web.cache.lock().unwrap() = Some((Instant::now(), body));
                eprintln!("report built in {:.1}s", started.elapsed().as_secs_f64());
            }
            Ok(Err(e)) => eprintln!("report: {e}"),
            Err(e) => eprintln!("report: {e}"),
        }
        tokio::time::sleep(Duration::from_secs(300)).await;
    }
}

pub async fn run(shared: Arc<Shared>) -> std::io::Result<()> {
    let addr = shared.cfg.web;
    let web = Arc::new(Web {
        shared,
        asn: asn::Cache::default(),
        cache: Mutex::new(None),
    });
    tokio::spawn(refresh(web.clone()));
    let app = Router::new()
        .route("/", get(page))
        .route("/day/{date}", get(day_page))
        .route("/name/{label}", get(name_page))
        .route("/ua", get(ua_page))
        .route("/asn/{number}", get(asn_page))
        .route("/addr/{ip}", get(addr_page))
        .with_state(web);
    let mut tries = 0;
    let listener = loop {
        match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => break l,
            Err(e) if tries < 30 && e.kind() == std::io::ErrorKind::AddrNotAvailable => {
                tries += 1;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Err(e) => return Err(e),
        }
    };
    axum::serve(listener, app).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrubbing_hides_the_zone_and_daily_names() {
        let cfg = crate::config::test();
        let today = names::day_of(now_ms() / 1000);
        let name = cfg.key.name(today);
        let text = format!("GET http://{name}.StartGuess.ing/x?h=www.startguess.ing");
        let out = scrub(&cfg, &text);
        assert!(!out.to_ascii_lowercase().contains("startguess"), "{out}");
        assert!(!out.contains(&name), "{out}");
        assert!(out.contains(&format!("[name {}]", date(today))), "{out}");
        assert_eq!(scrub(&cfg, "curl/8.5.0"), "curl/8.5.0");
    }

    #[test]
    fn the_request_log_hides_names_in_headers_and_bodies() {
        let cfg = crate::config::test();
        let today = names::day_of(now_ms() / 1000);
        let host = format!("{}.startguess.ing", cfg.key.name(today));
        let req = Request {
            at: today * DAY_MS,
            proto: "https".into(),
            src: "198.51.100.7".into(),
            port: 443,
            summary: format!("GET https://{host}/wp-login.php"),
            detail: serde_json::json!({"headers": [["host", host], ["referer", format!("https://{host}/")]], "ua": "curl/8"}),
            body: Some(format!("u=admin&next={host}").into_bytes()),
        };
        let lookup = Lookup {
            at: today * DAY_MS,
            src: "192.0.2.53".into(),
            transport: "udp".into(),
            name: format!("_acme-challenge.{host}"),
            acme: true,
            detail: serde_json::json!({"qtype": "TXT", "ecs": "198.51.100.0/24"}),
        };
        let html = render_log(
            &cfg,
            &asn::Table::default(),
            Log {
                title: "t",
                intro: html! {},
                own: Some(today),
                base: None,
                lookups: Some(&[lookup]),
                requests: &[req],
                others: None,
                summary: None,
            },
        );
        assert!(!html.to_ascii_lowercase().contains("startguess"), "{html}");
        assert!(html.contains("/wp-login.php"));
        assert!(html.contains("curl/8"));
        assert!(html.contains("/ua?q=curl%2F8"));
        assert!(html.contains("_acme-challenge.[name].[domain]"), "{html}");
        assert!(html.contains("198.51.100.0/24"));
    }

    #[test]
    fn the_page_renders_from_an_empty_database() {
        let s = crate::shared::test();
        let conn = store::open(&s.store.path).unwrap();
        let html = render(
            &s.cfg,
            &report(&conn, &s.cfg, &asn::Table::default()).unwrap(),
        );
        assert!(html.contains("CT honeypot"));
        assert!(!html.contains("startguess"));
    }
}
