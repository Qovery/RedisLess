use std::net::{IpAddr, Ipv4Addr};

use ipnet::{Ipv4AddrRange, Ipv4Net};

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

    for interfaces in get_if_addrs::get_if_addrs() {
        for interface in interfaces {
            ip_addresses.push(interface.ip());
        }
    }

    ip_addresses
}

/// from an `ip_address` return all the ip_addresses coming from the same range
/// supported ranges:
/// - 10.0.0.0/8
/// - 172.16.0.0/12
/// - 192.168.0.0/16
pub fn get_range_from_ip_address(ip_address: IpAddr) -> Vec<IpAddr> {
    let ip_address = match ip_address {
        IpAddr::V4(ip_address) => ip_address,
        IpAddr::V6(_) => return vec![], // do not support ipv6
    };

    let ip_addresses = match ip_address.octets() {
        [10, _, _, _] => Ipv4AddrRange::new(
            "10.0.0.0".parse().unwrap(),
            "10.255.255.255".parse().unwrap(),
        ), // 10.0.0.0/8
        [172, b, _, _] if b >= 16 && b <= 31 => Ipv4AddrRange::new(
            "172.16.0.0".parse().unwrap(),
            "172.31.255.255".parse().unwrap(),
        ), // 172.16.0.0/12
        [192, 168, _, _] => Ipv4AddrRange::new(
            "192.168.0.0".parse().unwrap(),
            "192.168.255.255".parse().unwrap(),
        ), // 192.168.0.0/16,
        _ => return vec![],
    };

    ip_addresses
        .into_iter()
        .map(|ip_address| IpAddr::V4(ip_address))
        .collect()
}
