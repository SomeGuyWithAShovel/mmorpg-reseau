use bevy::{
    prelude::*,
};

use crate::{
    sockets::PlayerId,
    player::*,
};

// -------------------------------------------------------------------------------------------------------------------

pub const AREA_OF_INTEREST_DIST_SQUARED: f32 = 50.0 * 50.0;

pub struct AreaOfInterestPlugin;

impl Plugin for AreaOfInterestPlugin
{
    fn build(&self, app: &mut App)
    {
        info!("building AreaOfInterestPlugin");

        app.add_systems(Last,
            (
                update_relevant_entitiies, 
                send_relevant_entities,
            ).chain()
        );

    }
}

// -------------------------------------------------------------------------------------------------------------------

/**
 * Currently, only players have an ID, and we send only other players data to clients.
 * We should use IDs on entities (either our own, or the one used by bevy (only if clients can specify some id when they spawn their local replicated entity)),
 * and send all entities data if they are relevant
 */

#[derive(Component, Default)]
pub struct PlayerListOfPlayersInAreaOfInterest
{
    id_list: Vec<PlayerId>,
}


// -------------------------------------------------------------------------------------------------------------------

pub fn update_relevant_entitiies(
    mut players: Query<(&Transform, &PlayerTag, &mut PlayerListOfPlayersInAreaOfInterest)>, 
    others: Query<(&Transform, &PlayerTag)>
)
{
    for (transform, player, mut relevant_players) in &mut players
    {
        relevant_players.id_list.clear();

        for (other_transform, other_player) in &others
        {
            if (player.id != other_player.id) && 
               (other_transform.translation - transform.translation).length_squared() <= AREA_OF_INTEREST_DIST_SQUARED
            {
                relevant_players.id_list.push(other_player.id.clone());
            }
        }
    }
    return;
}

pub fn send_relevant_entities(
    players: Query<(&PlayerTag, &PlayerListOfPlayersInAreaOfInterest)>,
    others: Query<(&Transform, &PlayerTag)>
)
{
    for (player, relevant_players) in players
    {
        let mut data_to_send : Vec<(PlayerId, Transform)> = vec![];

        // we filter the result of the bevy query "others" to contains only "(transform, player_tag)"" where "player_tag.id" is in "relevant_players.id_list"
        let relevant_players_data = others.iter().filter(|(_, player)| relevant_players.id_list.contains(&player.id));

        for (transform, other_player) in relevant_players_data
        {
            // we could use a for loop on others, and filter here with a if(relevant_players.contains(other_player_id)) {to_send.push_back(other_data)}
            data_to_send.push((other_player.id.clone(), transform.clone()));
        }

        // TODO : send a packet containing data_to_send, to player.id (which is a GameConnection from GameSockets).
        // ideally, we would add this to a buffer via a bevy message, and have a system that runs last and centralize all the data that needs to be sent to a player.
        let _ = player;
    }
    return;
}