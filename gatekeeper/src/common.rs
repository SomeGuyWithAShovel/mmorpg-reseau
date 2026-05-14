// use crate::common::* ; in other files

use std::net::{
    Ipv4Addr,
    SocketAddrV4,
};

pub const LISTEN_IP: [u8; 4] = [127, 0, 0, 1];
pub const LISTEN_PORT:  u16  = 3000;

pub const fn get_socket_addr_v4(ip: [u8; 4], port: u16) -> SocketAddrV4
{
    return SocketAddrV4::new(Ipv4Addr::from_octets(ip), port);
}

pub const LISTEN_SOCK_ADDR_V4: SocketAddrV4 = get_socket_addr_v4(LISTEN_IP, LISTEN_PORT);
