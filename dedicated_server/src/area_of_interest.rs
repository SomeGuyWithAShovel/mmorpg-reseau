use bevy::{
    prelude::*,
};
use bytes::{BufMut, BytesMut};

use crate::{
    entity::*,
    player::*,
    sockets::{
        DedicatedServerPeer,
        PlayerRegistry,
    },
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
pub struct AreaOfInterestEntities
{
    pub list: Vec<EntityId>,
    // multiple lists for multiple levels of interest
}


// -------------------------------------------------------------------------------------------------------------------

pub fn update_relevant_entitiies(
    mut players: Query<(&EntityTag, &Transform, &mut AreaOfInterestEntities), With<PlayerTag>>, 
    entities: Query<(&EntityTag, &Transform)>
)
{
    for (player_entity, player_transform, mut relevant_entities) in &mut players
    {
        relevant_entities.list.clear();

        for (other_entity, other_transform) in &entities
        {
            if (player_entity.id != other_entity.id) && 
               (other_transform.translation - player_transform.translation).length_squared() <= AREA_OF_INTEREST_DIST_SQUARED
            {
                relevant_entities.list.push(player_entity.id.clone());
            }
        }
    }
    return;
}

pub fn send_relevant_entities(
    players: Query<(&PlayerTag, &AreaOfInterestEntities)>,
    entities: Query<(&EntityTag, &Transform)>,
    peer_res : Res<DedicatedServerPeer>,
    player_registry : Res<PlayerRegistry>,
)
{
    for (player, relevant_entities) in players
    {
        // it should be [u16,u16,16] representing [x,y,angle], with a mapping [f32,f32] => [u16,u16] and f32[0,2*PI] => [0,65535]
        // right now, it's only [f32, f32]
        let mut data_to_send : Vec<(EntityId, Vec2)> = vec![];

        // we filter the result of the bevy query "entities" to contains only "(entity_tag, transform)"" where "entity_tag.id" is in "relevant_entities.list"
        let relevant_entities_data = entities.iter().filter(|(entity, _)| relevant_entities.list.contains(&entity.id));

        for (entity, transform) in relevant_entities_data
        {
            // we could use a for loop on others, and filter here with a if(relevant_players.contains(other_player_id)) {to_send.push_back(other_data)}
            let transform_to_send: Vec2 = transform.translation.xy();

            data_to_send.push((entity.id.clone(), transform_to_send));
        }

        // ideally, we would add this to a buffer via a bevy message, and have a system that runs last and centralize all the data that needs to be sent to a player.
        
        // gathering data to know where to send packet

        let player_stream = &player_registry.players.get(&player.connection)
            .expect("send_relevant_entities() : a PlayerTag contains a GameConnection that isn't in PlayerRegistry")
            .game_stream;

        //

        // packet construction

        let header_size : usize = 0;
        let data_size : usize = data_to_send.len() * (size_of::<EntityId>() + 2 * size_of::<f32>());

        let mut packet = BytesMut::with_capacity(header_size + data_size);
        
        packet.put_u8(0x3); // TODO : shared::BinaryDataType::Transform2D (it isn't in this branch yet)

        for (entity_id, entity_transform) in data_to_send
        {
            packet.put_u32_le(entity_id);
            packet.put_f32_le(entity_transform.x);
            packet.put_f32_le(entity_transform.y);
        }

        //

        // packet sending

        if let Err(_err) = peer_res.peer.send(&player.connection, &player_stream, packet.freeze())
        {
            error!("send_relevant_entities() : send failed.\n{}", _err);
            return;
        };

        //

    }
    return;
}