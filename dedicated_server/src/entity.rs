use bevy::{
    prelude::*,
    math::{
        NormedVectorSpace,
    }, 
};

pub const PLAYABLE_DIST_EPSILON: f32 = 0.5;

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
    #[allow(dead_code)]
    pub fn reset(&mut self)
    {
        self.v = Vec2::ZERO;
    }
}

// -------------------------------------------------------------------------------------------------------------------

pub fn move_entities(mut entities : Query <(&Velocity, &mut Transform), With<EntityTag>>, time: Res<Time>)
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
