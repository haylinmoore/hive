use crate::names::{self, Key};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;

#[derive(Clone)]
pub struct Config {
    pub key: Key,
    pub zone: String,
    pub apex4: Ipv4Addr,
    pub ns2_4: Ipv4Addr,
    pub baseline4: Ipv4Addr,
    pub first4: Ipv4Addr,
    pub prefix6: Ipv6Addr,
    pub state: PathBuf,
    pub web: SocketAddr,
    pub listen_port: u16,
    pub dns_port: u16,
    pub acme_url: String,
    pub launch_day: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Daily(i64),
    Apex,
    Ns(u8),
    Baseline,
    Other(String),
    Foreign(String),
}

impl Target {
    pub fn via(&self) -> &'static str {
        match self {
            Target::Daily(_) => "bait",
            Target::Apex => "apex",
            Target::Ns(_) => "ns",
            Target::Baseline => "baseline",
            Target::Other(_) => "unknown",
            Target::Foreign(_) => "foreign",
        }
    }
}

fn env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

fn parsed<T: std::str::FromStr>(key: &str, default: T) -> T {
    env(key).and_then(|v| v.parse().ok()).unwrap_or(default)
}

impl Config {
    pub fn from_env(key: Key) -> Config {
        Config {
            key,
            zone: env("HONEYPOT_ZONE")
                .unwrap_or_else(|| "startguess.ing".into())
                .trim_end_matches('.')
                .to_ascii_lowercase(),
            apex4: parsed("HONEYPOT_APEX4", Ipv4Addr::new(23, 144, 156, 224)),
            ns2_4: parsed("HONEYPOT_NS2_4", Ipv4Addr::new(23, 144, 156, 225)),
            baseline4: parsed("HONEYPOT_BASELINE4", Ipv4Addr::new(23, 144, 156, 226)),
            first4: parsed("HONEYPOT_V4_FIRST", Ipv4Addr::new(23, 144, 156, 227)),
            prefix6: parsed(
                "HONEYPOT_V6_PREFIX",
                Ipv6Addr::new(0x2602, 0xfbf5, 3, 0xc7, 0, 0, 0, 0),
            ),
            state: PathBuf::from(
                env("HONEYPOT_STATE").unwrap_or_else(|| "/var/lib/honeypot".into()),
            ),
            web: parsed("HONEYPOT_WEB", "127.0.0.1:15710".parse().unwrap()),
            listen_port: parsed("HONEYPOT_LISTEN_PORT", 61999),
            dns_port: parsed("HONEYPOT_DNS_PORT", 53),
            acme_url: env("HONEYPOT_ACME_URL")
                .unwrap_or_else(|| "https://acme-v02.api.letsencrypt.org/directory".into()),
            launch_day: parsed("HONEYPOT_LAUNCH_DAY", 0),
        }
    }

    pub fn apex6(&self) -> Ipv6Addr {
        self.prefix6
    }

    pub fn ns2_6(&self) -> Ipv6Addr {
        Ipv6Addr::from(u128::from(self.prefix6) | 0x53)
    }

    pub fn baseline6(&self) -> Ipv6Addr {
        Ipv6Addr::from(u128::from(self.prefix6) | 1)
    }

    pub fn label<'a>(&self, name: &'a str) -> Option<&'a str> {
        let name = name.trim_end_matches('.');
        if name.eq_ignore_ascii_case(&self.zone) {
            return Some("");
        }
        let cut = name.len().checked_sub(self.zone.len() + 1)?;
        (name.as_bytes()[cut] == b'.' && name[cut + 1..].eq_ignore_ascii_case(&self.zone))
            .then(|| &name[..cut])
    }

    pub fn by_name(&self, name: &str) -> Target {
        let lower = name.trim_end_matches('.').to_ascii_lowercase();
        match self.label(&lower) {
            None => Target::Foreign(lower),
            Some("") => Target::Apex,
            Some("ns1") => Target::Ns(1),
            Some("ns2") => Target::Ns(2),
            Some(label) => match self
                .key
                .decode_name(label.rsplit('.').next().unwrap_or(label))
            {
                Some(day) => Target::Daily(day),
                None => Target::Other(label.to_string()),
            },
        }
    }

    pub fn by_address(&self, ip: IpAddr, today: i64) -> Target {
        match ip {
            IpAddr::V4(a) if a == self.apex4 => Target::Ns(1),
            IpAddr::V4(a) if a == self.ns2_4 => Target::Ns(2),
            IpAddr::V4(a) if a == self.baseline4 => Target::Baseline,
            IpAddr::V4(a) => match names::decode_v4(self.first4, a, today) {
                Some(day) if day >= self.launch_day => Target::Daily(day),
                _ => Target::Foreign(a.to_string()),
            },
            IpAddr::V6(a) if a == self.apex6() => Target::Ns(1),
            IpAddr::V6(a) if a == self.ns2_6() => Target::Ns(2),
            IpAddr::V6(a) if a == self.baseline6() => Target::Baseline,
            IpAddr::V6(a) => match self.key.decode_v6(a) {
                Some(day) if (self.launch_day..=today + 1).contains(&day) => Target::Daily(day),
                _ => Target::Foreign(a.to_string()),
            },
        }
    }

    pub fn v4(&self, day: i64) -> Ipv4Addr {
        names::v4(self.first4, day)
    }

    pub fn v6(&self, day: i64) -> Ipv6Addr {
        self.key.v6(self.prefix6, day)
    }
}

#[cfg(test)]
pub fn test() -> Config {
    let mut c = Config::from_env(Key::new([7; 32]));
    c.zone = "startguess.ing".into();
    c.launch_day = 20000;
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_map_to_targets() {
        let c = test();
        let name = format!("{}.StartGuess.ing.", c.key.name(20751));
        assert_eq!(c.by_name(&name), Target::Daily(20751));
        assert_eq!(c.by_name("startguess.ing"), Target::Apex);
        assert_eq!(c.by_name("ns2.startguess.ing"), Target::Ns(2));
        assert_eq!(c.by_name("www.startguess.ing"), Target::Other("www".into()));
        assert_eq!(c.by_name("a.b.startguess.ing"), Target::Other("a.b".into()));
        let deep = format!("_acme-challenge.{}.startguess.ing", c.key.name(20751));
        assert_eq!(c.by_name(&deep), Target::Daily(20751));
        assert_eq!(
            c.by_name("xstartguess.ing"),
            Target::Foreign("xstartguess.ing".into())
        );
        assert_eq!(
            c.by_name("example.com"),
            Target::Foreign("example.com".into())
        );
    }

    #[test]
    fn addresses_map_to_targets() {
        let c = test();
        let today = 20751;
        assert_eq!(
            c.by_address(IpAddr::V6(c.v6(today - 3)), today),
            Target::Daily(today - 3)
        );
        assert_eq!(
            c.by_address(IpAddr::V4(c.v4(today - 3)), today),
            Target::Daily(today - 3)
        );
        assert_eq!(
            c.by_address(IpAddr::V4(c.baseline4), today),
            Target::Baseline
        );
        assert_eq!(c.by_address(IpAddr::V6(c.ns2_6()), today), Target::Ns(2));
        assert!(matches!(
            c.by_address("2602:fbf5:3:c7::abcd".parse().unwrap(), today),
            Target::Foreign(_)
        ));
    }
}
