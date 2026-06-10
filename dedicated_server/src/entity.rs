use shared::{entity::*, input::*, game_message::Border};

use bevy::{
    prelude::*,
    math::{
        NormedVectorSpace,
    }, 
};
use crate::ServerConfig;

#[derive(Message)]
pub struct PlayerActionHolderMessage
{
    pub id : ClientId,
    pub act: PlayerActionHolder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityNetworkState {
    Owned,
    PendingHandoff,
    Ghost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct ServerEntityTag {
    pub id : EntityId,
    pub state : EntityNetworkState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct PlayerTag {
    pub id : ClientId,
}


#[derive(EntityEvent, Debug)]
// Si CrossedBorder, envoie dans messages.rs un HandoffRequest
pub struct CrossedBorder {
    pub entity: Entity,
    pub border: Border,
}

#[derive(EntityEvent, Debug)]
// Si HandoffAuthority, envoie dans messages.rs un HandoffComplete
pub struct HandoffAuthority {
    pub entity: Entity,
    pub border: Border,
}

pub fn move_entities(
    mut entities : Query<(&Velocity, &mut Transform), With<ServerEntityTag>>,
    time: Res<Time>)
{
    for (velocity, mut transform) in &mut entities
    {
        if velocity.v.norm_squared() > PLAYABLE_DIST_EPSILON
        {
            transform.translation += velocity.v.extend(0.0) * time.delta_secs();
                    
            transform.rotation = Quat::from_rotation_z(velocity.v.to_angle() - std::f32::consts::FRAC_PI_2);
        }
    }
}

fn circle_touched_rectangle_borders(rect : &Rect, pos : Vec2, radius : f32) -> Vec<Border> {
    let mut res = Vec::new();
    let Rect{min:Vec2{x:left, y:bottom}, max:Vec2{x:right, y:top}} = rect;
    
    if (pos.x - radius) < *left {
        res.push(Border::Left);
    }
    if (pos.y - radius) < *bottom {
        res.push(Border::Bottom);
    }
    if (pos.x + radius) > *right {
        res.push(Border::Right);
    }    
    if (pos.y + radius) > *top {
        res.push(Border::Top);
    }
    return res;
}

pub fn check_border_crossings(
    mut commands : Commands,
    query : Query<(Entity, &ServerEntityTag, &Transform, &MaxSpeed)>,
    config : Res<ServerConfig>) {

    for item in &query {
        let (entity, tag, transform, speed) = item;
        let dist = (speed.0*config.min_border_seconds) / 2.0;
        match tag.state {
            EntityNetworkState::Owned => {
                let pending_borders = circle_touched_rectangle_borders(
                    &config.map_borders,
                    transform.translation.xy(),
                    dist
                );        
                for border in pending_borders {
                    commands.trigger(CrossedBorder{entity, border});
                }
            }
            EntityNetworkState::PendingHandoff => {
                let handoff_borders = circle_touched_rectangle_borders(
                    &config.map_borders.inflate(dist),
                    transform.translation.xy(),
                    0.0 // Si on traverse le bord, indépendant de la vitesse
                );

                if !handoff_borders.is_empty() {
                    let mut handoff_to  = handoff_borders[0];
                    for border in &handoff_borders[1..] {
                        // Il ne devrait pas y en avoir plus d'un à un instant donné
                        if let Some(combination) = handoff_to.combine(*border) {
                            handoff_to = combination;
                        }
                        else {
                            error!("Entité au serveur {:?} {:?} invalide", border, handoff_to.combine(*border));
                        }
                    }
                    commands.trigger(HandoffAuthority{entity, border:handoff_to});
                }
            }
            EntityNetworkState::Ghost => {}
        }
    }
}

pub struct EntityPlugin;

impl Plugin for EntityPlugin
{
    fn build(&self, app: &mut App)
    {
        info!("building EntityPlugin");

        app.add_systems(FixedUpdate, (move_entities, check_border_crossings).chain());
    }
}
