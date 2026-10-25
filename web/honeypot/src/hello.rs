use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Default, Serialize)]
pub struct Hello {
    pub sni: Option<String>,
    pub alpn: Vec<String>,
    pub ja4: String,
}

pub fn record_len(buf: &[u8]) -> Option<usize> {
    (buf.len() >= 5 && buf[0] == 0x16 && buf[1] == 0x03)
        .then(|| 5 + u16::from_be_bytes([buf[3], buf[4]]) as usize)
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.0.len() < n {
            return None;
        }
        let (head, rest) = self.0.split_at(n);
        self.0 = rest;
        Some(head)
    }

    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    fn u16(&mut self) -> Option<u16> {
        let b = self.take(2)?;
        Some(u16::from_be_bytes([b[0], b[1]]))
    }

    fn vec8(&mut self) -> Option<Reader<'a>> {
        let n = self.u8()? as usize;
        Some(Reader(self.take(n)?))
    }

    fn vec16(&mut self) -> Option<Reader<'a>> {
        let n = self.u16()? as usize;
        Some(Reader(self.take(n)?))
    }

    fn u16s(mut self) -> Vec<u16> {
        let mut out = Vec::new();
        while let Some(v) = self.u16() {
            out.push(v);
        }
        out
    }
}

fn grease(v: u16) -> bool {
    v & 0x0f0f == 0x0a0a && v >> 8 == v & 0xff
}

fn hash12(items: &[String]) -> String {
    if items.is_empty() {
        return "000000000000".into();
    }
    let digest = Sha256::digest(items.join(",").as_bytes());
    digest.iter().take(6).map(|b| format!("{b:02x}")).collect()
}

fn version(v: u16) -> &'static str {
    match v {
        0x0304 => "13",
        0x0303 => "12",
        0x0302 => "11",
        0x0301 => "10",
        0x0300 => "s3",
        0x0002 => "s2",
        _ => "00",
    }
}

fn alpn_chars(alpn: Option<&String>) -> String {
    let Some(a) = alpn.filter(|a| !a.is_empty()) else {
        return "00".into();
    };
    let (first, last) = (a.as_bytes()[0], a.as_bytes()[a.len() - 1]);
    if first.is_ascii_alphanumeric() && last.is_ascii_alphanumeric() {
        format!("{}{}", first as char, last as char)
    } else {
        let hex = format!("{first:02x}{last:02x}");
        format!("{}{}", &hex[..1], &hex[3..])
    }
}

