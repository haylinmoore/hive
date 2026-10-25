use crate::chain::Link;
use crate::record::Record;
use maud::{DOCTYPE, Markup, PreEscaped, html};
use std::collections::BTreeSet;

const CSS: &str = r#"
:root { --fg: #1c1917; --muted: #8a8580; --accent: #c2410c; --bg: #fafaf9; color-scheme: light; }
@media (prefers-color-scheme: dark) { :root { --fg: #e7e5e4; --muted: #8f8a86; --accent: #fb923c; --bg: #161514; color-scheme: dark; } }
body { margin: 0; background: var(--bg); color: var(--fg); font: 13px/1.55 ui-monospace, "Berkeley Mono", "JetBrains Mono", Menlo, monospace; }
a { color: inherit; }
header, main { max-width: 1100px; margin: 0 auto; }
header { padding: 16px 16px 0; }
header a { text-decoration: none; }
header .nav { margin-left: 20px; }
header .nav a { margin-right: 12px; text-decoration: underline; }
ul.tree, ul.tree ul { list-style: none; margin: 0; padding-left: 18px; }
ul.tree { padding-left: 0; }
ul.tree summary { cursor: pointer; }
ul.tree li > a, ul.tree summary > a { margin-right: 8px; }
.crumbs a { margin-right: 4px; }
main { padding: 8px 16px 32px; }
h1 { font-size: 13px; font-weight: 700; margin: 0 0 10px; }
h2 { font-size: 13px; font-weight: 400; color: var(--muted); margin: 20px 0 6px; }
p { margin: 0 0 10px; }
.muted { color: var(--muted); }
.new { color: var(--accent); }
.filter a, .pager > * { margin-right: 12px; }
.filter .on { font-weight: 700; text-decoration: none; }
.scroll { overflow-x: auto; }
table { border-collapse: collapse; width: 100%; }
th, td { text-align: left; padding: 2px 16px 2px 0; white-space: nowrap; vertical-align: top; }
th { color: var(--muted); font-weight: 400; }
td.wrap { white-space: normal; word-break: break-all; }
tr.expired td { opacity: .5; }
dl { display: grid; grid-template-columns: max-content 1fr; gap: 1px 20px; margin: 0; }
dt { color: var(--muted); }
dd { margin: 0; word-break: break-all; }
pre { overflow-x: auto; margin: 0; }
details.raw > summary { cursor: pointer; color: var(--muted); margin: 20px 0 6px; }
"#;

const LIVE_JS: &str = r#"
const live = document.querySelector('main');
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
new EventSource('/live' + location.search).onmessage = (e) => apply(e.data);
"#;

const TREE_JS: &str = r#"
document.addEventListener('click', (e) => {
  const summary = e.target.closest('summary');
  if (!summary || !e.shiftKey || e.target.closest('a')) return;
  e.preventDefault();
  const details = summary.parentElement;
  const open = !details.open;
  details.open = open;
  details.querySelectorAll('details').forEach((d) => (d.open = open));
});
"#;

fn layout(title: &str, live: bool, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " · certs.hayl.in" }
                style { (PreEscaped(CSS)) }
            }
            body {
                header {
                    a href="/" { "certs.hayl.in" }
                    span.nav { a href="/" { "log" } a href="/tree" { "tree" } }
                }
                main { (body) }
                @if live {
                    script { (PreEscaped(LIVE_JS)) }
                }
            }
        }
    }
}

pub fn until(now: i64, then: i64) -> String {
    let d = then - now;
    match d {
        ..=0 => "expired".to_string(),
        1..3600 => format!("in {}m", d / 60),
        3600..86400 => format!("in {}h", d / 3600),
        _ => format!("in {}d", d / 86400),
    }
}

pub fn date(t: i64) -> String {
    chrono::DateTime::from_timestamp(t, 0)
        .map(|d| d.format("%b %-d %Y %H:%M").to_string())
        .unwrap_or_default()
}

