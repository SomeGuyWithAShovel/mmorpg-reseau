use shared::{DEFAULT_BROKER_PORT};
use game_sockets::{*, protocols::QuicBackend};

#[allow(unused)]
use log::{debug, info, warn, error};

mod pubsub;

use crate::pubsub::*;
use bevy::prelude::*;

// On utilise bevy pour simplifier les actions de timing des boucles
#[derive(Resource)]
struct BevyGamePeer(GamePeer);

#[derive(Resource)]
struct BevyPubSub(PubSub);

const SECONDS_BETWEEN_FLUSHES : f32 = 1./60.;
#[derive(Resource)]
struct FlushTimer(Timer);

fn receive_packets(
    mut peer: ResMut<BevyGamePeer>,
    mut pub_sub : ResMut<BevyPubSub>
) -> Result {
    while let Some(event) = peer.0.poll()? {
        match event {
            GameNetworkEvent::Connected(connection) => {
                info!("Connexion : {:?}", connection);
            }
            GameNetworkEvent::Disconnected(connection) => {
                info!("Déconnexion : {:?}", connection);
            }
            GameNetworkEvent::Message { connection, stream, data } => {
                info!("Paquet reçu : {:?}", data);
                let peer_id = PeerSocketId(connection, stream);
                pub_sub.0.process_received_packet(peer_id, data);
            }
            GameNetworkEvent::StreamCreated(connection, stream) => {
                info!("Stream créé : {:?}, {:?}", connection, stream);
            }
            GameNetworkEvent::StreamClosed(connection, stream) => {
                info!("Stream fermé : {:?}, {:?}", connection, stream);
            }
            GameNetworkEvent::Error {inner, .. } => {
                error!("Erreur : {:?}", inner);
            }
        }
    }
    Ok(())
}

fn flush_packets(
    time: Res<Time>,
    mut timer : ResMut<FlushTimer>,
    mut pub_sub : ResMut<BevyPubSub>,
    mut peer: ResMut<BevyGamePeer>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        pub_sub.0.flush_peer_buffers(&mut peer.0);
    }
}

fn start(mut commands : Commands) -> Result {
    let port = std::env::var("BROKER_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(DEFAULT_BROKER_PORT);


    let pub_sub: PubSub = PubSub::default();
    let peer = GamePeer::new(QuicBackend::new());
    peer.listen("0.0.0.0", port)?;

    commands.insert_resource(BevyGamePeer(peer));
    commands.insert_resource(BevyPubSub(pub_sub));
    Ok(())
}

fn main()
{
    // allow info!() logging without needing to set any environment variables
    env_logger::Builder::new().filter_level(
        log::LevelFilter::Info
        // log::LevelFilter::Debug
    ).parse_default_env().init();

    App::new()
        .add_plugins(MinimalPlugins)
        .insert_resource(FlushTimer(Timer::from_seconds(SECONDS_BETWEEN_FLUSHES, TimerMode::Repeating)))
        .add_systems(Startup, start)
        .add_systems(PreUpdate, receive_packets)
        .add_systems(PostUpdate, flush_packets)
        .run();
}