pub fn parse(record: &[u8]) -> Option<Hello> {
    let mut r = Reader(record);
    r.take(5)?;
    if r.u8()? != 1 {
        return None;
    }
    let len = r.take(3)?;
    let len = u32::from_be_bytes([0, len[0], len[1], len[2]]) as usize;
    let mut r = Reader(r.0.get(..len.min(r.0.len()))?);
    let legacy = r.u16()?;
    r.take(32)?;
    r.vec8()?;
    let ciphers: Vec<u16> = r
        .vec16()?
        .u16s()
        .into_iter()
        .filter(|c| !grease(*c))
        .collect();
    r.vec8()?;
    let mut exts = Vec::new();
    let mut sni = None;
    let mut alpn = Vec::new();
    let mut versions = Vec::new();
    let mut sigs = Vec::new();
    if let Some(mut list) = r.vec16() {
        while let (Some(kind), Some(mut data)) = (list.u16(), list.vec16()) {
            if grease(kind) {
                continue;
            }
            exts.push(kind);
            match kind {
                0x0000 => {
                    let mut names = data.vec16()?;
                    if names.u8()? == 0 {
                        sni = Some(String::from_utf8_lossy(names.vec16()?.0).into_owned());
                    }
                }
                0x0010 => {
                    let mut list = data.vec16()?;
                    while let Some(p) = list.vec8() {
                        alpn.push(String::from_utf8_lossy(p.0).into_owned());
                    }
                }
                0x002b => versions = data.vec8()?.u16s(),
                0x000d => sigs = data.vec16()?.u16s(),
                _ => {}
            }
        }
    }
    let top = versions
        .into_iter()
        .filter(|v| !grease(*v))
        .max()
        .unwrap_or(legacy);
    let mut sorted_ciphers: Vec<String> = ciphers.iter().map(|c| format!("{c:04x}")).collect();
    sorted_ciphers.sort();
    let mut sorted_exts: Vec<String> = exts
        .iter()
        .filter(|e| **e != 0x0000 && **e != 0x0010)
        .map(|e| format!("{e:04x}"))
        .collect();
    sorted_exts.sort();
    let mut ext_input = sorted_exts.join(",");
    let sigs: Vec<String> = sigs
        .iter()
        .filter(|s| !grease(**s))
        .map(|s| format!("{s:04x}"))
        .collect();
    if !sigs.is_empty() {
        ext_input = format!("{ext_input}_{}", sigs.join(","));
    }
    let ext_hash = if sorted_exts.is_empty() {
        "000000000000".into()
    } else {
        hash12(&[ext_input])
    };
    let ja4 = format!(
        "t{}{}{:02}{:02}{}_{}_{}",
        version(top),
        if sni.is_some() { "d" } else { "i" },
        ciphers.len().min(99),
        exts.len().min(99),
        alpn_chars(alpn.first()),
        hash12(&sorted_ciphers),
        ext_hash,
    );
    Some(Hello { sni, alpn, ja4 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    fn ext(kind: u16, data: &[u8]) -> Vec<u8> {
        let mut out = kind.to_be_bytes().to_vec();
        out.extend((data.len() as u16).to_be_bytes());
        out.extend(data);
        out
    }

    fn hello(exts: &[Vec<u8>]) -> Vec<u8> {
        let mut body = hex("0303");
        body.extend([0u8; 32]);
        body.push(0);
        body.extend(hex("0006aaaa13011302"));
        body.extend(hex("0100"));
        let all: Vec<u8> = exts.concat();
        body.extend((all.len() as u16).to_be_bytes());
        body.extend(all);
        let mut hs = vec![1, 0, 0, 0];
        hs[1..4].copy_from_slice(&(body.len() as u32).to_be_bytes()[1..]);
        hs.extend(body);
        let mut rec = hex("160301");
        rec.extend((hs.len() as u16).to_be_bytes());
        rec.extend(hs);
        rec
    }

    #[test]
    fn reads_sni_alpn_and_builds_ja4() {
        let sni = ext(0x0000, &hex("000e00000b65782e746573742e6f7267"));
        let alpn = ext(0x0010, &hex("000c02683208687474702f312e31"));
        let versions = ext(0x002b, &hex("040a0a0304"));
        let sigs = ext(0x000d, &hex("000404030804"));
        let rec = hello(&[ext(0x1a1a, &[]), sni, alpn, versions, sigs]);
        assert_eq!(record_len(&rec), Some(rec.len()));
        let h = parse(&rec).unwrap();
        assert_eq!(h.sni.as_deref(), Some("ex.test.org"));
        assert_eq!(h.alpn, ["h2", "http/1.1"]);
        assert!(h.ja4.starts_with("t13d0204h2_"), "{}", h.ja4);
        let parts: Vec<&str> = h.ja4.split('_').collect();
        assert_eq!(parts[1], hash12(&["1301".into(), "1302".into()]));
        assert_eq!(parts[2], hash12(&["000d,002b_0403,0804".into()]));
    }

    #[test]
    fn no_sni_and_no_extensions() {
        let h = parse(&hello(&[])).unwrap();
        assert_eq!(h.sni, None);
        assert_eq!(
            h.ja4,
            format!(
                "t12i020000_{}_000000000000",
                hash12(&["1301".into(), "1302".into()])
            )
        );
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(parse(b"GET / HTTP/1.1\r\n").is_none());
        assert!(parse(&hex("160301000501")).is_none());
    }
}
