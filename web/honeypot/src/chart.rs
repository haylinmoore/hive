use maud::{Markup, html};

const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;
const DAY: i64 = 24 * HOUR;

pub struct Timeline {
    pub start: i64,
    pub bucket: i64,
    pub rows: Vec<[i64; 3]>,
    pub issued: Vec<i64>,
}

pub fn bucket_for(span: i64) -> i64 {
    [
        MIN,
        5 * MIN,
        10 * MIN,
        30 * MIN,
        HOUR,
        3 * HOUR,
        6 * HOUR,
        DAY,
    ]
    .into_iter()
    .find(|b| span / b <= 144)
    .unwrap_or(DAY)
}

pub fn bucket_name(bucket: i64) -> &'static str {
    match bucket {
        MIN => "minute",
        b if b == 5 * MIN => "5 minutes",
        b if b == 10 * MIN => "10 minutes",
        b if b == 30 * MIN => "30 minutes",
        HOUR => "hour",
        b if b == 3 * HOUR => "3 hours",
        b if b == 6 * HOUR => "6 hours",
        _ => "day",
    }
}

pub fn thousands(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

pub fn compact(n: i64) -> String {
    if n < 1_000 {
        n.to_string()
    } else if n < 10_000 {
        format!("{:.1}K", n as f64 / 1e3).replace(".0K", "K")
    } else if n < 1_000_000 {
        format!("{}K", n / 1000)
    } else {
        format!("{:.1}M", n as f64 / 1e6)
    }
}

fn nice(max: i64) -> i64 {
    let mut step = 1;
    loop {
        for m in [1, 2, 5] {
            if m * step >= max {
                return m * step;
            }
        }
        step *= 10;
    }
}

fn clock(ms: i64, fmt: &str) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|t| t.format(fmt).to_string())
        .unwrap_or_default()
}

fn column(x: f64, w: f64, base: f64, h: f64) -> String {
    let r = 3f64.min(w / 2.0).min(h);
    let top = base - h;
    format!(
        "M{x:.1},{base:.1}V{:.1}Q{x:.1},{top:.1} {:.1},{top:.1}H{:.1}Q{:.1},{top:.1} {:.1},{:.1}V{base:.1}Z",
        top + r,
        x + r,
        x + w - r,
        x + w,
        x + w,
        top + r
    )
}

