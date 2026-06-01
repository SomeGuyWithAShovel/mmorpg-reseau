use bevy::prelude::*;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

mod server;
use crate::server::*;
mod entity;
use crate::entity::*;
use shared::{*, game_message::*, input::*, entity::EntityState};
mod messages;
use messages::*;

use game_sockets::*;
use game_sockets::protocols::{QuicBackend, UdpBackend};
use bytes::Bytes;

/* 
 *  Écrit à l'aide des exemples issus du dossier game_sockets/bin
 */


// TODO : PlayerInfo doit contenir le DedicatedServerPeer
pub struct PlayerInfo {
    // S'il y a pas de username, le joueur n'a pas join
    username : String,
    stream : GameStream,
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
        .add_plugins(EntityPlugin)
        .add_plugins(MessagePlugin)
        .insert_resource(ServerConfig::from_env())
        .insert_resource(HeartbeatTimer(Timer::from_seconds(SECONDS_BETWEEN_HEARTBEATS , TimerMode::Repeating)))
        .add_systems(Startup, bind_socket.chain())
        .add_systems(Startup, debug_info)
    // PreUpdate pour passer avant FixedUpdate de EntityPlugin
    // Ordre : https://docs.rs/bevy/0.13.2/bevy/app/struct.Main.html
        .add_systems(PreUpdate, receive_packets)
        .add_systems(Update, send_heartbeat_periodically)        
        .run();
}

