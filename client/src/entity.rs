use bevy::{
    prelude::*,
    math::{
        NormedVectorSpace,
    }, 
};

use crate::{
    common::{
        PLAYABLE_DIST_EPSILON,
    },
};

// -------------------------------------------------------------------------------------------------------------------

pub struct EntityPlugin;

impl Plugin for EntityPlugin
{
    fn build(&self, app: &mut App)
    {
        info!("building EntityPlugin");

        app.add_systems(FixedUpdate,move_entities);
    }
}

// -------------------------------------------------------------------------------------------------------------------

#[derive(Component, Default)]
#[require(Velocity)]
pub struct EntityTag;

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

// -------------------------------------------------------------------------------------------------------------------

pub fn move_entities(mut entities : Query <(&mut Velocity, &mut Transform), With<EntityTag>>, time: Res<Time>)
{
    for (mut velocity, mut transform) in &mut entities
    {
        if velocity.v.norm_squared() > PLAYABLE_DIST_EPSILON
        {
            transform.translation += velocity.v.extend(0.0) * time.delta_secs();
                    
            transform.rotation = Quat::from_rotation_z(velocity.v.to_angle() - std::f32::consts::FRAC_PI_2);
        }
        velocity.reset();
    }
}

// -------------------------------------------------------------------------------------------------------------------
