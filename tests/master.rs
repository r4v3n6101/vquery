use std::net::ToSocketAddrs;
use vquery::master::*;

const ADDR: &str = "hl2master.steampowered.com:27011";

#[test]
fn print_master_server_list() {
    let master = MasterServerQuery::bind("0.0.0.0:0".parse().unwrap()).unwrap();
    master
        .connect(ADDR.to_socket_addrs().unwrap().next().unwrap())
        .unwrap();
    let ips = master.request(Region::All, &[]).unwrap();
    println!("{:?}", ips);
}
