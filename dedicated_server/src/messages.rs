use bevy::prelude::*;
use crate::entity::*;
use shared::{entity::*, input::*, game_message::*};
use crate::{receive_packets, NetworkMessage};

#[derive(Message)]
pub struct CreateEntity {
    pub tag : ServerEntityTag,
    pub pos : Vec2,
    pub vel : Vec2,
    pub state : EntityState,
    // Aucun state pour l'instant
}

#[derive(Message)]
pub struct UpdateGhostEntity {
    pub id: EntityId,
    pub pos : Vec2,
    pub vel : Vec2,
    pub state : EntityState,
}

#[derive(Message)]
pub struct GhostToOwned {
    pub id : EntityId,
}

#[derive(Message)]
pub struct OwnedToPending {
    pub id : EntityId,
}

pub struct MessagePlugin;

impl Plugin for MessagePlugin {
    fn build(&self, app: &mut App) {

        /*
        StateTransition tourne après PreUpdate,
        On veut que le passage Ghost -> Owned se fasse avant update_ghost
        Or, update_ghost doit se passer avant le FixedUpdate de move_entity

        Donc, on doit mettre update_ghost dans PreUpdate, mais donc ghost_to_owned aussi, avant update_ghost
        Donc, on met les fonctions X_to_X dans le PreUpdate, bien qu'elles auraient sémantiquement
        leur place dans StateTransition
        */
        
        app
            .add_systems(PreUpdate,
                         (
                             create_entities.after(receive_packets),
                                 
                             interpret_player_input.after(create_entities),
                             ghost_to_owned.after(create_entities),
                             owned_to_pending.after(create_entities),
                             
                             update_ghosts.after(ghost_to_owned),                            
                         )
            )
            .add_observer(notify_border_crossing)
            .add_observer(notify_authority_handoff);
    }
}

fn create_entities(
    mut spawn_messages : MessageReader<CreateEntity>,
    mut commands : Commands) {

    const OTHER_MAX_SPEED : f32 = 50.0;
    
    for CreateEntity{tag, pos, vel, state} in spawn_messages.read() {        
        let mut handle = commands.spawn((
            *tag,
            Transform::from_translation(pos.extend(0.0)),
            Velocity{v:*vel},
            )
        );
        match state {
            EntityState::PlayerState{id} => {
                handle.insert((
                    PlayerTag{id: *id},
                    MaxSpeed(PLAYER_DEFAULT_PARAMS.speed),
                ));
            }
            _ => {
                handle.insert(MaxSpeed(OTHER_MAX_SPEED));
            }
        }
    }
}

use std::collections::{HashMap, HashSet};

fn interpret_player_input(
    mut input_reader : MessageReader<PlayerActionHolderMessage>,
    mut players : Query<(&PlayerTag, &mut Velocity, &MaxSpeed)>
) {
    let mut inputs = HashMap::new();
    for PlayerActionHolderMessage{id, act} in input_reader.read() {
        inputs.insert(id, act);
    }
    for (tag, mut velocity, speed) in &mut players {
        if let Some(act) = inputs.get(&tag.id) {
            let direction = act.get_move_dir();
            if direction.length_squared() > PLAYABLE_DIST_EPSILON {
                velocity.v = direction * speed.0;
            }
            else {
                velocity.reset();
            }
        }
    }
}

fn update_ghosts(
    mut ghost_update_reader : MessageReader<UpdateGhostEntity>,
    mut entities : Query<(&ServerEntityTag, &mut Transform, &mut Velocity)>) {

    struct UpdateInfo(Vec2, Vec2, EntityState);
    
    let mut updates = HashMap::new();

    for UpdateGhostEntity{id, pos, vel, state} in ghost_update_reader.read() {
        updates.insert(id, UpdateInfo(*pos, *vel, *state));
    }

    for (tag, mut transform, mut velocity) in &mut entities {
        if let Some(UpdateInfo(pos, vel, _state)) = updates.get(&tag.id) {
            transform.translation = pos.extend(0.0);
            *velocity = Velocity{v:*vel};
            // Faire quelque chose avec state si besoin
        }
    }
}

fn ghost_to_owned(
    tags : Query<&mut ServerEntityTag>,
    mut unghost_reader : MessageReader<GhostToOwned>
) {
    let mut unghosted = HashSet::new();
    for GhostToOwned{id} in unghost_reader.read() {
        unghosted.insert(id);
    }
    for mut tag in tags {
        if unghosted.contains(&tag.id) {
            tag.state = EntityNetworkState::Owned;
        }
    }
}

fn owned_to_pending(
    tags : Query<&mut ServerEntityTag>,
    mut pending_reader : MessageReader<OwnedToPending>
) {
    let mut pending = HashSet::new();
    for OwnedToPending{id} in pending_reader.read() {
        pending.insert(id);
    }
    for mut tag in tags {
        if pending.contains(&tag.id) {
            tag.state = EntityNetworkState::PendingHandoff;
        }
    }
}

fn notify_border_crossing(event : On<CrossedBorder>,
                          query : Query<(&ServerEntityTag, &Velocity, &Transform, Option<&PlayerTag>)>,
                          mut network_message : ResMut<NetworkMessage>,
) {
    if let Ok((tag, vel, transform, opt_player_tag)) = query.get(event.entity) {
        let msg : GameMessage;
        if let Some(player_tag) = opt_player_tag {
            msg = GameMessage::HandoffRequest {
                entity_id : tag.id,
                pos: transform.translation.xy(),
                vel: vel.v,
                border : event.border,
                state : EntityState::PlayerState{id: player_tag.id},
            }
        }
        else {
            msg = GameMessage::HandoffRequest {
                entity_id : tag.id,
                pos: transform.translation.xy(),
                vel: vel.v,
                border : event.border,
                state : EntityState::Other,
            };

        }
        msg.append_bytes(&mut network_message);        
    }
}

fn notify_authority_handoff(event : On<HandoffAuthority>,
                            tags : Query<&mut ServerEntityTag>,
                            mut network_message : ResMut<NetworkMessage>) {
    if let Ok(tag) = tags.get(event.entity) {
        GameMessage::HandoffComplete {
            entity_id: tag.id,
            border: event.border,
        }.append_bytes(&mut network_message);
    }
}
