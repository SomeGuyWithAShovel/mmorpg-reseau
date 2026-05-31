#[derive(Debug, Clone, Copy)]
pub struct ClientId(pub u32);
#[derive(Debug, Clone, Copy)]
pub struct EntityId(pub u32);

pub const PLAYABLE_DIST_EPSILON: f32 = 0.5; // f32::EPSILON is too small for our use-case
