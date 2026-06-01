use bevy::prelude::*;
use crate::entity::*;
use shared::{entity::*, input::*};
use crate::receive_packets;

#[derive(Message)]
pub struct CreateEntity {
    pub tag : EntityTag,
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

pub struct MessagePlugin;

impl Plugin for MessagePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate,
                        (create_entities.after(receive_packets),
                         interpret_player_input.after(create_entities),
                         update_ghosts.after(create_entities))
        );
    }
}

fn create_entities(
    mut spawn_messages : MessageReader<CreateEntity>,
    mut commands : Commands) {
    for CreateEntity{tag, pos, vel, state} in spawn_messages.read() {        
        let mut handle = commands.spawn((
            *tag,
            Transform::from_translation(pos.extend(0.0)),
            Velocity{v:*vel},
            )
        );
        match state {
            EntityState::PlayerState{id} => {
                handle.insert(PlayerTag{id: *id});
            }
            _ => {}                
        }
    }
}

use std::collections::{HashMap, HashSet};

fn interpret_player_input(
    mut input_reader : MessageReader<PlayerActionHolderMessage>,
    mut players : Query<(&PlayerTag, &mut Velocity)>
) {
    let mut inputs = HashMap::new();
    const SPEED : f32 = PLAYER_DEFAULT_PARAMS.speed;
    for PlayerActionHolderMessage{id, act} in input_reader.read() {
        inputs.insert(id, act);
    }
    for (tag, mut velocity) in &mut players {
        if let Some(act) = inputs.get(&tag.id) {
            let direction = act.get_move_dir();
            if direction.length_squared() > PLAYABLE_DIST_EPSILON {
                velocity.v = direction * SPEED;
            }
            else {
                velocity.reset();
            }
        }
    }
}

fn update_ghosts(
    mut ghost_update_reader : MessageReader<UpdateGhostEntity>,
    mut unghost_reader : MessageReader<GhostToOwned>,
    mut entities : Query<(&mut EntityTag, &mut Transform, &mut Velocity)>) {

    struct UpdateInfo(Vec2, Vec2, EntityState);
    
    let mut unghosted = HashSet::new();
    let mut updates = HashMap::new();
    for GhostToOwned{id} in unghost_reader.read() {
        unghosted.insert(id);
    }
    for UpdateGhostEntity{id, pos, vel, state} in ghost_update_reader.read() {
        updates.insert(id, UpdateInfo(*pos, *vel, *state));
    }

    for entity in &mut entities {
        let (mut tag, mut transform, mut velocity) = entity;
        if let Some(UpdateInfo(pos, vel, _state)) = updates.get(&tag.id) {
            transform.translation = pos.extend(0.0);
            *velocity = Velocity{v:*vel};
            // Faire quelque chose avec state si besoin
        }
        else if unghosted.contains(&tag.id) {
            tag.state = EntityNetworkState::Owned;
        }
    }
}
