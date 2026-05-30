use game_sockets::*;
use game_sockets::protocols::QuicBackend;
use tokio::runtime::Builder;
use uuid::Uuid;
use shared::*;
use crate::entity::EntityTag;
use crate::player::{LocalPlayerTag, spawn_player};
use bytes::*;
use bevy::prelude::*;
use std::collections::HashMap;

pub struct GameSocketId {
    connection : GameConnection,
    stream : GameStream,
}

#[derive(Event)]
pub struct WelcomeEvent {
    pub id : u64,
}

#[derive(Resource)]
pub struct DedicatedServerConnection {
    player_id : Uuid,
    peer : GamePeer,
    game_socket : Option<GameSocketId>,
}

#[derive(Message)]
pub enum ServerMessage {
    ChangeTransform{id : u64, transform : Transform},
}
    
pub struct ConnectionPlugin;

impl Plugin for ConnectionPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, find_server)
            .add_systems(PreUpdate, recieve_packets)
            .add_systems(Update, reajust_position)
            .add_systems(PostUpdate, send_packets)
            .add_message::<ServerMessage>();
    }
}

const GATEKEEPER_PORT : u16 = 3000;

fn get_gatekeeper_uri() -> String {
    return format!("http://localhost:{}", GATEKEEPER_PORT).to_string();
}

fn get_user() -> Option<LoginRequest> {
    let username = std::env::var("USERNAME").unwrap_or("toto".to_string());
    let password = std::env::var("PASSWORD").unwrap_or("1234".to_string());
    Some(LoginRequest { username, password })
}

fn find_server(mut commands : Commands) -> Result {
    let handle = Builder::new_multi_thread()
        .thread_name("gatekeeper_connection")
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            gatekeeper_connection().await
        });
    
    if let Some(user) = handle {
        let peer = GamePeer::new(QuicBackend::new());
        peer.connect(user.server.ip.to_string().as_str(), user.server.port)?;
        commands.insert_resource(DedicatedServerConnection{
            player_id: user.player_id,
            peer,
            game_socket: None,
        });
    }
    else {
        error!("Échec de la récupération du serveur");
        
    }
    Ok(())
}

async fn gatekeeper_connection() -> Option<LoginSuccess> {
    let user = get_user()?;
    if let Ok(post_body) = serde_json::to_string(&user) {
        match reqwest::Client::new().post(get_gatekeeper_uri())
            .header("Content-Type", "application/json")
            .body(post_body).send()
            .await
        {            
            Ok(sent) => {
                if let Some(res_body) = sent.text().await.ok() {
                    serde_json::from_str::<LoginSuccess>(res_body.as_str()).ok()
                }
                else {
                    warn!("Échec de la désérialisation du corps de la réponse du serveur");
                    return None;
                }
            }
            Err(s) => {
                warn!("Échec de l'envoi de la requête {:?}", s);
                return None;
            }
        }            
    }
    else {
        error!("Corps de requête au gatekeeper invalide");
        return None;
    }    
}

fn recieve_packets(mut ds_conn : ResMut<DedicatedServerConnection>,
                   mut msg_writer: MessageWriter<ServerMessage>,
                   mut commands : Commands) -> Result {
    while let Some(event) = ds_conn.peer.poll()? {
        match event {
            GameNetworkEvent::Connected(connection) => {
                info!("Connexion au serveur : {:?}", connection);
                ds_conn.peer.create_stream(connection, GameStreamReliability::Unreliable)?;
            }
            GameNetworkEvent::Disconnected(connection) => {
                info!("Déconnexion du serveur : {:?}", connection);
                ds_conn.game_socket = None;
            }
            GameNetworkEvent::Message{connection, stream, data} => {
                info!("Message du serveur reçu");
                handle_server_message(connection, stream, data.clone(), &mut msg_writer, &mut commands, &mut ds_conn);
            }
            GameNetworkEvent::StreamCreated(connection, stream) => {
                info!("Création de stream au serveur : {:?}", stream);
                ds_conn.peer.send(&connection, &stream, join_message(ds_conn.player_id.to_string()))?;
            }
            GameNetworkEvent::StreamClosed(connection, _) => {
                info!("Fermeture de stream du serveur : {:?}", connection);
                ds_conn.game_socket = None;
            }
            GameNetworkEvent::Error{connection:_, inner} => {
                warn!("Erreur du serveur : {:?}", inner);
            }
        }
    }
    Ok(())
}

