use std::net::SocketAddr;

pub const SECONDS_BETWEEN_HEARTBEATS : f32 = 5.0;

pub struct Heartbeat {
    pub id : String,
    pub ip : SocketAddr,
    pub port : u16,
    pub zone : String,
    pub player_count : usize,
}
