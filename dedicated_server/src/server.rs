use bevy::prelude::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::fmt;
use std::str::FromStr;

const DEFAULT_ADDRESS : Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
const DEFAULT_PORT : u16 = 28080;

#[derive(Resource, Debug)]
pub struct ServerConfig {
    pub id: String,          // UUID généré au démarrage
    pub port: u16,
    pub zone: String,        // ex: "zone_A"
    pub max_players: usize,
    pub orchestrator_address: SocketAddr,
}

impl ServerConfig {    
    pub fn from_env() -> Self {
        use uuid::Uuid;
        let id = Uuid::new_v4().to_string();
        
        let port = std::env::var("DS_PORT")
            .ok()
            .map(|s| s.parse::<u16>().ok())
            .flatten()
            .unwrap_or(DEFAULT_PORT);

        let orch_port = std::env::var("ORCH_PORT")
            .ok()
            .map(|s| s.parse::<u16>().ok())
            .flatten()
            .unwrap_or(DEFAULT_PORT);
        
        ServerConfig {
            id,
            port,
            zone : std::env::var("DS_ZONE")
                .unwrap_or("unknown".to_string()),
            max_players : std::env::var("DS_MAX_PLAYERS")
                .ok()
                .map(|s| s.parse::<usize>().ok())
                .flatten()
                .unwrap_or(0),
            orchestrator_address : std::env::var("ORCH_ADDRESS").ok()
                .map(|s| Ipv4Addr::from_str(s.as_str()).ok())
                .flatten()
                .map(IpAddr::from)
                .map(|ipv4| SocketAddr::new(ipv4, orch_port))
                .unwrap_or(SocketAddr::new(IpAddr::from(DEFAULT_ADDRESS), orch_port)),
        }
    }
}

impl fmt::Display for ServerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ServerConfig{{\
            id:{},          \
            port:{},        \
            zone:{},        \
            max_players:{}, \
            orchestrator_address:{}\
            }}",
            self.id, self.port, self.zone, self.max_players, self.orchestrator_address)        
    }
}
