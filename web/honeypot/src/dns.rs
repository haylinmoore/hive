use crate::config::Target;
use crate::net;
use crate::shared::Shared;
use crate::store::Event;
use hickory_proto::op::{Edns, Message, MessageType, OpCode, ResponseCode};
use hickory_proto::rr::rdata::opt::{ClientSubnet, EdnsCode, EdnsOption};
use hickory_proto::rr::rdata::{A, AAAA, CAA, NS, SOA, TXT};
use hickory_proto::rr::{DNSClass, Name, RData, Record, RecordType};
use serde_json::json;
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::time::timeout;

const TTL: u32 = 300;

fn zone(shared: &Shared) -> Name {
    Name::from_ascii(format!("{}.", shared.cfg.zone)).expect("zone is a valid name")
}

fn under(shared: &Shared, label: &str) -> Name {
    Name::from_ascii(format!("{label}.{}.", shared.cfg.zone)).expect("label is a valid name")
}

fn addresses(shared: &Shared, target: &Target, now: i64) -> (Ipv4Addr, Ipv6Addr) {
    let cfg = &shared.cfg;
    match target {
        Target::Ns(2) => (cfg.ns2_4, cfg.ns2_6()),
        Target::Daily(day) => match shared.ordered_at(*day) {
            Some(at) if now >= at => (cfg.v4(*day), cfg.v6(*day)),
            _ => (cfg.apex4, cfg.apex6()),
        },
        _ => (cfg.apex4, cfg.apex6()),
    }
}

fn soa(shared: &Shared) -> RData {
    RData::SOA(SOA::new(
        under(shared, "ns1"),
        under(shared, "hostmaster"),
        1,
        3600,
        600,
        604800,
        TTL,
    ))
}

fn rdata(shared: &Shared, target: &Target, qname: &str, qtype: RecordType, now: i64) -> Vec<RData> {
    let (v4, v6) = addresses(shared, target, now);
    let apex = *target == Target::Apex;
    let mut out = Vec::new();
    if matches!(qtype, RecordType::A | RecordType::ANY) {
        out.push(RData::A(A(v4)));
    }
    if matches!(qtype, RecordType::AAAA | RecordType::ANY) {
        out.push(RData::AAAA(AAAA(v6)));
    }
    match qtype {
        RecordType::SOA if apex => out.push(soa(shared)),
        RecordType::NS if apex => {
            out.push(RData::NS(NS(under(shared, "ns1"))));
            out.push(RData::NS(NS(under(shared, "ns2"))));
        }
        RecordType::CAA if apex => {
            let le = Name::from_ascii("letsencrypt.org").ok();
            out.push(RData::CAA(CAA::new_issue(false, le.clone(), vec![])));
            out.push(RData::CAA(CAA::new_issuewild(false, le, vec![])));
        }
        RecordType::TXT if apex => out.push(RData::TXT(TXT::new(vec!["v=spf1 -all".into()]))),
        RecordType::TXT => {
            let key = qname.trim_end_matches('.').to_ascii_lowercase();
            if let Some(values) = shared.challenges.read().unwrap().get(&key) {
                out.extend(values.iter().map(|v| RData::TXT(TXT::new(vec![v.clone()]))));
            }
        }
        _ => {}
    }
    out
}

pub fn respond(shared: &Shared, req: &Message, now: i64) -> Message {
    let mut resp = Message::response(req.metadata.id, req.metadata.op_code);
    resp.metadata.recursion_desired = req.metadata.recursion_desired;
    resp.add_queries(req.queries.iter().cloned());
    if let Some(edns) = &req.edns {
        let mut out = Edns::new();
        out.set_max_payload(1232);
        if let Some(EdnsOption::Subnet(s)) = edns.option(EdnsCode::Subnet) {
            out.options_mut()
                .insert(EdnsOption::Subnet(ClientSubnet::new(
                    s.addr(),
                    s.source_prefix(),
                    0,
                )));
        }
        resp.set_edns(out);
    }
    if req.metadata.message_type != MessageType::Query || req.metadata.op_code != OpCode::Query {
        resp.metadata.response_code = ResponseCode::NotImp;
        return resp;
    }
    let [q] = req.queries.as_slice() else {
        resp.metadata.response_code = ResponseCode::FormErr;
        return resp;
    };
    let qname = q.name().to_ascii();
    let target = shared.cfg.by_name(&qname);
    if q.query_class() != DNSClass::IN
        || matches!(target, Target::Foreign(_))
        || matches!(q.query_type(), RecordType::AXFR | RecordType::IXFR)
    {
        resp.metadata.response_code = ResponseCode::Refused;
        return resp;
    }
    resp.metadata.authoritative = true;
    let answers = rdata(shared, &target, &qname, q.query_type(), now);
    if answers.is_empty() {
        resp.add_authority(Record::from_rdata(zone(shared), TTL, soa(shared)));
    }
    for r in answers {
        resp.add_answer(Record::from_rdata(q.name().clone(), TTL, r));
    }
    resp
}