fn names(r: &Record) -> String {
    match r.dns_names.len() {
        0 => String::new(),
        1..=3 => r.dns_names.join(" "),
        n => format!("{} +{} more", r.dns_names[..3].join(" "), n - 3),
    }
}

pub fn firsts(records: &[Record]) -> Vec<bool> {
    let mut order: Vec<usize> = (0..records.len()).collect();
    order.sort_by_key(|&i| records[i].issued());
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut out = vec![false; records.len()];
    for i in order {
        let r = &records[i];
        out[i] = !r.dns_names.iter().any(|n| seen.contains(n.as_str()));
        seen.extend(r.dns_names.iter().map(String::as_str));
    }
    out
}

pub const PER_PAGE: usize = 200;

fn href(hive_only: bool, page: usize) -> String {
    match (hive_only, page) {
        (false, 1) => "/".to_string(),
        (true, 1) => "/?only=hive".to_string(),
        (false, p) => format!("/?page={p}"),
        (true, p) => format!("/?only=hive&page={p}"),
    }
}

pub fn tables(records: &[Record], hive_only: bool, page: usize, now: i64) -> Markup {
    let first = firsts(records);
    let pages = records.len().div_ceil(PER_PAGE).max(1);
    let page = page.clamp(1, pages);
    let start = (page - 1) * PER_PAGE;
    let pager = html! {
        p.pager {
            @if page > 1 { a href=(href(hive_only, page - 1)) { "newer" } } @else { span.muted { "newer" } }
            span.muted { (page) " of " (pages) }
            @if page < pages { a href=(href(hive_only, page + 1)) { "older" } } @else { span.muted { "older" } }
        }
    };
    html! {
        p.filter {
            a.on[!hive_only] href="/" { "all" }
            a.on[hive_only] href="/?only=hive" { "hive" }
        }
        @if records.is_empty() {
            p.muted { "none yet" }
        } @else {
            div.scroll { table {
                tr { th { "issued" } th { "" } th { "names" } th { "issuer" } th { "valid until" } }
                @for (r, new) in records.iter().zip(&first).skip(start).take(PER_PAGE) {
                    tr.expired[!r.active(now)] {
                        td { a href={ "/cert/" (r.cert_sha256) } { (date(r.issued())) } }
                        td { @if *new { span.new { "new" } } @else { span.muted { "renewed" } } }
                        td.wrap { (names(r)) }
                        td { (r.issuer_name()) }
                        td { (r.not_after.map(date).unwrap_or_default()) }
                    }
                }
            } }
            @if pages > 1 { (pager) }
        }
    }
}

pub fn index(records: &[Record], hive_only: bool, page: usize, now: i64) -> Markup {
    layout("certificates", true, tables(records, hive_only, page, now))
}

pub fn cert(r: &Record, links: &[Link], now: i64) -> Markup {
    let title = r
        .dns_names
        .first()
        .map(String::as_str)
        .unwrap_or("certificate");
    html! {
        (layout(title, false, html! {
            h1 { (title) }
            dl {
                dt { "names" } dd { (r.dns_names.join(" ")) }
                dt { "issuer" } dd { (r.issuer.as_deref().unwrap_or("")) }
                @if let Some(serial) = &r.serial { dt { "serial" } dd { (serial) } }
                dt { "valid from" } dd { (r.not_before.map(date).unwrap_or_default()) }
                dt { "until" } dd { (r.not_after.map(|t| format!("{} ({})", date(t), until(now, t))).unwrap_or_default()) }
                @if r.log_uri.is_empty() {
                    dt { "from" } dd { (r.source.as_deref().unwrap_or("")) }
                } @else {
                    dt { "log" } dd { (r.log_uri) " entry " (r.entry_index) }
                    dt { "seen" } dd { (date(r.discovered_at)) }
                }
                @if !r.cert_sha256.starts_with("crtsh-") { dt { "sha-256" } dd { (r.cert_sha256) } }
                @if !r.tbs_sha256.is_empty() { dt { "tbs sha-256" } dd { (r.tbs_sha256) } }
                dt { "elsewhere" } dd { a href=(r.crt_sh()) { "crt.sh" } }
            }
            @if links.len() > 1 {
                h2 { "chain" }
                (chain_table(links, now))
            }
            @if let Some(pem) = &r.pem { (decoded(pem)) }
        }))
    }
}

