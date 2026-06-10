use bytes::{Buf, BufMut, Bytes, BytesMut};
use bevy::prelude::*;
pub use crate::ClientId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u32);

pub const PLAYABLE_DIST_EPSILON: f32 = 0.5; // f32::EPSILON is too small for our use-case

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

#[derive(Component)]
pub struct MaxSpeed(pub f32);

#[derive(Debug, Copy, Clone)]
pub enum EntityState {
    PlayerState {
        id: ClientId
    },
    Other,
}

impl EntityState {
    const PLAYER_STATE : u8 = 0x01;
    const OTHER : u8 = 0xFF;

    // Toujours de taille 64
    pub fn to_bytes(&self) -> Bytes {
        let mut out = BytesMut::with_capacity(64);
        match self {
            EntityState::PlayerState{id} => {
                out.put_u8(Self::PLAYER_STATE);
                out.put_u32(id.0);
            }
            EntityState::Other => {
                out.put_u8(Self::OTHER);
            }
        }
        out.resize(64, 0u8);
        out.freeze()
    }
    
    pub fn from_bytes(mut data : Bytes) -> Option<Self> {
        if !data.remaining() < 64 { return None; }        
        let mut data = data.split_to(64);
        let tag = data.get_u8();

        match tag {
            Self::PLAYER_STATE => {
                let id = data.get_u32();
                Some(Self::PlayerState{id:ClientId(id)})
            }
            Self::OTHER => {
                Some(Self::Other)
            }
            _ => {
                None
            }            
        }
    }
}
