use game_sockets::*;
use game_sockets::protocols::QuicBackend;
use tokio::runtime::Builder;
use shared::{*, entity::*, input::*, game_message::*};
use bytes::*;
use bevy::prelude::*;

pub struct GameSocketId {
    connection : GameConnection,
    stream : GameStream,
}

#[derive(Resource)]
pub struct BrokerConnection {
    pub client_id : ClientId,
    pub peer : GamePeer,
    pub game_socket : Option<GameSocketId>,
}

#[derive(Message)]
pub struct UpdateEntity {
    pub id : EntityId,
    pub pos: Vec2,
    pub vel: Vec2,
    pub state : EntityState,
}
    
pub struct ConnectionPlugin;

impl Plugin for ConnectionPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, find_server)
            .add_systems(PreUpdate, recieve_packets)
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
            client_id: ClientId::of_player(user.player_id.as_u128()),
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
        match reqwest::Client::new().post(format!("{}/login", get_gatekeeper_uri()))
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
            GameNetworkEvent::Error{inner, ..} => {
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