fn decoded(pem: &str) -> Markup {
    html! {
        h2 { "certificate" }
        dl {
            @for (label, values) in crate::decode::fields(pem) {
                dt { (label) }
                dd { @for v in &values { div { (v) } } }
            }
        }
        details.raw {
            summary { "pem" }
            pre { (pem) }
        }
    }
}

fn chain_table(links: &[Link], now: i64) -> Markup {
    html! {
        div.scroll { table {
            tr { th { "certificate" } th { "signed by" } th { "valid until" } }
            @for (i, l) in links.iter().enumerate() {
                tr {
                    td {
                        @if i == 0 {
                            "this certificate"
                        } @else {
                            a href={ "/ca/" (l.sha256) } { (l.subject_cn) }
                        }
                    }
                    td { @if l.self_signed() { span.muted { "itself" } } @else { (crate::record::org_cn(&l.issuer)) } }
                    td { (date(l.not_after)) " (" (until(now, l.not_after)) ")" }
                }
            }
        } }
    }
}

pub fn ca(link: &Link, above: &[Link], issued: &[&Record], now: i64) -> Markup {
    layout(
        &link.subject_cn,
        false,
        html! {
            h1 { (link.subject_cn) }
            dl {
                dt { "subject" } dd { (link.subject) }
                dt { "issuer" } dd {
                    @if link.self_signed() {
                        "itself, a root"
                    } @else if let Some(up) = above.first() {
                        a href={ "/ca/" (up.sha256) } { (link.issuer) }
                    } @else {
                        (link.issuer)
                    }
                }
                dt { "valid from" } dd { (date(link.not_before)) }
                dt { "until" } dd { (date(link.not_after)) " (" (until(now, link.not_after)) ")" }
                dt { "sha-256" } dd { (link.sha256) }
                dt { "elsewhere" } dd { a href={ "https://crt.sh/?sha256=" (link.sha256) } { "crt.sh" } }
            }
            @if !above.is_empty() {
                h2 { "signed by" }
                (chain_table(&[std::slice::from_ref(link), above].concat(), now))
            }
            @if !issued.is_empty() {
                h2 { "issued" }
                div.scroll { table {
                    tr { th { "issued" } th { "names" } th { "valid until" } }
                    @for r in issued {
                        tr {
                            td { a href={ "/cert/" (r.cert_sha256) } { (date(r.issued())) } }
                            td.wrap { (names(r)) }
                            td { (r.not_after.map(date).unwrap_or_default()) }
                        }
                    }
                } }
            }
            (decoded(&link.pem))
        },
    )
}

fn branch(full: &str, node: &crate::tree::Node, depth: usize) -> Markup {
    let link = html! {
        a href={ "/d/" (full) } { (if depth == 0 { full } else { crate::tree::label(full) }) }
        span.muted { (node.total) }
    };
    html! {
        li {
            @if node.children.is_empty() {
                (link)
            } @else {
                details open[depth == 0] {
                    summary { (link) }
                    ul { @for (child, n) in &node.children { (branch(child, n, depth + 1)) } }
                }
            }
        }
    }
}

pub fn tree(root: &crate::tree::Node, live_only: bool) -> Markup {
    layout(
        "tree",
        false,
        html! {
            p.filter {
                a.on[!live_only] href="/tree" { "all" }
                a.on[live_only] href="/tree?live=1" { "live" }
            }
            ul.tree { @for (tld, n) in &root.children { (branch(tld, n, 0)) } }
        script { (PreEscaped(TREE_JS)) }
        },
    )
}

