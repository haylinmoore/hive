use crate::hello;
use crate::http;
use crate::net;
use crate::shared::Shared;
use crate::store::{Event, now_ms};
use serde_json::json;
use std::collections::HashMap;
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::time::timeout;

const FIRST_BYTE: Duration = Duration::from_secs(5);
const IDLE: Duration = Duration::from_secs(30);
const RAW_CAP: usize = 4096;
const HELLO_CAP: usize = 16 * 1024 + 5;

pub struct Prefixed<S> {
    buf: Vec<u8>,
    pos: usize,
    inner: S,
}

impl<S> Prefixed<S> {
    pub fn new(buf: Vec<u8>, inner: S) -> Self {
        Prefixed { buf, pos: 0, inner }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for Prefixed<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        out: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if this.pos < this.buf.len() {
            let n = out.remaining().min(this.buf.len() - this.pos);
            out.put_slice(&this.buf[this.pos..this.pos + n]);
            this.pos += n;
            return Poll::Ready(Ok(()));
        }
        Pin::new(&mut this.inner).poll_read(cx, out)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for Prefixed<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().inner).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

async fn fill<S: AsyncRead + Unpin>(
    s: &mut S,
    buf: &mut Vec<u8>,
    cap: usize,
    wait: Duration,
) -> bool {
    let mut chunk = [0u8; 4096];
    let want = cap.saturating_sub(buf.len()).min(chunk.len());
    if want == 0 {
        return false;
    }
    match timeout(wait, s.read(&mut chunk[..want])).await {
        Ok(Ok(n)) if n > 0 => {
            buf.extend_from_slice(&chunk[..n]);
            true
        }
        _ => false,
    }
}

fn banner(port: u16) -> Option<&'static [u8]> {
    Some(match port {
        21 => b"220 (vsFTPd 3.0.5)\r\n",
        22 | 2222 => b"SSH-2.0-OpenSSH_9.6p1 Ubuntu-3ubuntu13.5\r\n",
        23 | 2323 => b"\r\nlogin: ",
        25 | 587 => b"220 mail ESMTP Postfix (Ubuntu)\r\n",
        110 => b"+OK Dovecot (Ubuntu) ready.\r\n",
        143 => b"* OK [CAPABILITY IMAP4rev1 SASL-IR LOGIN-REFERRALS ID ENABLE IDLE LITERAL+ STARTTLS AUTH=PLAIN] Dovecot (Ubuntu) ready.\r\n",
        _ => return None,
    })
}

fn by_port(port: u16) -> &'static str {
    match port {
        21 => "ftp",
        22 | 2222 => "ssh",
        23 | 2323 => "telnet",
        25 | 587 => "smtp",
        110 => "pop3",
        143 => "imap",
        445 => "smb",
        502 => "modbus",
        1433 => "mssql",
        1883 => "mqtt",
        3306 => "mysql",
        3389 => "rdp",
        5432 => "postgres",
        5900..=5910 => "vnc",
        6379 => "redis",
        11211 => "memcached",
        27017 => "mongodb",
        _ => "unknown",
    }
}

fn identify(buf: &[u8], port: u16) -> &'static str {
    if buf.starts_with(b"SSH-") {
        "ssh"
    } else if buf.starts_with(&[0x03, 0x00]) {
        "rdp"
    } else if buf.len() > 8 && (&buf[4..8] == b"\xffSMB" || &buf[4..8] == b"\xfeSMB") {
        "smb"
    } else if buf.starts_with(b"*") && buf.get(1).is_some_and(u8::is_ascii_digit) {
        "redis"
    } else if buf.starts_with(b"JRMI") {
        "rmi"
    } else if buf.starts_with(b"RFB ") {
        "vnc"
    } else if buf.starts_with(&[0x10]) && port == 1883 {
        "mqtt"
    } else {
        by_port(port)
    }
}

fn udp_proto(port: u16) -> &'static str {
    match port {
        53 => "dns",
        69 => "tftp",
        123 => "ntp",
        137 => "netbios",
        161 => "snmp",
        443 => "quic",
        500 | 4500 => "ike",
        1194 => "openvpn",
        1900 => "ssdp",
        3478 => "stun",
        5060 => "sip",
        5353 => "mdns",
        11211 => "memcached",
        51820 => "wireguard",
        _ => "unknown",
    }
}

fn raw_event(src: SocketAddr, dst: SocketAddr, proto: &str, buf: &[u8], summary: String) -> Event {
    let mut event = Event::new("tcp", proto, src, dst);
    event.summary = summary;
    event.detail = json!({ "bytes": buf.len() });
    if !buf.is_empty() {
        event.raw = Some(buf[..buf.len().min(RAW_CAP)].to_vec());
    }
    event
}

async fn drain<S: AsyncRead + Unpin>(s: &mut S, buf: &mut Vec<u8>) {
    while buf.len() < RAW_CAP && fill(s, buf, RAW_CAP, IDLE).await {}
}

