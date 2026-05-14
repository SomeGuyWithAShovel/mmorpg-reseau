use game_sockets::*
use bevy::prelude::*
mod common;
use crate::common::*;

const DEFAULT_ADDRESS : Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

#[derive(Resource)]
pub struct ServerConfig {
    pub id: String,          // UUID généré au démarrage
    pub port: u16,
    pub zone: String,        // ex: "zone_A"
    pub max_players: usize,
    pub orchestrator_address: SocketAddr,
}

impl ServerConfig {    
    fn from_env() -> Self {
        use uuid::Uuid;
        let id = Uuid::new_v4().to_string();
        
        let port = std::env::var("SERVER_PORT")
            .ok()
            .map(|s| s.parse::<u16>().ok())
            .flatten()
            .unwrap_or(0);
        
        ServerConfig {
            id,
            port,
            zone : std::env::var("SERVER_ZONE")
                .unwrap_or("unknown".to_string()),
            max_players : std::env::var("SERVER_MAX_PLAYERS")
                .ok()
                .map(|s| s.parse::<usize>().ok())
                .flatten()
                .unwrap_or(0),
            orchestrator_address : std::env::var("SERVER_SOCKET_ADDRESS").ok()
                .map(|s| Ipv4Addr::from_str(s.as_str()).ok())
                .flatten()
                .map(IpAddr::from)
                .map(|ipv4| SocketAddr::new(ipv4, port))
                .unwrap_or(SocketAddr::new(IpAddr::from(DEFAULT_ADDRESS), port)),
        }
    }
}

impl fmt::Display for ServerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ServerConfig{{\n\
            id:{},          \n\
            port:{},        \n\
            zone:{},        \n\
            max_players:{}, \n\
            orchestrator_address:{}\n\
            }}",
            self.id, self.port, self.zone, self.max_players, self.orchestrator_address)        
    }
}
