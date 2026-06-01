use shared::{entity::*, input::*};

use bevy::{
    prelude::*,
    math::{
        NormedVectorSpace,
    }, 
};

// -------------------------------------------------------------------------------------------------------------------

pub struct EntityPlugin;

impl Plugin for EntityPlugin
{
    fn build(&self, app: &mut App)
    {
        info!("building EntityPlugin");

        app.add_systems(FixedUpdate, move_entities);
    }
}

// -------------------------------------------------------------------------------------------------------------------


#[derive(Component)]
pub struct Velocity
{
    pub v: Vec2,
}

impl Default for Velocity
{
    fn default() -> Self {
        Velocity {v: Vec2::ZERO }
    }
}
impl Velocity
{
    pub fn reset(&mut self)
    {
        self.v = Vec2::ZERO;
    }
}

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
pub struct EntityTag {
    pub id : EntityId,
    pub state : EntityNetworkState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct PlayerTag {
    pub id : ClientId,
}




// -------------------------------------------------------------------------------------------------------------------

pub fn move_entities(mut entities : Query <(&mut Velocity, &mut Transform), With<EntityTag>>, time: Res<Time>)
{
    for (velocity, mut transform) in &mut entities
    {
        // info!("entity transform : {}", transform.translation);
        // info!("           speed : {}", velocity.v.length());
        if velocity.v.norm_squared() > PLAYABLE_DIST_EPSILON
        {
            transform.translation += velocity.v.extend(0.0) * time.delta_secs();
                    
            transform.rotation = Quat::from_rotation_z(velocity.v.to_angle() - std::f32::consts::FRAC_PI_2);
        }
    }
}

// -------------------------------------------------------------------------------------------------------------------
