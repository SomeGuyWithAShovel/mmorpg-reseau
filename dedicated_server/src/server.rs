use bevy::prelude::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

const DEFAULT_ADDRESS : Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
const DEFAULT_PORT : u16 = 28080;

#[derive(Resource, Debug)]
pub struct ServerConfig {
    pub id: Uuid,          // UUID généré au démarrage
    pub port: u16,
    pub zone: String,        // ex: "zone_A"
    pub max_players: usize,
    pub orchestrator_address: SocketAddr,
    pub map_borders : Rect,
}

impl ServerConfig {    
    pub fn from_env() -> Self {
        let orch_port = Self::parse_env_var("ORCH_PORT", DEFAULT_PORT);
        
        ServerConfig {
            id : Uuid::new_v4(),
            port : Self::parse_env_var("DS_PORT", DEFAULT_PORT),
            zone : std::env::var("DS_ZONE")
                .unwrap_or("unknown".to_string()),
            max_players : Self::parse_env_var("DS_MAX_PLAYERS", 0usize),
            orchestrator_address : std::env::var("ORCH_ADDRESS").ok()
                .and_then(|s| Ipv4Addr::from_str(s.as_str()).ok())
                .map(IpAddr::from)
                .map(|ipv4| SocketAddr::new(ipv4, orch_port))
                .unwrap_or(SocketAddr::new(IpAddr::from(DEFAULT_ADDRESS), orch_port)),
            map_borders: Self::get_borders_from_env(),
        }
    }

    fn parse_env_var<T : FromStr>(s : &str, default : T) -> T{
        std::env::var(s)
            .ok()
            .and_then(|s| s.parse::<T>().ok())
            .unwrap_or(default)
    }
    
    fn get_borders_from_env() -> Rect {
        let top    = Self::parse_env_var("DS_BORDER_TOP"    , 0.0);
        let left   = Self::parse_env_var("DS_BORDER_LEFT"   , 0.0);
        let bottom = Self::parse_env_var("DS_BORDER_BOTTOM" , 0.0);
        let right  = Self::parse_env_var("DS_BORDER_RIGHT"  , 0.0);
        return Rect::new(top, left, bottom, right);
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
