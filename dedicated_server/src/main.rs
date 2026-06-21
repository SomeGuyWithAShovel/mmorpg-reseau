use bevy::prelude::*;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use bytes::{Buf, BytesMut};

mod server;
use crate::server::*;
mod entity;
use crate::entity::*;
use shared::{*, game_message::*, entity::{Velocity, EntityState}, topic::TopicContent};
mod messages;
use messages::*;

use game_sockets::*;
use game_sockets::protocols::{QuicBackend, UdpBackend};
use bytes::Bytes;

/* 
 *  Écrit à l'aide des exemples issus du dossier game_sockets/bin
 */

#[derive(Resource)]
struct HeartbeatTimer(Timer);

#[derive(Clone)]
struct DedicatedServerConnection {
    connection : GameConnection,
    stream : GameStream,
}

#[derive(Resource, Deref, DerefMut)]
struct NetworkMessage(BytesMut);

#[derive(Resource)]
struct DedicatedServerPeer {
    broker_peer: GamePeer,
    broker_connection : Option<DedicatedServerConnection>,
    heartbeat_peer: GamePeer,
    orchestrator_connection : Option<DedicatedServerConnection>,
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
        .insert_resource(NetworkMessage(BytesMut::new()))
        .insert_resource(ServerConfig::from_env())
        .insert_resource(HeartbeatTimer(Timer::from_seconds(SECONDS_BETWEEN_HEARTBEATS , TimerMode::Repeating)))
        .add_systems(Startup, bind_socket.chain())
        .add_systems(Startup, debug_info)
    // PreUpdate pour passer avant FixedUpdate de EntityPlugin
    // Ordre : https://docs.rs/bevy/0.13.2/bevy/app/struct.Main.html
        .add_systems(PreUpdate, receive_packets)
        .add_systems(Update, send_heartbeat_periodically)
        .add_systems(PostUpdate, (write_entities_to_package, send_network_package).chain())
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
    
    let heartbeat_peer = GamePeer::new(UdpBackend::new());
    let broker_peer = GamePeer::new(QuicBackend::new());
    
    let orch_address = config.orchestrator_address;
    heartbeat_peer.connect(&orch_address.ip().to_string().as_str(), orch_address.port())?;

    let broker_address = config.broker_address;
    broker_peer.connect(&broker_address.ip().to_string().as_str(), broker_address.port())?;
    
    commands.insert_resource(DedicatedServerPeer{
        broker_peer,
        broker_connection:None,
        heartbeat_peer,
        orchestrator_connection:None,
    });
    
    Ok(())
}

fn handle_publish(
    topic : &Topic,
    payload : Vec<u8>,
    player_input_writer : &mut MessageWriter<PlayerActionHolderMessage>) {
    
    if let Some(topic_content) = TopicContent::from_publish(topic, payload) {            
        match topic_content {
            TopicContent::PlayerInput{client_id, input} => {
                player_input_writer.write(PlayerActionHolderMessage{id:client_id, act:input});
            }
            TopicContent::EntityInfo{..} => {
                error!("Le game server publie des entity info, il ne devrait pas en recevoir");
            }
        }
    }
}

fn message_received(
    network_message : &mut NetworkMessage,
    data : Bytes,
    player_input_writer : &mut MessageWriter<PlayerActionHolderMessage>,
    entity_creation_writer : &mut MessageWriter<CreateEntity>,
    ghost_update_writer : &mut MessageWriter<UpdateGhostEntity>,
    unghost_writer : &mut MessageWriter<GhostToOwned>,
    pending_writer : &mut MessageWriter<OwnedToPending>) {

    // Pour le message d'erreur
    let mut data_copy = data.clone();
    let mut remaining = data.remaining();
    
    while let Some(message) = GameMessage::from_bytes(&mut data_copy) {
        match message {
            GameMessage::Publish{ topic, payload } => {
                handle_publish(&topic, payload, player_input_writer);                
            }
            GameMessage::HandoffRequest { entity_id, pos, vel, state, .. } => {
                // Créer l'entité dans le serveur en mode "Ghost"
                entity_creation_writer.write(CreateEntity {
                    tag: ServerEntityTag {
                        id: entity_id,
                        state: EntityNetworkState::Ghost,
                    },
                    pos, vel, state,
                });
                GameMessage::HandoffAccept{entity_id}.append_bytes(&mut *network_message);
            }
            GameMessage::HandoffAccept { entity_id } => {
                pending_writer.write(OwnedToPending{id: entity_id});
            }
            GameMessage::GhostUpdate { entity_id, pos, vel, state } => {
                ghost_update_writer.write(UpdateGhostEntity{id: entity_id, pos, vel, state});
            }
            GameMessage::HandoffComplete { entity_id, .. } => {
                unghost_writer.write(GhostToOwned{id: entity_id});
            }
            _ => {
                warn!("Message non interprétable par le serveur : {:?}", message);
            }
        }
        remaining = data_copy.remaining();
    }
    
    if remaining > 0 {
        let slice = data.slice((data.len() - remaining)..);
        warn!("Message non désérialisable : {:?}", slice);
    }
}

