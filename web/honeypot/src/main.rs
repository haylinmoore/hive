mod names;

use names::{Key, Kind, Slot};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

fn env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

fn key() -> Key {
    let path = env("HONEYPOT_KEY_FILE").unwrap_or_else(|| {
        eprintln!("HONEYPOT_KEY_FILE is not set");
        std::process::exit(2);
    });
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("cannot read {path}: {e}");
        std::process::exit(2);
    });
    Key::parse(&text).unwrap_or_else(|| {
        eprintln!("{path} must hold 64 hex characters");
        std::process::exit(2);
    })
}

fn v4_first() -> Ipv4Addr {
    env("HONEYPOT_V4_FIRST")
        .and_then(|v| v.parse().ok())
        .unwrap_or(Ipv4Addr::new(23, 144, 156, 227))
}

fn v6_prefix() -> Ipv6Addr {
    env("HONEYPOT_V6_PREFIX")
        .and_then(|v| v.parse().ok())
        .unwrap_or(Ipv6Addr::new(0x2602, 0xfbf5, 3, 0xc7, 0, 0, 0, 0))
}

fn today() -> i64 {
    names::day_of(chrono::Utc::now().timestamp())
}

fn date(day: i64) -> String {
    chrono::DateTime::from_timestamp(day * 86400, 0)
        .map(|t| t.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

fn parse_day(text: Option<&str>) -> i64 {
    match text {
        None => today(),
        Some(t) => chrono::NaiveDate::parse_from_str(t, "%Y-%m-%d")
            .map(|d| names::day_of(d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp()))
            .unwrap_or_else(|_| {
                eprintln!("dates look like 2026-10-25");
                std::process::exit(2);
            }),
    }
}

fn show(key: &Key, slot: Slot) {
    print!(
        "{} {} {}.startguess.ing {}",
        date(slot.day),
        slot.kind.as_str(),
        key.name(slot),
        key.v6(v6_prefix(), slot)
    );
    if slot.kind == Kind::Bait {
        print!(" {}", names::v4(v4_first(), slot.day));
    }
    println!();
}

fn decode(key: &Key, what: &str) {
    let label = what.split('.').next().unwrap_or(what);
    let slot = match what.parse::<IpAddr>() {
        Ok(IpAddr::V6(a)) => key.decode_v6(a),
        Ok(IpAddr::V4(a)) => names::decode_v4(v4_first(), a, today()).map(|day| Slot {
            day,
            kind: Kind::Bait,
        }),
        Err(_) => key.decode_name(label),
    };
    match slot {
        Some(slot) => show(key, slot),
        None => {
            eprintln!("{what} is not one of ours");
            std::process::exit(1);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("name") => {
            let key = key();
            let day = parse_day(args.get(2).map(String::as_str));
            show(
                &key,
                Slot {
                    day,
                    kind: Kind::Bait,
                },
            );
            show(
                &key,
                Slot {
                    day,
                    kind: Kind::Control,
                },
            );
        }
        Some("decode") if args.len() > 2 => decode(&key(), &args[2]),
        _ => {
            eprintln!("usage: honeypot name [YYYY-MM-DD] | honeypot decode <name|address>");
            std::process::exit(2);
        }
    }
}
