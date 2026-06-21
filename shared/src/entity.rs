use bytes::{Buf, BufMut, Bytes, BytesMut};
use bevy::prelude::*;
use crate::game_message::{ClientId, PeerType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u32);

pub const PLAYABLE_DIST_EPSILON: f32 = 0.5; // f32::EPSILON is too small for our use-case

#[derive(Component, Copy, Clone)]
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
    pub fn new(x : f32, y : f32) -> Self {
        Self {v: Vec2::new(x, y) }
    }
    
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

    pub fn to_bytes(&self) -> Bytes {
        let mut bytes = BytesMut::new();
        self.append_bytes(&mut bytes);
        bytes.freeze()
    }
    
    // Toujours de taille 64
    pub fn append_bytes(&self, out : &mut BytesMut) {
        match self {
            EntityState::PlayerState{id} => {
                out.put_u8(Self::PLAYER_STATE);
                out.put_u8(id.peer_type.to_byte());
                out.put_u128(id.value);
            }
            EntityState::Other => {
                out.put_u8(Self::OTHER);
            }
        }
        out.resize(64, 0u8);
    }
    
    pub fn from_bytes(data : &mut Bytes) -> Option<Self> {
        if !data.remaining() < 64 + 1 { return None; }
        let mut data = data.split_to(64);
        let tag = data.get_u8();

        match tag {
            Self::PLAYER_STATE => {
                let peer_type = PeerType::from_byte(data.get_u8())?;
                let id = data.get_u128();
                Some(Self::PlayerState{id:ClientId{
                    peer_type,
                    value:id,                    
                }})
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