pub fn handle(
    shared: &Shared,
    bytes: &[u8],
    transport: &'static str,
    src: SocketAddr,
    dst: SocketAddr,
) -> Option<Vec<u8>> {
    let mut event = Event::new(transport, "dns", src, dst);
    let Ok(req) = Message::from_vec(bytes) else {
        event.summary = "malformed".into();
        event.raw = Some(bytes[..bytes.len().min(4096)].to_vec());
        shared.store.record(event);
        return None;
    };
    let resp = respond(shared, &req, event.at_ms);
    let ecs = req
        .edns
        .as_ref()
        .and_then(|e| match e.option(EdnsCode::Subnet) {
            Some(EdnsOption::Subnet(s)) => Some(format!("{}/{}", s.addr(), s.source_prefix())),
            _ => None,
        });
    if let Some(q) = req.queries.first() {
        let name = q.name().to_ascii();
        let name = name.trim_end_matches('.');
        event.summary = format!("{} {}", q.query_type(), name);
        event.acme = shared.is_pending(name);
        event.name = Some(name.to_string());
    }
    event.detail = json!({
        "qtype": req.queries.first().map(|q| q.query_type().to_string()),
        "rd": req.metadata.recursion_desired,
        "rcode": resp.metadata.response_code.to_string(),
        "answers": resp.answers.len(),
        "edns": req.edns.as_ref().map(|e| json!({"size": e.max_payload(), "do": e.flags().dnssec_ok})),
        "ecs": ecs,
    });
    shared.store.record(event);
    let out = resp.to_vec().ok()?;
    if transport == "udp" && out.len() > req.max_payload() as usize {
        return resp.truncate().to_vec().ok();
    }
    Some(out)
}

async fn serve_udp(shared: Arc<Shared>, sock: UdpSocket) {
    let mut buf = vec![0u8; 65535];
    loop {
        let (n, src, dst) = match net::recv(&sock, &mut buf).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("dns udp: {e}");
                continue;
            }
        };
        if let Some(out) = handle(&shared, &buf[..n], "udp", src, dst) {
            let _ = net::send(&sock, &out, dst.ip(), src).await;
        }
    }
}

async fn serve_tcp_conn(shared: Arc<Shared>, mut stream: TcpStream, src: SocketAddr) {
    let Ok(dst) = stream.local_addr() else { return };
    let wait = Duration::from_secs(10);
    for _ in 0..100 {
        let mut len = [0u8; 2];
        let Ok(Ok(_)) = timeout(wait, stream.read_exact(&mut len)).await else {
            return;
        };
        let mut buf = vec![0u8; u16::from_be_bytes(len) as usize];
        let Ok(Ok(_)) = timeout(wait, stream.read_exact(&mut buf)).await else {
            return;
        };
        let Some(out) = handle(&shared, &buf, "tcp", src, dst) else {
            return;
        };
        let mut framed = (out.len() as u16).to_be_bytes().to_vec();
        framed.extend(out);
        if !matches!(timeout(wait, stream.write_all(&framed)).await, Ok(Ok(()))) {
            return;
        }
    }
}

async fn serve_tcp(shared: Arc<Shared>, listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((stream, src)) => {
                tokio::spawn(serve_tcp_conn(shared.clone(), stream, src));
            }
            Err(e) => eprintln!("dns tcp: {e}"),
        }
    }
}

