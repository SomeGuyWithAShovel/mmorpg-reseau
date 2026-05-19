use bevy::prelude::*;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
mod server;
use crate::server::*;
use shared::*;
use game_sockets::*;
use game_sockets::protocols::{QuicBackend, UdpBackend};
use bytes::Bytes;

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
    heartbeat_peer: GamePeer,
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
        .add_systems(Startup, bind_socket.chain())
        .add_systems(Startup, debug_info)
        .add_systems(Update, (receive_packets, send_heartbeat_periodically).chain())
        .run();
}

fn debug_info(config : Res<ServerConfig>) {
    println!("{:?}", config);
    if let Ok(ip) = IpAddr::from_str(get_own_ip()) {
        let ds_address = SocketAddr::new(ip, config.port);
        println!("{:?}", Heartbeat{
            id: config.id.clone(),
            addr: ds_address,
            zone: config.zone.clone(),
            player_count: 0,
            is_full: false,
        }.to_bytes().to_vec());
    }
    else {
        println!("Adresse localhost invalide ?");
    }
}

fn bind_socket(mut commands : Commands, config : Res<ServerConfig>) -> Result {

    const HEARTBEAT_PORT : u16 = 47347;
    
    let heartbeat_peer = GamePeer::new(UdpBackend::new());
    let quic_game_peer = GamePeer::new(QuicBackend::new());

    quic_game_peer.listen("0.0.0.0", config.port)?;
    
    let orch_address = config.orchestrator_address;
    heartbeat_peer.connect(&orch_address.ip().to_string().as_str(), HEARTBEAT_PORT)?;    
    
    commands.insert_resource(DedicatedServerPeer{peer:quic_game_peer, heartbeat_peer, orchestrator:None});
    commands.insert_resource(PlayerRegistry{players:HashMap::new()});
    
    Ok(())
}



fn receive_packets(
    config : Res<ServerConfig>,
    mut peer_res : ResMut<DedicatedServerPeer>,
    mut player_registry : ResMut<PlayerRegistry>) -> Result {
    if let Some(event) = peer_res.peer.poll()? {
        match event {
            GameNetworkEvent::Connected(connection) => {
                println!("Connexion client : {:?}", connection);
            }
            GameNetworkEvent::Disconnected(connection) => {
                println!("Déconnexion client {:?}", connection);
                player_registry.players.remove(&connection);
            }
            GameNetworkEvent::Message{ connection, stream, data } => {
                if let Ok(str_data) = str::from_utf8(&data[..]) {
                    if str_data.starts_with("JOIN") && let Some(username) = str_data.strip_prefix("JOIN { ").and_then(|s| s.strip_suffix(" }")) {
                        let response = Bytes::from(format!("WELCOME {{ {} }}", username));
                        peer_res.peer.send(&connection, &stream, response)?;
                        let player_info = PlayerInfo {
                            username: username.to_string(),
                            stream
                        };
                        player_registry.players.insert(connection, player_info);
                    }
                }
                else {
                    println!("Donnée non UTF8 envoyée par {:?}", connection);
                }
            }
            GameNetworkEvent::StreamCreated(_, _) => {}
            GameNetworkEvent::StreamClosed(connection, _) => {
                player_registry.players.remove(&connection);
            }
            GameNetworkEvent::Error { connection:_connection, inner } => {
                println!("Erreur du client : {:?}", inner);
            }
        }
    }

    if let Some(event) = peer_res.heartbeat_peer.poll()? {
        match event {
            GameNetworkEvent::Connected(connection) => {                
                peer_res.heartbeat_peer.create_stream(connection, GameStreamReliability::Unreliable)?;
            }
            GameNetworkEvent::Disconnected(_) => {
                println!("Déconnexion de l'orchestrateur");
                peer_res.orchestrator = None;
            }
            GameNetworkEvent::Message{ connection:_, stream:_, data } => {
                if let Ok(msg) = str::from_utf8(&data[..]) {
                    println!("Message de l'orchestrateur : {}", msg);
                }
                else {
                    println!("Message de l'orchestrateur reçu (non convertible en utf8: {:?}", data);
                }
            }
            GameNetworkEvent::Error { connection:_, inner } => {
                println!("Erreur de l'orchestrateur : {:?}", inner);
            }
            GameNetworkEvent::StreamCreated(connection, stream) => {
                if !peer_res.orchestrator.is_some() {
                    println!("Création du stream avec l'orchestrateur : {:?}", connection);
                    peer_res.orchestrator = Some(OrchestratorConnection{connection, stream});
                    send_heartbeat(player_registry.into(), config, peer_res);
                }
            }
            GameNetworkEvent::StreamClosed(_connection, _stream) => {
                if let Some(_) = &peer_res.orchestrator {
                    println!("Stream de l'orchestrateur fermé ?");
                    peer_res.orchestrator = None;
                }
            }
        }
    }
    
    Ok(())
}


fn send_heartbeat(
    player_registry : Res<PlayerRegistry>,
    config : Res<ServerConfig>,
    peer_res : ResMut<DedicatedServerPeer>) -> Result {
    
    if let Some(orchestrator) = &peer_res.orchestrator {
        let player_count = player_registry.players.len();
        let is_full = player_count == config.max_players;
        let ip = IpAddr::from_str(get_own_ip())?;
        let ds_address = SocketAddr::new(ip, config.port);
        
        peer_res.heartbeat_peer.send(
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
    Ok(())
}

fn send_heartbeat_periodically(
    time: Res<Time>,
    mut timer: ResMut<HeartbeatTimer>,
    player_registry : Res<PlayerRegistry>,
    config : Res<ServerConfig>,
    peer_res : ResMut<DedicatedServerPeer>) -> Result {
    
    if timer.0.tick(time.delta()).just_finished() {
        return send_heartbeat(player_registry, config, peer_res);
    }
    else {
        Ok(())
    }
}
