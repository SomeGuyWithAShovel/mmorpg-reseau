use bytes::{Buf, BufMut, Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use bevy::prelude::*;

use crate::entity::*;
use crate::input::PlayerActionHolder;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PeerType
{
    Client,
    GameServer,
    OtherServer,
}

impl PeerType
{
    pub const CLIENT : u8 = 0x00;
    pub const GAME_SERVER : u8 = 0x01;
    pub const OTHER_SERVER : u8 = 0x02;
    
    pub const fn to_byte(&self) -> u8
    {
        match self
        {
            Self::Client      => { Self::CLIENT }
            Self::GameServer  => { Self::GAME_SERVER }
            Self::OtherServer => { Self::OTHER_SERVER }
        }
    }
    
    pub const fn from_byte(byte: u8) -> Option<Self>
    {
        match byte {
            Self::CLIENT       => Some(Self::Client),
            Self::GAME_SERVER  => Some(Self::GameServer),
            Self::OTHER_SERVER => Some(Self::OtherServer),
            _ => None
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientId{
    pub peer_type: PeerType,
    pub value : u128,
}

#[derive(Debug, Deref, DerefMut)]
pub struct Topic(pub String);

impl Topic {
    fn append_bytes(&self, out : &mut BytesMut) {
        out.put_u16(self.len() as u16);
        out.put_slice(&(*self).clone().into_bytes());
    }
    fn from_bytes(data : &mut Bytes) -> Option<Topic> {
        let len = data.get_u16() as usize;
        if data.remaining() < len { return None; }
        let mut topic = vec![0u8; len];
        data.copy_to_slice(&mut topic);
        let topic_str = String::from_utf8(topic).ok()?;
        Some(Topic(topic_str))
    }
}

#[derive(Debug)]
pub enum GameMessage {
    Subscribe {
        client_id : ClientId,
        topic : Topic,
    },
    Unsubscribe {
        client_id : ClientId,
        topic : Topic,
    },
    Publish {
        topic : Topic,
        payload : Vec<u8>,
    },
    Broadcast {
        payload : Vec<u8>,
    },
    ClientInput {
        client_id : ClientId,
        input : PlayerActionHolder,
    },
    // Envoyé par un Dedicated Server
    HandoffRequest {
        entity_id : EntityId,
        pos : Vec2,
        vel : Vec2,
        border : Border,
        state : EntityState,
    },
    HandoffAccept {
        entity_id : EntityId,
    },
    HandoffReject {
        entity_id : EntityId,
    },
    GhostUpdate {
        entity_id : EntityId,
        pos : Vec2,
        vel : Vec2,
        // Si on reçoit pas de state sur le dedicated server lors des updates, je vois pas comment faire
        state : EntityState,
    },
    HandoffComplete {
        entity_id : EntityId,
        border : Border,
    },
    ClientUpdate {
        entity_id : EntityId,
        pos : Vec2,
        vel : Vec2,
        state : EntityState,
    },
    Register {
        client_id : ClientId,
    },
}

impl GameMessage {
    pub const SUBSCRIBE: u8 = 0x01;
    pub const UNSUBSCRIBE: u8 = 0x02;
    pub const PUBLISH: u8 = 0x03;
    pub const BROADCAST: u8 = 0x04;
    pub const CLIENT_INPUT: u8 = 0x05;

    pub const HANDOFF_REQUEST: u8 = 0x20;
    pub const HANDOFF_ACCEPT: u8 = 0x21;
    pub const HANDOFF_REJECT: u8 = 0x22;
    pub const GHOST_UPDATE: u8 = 0x23;
    pub const HANDOFF_COMPLETE: u8 = 0x24;

    pub const CLIENT_UPDATE : u8 = 0x30;
    pub const REGISTER : u8 = 0x31;

    pub fn as_bytes(&self) -> Bytes {
        let mut buf = BytesMut::new();
        self.append_bytes(&mut buf);
        buf.freeze()
    }
    
    pub fn append_bytes(&self, out : &mut BytesMut) {
        match self {
            GameMessage::Subscribe { client_id, topic } => {
                out.put_u8(Self::SUBSCRIBE);
                out.put_u8(client_id.peer_type.to_byte());
                out.put_u128(client_id.value);
                topic.append_bytes(out);
            }
            GameMessage::Unsubscribe { client_id, topic } => {
                out.put_u8(Self::UNSUBSCRIBE);
                out.put_u8(client_id.peer_type.to_byte());
                out.put_u128(client_id.value);
                topic.append_bytes(out);
            }
            GameMessage::Publish { topic, payload } => {
                out.put_u8(Self::PUBLISH);
                topic.append_bytes(out);
                out.put_u64(payload.len() as u64);
                out.put_slice(payload);
            }
            GameMessage::Broadcast { payload } => {
                out.put_u8(Self::BROADCAST);
                out.put_u64(payload.len() as u64);
                out.put_slice(payload);
            }
            GameMessage::ClientInput { client_id, input } => {
                out.put_u8(Self::CLIENT_INPUT);
                out.put_u8(client_id.peer_type.to_byte());
                out.put_u128(client_id.value);
                out.put_u8(input.data);
            }
            GameMessage::HandoffRequest { entity_id, pos, vel, state, border } => {
                out.put_u8(Self::HANDOFF_REQUEST);
                out.put_u32(entity_id.0);
                out.put_f32(pos.x);
                out.put_f32(pos.y);
                out.put_f32(vel.x);
                out.put_f32(vel.y);
                out.put_u8(border.to_byte());
                out.put(state.to_bytes());
            }
            GameMessage::HandoffAccept { entity_id } => {
                out.put_u8(Self::HANDOFF_ACCEPT);
                out.put_u32(entity_id.0);
            }
            GameMessage::HandoffReject { entity_id } => {
                out.put_u8(Self::HANDOFF_REJECT);
                out.put_u32(entity_id.0);
            }
            GameMessage::GhostUpdate { entity_id, pos, vel, state } => {
                out.put_u8(Self::GHOST_UPDATE);
                out.put_u32(entity_id.0);
                out.put_f32(pos.x);
                out.put_f32(pos.y);
                out.put_f32(vel.x);
                out.put_f32(vel.y);
                out.put(state.to_bytes());
            }
            GameMessage::HandoffComplete { entity_id ,border} => {
                out.put_u8(Self::HANDOFF_COMPLETE);
                out.put_u32(entity_id.0);
                out.put_u8(border.to_byte());
            }
            GameMessage::ClientUpdate { entity_id, pos, vel, state } => {
                out.put_u8(Self::CLIENT_UPDATE);
                out.put_u32(entity_id.0);
                out.put_f32(pos.x);
                out.put_f32(pos.y);
                out.put_f32(vel.x);
                out.put_f32(vel.y);
                out.put(state.to_bytes());
            }
            GameMessage::Register { client_id } => {
                out.put_u8(Self::REGISTER);
                out.put_u8(client_id.peer_type.to_byte());
                out.put_u128(client_id.value);
            }
        }
    }

    pub fn from_bytes(data: &mut Bytes) -> Option<Self> {
        if !data.has_remaining() { return None; }
        let tag = data.get_u8();

        match tag {
            Self::SUBSCRIBE => {
                if data.remaining() < 1 + size_of::<u128>() { return None; }
                let peer_type = PeerType::from_byte(data.get_u8())?;
                let client = data.get_u128();
                let topic = Topic::from_bytes(data)?;
                Some(GameMessage::Subscribe { client_id: ClientId{peer_type, value:client}, topic })
            }
            Self::UNSUBSCRIBE => {
                if data.remaining() < 1 + size_of::<u128>() { return None; }
                let peer_type = PeerType::from_byte(data.get_u8())?;
                let client = data.get_u128();
                let topic = Topic::from_bytes(data)?;
                Some(GameMessage::Unsubscribe { client_id: ClientId{peer_type, value:client}, topic })
            }
            Self::PUBLISH => {
                let topic = Topic::from_bytes(data)?;
                if data.remaining() < size_of::<u64>() { return None; }
                let len = data.get_u64() as usize;
                if data.remaining() < len { return None; }
                let mut payload = vec![0u8; len];
                data.copy_to_slice(&mut payload);
                Some(GameMessage::Publish { topic, payload })
            }
            Self::BROADCAST => {
                if data.remaining() < size_of::<u64>() { return None; }
                let len = data.get_u64() as usize;
                if data.remaining() < len { return None; }
                let mut payload = vec![0u8; len];
                data.copy_to_slice(&mut payload);
                Some(GameMessage::Broadcast { payload })
            }
            Self::CLIENT_INPUT => {
                if data.remaining() < 1 + size_of::<u128>() + size_of::<u8>() { return None; }
                let peer_type = PeerType::from_byte(data.get_u8())?;
                let client = data.get_u128();
                let input = data.get_u8();
                Some(GameMessage::ClientInput {
                    client_id: ClientId{peer_type, value:client},
                    input: PlayerActionHolder{data:input}
                })
            }
            Self::HANDOFF_REQUEST => {
                if data.remaining() < size_of::<u32>() + 4*size_of::<f32>() + 1 + 64 { return None; }
                let entity = data.get_u32();
                let px = data.get_f32();
                let py = data.get_f32();
                let vx = data.get_f32();
                let vy = data.get_f32();
                let border = Border::from_byte(data.get_u8())?;
                let mut state = [0u8; 64];
                data.copy_to_slice(&mut state);
                Some(GameMessage::HandoffRequest {
                    entity_id: EntityId(entity),
                    pos: Vec2::new(px, py),
                    vel: Vec2::new(vx, vy),
                    state: EntityState::from_bytes(Bytes::copy_from_slice(&state))?,
                    border,
                })
            }
            Self::HANDOFF_ACCEPT => {
                if data.remaining() < size_of::<u32>() { return None; }
                let entity = data.get_u32();
                Some(GameMessage::HandoffAccept { entity_id: EntityId(entity) })
            }
            Self::HANDOFF_REJECT => {
                if data.remaining() < size_of::<u32>() { return None; }
                let entity = data.get_u32();
                Some(GameMessage::HandoffReject { entity_id: EntityId(entity) })
            }
            Self::GHOST_UPDATE => {
                if data.remaining() < size_of::<u32>() + 4*size_of::<f32>() + 64 { return None; }
                let entity = data.get_u32();
                let px = data.get_f32();
                let py = data.get_f32();
                let vx = data.get_f32();
                let vy = data.get_f32();
                let mut state = [0u8; 64];
                data.copy_to_slice(&mut state);
                Some(GameMessage::GhostUpdate {
                    entity_id: EntityId(entity),
                    pos: Vec2::new(px, py),
                    vel: Vec2::new(vx, vy),
                    state: EntityState::from_bytes(Bytes::copy_from_slice(&state))?,
                })
            }
            Self::HANDOFF_COMPLETE => {
                if data.remaining() < size_of::<u32>() + 1 { return None; }
                let entity = data.get_u32();
                let border = Border::from_byte(data.get_u8())?;
                Some(GameMessage::HandoffComplete {
                    entity_id: EntityId(entity),
                    border,
                })
            }
            Self::CLIENT_UPDATE => {
                if data.remaining() < size_of::<u32>() + 4*size_of::<f32>() + 64 { return None; }
                let entity = data.get_u32();
                let px = data.get_f32();
                let py = data.get_f32();
                let vx = data.get_f32();
                let vy = data.get_f32();
                let mut state = [0u8; 64];
                data.copy_to_slice(&mut state);
                Some(GameMessage::ClientUpdate {
                    entity_id: EntityId(entity),
                    pos: Vec2::new(px, py),
                    vel: Vec2::new(vx, vy),
                    state: EntityState::from_bytes(Bytes::copy_from_slice(&state))?,
                })
            }
            Self::REGISTER => {
                if data.remaining() < 1 + size_of::<u128>() { return None; }
                let peer_type = PeerType::from_byte(data.get_u8())?;
                let client = data.get_u128();
                Some(GameMessage::Register {
                    client_id: ClientId{peer_type, value: client},
                })
            }
            _ => None,
        }        
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Border {
    Left,
    Top,
    Right,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Border {
    pub fn combine(self, other: Border) -> Option<Border> {
        match self {
            Self::Left => {
                match other {
                    Self::Left   => { Some(Self::Left) }
                    Self::Right  => { None }
                    Self::Top    => { Some(Self::TopLeft) }
                    Self::Bottom => { Some(Self::BottomLeft) }
                    _ => { None }
                }
            }
            Self::Right => {
                match other {       
                    Self::Left   => { None }
                    Self::Right  => { Some(Self::Right) }
                    Self::Top    => { Some(Self::TopRight) }
                    Self::Bottom => { Some(Self::BottomRight) }
                    _ => { None }
                }
            }
            Self::Top => {
                match other {       
                    Self::Left   => { Some(Self::TopLeft) }
                    Self::Right  => { Some(Self::TopRight) }
                    Self::Top    => { Some(Self::Top) }
                    Self::Bottom => { None }
                    _ => { None }
                }                                   
            }
            Self::Bottom => {
                match other {       
                    Self::Left   => { Some(Self::BottomLeft) }
                    Self::Right  => { Some(Self::BottomRight) }
                    Self::Top    => { None }
                    Self::Bottom => { Some(Self::Bottom) }
                    _ => { None }
                }
            }
            _ => { None }
        }
    }

    fn to_byte(self) -> u8 {
        match self {
            Self::Left        => { return 0x00; }
            Self::Top         => { return 0x01; }
            Self::Right       => { return 0x02; }
            Self::Bottom      => { return 0x03; }
            Self::TopLeft     => { return 0x04; }
            Self::TopRight    => { return 0x05; }
            Self::BottomLeft  => { return 0x06; }
            Self::BottomRight => { return 0x07; }
        }
    }

    fn from_byte(b : u8) -> Option<Border> {
        match b {
            0x00 => { Some(Self::Left) }
            0x01 => { Some(Self::Top) }
            0x02 => { Some(Self::Right) }
            0x03 => { Some(Self::Bottom) }
            0x04 => { Some(Self::TopLeft) }
            0x05 => { Some(Self::TopRight) }
            0x06 => { Some(Self::BottomLeft) }
            0x07 => { Some(Self::BottomRight) }
            _    => { None }
        }
    }
}
