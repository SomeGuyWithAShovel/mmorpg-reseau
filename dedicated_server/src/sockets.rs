use bevy::{
    prelude::*
};

use game_sockets::*;

use std::collections::HashMap;

// -------------------------------------------------------------------------------------------------------------------

/* 
 *  Écrit à l'aide des exemples issus du dossier game_sockets/bin
 */

pub type PlayerId = GameStream;

pub struct PlayerInfo
{
    pub id : PlayerId,
    // S'il y a pas de username, le joueur n'a pas join
    pub username : String,
}

#[derive(Resource, Default)]
pub struct PlayerRegistry
{
    pub players: HashMap<GameConnection, PlayerInfo>,
}

#[derive(Resource)]
pub struct HeartbeatTimer(pub Timer);

#[derive(Clone)]
pub struct OrchestratorConnection {
    pub connection : GameConnection,
    pub stream : GameStream,
}

#[derive(Resource)]
pub struct DedicatedServerPeer // allows to use game-sockets
{
    pub peer: GamePeer,
    pub heartbeat_peer: GamePeer,
    pub orchestrator : Option<OrchestratorConnection>,
}

pub fn get_own_ip() -> &'static str {
    // On peut mieux faire c'est sûr
    return "127.0.0.1";
}
