use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::thread;
use std::time::Duration;

use crossbeam_channel::unbounded;
use ipnet::Ipv4AddrRange;

use crate::cluster::node::{GETINFO_REQUEST, GETINFO_RESPONSE};

pub fn get_local_network_ip_addresses(ip_addresses: Vec<IpAddr>) -> Vec<IpAddr> {
    ip_addresses
        .into_iter()
        .filter(|ip_address| {
            ip_address.is_ipv4()
                && !ip_address.is_loopback()
                && !ip_address.is_unspecified()
                && !ip_address.is_multicast()
                && match ip_address {
                    IpAddr::V4(ip_address) => ip_address.is_private(),
                    IpAddr::V6(_) => false,
                }
        })
        .collect::<Vec<IpAddr>>()
}

pub fn get_ip_addresses() -> Vec<IpAddr> {
    let mut ip_addresses = vec![];

    if let Ok(interfaces) = get_if_addrs::get_if_addrs() {
        for interface in interfaces {
            ip_addresses.push(interface.ip());
        }
    }

    ip_addresses
}

#[allow(dead_code)]
pub enum Range {
    Sixteen,
    TwentyFour,
}

/// from an `ip_address` return all the ip_addresses coming from the same range
/// supported ranges:
/// - 10.0.0.0/8
/// - 172.16.0.0/12
/// - 192.168.0.0/16
#[allow(dead_code)]
pub fn get_range_from_ip_address(ip_address: IpAddr, range: Range) -> Vec<IpAddr> {
    let ip_address = match ip_address {
        IpAddr::V4(ip_address) => ip_address,
        IpAddr::V6(_) => return vec![], // do not support ipv6
    };

    let ip_addresses = match ip_address.octets() {
        [10, b, c, _] => match range {
            Range::Sixteen => Ipv4AddrRange::new(
                format!("10.{}.0.0", b).parse().unwrap(),
                format!("10.{}.255.255", b).parse().unwrap(),
            ),
            Range::TwentyFour => Ipv4AddrRange::new(
                format!("10.{}.{}.0", b, c).parse().unwrap(),
                format!("10.{}.{}.255", b, c).parse().unwrap(),
            ),
        }, // 10.0.0.0/8
        [172, b, c, _] if (16..=31).contains(&b) => match range {
            Range::Sixteen => Ipv4AddrRange::new(
                format!("172.{}.0.0", b).parse().unwrap(),
                format!("172.{}.255.255", b).parse().unwrap(),
            ),
            Range::TwentyFour => Ipv4AddrRange::new(
                format!("172.{}.{}.0", b, c).parse().unwrap(),
                format!("172.{}.{}.255", b, c).parse().unwrap(),
            ),
        }, // 172.16.0.0/12
        [192, 168, c, _] => match range {
            Range::Sixteen => Ipv4AddrRange::new(
                "192.168.0.0".parse().unwrap(),
                "192.168.255.255".parse().unwrap(),
            ),
            Range::TwentyFour => Ipv4AddrRange::new(
                format!("192.168.{}.0", c).parse().unwrap(),
                format!("192.168.{}.255", c).parse().unwrap(),
            ),
        }, // 192.168.0.0/16,
        _ => return vec![],
    };

    ip_addresses
        .into_iter()
        .map(IpAddr::V4)
        .collect()
}

enum ParallelResponse<T> {
    Ok(T),
    Continue,
    #[allow(dead_code)]
    End,
}

type PeerId = String;

/// TCP scan a range of ip addresses with a list of ports
/// return a list of ip addresses with the associated port that are open
pub fn scan_ip_range(
    ip_addresses: Vec<IpAddr>,
    ports_to_scan: Vec<u16>,
) -> Vec<(PeerId, SocketAddr)> {
    let mut opened_sockets = vec![];

    let thread_pool = match rayon::ThreadPoolBuilder::new()
        .thread_name(|_| "scan range".to_string())
        .build()
    {
        Ok(pool) => pool,
        Err(err) => {
            panic!("{:?}", err);
        }
    };

    let (tx, rx) = unbounded::<ParallelResponse<(PeerId, SocketAddr)>>();

    thread::spawn(move || {
        for ip_address in ip_addresses {
            let _tx = tx.clone();
            let ports = ports_to_scan.clone();

            let _ = thread_pool.spawn(move || {
                let tx = _tx.clone();
                let ports = ports;

                for port in ports.iter() {
                    let socket_addr = SocketAddr::new(ip_address, *port);

                    let res =
                        match TcpStream::connect_timeout(&socket_addr, Duration::from_millis(10)) {
                            Ok(mut tcp_stream) => {
                                // check that the remote is valid
                                let _ = tcp_stream.write(GETINFO_REQUEST);

                                let mut response_buffer = [0; 256];
                                let _ = tcp_stream.read(&mut response_buffer);

                                match response_buffer {
                                    res if res.starts_with(GETINFO_RESPONSE) => {
                                        let node_id = &res[GETINFO_RESPONSE.len()..]; // "getinfo:<node_id>"
                                        ParallelResponse::Ok((
                                            String::from_utf8(node_id.to_vec()).unwrap(),
                                            socket_addr,
                                        ))
                                    }
                                    _ => ParallelResponse::Continue,
                                }
                            } // socket opened - ip + port does exist
                            Err(_) => ParallelResponse::Continue, // can't open a socket - then continue
                        };

                    let _ = tx.send(res);
                }
            });
        }
    });

    for res in rx {
        match res {
            ParallelResponse::Ok(res) => {
                opened_sockets.push(res);
            }
            ParallelResponse::Continue => continue,
            ParallelResponse::End => break,
        }
    }

    opened_sockets
}