pub fn domain(
    name: &str,
    node: Option<&crate::tree::Node>,
    records: &[Record],
    now: i64,
) -> Markup {
    let labels: Vec<&str> = name.split('.').collect();
    let crumbs: Vec<String> = (0..labels.len())
        .rev()
        .map(|i| labels[i..].join("."))
        .collect();
    let first = firsts(records);
    layout(
        name,
        false,
        html! {
            p.crumbs {
                @for (i, c) in crumbs.iter().enumerate() {
                    @if i > 0 { span.muted { "/ " } }
                    @if c == name { b { (crate::tree::label(c)) } } @else { a href={ "/d/" (c) } { (crate::tree::label(c)) } }
                }
            }
            h1 { (name) }
            @if let Some(node) = node {
                @if !node.children.is_empty() {
                    p {
                        span.muted { "under it: " }
                        @for (child, n) in &node.children {
                            a href={ "/d/" (child) } { (crate::tree::label(child)) } span.muted { " " (n.total) "  " }
                        }
                    }
                }
            }
            @if records.is_empty() {
                p.muted { "no certificates" }
            } @else {
                div.scroll { table {
                    tr { th { "issued" } th { "" } th { "names" } th { "issuer" } th { "valid until" } }
                    @for (r, new) in records.iter().zip(&first) {
                        tr.expired[!r.active(now)] {
                            td { a href={ "/cert/" (r.cert_sha256) } { (date(r.issued())) } }
                            td { @if *new { span.new { "new" } } @else { span.muted { "renewed" } } }
                            td.wrap { (names(r)) }
                            td { (r.issuer_name()) }
                            td { (r.not_after.map(date).unwrap_or_default()) }
                        }
                    }
                } }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(sha: &str, names: &[&str], issued: i64, until: i64) -> Record {
        Record {
            tbs_sha256: sha.into(),
            cert_sha256: sha.into(),
            zone: "hayl.in".into(),
            log_uri: "https://log.example/".into(),
            entry_index: 1,
            issuer: Some("CN=YE2".into()),
            serial: Some("06".into()),
            not_before: Some(issued),
            not_after: Some(until),
            dns_names: names.iter().map(|s| s.to_string()).collect(),
            discovered_at: issued,
            pem: Some("<script>".into()),
            chain: Vec::new(),
            source: None,
        }
    }

    #[test]
    fn long_name_lists_are_cut_short() {
        let r = record("a", &["a", "b", "c", "d", "e"], 0, 1);
        assert_eq!(names(&r), "a b c +2 more");
    }

    #[test]
    fn pages_escape_what_the_ca_put_in_the_cert() {
        let page = cert(&record("a", &["<b>x</b>"], 0, 1), &[], 10).into_string();
        assert!(!page.contains("<script>"));
        assert!(!page.contains("<b>x</b>"));
    }

    #[test]
    fn the_first_cert_for_a_name_is_new_and_later_ones_renew_it() {
        let records = vec![
            record("c", &["x.hayl.in"], 300, 900),
            record("b", &["y.hayl.in"], 200, 900),
            record("a", &["x.hayl.in"], 100, 200),
        ];
        assert_eq!(firsts(&records), vec![false, true, true]);
    }

    #[test]
    fn the_log_pages_every_two_hundred() {
        let records: Vec<Record> = (0..450)
            .map(|i| record(&format!("c{i}"), &["x.hayl.in"], 1000 - i, 2000))
            .collect();
        let html = |p| tables(&records, true, p, 0).into_string();
        assert_eq!(html(1).matches("<tr").count(), 201);
        assert_eq!(html(3).matches("<tr").count(), 51);
        assert!(html(1).contains("1 of 3"));
        assert!(html(2).contains("href=\"/?only=hive&amp;page=3\""));
        assert!(
            html(9).contains("3 of 3"),
            "out of range clamps to the last page"
        );
    }

    #[test]
    fn expired_certs_stay_in_the_log_but_greyed() {
        let records = vec![
            record("live", &["live.hayl.in"], 100, 1000),
            record("gone", &["gone.hayl.in"], 100, 200),
        ];
        let html = tables(&records, false, 1, 500).into_string();
        assert!(html.contains("live.hayl.in") && html.contains("gone.hayl.in"));
        assert_eq!(html.matches("class=\"expired\"").count(), 1);
    }
}
