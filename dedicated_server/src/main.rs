use bevy::prelude::*;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
mod common;
use crate::common::*;
mod server;
use crate::server::*;
use game_sockets::*;
use game_sockets::protocols::QuicBackend;

// TODO : PlayerInfo doit contenir le DedicatedServerPeer
pub struct PlayerInfo {
    // S'il y a pas de username, le joueur n'a pas join
    username : String,
    stream : GameStream,
}

#[derive(Resource, Default)]
pub struct PlayerRegistry {
    pub players: HashMap<GameConnection, PlayerInfo>,
}

#[derive(Resource)]
struct HeartbeatTimer(Timer);

#[derive(Clone)]
struct OrchestratorConnection {
    connection : GameConnection,
    stream : GameStream,
}

#[derive(Resource)]
struct DedicatedServerPeer {
    peer: GamePeer,
    orchestrator : Option<OrchestratorConnection>,
}

fn get_own_ip() -> &'static str {
    // On peut mieux faire c'est sûr
    return "127.0.0.1";
}

// main.rs
fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .insert_resource(ServerConfig::from_env())
        .insert_resource(HeartbeatTimer(Timer::from_seconds(SECONDS_BETWEEN_HEARTBEATS , TimerMode::Repeating)))
        .add_systems(Startup, bind_socket)
        .add_systems(Startup, debug_info)
        .add_systems(Update, (receive_packets, send_heartbeat).chain())
        .run();
}

fn debug_info(config : Res<ServerConfig>) {
    info!("{}", config.into_inner());
}

fn bind_socket(mut commands : Commands, config : Res<ServerConfig>) -> Result {       
    let game_peer = GamePeer::new(QuicBackend::new());

    game_peer.listen("0.0.0.0", config.port)?;   
    let orch_address = config.orchestrator_address;    
    game_peer.connect(&orch_address.ip().to_string().as_str(), orch_address.port())?;
    
    commands.insert_resource(DedicatedServerPeer{peer:game_peer, orchestrator:None});
    commands.insert_resource(PlayerRegistry{players:HashMap::new()});
    
    Ok(())
}



fn receive_packets(mut peer_res : ResMut<DedicatedServerPeer>,
                   mut player_registry : ResMut<PlayerRegistry>) -> Result {
    if let Some(event) = peer_res.peer.poll()? {
        // Joueur créent le stream
        // DS crée le stream avec l'orchestrateur
        match event {
            GameNetworkEvent::Connected(connection) => {
                info!("Connexion client : {:?}", connection);
            }
            GameNetworkEvent::Disconnected(connection) => {
                info!("Déconnexion client : {:?}", connection);
                player_registry.players.remove(&connection);
            }
            GameNetworkEvent::Message{ connection, stream, data } => {
                if let Ok(str_data) = str::from_utf8(&data[..]) {
                    if str_data.starts_with("JOIN") && let Some(username) = str_data.strip_prefix("JOIN { ").and_then(|s| s.strip_suffix(" }")) {
                        let player_info = PlayerInfo{
                            username: username.to_string(),
                            stream
                        };
                        player_registry.players.insert(connection, player_info);
                    }                        
                }
                else {
                    warn!("Donnée non UTF8 envoyée par {:?}", connection);
                }
            }
            GameNetworkEvent::Error { connection:_connection, inner } => {
                warn!("Erreur de l'orchestrateur : {:?}", inner);
            }
            GameNetworkEvent::StreamCreated(connection, stream) => {
                // Stream avec l'orchestrateur
                peer_res.orchestrator = Some(OrchestratorConnection{connection, stream});
            }
            GameNetworkEvent::StreamClosed(_, _) => {
                peer_res.orchestrator = None;
            }
        }
    }
    Ok(())
}


fn send_heartbeat(
    time: Res<Time>,
    mut timer: ResMut<HeartbeatTimer>,
    player_registry : Res<PlayerRegistry>,
    config : Res<ServerConfig>,
    peer_res : ResMut<DedicatedServerPeer>) -> Result {
    
    if timer.0.tick(time.delta()).just_finished() {
        if let Some(orchestrator) = &peer_res.orchestrator {
            let player_count = player_registry.players.len();
            let is_full = player_count == config.max_players;
            let ip = IpAddr::from_str(get_own_ip())?;
            let ds_address = SocketAddr::new(ip, config.port);
            
            peer_res.peer.send(
                &orchestrator.connection,
                &orchestrator.stream,
                Heartbeat{
                    id: config.id.clone(),
                    addr: ds_address,
                    zone: config.zone.clone(),
                    player_count,
                    is_full,
                }.to_bytes(),
            )?;
        }
    }
    Ok(())
}