fn join_message(uuid : String) -> Bytes {
    let mut buf = BytesMut::with_capacity(1 + 9 + uuid.len());
    buf.put_u8(BinaryDataType::Join.as_byte());
    let msg = format!("JOIN {{ {} }}", uuid);
    buf.extend_from_slice(msg.as_bytes());

    buf.freeze()
}

fn handle_server_message(
    connection : GameConnection,
    stream : GameStream,
    mut data : Bytes,
    msg_writer : &mut MessageWriter<ServerMessage>,
    commands : &mut Commands,
    ds_conn : &mut DedicatedServerConnection) {
    match BinaryDataType::from_byte(data.get_u8()) {
        Some(BinaryDataType::Join) => {
            warn!("Le serveur essaye de JOIN ?");
            while data.get_u8() != ('}' as u8) {}
        }
        Some(BinaryDataType::Welcome) => {
            ds_conn.game_socket = Some(GameSocketId{connection, stream});            
            let _ = data.split_to("WELCOME { ".len());
            let player_id = data.get_u64();
            let _ = data.split_to(" }".len());
            commands.trigger(WelcomeEvent{id : player_id});
        }
        Some(BinaryDataType::List) => {
            let len = data.get_u64();
            for _ in 0..len {
                handle_server_message(connection, stream.clone(), data.clone(), msg_writer, commands, ds_conn);
            }
        }
        Some(BinaryDataType::Transform2d) => {
            let entity_id = data.get_u64();
            let new_transform = shared::bytes_as_unscaled_transform_2d(data.clone());
            msg_writer.write(ServerMessage::ChangeTransform{id: entity_id, transform:new_transform});
        }
        None => {
            warn!("Message serveur corrompu");
        }
    }
}

fn send_packets(local_player : Single<(&EntityTag, &Transform), With<LocalPlayerTag>>, ds_conn : ResMut<DedicatedServerConnection>) -> Result {
    if let Some(game_socket) = &ds_conn.game_socket {        
        let (tag, transform) = local_player.into_inner();
        let mut buf = BytesMut::new();
        
        buf.put_u8(BinaryDataType::Transform2d.as_byte());
        buf.put_u64(tag.0);
        buf.extend(unscaled_transform_2d_as_bytes(*transform));
        ds_conn.peer.send(&game_socket.connection, &game_socket.stream, buf.freeze())?;
    }
    Ok(())
}

fn reajust_position(mut msg_reader : MessageReader<ServerMessage>, mut query : Query<(&EntityTag, &mut Transform), Without<LocalPlayerTag>>, mut commands : Commands, asset_server : Res<AssetServer>) {
    let mut map = HashMap::<u64, Transform>::new();
    for msg in msg_reader.read() {
        match msg {            
            ServerMessage::ChangeTransform{id, transform} => {
                map.insert(*id, *transform);
            }
        }
    }

    for (EntityTag(id), mut transform) in &mut query {
        if let Some(new_transform) = map.get(id) {
            transform.translation = new_transform.translation;
            transform.rotation = new_transform.rotation;
            map.remove(id);
        }
    }

    // Entitées non encore créees
    for (id, transform) in map {
        spawn_player(id, &mut commands, &asset_server).entry::<Transform>().and_modify(move |mut t| {
            t.translation = transform.translation;
            t.rotation = transform.rotation;
        });
    }
}
