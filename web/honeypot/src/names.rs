use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::LazyLock;

static WORDS: LazyLock<Vec<&'static str>> =
    LazyLock::new(|| include_str!("words.txt").lines().collect());

const NAME_SPACE: u64 = 2048 * 2048 * 2048;
const NAME_HALF_BITS: u32 = 17;
const ROTATION: u64 = 29;
const ORDER_WINDOW: u64 = 20 * 3600;

// Days sit at even indices because odd ones once belonged to a retired
// second name per day. Renumbering would change every name already issued.
fn index(day: i64) -> u64 {
    day as u64 * 2
}

fn day(n: u64) -> Option<i64> {
    (n < NAME_SPACE && n.is_multiple_of(2)).then_some((n / 2) as i64)
}

#[derive(Clone)]
pub struct Key([u8; 32]);

impl Key {
    #[cfg(test)]
    pub fn new(bytes: [u8; 32]) -> Key {
        Key(bytes)
    }

    pub fn parse(text: &str) -> Option<Key> {
        let hex = text.trim();
        if hex.len() != 64 {
            return None;
        }
        let mut out = [0u8; 32];
        for (i, byte) in out.iter_mut().enumerate() {
            *byte = u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()?;
        }
        Some(Key(out))
    }

    fn round(&self, tag: &[u8], round: u8, half: u64, bits: u32) -> u64 {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.0).expect("any key length");
        mac.update(tag);
        mac.update(&[round]);
        mac.update(&half.to_le_bytes());
        let out = mac.finalize().into_bytes();
        u64::from_le_bytes(out[..8].try_into().unwrap()) & mask(bits)
    }

    fn feistel(&self, tag: &[u8], x: u64, bits: u32, inverse: bool) -> u64 {
        let (mut l, mut r) = (x >> bits, x & mask(bits));
        if inverse {
            for round in (0..4).rev() {
                (l, r) = (r ^ self.round(tag, round, l, bits), l);
            }
        } else {
            for round in 0..4 {
                (l, r) = (r, l ^ self.round(tag, round, r, bits));
            }
        }
        (l << bits) | r
    }

    fn walk(&self, n: u64, inverse: bool) -> u64 {
        let mut x = self.feistel(b"name", n, NAME_HALF_BITS, inverse);
        while x >= NAME_SPACE {
            x = self.feistel(b"name", x, NAME_HALF_BITS, inverse);
        }
        x
    }

    pub fn name(&self, day: i64) -> String {
        let x = self.walk(index(day), false);
        let w = WORDS.len() as u64;
        [x / (w * w), (x / w) % w, x % w]
            .iter()
            .map(|&i| WORDS[i as usize])
            .collect::<Vec<_>>()
            .join("-")
    }

    pub fn decode_name(&self, label: &str) -> Option<i64> {
        let mut x = 0u64;
        let mut parts = 0;
        for part in label.to_ascii_lowercase().split('-') {
            let i = WORDS.binary_search(&part).ok()? as u64;
            x = x * WORDS.len() as u64 + i;
            parts += 1;
        }
        if parts != 3 {
            return None;
        }
        day(self.walk(x, true))
    }

    pub fn order_offset(&self, day: i64) -> i64 {
        (self.round(b"order", 0, day as u64, 32) % ORDER_WINDOW) as i64
    }

    pub fn v6(&self, prefix: Ipv6Addr, day: i64) -> Ipv6Addr {
        let iid = self.feistel(b"v6", index(day), 32, false);
        Ipv6Addr::from((u128::from(prefix) & !(u64::MAX as u128)) | iid as u128)
    }

    pub fn decode_v6(&self, addr: Ipv6Addr) -> Option<i64> {
        day(self.feistel(b"v6", u128::from(addr) as u64, 32, true))
    }
}

fn mask(bits: u32) -> u64 {
    (1u64 << bits) - 1
}