fn receive_packets(
    config : Res<ServerConfig>,
    players : Query<(), With<PlayerTag>>,
    mut peer_res : ResMut<DedicatedServerPeer>,
    mut network_message : ResMut<NetworkMessage>,
    mut player_input_writer : MessageWriter<PlayerActionHolderMessage>,
    mut entity_creation_writer : MessageWriter<CreateEntity>,
    mut ghost_update_writer : MessageWriter<UpdateGhostEntity>,
    mut unghost_writer : MessageWriter<GhostToOwned>,
    mut pending_writer : MessageWriter<OwnedToPending>) -> Result {
    while let Some(event) = peer_res.broker_peer.poll()? {
        match event {
            GameNetworkEvent::Connected(connection) => {
                info!("Connexion broker : {:?}", connection);
                peer_res.broker_peer.create_stream(connection, GameStreamReliability::Unreliable)?;
            }
            GameNetworkEvent::Disconnected(connection) => {
                info!("Déconnexion broker {:?}", connection);
                peer_res.broker_connection = None;
            }
            GameNetworkEvent::Message{ data, .. } => {
                message_received(&mut network_message, data,
                                 &mut player_input_writer,
                                 &mut entity_creation_writer,
                                 &mut ghost_update_writer,
                                 &mut unghost_writer,
                                 &mut pending_writer);
            }
            GameNetworkEvent::StreamCreated(connection, stream) => {
                info!("Création de stream avec le broker {:?}", connection);                
                let client_id = ClientId::of_game_server(config.get_own_client_id());
                peer_res.broker_peer.send(&connection, &stream, GameMessage::Register{client_id}.as_bytes())?;
                peer_res.broker_connection = Some(DedicatedServerConnection{connection, stream});
            }
            GameNetworkEvent::StreamClosed(connection, _) => {
                warn!("Fermeture du stream avec le broker {:?} ?", connection);
                peer_res.broker_connection = None;
                
            }
            GameNetworkEvent::Error { connection:_connection, inner } => {
                error!("Erreur du broker : {:?}", inner);
            }
        }
    }

    while let Some(event) = peer_res.heartbeat_peer.poll()? {
        match event {
            GameNetworkEvent::Connected(connection) => {                
                peer_res.heartbeat_peer.create_stream(connection, GameStreamReliability::Unreliable)?;
            }
            GameNetworkEvent::Disconnected(_) => {
                warn!("Déconnexion de l'orchestrateur");
                peer_res.orchestrator_connection = None;
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
                info!("Création du stream avec l'orchestrateur : {:?}", connection);
                peer_res.orchestrator_connection = Some(
                    DedicatedServerConnection{connection, stream}
                );
                let player_count = players.count();
                send_heartbeat(&config, &peer_res, player_count)?;
                
            }
            GameNetworkEvent::StreamClosed(_connection, _stream) => {                
                warn!("Stream de l'orchestrateur fermé ?");
                peer_res.orchestrator_connection = None;
            }
        }
    }
    
    Ok(())
}


fn send_heartbeat(
    config : &ServerConfig,
    peer_res : &DedicatedServerPeer,
    player_count : usize) -> Result {
    
    if let Some(orchestrator) = &peer_res.orchestrator_connection {
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

        let DedicatedServerConnection{connection, stream} = &orchestrator;
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
        send_heartbeat(&config, &peer_res, player_count)?;
    }
    Ok(())    
}

fn write_entities_to_package(entities : Query<(&ServerEntityTag, &Velocity, &Transform, Option<&PlayerTag>)>, mut network_message : ResMut<NetworkMessage>) {
    for (tag, velocity, transform, player_tag) in entities {
        if tag.state == EntityNetworkState::Owned { // On publish ce qui nous apparitient
            let state = match player_tag {
                Some(PlayerTag{id}) => { EntityState::PlayerState{id: *id} }
                None => { EntityState::Other }
            };

            
            TopicContent::EntityInfo{
                entity_id: tag.id,
                velocity: *velocity,
                transform: *transform,
                state,
            }.to_publish()
                .append_bytes(&mut network_message)
        }
    }
}

fn send_network_package(peer_res : ResMut<DedicatedServerPeer>, mut network_message : ResMut<NetworkMessage>) -> Result {
    if let Some(DedicatedServerConnection{connection, stream}) = &peer_res.broker_connection {
        let bytes = network_message.clone().freeze();
        network_message.0 = BytesMut::new();
        peer_res.broker_peer.send(connection, stream, bytes)?;
    }
    else {
        warn!("Tentative d'envoi de message alors que le broker n'est pas connecté");
    }
    Ok(())
}