pub fn timeline(t: &Timeline) -> Markup {
    let (width, left, right) = (940.0, 44.0, 8.0);
    let (strip, gap, top) = (64.0, 40.0, 22.0);
    let plot = width - left - right;
    let n = t.rows.len().max(1);
    let slot = plot / n as f64;
    let bar = (slot - 2.0).clamp(1.0, 24.0);
    let strips = [
        ("Hits on the daily name", 0, "col a"),
        ("Lookups of the daily name", 1, "col a"),
        (
            "Background scanning, every other address and name",
            2,
            "col n",
        ),
    ];
    let bottom = top + 3.0 * strip + 2.0 * gap;
    let height = bottom + 22.0;
    let end = t.start + n as i64 * t.bucket;
    let x_of = |ms: i64| left + (ms - t.start) as f64 / (end - t.start) as f64 * plot;
    let step = [
        10 * MIN,
        15 * MIN,
        30 * MIN,
        HOUR,
        3 * HOUR,
        6 * HOUR,
        12 * HOUR,
        DAY,
        2 * DAY,
        7 * DAY,
    ]
    .into_iter()
    .find(|s| (end - t.start) / s <= 8)
    .unwrap_or(7 * DAY);
    let ticks: Vec<i64> = (t.start.div_euclid(step) + 1..)
        .map(|k| k * step)
        .take_while(|ms| *ms < end)
        .collect();
    let label_fmt = if step >= DAY { "%b %-d" } else { "%H:%M" };
    html! {
        svg.chart viewBox=(format!("0 0 {width} {height}")) role="img"
            aria-label="Events over time: hits and lookups of the daily name, and background scanning" {
            @for (i, (title, col, class)) in strips.iter().enumerate() {
                @let y0 = top + i as f64 * (strip + gap);
                @let base = y0 + strip;
                @let max = nice(t.rows.iter().map(|r| r[*col]).max().unwrap_or(0).max(1));
                text.title x=(left) y=(y0 - 10.0) { (title) }
                line.grid x1=(left) x2=(width - right) y1=(y0) y2=(y0) {}
                line.axis x1=(left) x2=(width - right) y1=(base) y2=(base) {}
                text.tick x=(left - 6.0) y=(y0 + 4.0) text-anchor="end" { (compact(max)) }
                text.tick x=(left - 6.0) y=(base) text-anchor="end" { "0" }
                @for (b, row) in t.rows.iter().enumerate() {
                    @let v = row[*col];
                    @if v > 0 {
                        @let h = (v as f64 / max as f64 * strip).max(1.5);
                        path class=(class) d=(column(left + b as f64 * slot + (slot - bar) / 2.0, bar, base, h)) {}
                    }
                }
                @if *col < 2 {
                    @for at in &t.issued {
                        @if *at >= t.start && *at < end {
                            line.mark x1=(x_of(*at)) x2=(x_of(*at)) y1=(y0) y2=(base) {}
                        }
                    }
                }
            }
            @for at in &t.issued {
                @if *at >= t.start && *at < end {
                    text.mark x=(x_of(*at) + 4.0) y=(top + 10.0) { "cert issued" }
                }
            }
            @for tick in &ticks {
                line.grid x1=(x_of(*tick)) x2=(x_of(*tick)) y1=(bottom) y2=(bottom + 4.0) {}
                text.tick x=(x_of(*tick)) y=(bottom + 16.0) text-anchor="middle" {
                    (clock(*tick, if *tick % DAY == 0 && step < DAY { "%b %-d" } else { label_fmt }))
                }
            }
            @for (b, row) in t.rows.iter().enumerate() {
                @let from = t.start + b as i64 * t.bucket;
                rect.hover x=(left + b as f64 * slot) y=(top - 4.0) width=(slot) height=(bottom - top + 4.0) {
                    title {
                        (clock(from, "%b %-d %H:%M")) " to " (clock(from + t.bucket, "%H:%M")) " UTC\n"
                        (row[0]) " hits on the daily name\n"
                        (row[1]) " lookups of the daily name\n"
                        (row[2]) " background events"
                    }
                }
            }
        }
    }
}

pub struct Cell {
    pub href: String,
    pub label: String,
    pub role: String,
    pub named: bool,
    pub events: i64,
    pub sources: i64,
}

pub fn block(cells: &[Cell]) -> Markup {
    let max = cells.iter().map(|c| c.events).max().unwrap_or(0).max(1);
    html! {
        div.block {
            @for c in cells {
                @let p = if c.events == 0 { 0.0 } else { 6.0 + 84.0 * c.events as f64 / max as f64 };
                a.cell.hot[p > 55.0].named[c.named] href=(c.href)
                    style=(format!("background: color-mix(in oklab, var(--mark) {p:.0}%, var(--cell))"))
                    title=(format!("{}, {}: {} events from {} sources", c.label, c.role, c.events, c.sources)) {
                    b { (c.label) }
                    span { (c.role) }
                    span.n { (compact(c.events)) }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_and_numbers_stay_readable() {
        assert_eq!(bucket_for(HOUR), MIN);
        assert_eq!(bucket_for(6 * HOUR), 5 * MIN);
        assert_eq!(bucket_for(2 * DAY), 30 * MIN);
        assert_eq!(bucket_for(5 * DAY), HOUR);
        assert_eq!(bucket_for(30 * DAY), 6 * HOUR);
        assert_eq!(nice(7), 10);
        assert_eq!(nice(130), 200);
        assert_eq!(compact(950), "950");
        assert_eq!(compact(1000), "1K");
        assert_eq!(compact(1530), "1.5K");
        assert_eq!(compact(23_301), "23K");
        assert_eq!(thousands(27_050), "27,050");
        assert_eq!(thousands(950), "950");
        assert_eq!(thousands(1_234_567), "1,234,567");
    }
}