pub fn start(shared: &Arc<Shared>) -> io::Result<()> {
    for ip in [
        IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        IpAddr::V6(Ipv6Addr::UNSPECIFIED),
    ] {
        let addr = SocketAddr::new(ip, shared.cfg.dns_port);
        tokio::spawn(serve_udp(shared.clone(), net::udp(addr)?));
        tokio::spawn(serve_tcp(shared.clone(), net::tcp(addr)?));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hickory_proto::op::Query;

    fn ask(shared: &Shared, name: &str, qtype: RecordType, now: i64) -> Message {
        let mut req = Message::query();
        req.add_query(Query::query(Name::from_ascii(name).unwrap(), qtype));
        respond(shared, &req, now)
    }

    fn ips(m: &Message) -> Vec<String> {
        m.answers.iter().map(|r| r.data.to_string()).collect()
    }

    #[test]
    fn the_apex_answers_with_fixed_records() {
        let s = crate::shared::test();
        let now = 20751 * 86_400_000;
        assert_eq!(
            ips(&ask(&s, "startguess.ing.", RecordType::A, now)),
            ["23.144.156.224"]
        );
        assert_eq!(
            ask(&s, "startguess.ing.", RecordType::NS, now)
                .answers
                .len(),
            2
        );
        assert_eq!(
            ask(&s, "startguess.ing.", RecordType::CAA, now)
                .answers
                .len(),
            2
        );
        assert_eq!(
            ips(&ask(&s, "ns2.startguess.ing.", RecordType::AAAA, now)),
            ["2602:fbf5:3:c7::53"]
        );
        let nodata = ask(&s, "www.startguess.ing.", RecordType::MX, now);
        assert!(nodata.answers.is_empty());
        assert_eq!(nodata.authorities.len(), 1);
        assert!(nodata.metadata.authoritative);
    }

    #[test]
    fn bait_moves_to_its_own_addresses_once_ordered() {
        let s = crate::shared::test();
        let day = 20751;
        let now = day * 86_400_000 + 3_600_000;
        let name = format!("{}.startguess.ing.", s.cfg.key.name(day));
        assert_eq!(ips(&ask(&s, &name, RecordType::A, now)), ["23.144.156.224"]);
        s.ordered.write().unwrap().insert(day, now);
        assert_eq!(
            ips(&ask(&s, &name, RecordType::A, now)),
            [s.cfg.v4(day).to_string()]
        );
        assert_eq!(
            ips(&ask(&s, &name, RecordType::AAAA, now)),
            [s.cfg.v6(day).to_string()]
        );
    }

    #[test]
    fn challenges_are_served_from_memory() {
        let s = crate::shared::test();
        s.challenges.write().unwrap().insert(
            "_acme-challenge.startguess.ing".into(),
            vec!["token".into()],
        );
        let m = ask(&s, "_ACME-challenge.startguess.ing.", RecordType::TXT, 0);
        assert_eq!(ips(&m), ["token"]);
    }

    #[test]
    fn other_zones_and_transfers_are_refused() {
        let s = crate::shared::test();
        let m = ask(&s, "example.com.", RecordType::A, 0);
        assert_eq!(m.metadata.response_code, ResponseCode::Refused);
        let m = ask(&s, "startguess.ing.", RecordType::AXFR, 0);
        assert_eq!(m.metadata.response_code, ResponseCode::Refused);
    }

    #[test]
    fn client_subnet_is_echoed_with_scope_zero() {
        let s = crate::shared::test();
        let mut req = Message::query();
        req.add_query(Query::query(
            Name::from_ascii("startguess.ing.").unwrap(),
            RecordType::A,
        ));
        let mut edns = Edns::new();
        edns.options_mut()
            .insert(EdnsOption::Subnet(ClientSubnet::new(
                "198.51.100.0".parse().unwrap(),
                24,
                0,
            )));
        req.set_edns(edns);
        let resp = respond(&s, &req, 0);
        let Some(EdnsOption::Subnet(ecs)) = resp.edns.as_ref().unwrap().option(EdnsCode::Subnet)
        else {
            panic!("no ecs");
        };
        assert_eq!((ecs.source_prefix(), ecs.scope_prefix()), (24, 0));
    }
}