async fn tls(
    shared: Arc<Shared>,
    mut stream: TcpStream,
    mut buf: Vec<u8>,
    src: SocketAddr,
    dst: SocketAddr,
) {
    let want = hello::record_len(&buf).unwrap_or(0).min(HELLO_CAP);
    while buf.len() < want && fill(&mut stream, &mut buf, want, FIRST_BYTE).await {}
    let hello = hello::parse(&buf);
    let mut event = Event::new("tcp", "tls", src, dst);
    event.name = hello.as_ref().and_then(|h| h.sni.clone());
    event.summary = match &hello {
        Some(h) => format!("tls {}", h.sni.as_deref().unwrap_or("(no sni)")),
        None => "tls (unparsed hello)".into(),
    };
    event.detail = json!(hello);
    if hello.is_none() {
        event.raw = Some(buf[..buf.len().min(RAW_CAP)].to_vec());
    }
    shared.store.record(event);
    let Some(cfg) = shared.tls.read().unwrap().clone() else {
        return;
    };
    let acceptor = tokio_rustls::TlsAcceptor::from(cfg);
    let Ok(Ok(mut inner)) = timeout(
        Duration::from_secs(10),
        acceptor.accept(Prefixed::new(buf, stream)),
    )
    .await
    else {
        return;
    };
    let hello = Arc::new(hello.unwrap_or_default());
    let h2 = inner.get_ref().1.alpn_protocol() == Some(b"h2");
    let mut first = Vec::new();
    if !h2 && !fill(&mut inner, &mut first, RAW_CAP, FIRST_BYTE).await {
        return;
    }
    if h2 || http::looks_like(&first) {
        http::serve(shared, Prefixed::new(first, inner), src, dst, Some(hello)).await;
        return;
    }
    drain(&mut inner, &mut first).await;
    let proto = identify(&first, dst.port());
    let mut event = raw_event(src, dst, "tls-raw", &first, format!("tls {proto}"));
    event.name = hello.sni.clone();
    shared.store.record(event);
}

async fn conn(shared: Arc<Shared>, mut stream: TcpStream, src: SocketAddr) {
    let Ok(dst) = stream.local_addr() else { return };
    let _ = stream.set_nodelay(true);
    let mut buf = Vec::new();
    if !fill(&mut stream, &mut buf, RAW_CAP, FIRST_BYTE).await {
        match banner(dst.port()) {
            Some(b) if timeout(FIRST_BYTE, stream.write_all(b)).await.is_ok() => {
                drain(&mut stream, &mut buf).await;
                let proto = identify(&buf, dst.port());
                shared.store.record(raw_event(
                    src,
                    dst,
                    proto,
                    &buf,
                    format!("{proto} after banner"),
                ));
            }
            _ => shared.store.record(raw_event(
                src,
                dst,
                "silent",
                &[],
                "connected, sent nothing".into(),
            )),
        }
        return;
    }
    if hello::record_len(&buf).is_some() {
        return tls(shared, stream, buf, src, dst).await;
    }
    if http::looks_like(&buf) {
        return http::serve(shared, Prefixed::new(buf, stream), src, dst, None).await;
    }
    drain(&mut stream, &mut buf).await;
    let proto = identify(&buf, dst.port());
    shared
        .store
        .record(raw_event(src, dst, proto, &buf, proto.to_string()));
}

async fn serve_tcp(shared: Arc<Shared>, listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((stream, src)) => {
                tokio::spawn(conn(shared.clone(), stream, src));
            }
            Err(e) => {
                eprintln!("capture tcp: {e}");
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

async fn serve_udp(shared: Arc<Shared>, sock: UdpSocket) {
    let mut seen: HashMap<(IpAddr, u16), i64> = HashMap::new();
    let mut buf = vec![0u8; 65535];
    loop {
        let (n, src, dst) = match net::recv(&sock, &mut buf).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("capture udp: {e}");
                continue;
            }
        };
        let now = now_ms();
        if seen.len() > 200_000 {
            seen.retain(|_, at| now - *at < 3_600_000);
        }
        let key = (src.ip(), dst.port());
        if seen.get(&key).is_some_and(|at| now - at < 3_600_000) {
            continue;
        }
        seen.insert(key, now);
        let proto = udp_proto(dst.port());
        let mut event = Event::new("udp", proto, src, dst);
        event.summary = format!("{proto} datagram");
        event.detail = json!({ "bytes": n });
        event.raw = Some(buf[..n.min(RAW_CAP)].to_vec());
        shared.store.record(event);
    }
}

pub fn start(shared: &Arc<Shared>) -> io::Result<()> {
    for ip in [
        IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        IpAddr::V6(Ipv6Addr::UNSPECIFIED),
    ] {
        let addr = SocketAddr::new(ip, shared.cfg.listen_port);
        tokio::spawn(serve_tcp(shared.clone(), net::tcp(addr)?));
        tokio::spawn(serve_udp(shared.clone(), net::udp(addr)?));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_common_protocols() {
        assert_eq!(identify(b"SSH-2.0-Go\r\n", 8022), "ssh");
        assert_eq!(identify(b"\x03\x00\x00\x13", 3389), "rdp");
        assert_eq!(identify(b"*1\r\n$4\r\nPING\r\n", 9999), "redis");
        assert_eq!(identify(b"\x00\x01", 3306), "mysql");
        assert_eq!(identify(b"\x00\x01", 40000), "unknown");
    }

    #[tokio::test]
    async fn prefixed_replays_the_buffer_first() {
        let mut p = Prefixed::new(b"abc".to_vec(), &b"def"[..]);
        let mut out = String::new();
        p.read_to_string(&mut out).await.unwrap();
        assert_eq!(out, "abcdef");
    }
}
