use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};

use crate::cluster::node::ClusterNode;
use crate::cluster::peer::{Peer, PeersDiscovery, DEFAULT_NODE_LISTENING_PORT};
use crate::cluster::util::{
    get_ip_addresses, get_local_network_ip_addresses, get_range_from_ip_address, scan_ip_range,
    Range,
};

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

#[test]
fn get_ip_range() {
    assert_eq!(
        get_range_from_ip_address(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 4)), Range::Sixteen).len(),
        65_536
    );

    assert_eq!(
        get_range_from_ip_address(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 4)), Range::TwentyFour)
            .len(),
        256
    );

    assert_eq!(
        get_range_from_ip_address(IpAddr::V4(Ipv4Addr::new(172, 24, 23, 188)), Range::Sixteen)
            .len(),
        65_536
    );

    assert_eq!(
        get_range_from_ip_address(
            IpAddr::V4(Ipv4Addr::new(172, 24, 23, 188)),
            Range::TwentyFour,
        )
        .len(),
        256
    );

    assert_eq!(
        get_range_from_ip_address(IpAddr::V4(Ipv4Addr::new(10, 55, 24, 254)), Range::Sixteen).len(),
        65_536
    );

    assert_eq!(
        get_range_from_ip_address(
            IpAddr::V4(Ipv4Addr::new(10, 55, 24, 254)),
            Range::TwentyFour,
        )
        .len(),
        256
    );
}

#[test]
fn test_scan_ip_range_no_result() {
    let ip_addresses = get_range_from_ip_address(
        IpAddr::V4(Ipv4Addr::new(10, 55, 24, 254)),
        Range::TwentyFour,
    );

    let opened_sockets = scan_ip_range(ip_addresses, vec![DEFAULT_NODE_LISTENING_PORT]);

    assert_eq!(opened_sockets.len(), 0);
}

#[test]
fn test_scan_ip_range_with_4_peers() {
    let mut nodes: Vec<ClusterNode> = (0..3u16)
        .map(|i| {
            Peer::new(
                format!("{}", i),
                PeersDiscovery::Automatic(DEFAULT_NODE_LISTENING_PORT),
                SocketAddr::V4(SocketAddrV4::new(
                    Ipv4Addr::UNSPECIFIED,
                    DEFAULT_NODE_LISTENING_PORT + i,
                )),
            )
            .into_cluster_node()
        })
        .collect();

    let ports: Vec<u16> = (0..3u16).map(|i| DEFAULT_NODE_LISTENING_PORT + i).collect();

    // flatten ip ranges from local ip addresses
    let ip_addresses = get_ip_addresses()
        .into_iter()
        .fold(vec![], |mut results, ip_addr| {
            results.extend(get_range_from_ip_address(ip_addr, Range::TwentyFour));
            results
        });

    let opened_sockets = scan_ip_range(ip_addresses, ports);

    assert_eq!(opened_sockets.len(), 0);

    // start nodes
    for node in nodes.iter_mut() {
        node.start_listener();
    }

    //assert_eq!(opened_sockets.len(), 4);

    // stop nodes
    for node in nodes.iter_mut() {
        node.stop_listener();
    }

    //assert_eq!(opened_sockets.len(), 0);
}
