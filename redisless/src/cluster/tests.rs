use crate::cluster::util::{get_ip_addresses, get_local_network_ip_addresses};
use std::net::{IpAddr, Ipv4Addr};

#[test]
fn start_cluster() {
    // TODO
}

#[test]
fn list_ip_addresses() {
    let ip_addresses = get_ip_addresses();
    assert!(ip_addresses.len() > 0);
}

#[test]
fn get_local_ip_addresses() {
    let ip_addresses = get_local_network_ip_addresses(vec![
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        IpAddr::V4(Ipv4Addr::new(10, 3, 0, 6)),
        IpAddr::V4(Ipv4Addr::BROADCAST),
        IpAddr::V4(Ipv4Addr::new(86, 66, 43, 4)),
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 4)),
    ]);

    assert_eq!(ip_addresses.len(), 2);
}
