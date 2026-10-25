use crate::chain::{self, Link};
use crate::record::Record;
use maud::{DOCTYPE, Markup, PreEscaped, html};
use std::collections::BTreeSet;

const CSS: &str = r#"
:root { --bg: #fafaf9; --fg: #1c1917; --muted: #78716c; --line: #e7e5e4; --panel: #f5f5f4; --accent: #c2410c; color-scheme: light; }
@media (prefers-color-scheme: dark) {
  :root { --bg: #161514; --fg: #e7e5e4; --muted: #a8a29e; --line: #2e2b29; --panel: #1f1d1b; --accent: #fb923c; color-scheme: dark; }
}
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--fg); font: 14px/1.5 ui-monospace, "Berkeley Mono", "JetBrains Mono", Menlo, monospace; }
a { color: inherit; }
header { padding: 14px 16px; border-bottom: 1px solid var(--line); }
header a { font-weight: 700; text-decoration: none; }
main { padding: 16px; max-width: 1200px; }
h1 { font-size: 16px; margin: 0 0 12px; }
h2 { font-size: 14px; margin: 28px 0 8px; color: var(--muted); font-weight: 400; }
h2:first-child { margin-top: 0; }
p { margin: 0 0 12px; }
.muted { color: var(--muted); }
.new { color: var(--accent); }
.scroll { overflow-x: auto; }
table { border-collapse: collapse; width: 100%; }
th, td { text-align: left; padding: 6px 14px 6px 0; border-bottom: 1px solid var(--line); white-space: nowrap; vertical-align: top; }
th { color: var(--muted); font-weight: 400; }
td.wrap { white-space: normal; word-break: break-all; }
dl { display: grid; grid-template-columns: max-content 1fr; gap: 4px 20px; margin: 0; }
dt { color: var(--muted); }
dd { margin: 0; word-break: break-all; }
pre { background: var(--panel); border: 1px solid var(--line); padding: 12px; overflow-x: auto; margin: 0; }
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
new EventSource('/live').onmessage = (e) => apply(e.data);
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
                header { a href="/" { "certs.hayl.in" } }
                main { (body) }
                @if live {
                    script { (PreEscaped(LIVE_JS)) }
                }
            }
        }
    }
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

pub fn tables(records: &[Record], now: i64) -> Markup {
    let mut live: Vec<&Record> = records.iter().filter(|r| r.active(now)).collect();
    live.sort_by(|a, b| {
        a.dns_names
            .cmp(&b.dns_names)
            .then(b.issued().cmp(&a.issued()))
    });
    let first = firsts(records);
    html! {
        h2 { "live" }
        @if live.is_empty() {
            p.muted { "none yet" }
        } @else {
            div.scroll { table {
                tr { th { "names" } th { "issuer" } th { "issued" } th { "expires" } }
                @for r in &live {
                    tr {
                        td.wrap { a href={ "/cert/" (r.cert_sha256) } { (names(r)) } }
                        td { (r.issuer_cn()) }
                        td { (ago(now, r.issued())) }
                        td { (r.not_after.map(|t| until(now, t)).unwrap_or_default()) }
                    }
                }
            } }
        }
        h2 { "log" }
        @if records.is_empty() {
            p.muted { "none yet" }
        } @else {
            div.scroll { table {
                tr { th { "issued" } th { "" } th { "names" } th { "issuer" } th { "valid until" } }
                @for (r, new) in records.iter().zip(&first) {
                    tr {
                        td { a href={ "/cert/" (r.cert_sha256) } { (date(r.issued())) } }
                        td { @if *new { span.new { "new" } } @else { span.muted { "renewed" } } }
                        td.wrap { (names(r)) }
                        td { (r.issuer_cn()) }
                        td { (r.not_after.map(date).unwrap_or_default()) }
                    }
                }
            } }
        }
    }
}

pub fn index(records: &[Record], now: i64) -> Markup {
    layout("certificates", true, tables(records, now))
}

pub fn cert(r: &Record, now: i64) -> Markup {
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
            @let links = r.pem.as_deref().map(chain::parse).unwrap_or_default();
            @if links.len() > 1 {
                h2 { "chain" }
                (chain_table(&links, now))
            }
            @if let Some(pem) = &r.pem {
                h2 { "certificate" }
                pre { (pem) }
            }
        }))
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
                    td { @if l.self_signed() { span.muted { "itself" } } @else { (l.issuer_cn) } }
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
            h2 { "certificate" }
            pre { (link.pem) }
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
        let page = cert(&record("a", &["<b>x</b>"], 0, 1), 10).into_string();
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
    fn only_unexpired_certs_are_live() {
        let records = vec![
            record("live", &["live.hayl.in"], 100, 1000),
            record("gone", &["gone.hayl.in"], 100, 200),
        ];
        let html = tables(&records, 500).into_string();
        let live = html.split("<h2>log</h2>").next().unwrap();
        assert!(live.contains("live.hayl.in"));
        assert!(!live.contains("gone.hayl.in"));
        assert!(html.contains("gone.hayl.in"), "the log still lists it");
    }
}
