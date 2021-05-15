use std::net::IpAddr;

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
                    IpAddr::V6(x) => false,
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
