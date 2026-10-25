use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub remote: String,
    #[serde(default)]
    pub up: bool,
    #[serde(default)]
    pub inbound: bool,
    #[serde(default)]
    pub uptime: f64,
    #[serde(default)]
    pub latency: u64,
    #[serde(default)]
    pub cost: u64,
    #[serde(default)]
    pub last_error: Option<String>,
}

impl Peer {
    pub fn lan_interface(&self) -> Option<&str> {
        let (_, rest) = self.remote.split_once("%25")?;
        rest.split(']').next()
    }

    pub fn host(&self) -> &str {
        let rest = self
            .remote
            .split_once("://")
            .map(|(_, r)| r)
            .unwrap_or(&self.remote);
        rest.split('?').next().unwrap_or(rest)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    pub key: String,
    pub parent: String,
    #[serde(default)]
    pub address: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Mesh {
    pub key: Option<String>,
    pub peers: Vec<Peer>,
    pub tree: Vec<TreeEntry>,
}

#[derive(Deserialize)]
struct Response {
    status: String,
    #[serde(default)]
    error: String,
    #[serde(default)]
    response: serde_json::Value,
}

fn request<T: DeserializeOwned>(socket: &Path, name: &str) -> io::Result<T> {
    let mut stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    stream.write_all(
        serde_json::json!({ "request": name })
            .to_string()
            .as_bytes(),
    )?;

    let mut raw = String::new();
    stream.read_to_string(&mut raw)?;
    let resp: Response = serde_json::from_str(&raw).map_err(io::Error::other)?;
    if resp.status != "success" {
        return Err(io::Error::other(format!("{name}: {}", resp.error)));
    }
    serde_json::from_value(resp.response).map_err(io::Error::other)
}

#[derive(Deserialize)]
struct SelfInfo {
    key: String,
}

#[derive(Deserialize)]
struct Peers {
    peers: Vec<Peer>,
}

#[derive(Deserialize)]
struct Tree {
    tree: Vec<TreeEntry>,
}

pub fn snapshot(socket: &Path) -> io::Result<Mesh> {
    Ok(Mesh {
        key: request::<SelfInfo>(socket, "getSelf").ok().map(|s| s.key),
        peers: request::<Peers>(socket, "getPeers")?.peers,
        tree: request::<Tree>(socket, "getTree")?.tree,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(remote: &str) -> Peer {
        Peer {
            key: String::new(),
            address: String::new(),
            remote: remote.into(),
            up: true,
            inbound: false,
            uptime: 0.0,
            latency: 0,
            cost: 0,
            last_error: None,
        }
    }

    #[test]
    fn a_multicast_peer_is_on_the_lan() {
        let p = peer("tls://[fe80::148b:4e5c:f773:6376%25wlp0s20f3]:9445");
        assert_eq!(p.lan_interface(), Some("wlp0s20f3"));
    }

    #[test]
    fn a_hub_peer_is_not() {
        let p = peer("tls://athena.infra.hayl.in:9444?password=x");
        assert_eq!(p.lan_interface(), None);
        assert_eq!(p.host(), "athena.infra.hayl.in:9444");
    }

    #[test]
    fn peers_parse_from_the_admin_socket() {
        let raw = r#"{"peers":[{"remote":"tls://a:1","up":true,"inbound":false,"address":"200::1","key":"ab","port":2,"priority":0,"cost":3,"bytes_recvd":1,"bytes_sent":2,"uptime":1.5,"latency":10750000}]}"#;
        let p: Peers = serde_json::from_str(raw).unwrap();
        assert_eq!(p.peers[0].key, "ab");
        assert_eq!(p.peers[0].latency, 10750000);
    }
}
