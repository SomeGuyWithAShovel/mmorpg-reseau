// common.rs

use bevy::{
    math::Vec2
};

// -------------------------------------------------------------------------------------------------------------------

pub const PLAYABLE_AREA: Vec2 = Vec2 {x: 480.0, y: 270.0};
pub const PLAYABLE_DIST_EPSILON: f32 = 0.5; // f32::EPSILON is too small for our use-case

pub const PLAYER_Z_ORDER: f32 = 3.0;
pub const ENEMY_Z_ORDER : f32 = 2.0;