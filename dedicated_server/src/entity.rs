use bevy::{
    prelude::*,
    math::{
        NormedVectorSpace,
    }, 
};

use std::sync::atomic::{AtomicU32, Ordering};

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

pub type EntityId = u32;

#[derive(Component)]
pub struct EntityTag
{
    pub id: EntityId,
}

impl EntityTag
{
    pub fn new() -> EntityTag
    {
        // https://doc.rust-lang.org/reference/items/static-items.html
        // ça marche pour un serveur (même multithreadé), mais je doute que ça marche avec plusieurs serveurs ("shards")...

        static NEW_ENTITY_ID: AtomicU32 = AtomicU32::new(0_u32);
        return EntityTag {
            id: NEW_ENTITY_ID.fetch_add(1_u32, Ordering::SeqCst)
        };
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
