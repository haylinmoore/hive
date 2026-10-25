use nix::libc;
use nix::sys::socket::{
    self, ControlMessage, ControlMessageOwned, MsgFlags, SockaddrStorage, setsockopt, sockopt,
};
use socket2::{Domain, Protocol, Socket, Type};
use std::io::{self, IoSlice, IoSliceMut};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::os::fd::AsRawFd;
use tokio::io::Interest;
use tokio::net::{TcpListener, UdpSocket};

fn socket(addr: SocketAddr, ty: Type, proto: Protocol) -> io::Result<Socket> {
    let s = Socket::new(Domain::for_address(addr), ty, Some(proto))?;
    s.set_reuse_address(true)?;
    if addr.is_ipv6() {
        s.set_only_v6(true)?;
        s.set_ip_transparent_v6(true)?;
    } else {
        s.set_ip_transparent_v4(true)?;
    }
    s.set_nonblocking(true)?;
    Ok(s)
}

pub fn tcp(addr: SocketAddr) -> io::Result<TcpListener> {
    let s = socket(addr, Type::STREAM, Protocol::TCP)?;
    s.bind(&addr.into())?;
    s.listen(1024)?;
    TcpListener::from_std(s.into())
}

pub fn udp(addr: SocketAddr) -> io::Result<UdpSocket> {
    let s = socket(addr, Type::DGRAM, Protocol::UDP)?;
    if addr.is_ipv6() {
        setsockopt(&s, sockopt::Ipv6OrigDstAddr, &true)?;
    } else {
        setsockopt(&s, sockopt::Ipv4OrigDstAddr, &true)?;
    }
    s.bind(&addr.into())?;
    UdpSocket::from_std(s.into())
}

fn std_addr(a: &SockaddrStorage) -> Option<SocketAddr> {
    if let Some(v4) = a.as_sockaddr_in() {
        return Some(SocketAddr::new(IpAddr::V4(v4.ip()), v4.port()));
    }
    let v6 = a.as_sockaddr_in6()?;
    Some(SocketAddr::new(IpAddr::V6(v6.ip()), v6.port()))
}

pub async fn recv(sock: &UdpSocket, buf: &mut [u8]) -> io::Result<(usize, SocketAddr, SocketAddr)> {
    let fallback = sock.local_addr()?;
    sock.async_io(Interest::READABLE, || {
        let mut cmsg = nix::cmsg_space!(libc::sockaddr_in6);
        let mut iov = [IoSliceMut::new(buf)];
        let msg = socket::recvmsg::<SockaddrStorage>(
            sock.as_raw_fd(),
            &mut iov,
            Some(&mut cmsg),
            MsgFlags::empty(),
        )?;
        let src = msg
            .address
            .as_ref()
            .and_then(std_addr)
            .ok_or_else(|| io::Error::other("datagram without a source"))?;
        let mut dst = fallback;
        for c in msg.cmsgs()? {
            match c {
                ControlMessageOwned::Ipv4OrigDstAddr(a) => {
                    dst = SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::from(u32::from_be(a.sin_addr.s_addr))),
                        u16::from_be(a.sin_port),
                    )
                }
                ControlMessageOwned::Ipv6OrigDstAddr(a) => {
                    dst = SocketAddr::new(
                        IpAddr::V6(Ipv6Addr::from(a.sin6_addr.s6_addr)),
                        u16::from_be(a.sin6_port),
                    )
                }
                _ => {}
            }
        }
        Ok((msg.bytes, src, dst))
    })
    .await
}

pub async fn send(sock: &UdpSocket, data: &[u8], from: IpAddr, to: SocketAddr) -> io::Result<()> {
    let to = SockaddrStorage::from(to);
    sock.async_io(Interest::WRITABLE, || {
        let iov = [IoSlice::new(data)];
        let v4;
        let v6;
        let cmsg = match from {
            IpAddr::V4(a) => {
                v4 = libc::in_pktinfo {
                    ipi_ifindex: 0,
                    ipi_spec_dst: libc::in_addr {
                        s_addr: u32::from(a).to_be(),
                    },
                    ipi_addr: libc::in_addr { s_addr: 0 },
                };
                ControlMessage::Ipv4PacketInfo(&v4)
            }
            IpAddr::V6(a) => {
                v6 = libc::in6_pktinfo {
                    ipi6_addr: libc::in6_addr {
                        s6_addr: a.octets(),
                    },
                    ipi6_ifindex: 0,
                };
                ControlMessage::Ipv6PacketInfo(&v6)
            }
        };
        socket::sendmsg(
            sock.as_raw_fd(),
            &iov,
            &[cmsg],
            MsgFlags::empty(),
            Some(&to),
        )?;
        Ok(())
    })
    .await
}
