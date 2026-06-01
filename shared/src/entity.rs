use bytes::{Buf, BufMut, Bytes, BytesMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u32);

pub const PLAYABLE_DIST_EPSILON: f32 = 0.5; // f32::EPSILON is too small for our use-case

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
        let mut out = BytesMut::new();
        match self {
            EntityState::PlayerState{id} => {
                out.put_u8(Self::PLAYER_STATE);
                out.put_u32(id.0);
            }
            EntityState::Other => {
                out.put_u8(Self::OTHER);
            }
        }
        out.freeze()
    }

    pub fn from_bytes(mut data : Bytes) -> Option<Self> {
        if !data.has_remaining() { return None; }
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
