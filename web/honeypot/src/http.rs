use crate::hello::Hello;
use crate::shared::Shared;
use crate::store::Event;
use bytes::Bytes;
use http_body_util::{BodyExt, Full, Limited};
use hyper::body::Incoming;
use hyper::header::{CONTENT_TYPE, HOST, SERVER, USER_AGENT};
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use hyper_util::server::conn::auto;
use serde_json::json;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::timeout;

const NGINX: &str = "nginx/1.24.0 (Ubuntu)";
const BODY_CAP: usize = 16 * 1024;

const WELCOME: &str = "<!DOCTYPE html>
<html>
<head>
<title>Welcome to nginx!</title>
<style>
html { color-scheme: light dark; }
body { width: 35em; margin: 0 auto;
font-family: Tahoma, Verdana, Arial, sans-serif; }
</style>
</head>
<body>
<h1>Welcome to nginx!</h1>
<p>If you see this page, the nginx web server is successfully installed and
working. Further configuration is required.</p>

<p>For online documentation and support please refer to
<a href=\"http://nginx.org/\">nginx.org</a>.<br/>
Commercial support is available at
<a href=\"http://nginx.com/\">nginx.com</a>.</p>

<p><em>Thank you for using nginx.</em></p>
</body>
</html>
";

const NOT_FOUND: &str = "<html>\r
<head><title>404 Not Found</title></head>\r
<body>\r
<center><h1>404 Not Found</h1></center>\r
<hr><center>nginx/1.24.0 (Ubuntu)</center>\r
</body>\r
</html>\r
";

pub fn looks_like(buf: &[u8]) -> bool {
    const METHODS: [&[u8]; 10] = [
        b"GET ",
        b"POST ",
        b"HEAD ",
        b"PUT ",
        b"DELETE ",
        b"OPTIONS ",
        b"PATCH ",
        b"CONNECT ",
        b"TRACE ",
        b"PRI * ",
    ];
    METHODS.iter().any(|m| buf.starts_with(m))
}

fn text(v: &[u8]) -> String {
    String::from_utf8_lossy(v).into_owned()
}

fn strip_port(host: &str) -> String {
    let name = match host.rsplit_once(':') {
        Some((name, port))
            if port.chars().all(|c| c.is_ascii_digit())
                && (name.ends_with(']') || !name.contains(':')) =>
        {
            name
        }
        _ => host,
    };
    name.trim_start_matches('[')
        .trim_end_matches(']')
        .to_string()
}

async fn handle(
    shared: Arc<Shared>,
    req: Request<Incoming>,
    src: SocketAddr,
    dst: SocketAddr,
    hello: Option<Arc<Hello>>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let (parts, body) = req.into_parts();
    let (body, truncated) = match timeout(
        Duration::from_secs(10),
        Limited::new(body, BODY_CAP).collect(),
    )
    .await
    {
        Ok(Ok(b)) => (b.to_bytes(), false),
        _ => (Bytes::new(), true),
    };
    let host = parts
        .headers
        .get(HOST)
        .map(|h| text(h.as_bytes()))
        .or_else(|| parts.uri.authority().map(|a| a.to_string()));
    let host_name = host
        .as_deref()
        .map(strip_port)
        .filter(|h| h.parse::<std::net::IpAddr>().is_err());
    let path = parts.uri.path().to_string();
    let target = parts
        .uri
        .path_and_query()
        .map(|p| p.to_string())
        .unwrap_or_else(|| path.clone());
    let mut event = Event::new(
        "tcp",
        if hello.is_some() { "https" } else { "http" },
        src,
        dst,
    );
    event.name = host_name.or_else(|| hello.as_ref().and_then(|h| h.sni.clone()));
    event.summary = format!("{} {}", parts.method, target);
    let headers: Vec<(String, String)> = parts
        .headers
        .iter()
        .map(|(k, v)| (k.to_string(), text(v.as_bytes())))
        .collect();
    event.detail = json!({
        "method": parts.method.as_str(),
        "path": path,
        "query": parts.uri.query(),
        "version": format!("{:?}", parts.version),
        "host": host,
        "ua": parts.headers.get(USER_AGENT).map(|v| text(v.as_bytes())),
        "headers": headers,
        "body_len": body.len(),
        "body_truncated": truncated,
        "sni": hello.as_ref().and_then(|h| h.sni.clone()),
        "alpn": hello.as_ref().map(|h| h.alpn.clone()),
        "ja4": hello.as_ref().map(|h| h.ja4.clone()),
    });
    if !body.is_empty() {
        event.raw = Some(body.to_vec());
    }
    shared.store.record(event);
    let welcome = path == "/" && matches!(parts.method, Method::GET | Method::HEAD);
    let (status, page) = if welcome {
        (StatusCode::OK, WELCOME)
    } else {
        (StatusCode::NOT_FOUND, NOT_FOUND)
    };
    let body = if parts.method == Method::HEAD {
        ""
    } else {
        page
    };
    Ok(Response::builder()
        .status(status)
        .header(SERVER, NGINX)
        .header(CONTENT_TYPE, "text/html")
        .body(Full::new(Bytes::from_static(body.as_bytes())))
        .unwrap())
}

pub async fn serve<I>(
    shared: Arc<Shared>,
    io: I,
    src: SocketAddr,
    dst: SocketAddr,
    hello: Option<Arc<Hello>>,
) where
    I: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let service =
        hyper::service::service_fn(move |req| handle(shared.clone(), req, src, dst, hello.clone()));
    let mut builder = auto::Builder::new(TokioExecutor::new());
    builder
        .http1()
        .timer(TokioTimer::new())
        .header_read_timeout(Duration::from_secs(10))
        .max_buf_size(64 * 1024);
    builder
        .http2()
        .timer(TokioTimer::new())
        .max_concurrent_streams(16);
    let conn = builder.serve_connection(TokioIo::new(io), service);
    let _ = timeout(Duration::from_secs(60), conn).await;
}