pub fn v4(first: Ipv4Addr, day: i64) -> Ipv4Addr {
    Ipv4Addr::from(u32::from(first) + day.rem_euclid(ROTATION as i64) as u32)
}

pub fn decode_v4(first: Ipv4Addr, addr: Ipv4Addr, today: i64) -> Option<i64> {
    let offset = u32::from(addr).checked_sub(u32::from(first))? as i64;
    if offset >= ROTATION as i64 {
        return None;
    }
    Some(today - (today - offset).rem_euclid(ROTATION as i64))
}

pub fn day_of(unix: i64) -> i64 {
    unix.div_euclid(86400)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> Key {
        Key::new([7; 32])
    }

    #[test]
    fn the_word_list_is_bip39_and_sorted() {
        assert_eq!(WORDS.len(), 2048);
        assert!(WORDS.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn names_and_addresses_never_change_for_a_given_key() {
        let k = key();
        assert_eq!(k.name(20751), "sentence-destroy-adult");
        assert_eq!(
            k.v6("2602:fbf5:3:c7::".parse().unwrap(), 20751),
            "2602:fbf5:3:c7:caec:5f75:86dc:871"
                .parse::<Ipv6Addr>()
                .unwrap()
        );
    }

    #[test]
    fn names_are_unique_and_decode_back() {
        let k = key();
        let mut seen = std::collections::HashSet::new();
        for day in 20000..21500 {
            let name = k.name(day);
            assert_eq!(name.split('-').count(), 3);
            assert!(seen.insert(name.clone()), "{name} repeated");
            assert_eq!(k.decode_name(&name), Some(day));
        }
    }

    #[test]
    fn a_different_key_gives_unrelated_names() {
        let other = Key::new([8; 32]);
        assert_ne!(key().name(20751), other.name(20751));
        assert_ne!(other.decode_name(&key().name(20751)), Some(20751));
    }

    #[test]
    fn neighbouring_days_share_nothing_visible() {
        let k = key();
        let (a, b) = (k.name(20751), k.name(20752));
        assert!(a.split('-').zip(b.split('-')).all(|(x, y)| x != y));
    }

    #[test]
    fn junk_labels_do_not_decode() {
        let k = key();
        assert_eq!(k.decode_name("www"), None);
        assert_eq!(k.decode_name("abandon-ability"), None);
        assert_eq!(k.decode_name("abandon-ability-able-about"), None);
        assert_eq!(k.decode_name("notaword-ability-able"), None);
    }

    #[test]
    fn v6_addresses_are_unique_keyed_and_decode_back() {
        let k = key();
        let prefix: Ipv6Addr = "2602:fbf5:3:c7::".parse().unwrap();
        let (a, b) = (k.v6(prefix, 20751), k.v6(prefix, 20752));
        assert_ne!(a, b);
        assert_eq!(a.segments()[..4], prefix.segments()[..4]);
        assert_eq!(k.decode_v6(a), Some(20751));
        assert_eq!(k.decode_v6(b), Some(20752));
        assert_eq!(k.decode_v6("2602:fbf5:3:c7::1".parse().unwrap()), None);
    }

    #[test]
    fn v4_rotates_every_29_days_and_maps_back_to_the_latest_day() {
        let first: Ipv4Addr = "23.144.156.227".parse().unwrap();
        assert_eq!(v4(first, 0), first);
        assert_eq!(v4(first, 28).octets()[3], 255);
        assert_eq!(v4(first, 29), first);
        let today = 20751;
        for day in today - 28..=today {
            assert_eq!(decode_v4(first, v4(first, day), today), Some(day));
        }
        assert_eq!(
            decode_v4(first, "23.144.156.226".parse().unwrap(), today),
            None
        );
    }

    #[test]
    fn keys_parse_from_hex() {
        assert!(Key::parse(&"ab".repeat(32)).is_some());
        assert!(Key::parse("abc").is_none());
    }
}
