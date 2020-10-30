use std::{
    collections::HashSet,
    net::{Ipv4Addr, SocketAddrV4, ToSocketAddrs},
};
use vquery::master::*;

const ADDR: &str = "hl2master.steampowered.com:27011";

#[test]
fn print_query_iter() {
    let master = MasterServerQuery::bind("0.0.0.0:0".parse().unwrap()).unwrap();
    master
        .connect(ADDR.to_socket_addrs().unwrap().next().unwrap())
        .unwrap();

    let ips: Vec<_> = master.iter(Region::All, &[]).take(1000).collect();
    println!("{:?}", ips);
}

#[test]
fn validate_query_iter() {
    let nul_addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0);

    let master = MasterServerQuery::bind("0.0.0.0:0".parse().unwrap()).unwrap();
    master
        .connect(ADDR.to_socket_addrs().unwrap().next().unwrap())
        .unwrap();

    let ips: Vec<_> = master.iter(Region::All, &[]).take(1000).collect();
    let unique_ips = ips.iter().collect::<HashSet<_>>();
    assert_eq!(ips.len(), unique_ips.len());
    assert!(!unique_ips.contains(&nul_addr));
}
