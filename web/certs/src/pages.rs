use crate::record::Record;
use maud::{DOCTYPE, Markup, html};

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
p { margin: 0 0 12px; }
.muted { color: var(--muted); }
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

fn layout(title: &str, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " · certs.hayl.in" }
                style { (maud::PreEscaped(CSS)) }
            }
            body {
                header { a href="/" { "certs.hayl.in" } }
                main { (body) }
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

fn table(records: &[&Record], now: i64) -> Markup {
    html! {
        div.scroll { table {
            tr { th { "seen" } th { "names" } th { "issuer" } th { "valid from" } th { "until" } }
            @for r in records {
                tr {
                    td { a href={ "/cert/" (r.cert_sha256) } { (ago(now, r.discovered_at)) } }
                    td.wrap { (names(r)) }
                    td { (r.issuer_cn()) }
                    td { (r.not_before.map(date).unwrap_or_default()) }
                    td { (r.not_after.map(date).unwrap_or_default()) }
                }
            }
        } }
    }
}

pub fn index(records: &[Record], zones: &[String], now: i64) -> Markup {
    let recent: Vec<&Record> = records.iter().take(50).collect();
    layout(
        "certificates",
        html! {
            h1 { "certificates" }
            p.muted {
                "Every certificate issued for these domains, as it appears in the public Certificate Transparency logs. "
                "Watched since this page went up, not before."
            }
            div.scroll { table {
                tr { th { "zone" } th { "certificates" } th { "newest" } }
                @for zone in zones {
                    @let mine: Vec<&Record> = records.iter().filter(|r| &r.zone == zone).collect();
                    tr {
                        td { a href={ "/zone/" (zone) } { (zone) } }
                        td { (mine.len()) }
                        td { (mine.first().map(|r| ago(now, r.discovered_at)).unwrap_or_default()) }
                    }
                }
            } }
            h2 { "recent" }
            @if recent.is_empty() {
                p.muted { "nothing logged yet" }
            } @else {
                (table(&recent, now))
            }
        },
    )
}

pub fn zone(records: &[Record], zone: &str, now: i64) -> Markup {
    let mine: Vec<&Record> = records.iter().filter(|r| r.zone == zone).collect();
    layout(
        zone,
        html! {
            h1 { (zone) }
            @if mine.is_empty() {
                p.muted { "nothing logged yet" }
            } @else {
                (table(&mine, now))
            }
        },
    )
}

pub fn cert(r: &Record, now: i64) -> Markup {
    layout(
        r.dns_names
            .first()
            .map(String::as_str)
            .unwrap_or("certificate"),
        html! {
            h1 { (r.dns_names.first().map(String::as_str).unwrap_or("certificate")) }
            dl {
                dt { "names" } dd { (r.dns_names.join(" ")) }
                dt { "zone" } dd { a href={ "/zone/" (r.zone) } { (r.zone) } }
                dt { "issuer" } dd { (r.issuer.as_deref().unwrap_or("")) }
                dt { "serial" } dd { (r.serial.as_deref().unwrap_or("")) }
                dt { "valid from" } dd { (r.not_before.map(date).unwrap_or_default()) }
                dt { "until" } dd { (r.not_after.map(date).unwrap_or_default()) }
                dt { "seen" } dd { (ago(now, r.discovered_at)) " (" (date(r.discovered_at)) ")" }
                dt { "log" } dd { (r.log_uri) " entry " (r.entry_index) }
                dt { "sha-256" } dd { (r.cert_sha256) }
                dt { "tbs sha-256" } dd { (r.tbs_sha256) }
                dt { "elsewhere" } dd { a href=(r.crt_sh()) { "crt.sh" } }
            }
            @if let Some(pem) = &r.pem {
                h2 { "certificate" }
                pre { (pem) }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(zone: &str, names: &[&str]) -> Record {
        Record {
            tbs_sha256: "t".into(),
            cert_sha256: "c".into(),
            zone: zone.into(),
            log_uri: "https://log.example/".into(),
            entry_index: 1,
            issuer: Some("CN=YE2".into()),
            serial: Some("06".into()),
            not_before: Some(0),
            not_after: Some(86400),
            dns_names: names.iter().map(|s| s.to_string()).collect(),
            discovered_at: 0,
            pem: Some("<script>".into()),
        }
    }

    #[test]
    fn long_name_lists_are_cut_short() {
        let r = record("hayl.in", &["a", "b", "c", "d", "e"]);
        assert_eq!(names(&r), "a b c +2 more");
    }

    #[test]
    fn pages_escape_what_the_ca_put_in_the_cert() {
        let page = cert(&record("hayl.in", &["<b>x</b>"]), 10).into_string();
        assert!(!page.contains("<script>"));
        assert!(!page.contains("<b>x</b>"));
    }

    #[test]
    fn a_zone_page_only_lists_its_zone() {
        let records = vec![
            record("hayl.in", &["dash.hive.hayl.in"]),
            record("uwu.estate", &["ha.uwu.estate"]),
        ];
        let page = zone(&records, "uwu.estate", 10).into_string();
        assert!(page.contains("ha.uwu.estate"));
        assert!(!page.contains("dash.hive.hayl.in"));
    }
}