fn debug_info(config : Res<ServerConfig>) {
    info!("Config serveur : {:?}", config);
    if let Ok(ip) = IpAddr::from_str(get_own_ip()) {
        let ds_address = SocketAddr::new(ip, config.port);
        let hb = Heartbeat{
            id: config.id.clone(),
            addr: ds_address,
            zone: config.zone.clone(),
            player_count: 0,
            is_full: false,
        };
        info!("Heartbeat : {:?}", hb);
        info!("Heartbeat en octets : {:?}", hb.to_bytes().to_vec());
    }
    else {
        error!("Adresse localhost invalide ?");
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
    
    Ok(())
}

fn message_received(
    peer_res : &DedicatedServerPeer,
    connection : &GameConnection,
    stream : &GameStream,
    data : Bytes,
    player_input_writer : &mut MessageWriter<PlayerActionHolderMessage>,
    entity_creation_writer : &mut MessageWriter<CreateEntity>,
    ghost_update_writer : &mut MessageWriter<UpdateGhostEntity>,
    unghost_writer : &mut MessageWriter<GhostToOwned>) -> Result {
    if let Some(message) = GameMessage::from_bytes(data.clone()) {
        match message {
            GameMessage::ClientInput{ client_id, input } => {
                let act = PlayerActionHolder{data : input[0]};
                player_input_writer.write(PlayerActionHolderMessage{id:client_id, act});
            }
            GameMessage::HandoffRequest { entity_id, pos, vel, state } => {
                // Créer l'entité dans le serveur en mode "Ghost"
                if let Some(entity_state) = EntityState::from_bytes(Bytes::copy_from_slice(&state)) {
                    entity_creation_writer.write(CreateEntity {
                        tag: EntityTag {
                            id: entity_id,
                            state: EntityNetworkState::Ghost,
                        },
                        pos, vel,
                        state: entity_state,
                    });
                    peer_res.peer.send(connection, stream, GameMessage::HandoffAccept{entity_id}.to_bytes())?;
                }
                else {
                    warn!("État d'entité invalide pour la mise à jour: {:?}", state);
                    peer_res.peer.send(connection, stream, GameMessage::HandoffReject{entity_id}.to_bytes())?;
                }
            }
            GameMessage::GhostUpdate { entity_id, pos, vel, state } => {
                if let Some(entity_state) = EntityState::from_bytes(Bytes::copy_from_slice(&state)) {
                    ghost_update_writer.write(UpdateGhostEntity{id: entity_id, pos, vel, state: entity_state});
                }
                else {
                    warn!("État d'entité invalide pour la mise à jour: {:?}", state);
                }
            }
            GameMessage::HandoffComplete { entity_id } => {
                unghost_writer.write(GhostToOwned{id: entity_id});
            }
            _ => {
                warn!("Message non interprétable par le serveur : {:?}", message);
            }
        }
    }
    else {
        warn!("Message non désérialisable : {:?}", data);
    }
    Ok(())
}

fn receive_packets(
    config : Res<ServerConfig>,
    players : Query<(), With<PlayerTag>>,
    mut peer_res : ResMut<DedicatedServerPeer>,
    mut player_input_writer : MessageWriter<PlayerActionHolderMessage>,
    mut entity_creation_writer : MessageWriter<CreateEntity>,
    mut ghost_update_writer : MessageWriter<UpdateGhostEntity>,
    mut unghost_writer : MessageWriter<GhostToOwned>) -> Result {
    if let Some(event) = peer_res.peer.poll()? {
        //todo!("Gestion de la connexion avec le broker plutôt que des clients");
        match event {
            GameNetworkEvent::Connected(connection) => {
                info!("Connexion client : {:?}", connection);
            }
            GameNetworkEvent::Disconnected(connection) => {
                info!("Déconnexion client {:?}", connection);
            }
            GameNetworkEvent::Message{ connection, stream, data } => {
                message_received(&peer_res, &connection, &stream, data,
                                 &mut player_input_writer,
                                 &mut entity_creation_writer,
                                 &mut ghost_update_writer,
                                 &mut unghost_writer)?;
            }
            GameNetworkEvent::StreamCreated(_, _) => {}
            GameNetworkEvent::StreamClosed(connection, _) => {}
            GameNetworkEvent::Error { connection:_connection, inner } => {
                error!("Erreur du client : {:?}", inner);
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
                    info!("Message de l'orchestrateur : {}", msg);
                }
                else {
                    info!("Message de l'orchestrateur reçu (non convertible en utf8: {:?}", data);
                }
            }
            GameNetworkEvent::Error { connection:_, inner } => {
                error!("Erreur de l'orchestrateur : {:?}", inner);
            }
            GameNetworkEvent::StreamCreated(connection, stream) => {
                if !peer_res.orchestrator.is_some() {
                    info!("Création du stream avec l'orchestrateur : {:?}", connection);
                    peer_res.orchestrator = Some(OrchestratorConnection{connection, stream});
                    let player_count = players.count();
                    send_heartbeat(config, peer_res, player_count)?;
                }
            }
            GameNetworkEvent::StreamClosed(_connection, _stream) => {
                if let Some(_) = &peer_res.orchestrator {
                    info!("Stream de l'orchestrateur fermé ?");
                    peer_res.orchestrator = None;
                }
            }
        }
    }
    
    Ok(())
}


fn send_heartbeat(
    config : Res<ServerConfig>,
    peer_res : ResMut<DedicatedServerPeer>,
    player_count : usize) -> Result {
    
    if let Some(orchestrator) = &peer_res.orchestrator {
        let is_full = player_count == config.max_players;
        let ip = IpAddr::from_str(get_own_ip())?;
        let ds_address = SocketAddr::new(ip, config.port);

        let hb = Heartbeat{
            id: config.id.clone(),
            addr: ds_address,
            zone: config.zone.clone(),
            player_count,
            is_full,
        };
        
        info!("Envoi du heartbeat: {:?}", hb);

        let OrchestratorConnection{connection, stream} = orchestrator;
        peer_res.heartbeat_peer.send(connection, stream, hb.to_bytes())?;
    }
    Ok(())
}

fn send_heartbeat_periodically(
    time: Res<Time>,
    mut timer: ResMut<HeartbeatTimer>,
    players : Query<(), With<PlayerTag>>,
    config : Res<ServerConfig>,
    peer_res : ResMut<DedicatedServerPeer>) -> Result {
    
    if timer.0.tick(time.delta()).just_finished() {
        let player_count = players.count();
        send_heartbeat(config, peer_res, player_count)?;
    }
    Ok(())    
}
