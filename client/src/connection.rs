use game_sockets::*;
use game_sockets::protocols::QuicBackend;
use tokio::runtime::Builder;
use shared::{*, entity::*, input::*, game_message::*};
use crate::{player::spawn_player, ClientEntityTag};
use bytes::*;
use bevy::prelude::*;
use std::collections::HashMap;

pub struct GameSocketId {
    connection : GameConnection,
    stream : GameStream,
}

#[derive(Event)]
pub struct WelcomeEvent {
    pub id : u32,
}

#[derive(Resource)]
pub struct BrokerConnection {
    client_id : ClientId,
    peer : GamePeer,
    game_socket : Option<GameSocketId>,
}

#[derive(Message)]
pub struct UpdateEntity {
    id : EntityId,
    pos: Vec2,
    vel: Vec2,
    state : EntityState,
}


    
pub struct ConnectionPlugin;

impl Plugin for ConnectionPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, find_server)
            .add_systems(PreUpdate, recieve_packets)
            .add_systems(StateTransition, update_states)
            .add_systems(Update, reajust_position)
            .add_systems(PostUpdate, send_inputs)
            .add_message::<UpdateEntity>();
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
        commands.insert_resource(BrokerConnection{
            client_id: ClientId{
                peer_type:PeerType::Client,
                value:user.player_id.as_u128(),
            },
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

fn recieve_packets(mut conn : ResMut<BrokerConnection>,
                   mut msg_writer: MessageWriter<UpdateEntity>) -> Result {
    while let Some(event) = conn.peer.poll()? {
        match event {
            GameNetworkEvent::Connected(connection) => {
                info!("Connexion au serveur : {:?}", connection);
                conn.peer.create_stream(connection, GameStreamReliability::Unreliable)?;
            }
            GameNetworkEvent::Disconnected(connection) => {
                info!("Déconnexion du serveur : {:?}", connection);
                conn.game_socket = None;
            }
            GameNetworkEvent::Message{data, ..} => {
                info!("Message du serveur reçu");
                handle_server_message(&data, &mut msg_writer);
            }
            GameNetworkEvent::StreamCreated(connection, stream) => {
                info!("Création de stream au broker : {:?}", stream);
                conn.peer.send(&connection, &stream, join_message(conn.client_id))?;
                conn.game_socket = Some(GameSocketId{connection, stream});
            }
            GameNetworkEvent::StreamClosed(connection, _) => {
                info!("Fermeture de stream du serveur : {:?}", connection);
                conn.game_socket = None;
            }
            GameNetworkEvent::Error{connection:_, inner} => {
                warn!("Erreur du serveur : {:?}", inner);
            }
        }
    }
    Ok(())
}

fn join_message(client_id : ClientId) -> Bytes {
    GameMessage::Register{
        client_id
    }.as_bytes()
}

fn handle_server_message(data : &Bytes, msg_writer : &mut MessageWriter<UpdateEntity>) {
    
    // Pour le message d'erreur
    let mut data_copy = data.clone();
    let mut remaining = data.remaining();
    
    while let Some(message) = GameMessage::from_bytes(&mut data_copy) {
        match message {
            GameMessage::ClientUpdate{ entity_id, pos, vel, state } => {
                msg_writer.write(UpdateEntity{id: entity_id, pos, vel, state});
            }
            _ => {
                warn!("Type de message non supporté par le client envoyé : {:?}", message);
            }
        }
        remaining = data_copy.remaining();
    }
    if remaining > 0 {
        let slice = data.slice((data.len() - remaining)..);
        warn!("Message non désérialisable : {:?}", slice);
    }
}

fn send_inputs(
    local_player : Single<&PlayerActionHolder>,
    conn : ResMut<BrokerConnection>) -> Result {

    if let Some(socket_id) = &conn.game_socket {
        let act = local_player.into_inner();
        if act.data != 0 {

            let mut buf = BytesMut::new();
            GameMessage::ClientInput {
                client_id: conn.client_id,
                input: *act,
            }.append_bytes(&mut buf);
            
            conn.peer.send(&socket_id.connection, &socket_id.stream, buf.freeze())?;
        }
            
    }
    
    Ok(())
}

fn update_states(mut msg_reader : MessageReader<UpdateEntity>) {
    let mut map = HashMap::<u32, EntityState>::new();
    for msg in msg_reader.read() {        
        let UpdateEntity{id: EntityId(id), state, ..} = msg;
        map.insert(*id, *state);
    }

    // Mise à jour des états en fonction des besoins...
}

fn reajust_position(mut msg_reader : MessageReader<UpdateEntity>,
                    mut query : Query<(&ClientEntityTag, &mut Transform)>,
                    mut commands : Commands,
                    asset_server : Res<AssetServer>) {
    let mut map = HashMap::<u32, Transform>::new();
    for msg in msg_reader.read() {        
        let UpdateEntity{id: EntityId(id), pos, vel, ..} = msg;
        let rot = Quat::from_rotation_z(vel.to_angle() - std::f32::consts::FRAC_PI_2);
        let transform = Transform::from_xyz(pos.x, pos.y, 0.0)
            .with_rotation(rot);
        map.insert(*id, transform);
    }

    for (ClientEntityTag(EntityId(id)), mut transform) in &mut query {
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
