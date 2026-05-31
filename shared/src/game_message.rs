use bytes::{Buf, BufMut, Bytes, BytesMut};
use bevy::prelude::*;

use crate::entity::*;

#[derive(Debug)]
pub enum GameMessage {
    Subscribe {
        client_id : ClientId,
        topic : [u8; 32],
    },
    Unsubscribe {
        client_id : ClientId,
        topic : [u8; 32],
    },
    Publish {
        topic : [u8; 32],
        payload : Vec<u8>,
    },
    Broadcast {
        payload : Vec<u8>,
    },
    ClientInput {
        client_id : ClientId,
        input : [u8; 16],
    },
    HandoffRequest {
        entity_id : EntityId,
        pos : Vec2,
        vel : Vec2,
        state : [u8; 64],
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
        state : [u8; 64],
    },
    HandoffComplete {
        entity_id : EntityId,
    },
}

impl GameMessage {
    const SUBSCRIBE: u8 = 0x01;
    const UNSUBSCRIBE: u8 = 0x02;
    const PUBLISH: u8 = 0x03;
    const BROADCAST: u8 = 0x04;
    const CLIENT_INPUT: u8 = 0x05;

    const HANDOFF_REQUEST: u8 = 0x20;
    const HANDOFF_ACCEPT: u8 = 0x21;
    const HANDOFF_REJECT: u8 = 0x22;
    const GHOST_UPDATE: u8 = 0x23;
    const HANDOFF_COMPLETE: u8 = 0x24;

    pub fn to_bytes(&self) -> Bytes {
        let mut out = BytesMut::new();

        match self {
            GameMessage::Subscribe { client_id, topic } => {
                out.put_u8(Self::SUBSCRIBE);
                out.put_u32(client_id.0);
                out.put_slice(topic);
            }
            GameMessage::Unsubscribe { client_id, topic } => {
                out.put_u8(Self::UNSUBSCRIBE);
                out.put_u32(client_id.0);
                out.put_slice(topic);
            }
            GameMessage::Publish { topic, payload } => {
                out.put_u8(Self::PUBLISH);
                out.put_slice(topic);
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
                out.put_u32(client_id.0);
                out.put_slice(input);
            }
            GameMessage::HandoffRequest { entity_id, pos, vel, state } => {
                out.put_u8(Self::HANDOFF_REQUEST);
                out.put_u32(entity_id.0);
                out.put_f32(pos.x);
                out.put_f32(pos.y);
                out.put_f32(vel.x);
                out.put_f32(vel.y);
                out.put_slice(state);
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
                out.put_slice(state);
            }
            GameMessage::HandoffComplete { entity_id } => {
                out.put_u8(Self::HANDOFF_COMPLETE);
                out.put_u32(entity_id.0);
            }
        }

        out.freeze()
    }

    pub fn from_bytes(mut data: Bytes) -> Option<Self> {
        if !data.has_remaining() { return None; }
        let tag = data.get_u8();

        match tag {
            Self::SUBSCRIBE => {
                if data.remaining() < 4 + 32 { return None; }
                let client = data.get_u32();
                let mut topic = [0u8; 32];
                data.copy_to_slice(&mut topic);
                Some(GameMessage::Subscribe { client_id: ClientId(client), topic })
            }
            Self::UNSUBSCRIBE => {
                if data.remaining() < 4 + 32 { return None; }
                let client = data.get_u32();
                let mut topic = [0u8; 32];
                data.copy_to_slice(&mut topic);
                Some(GameMessage::Unsubscribe { client_id: ClientId(client), topic })
            }
            Self::PUBLISH => {
                if data.remaining() < 32 + 8 { return None; }
                let mut topic = [0u8; 32];
                data.copy_to_slice(&mut topic);
                let len = data.get_u64() as usize;
                if data.remaining() < len { return None; }
                let mut payload = vec![0u8; len];
                data.copy_to_slice(&mut payload);
                Some(GameMessage::Publish { topic, payload })
            }
            Self::BROADCAST => {
                if data.remaining() < 8 { return None; }
                let len = data.get_u64() as usize;
                if data.remaining() < len { return None; }
                let mut payload = vec![0u8; len];
                data.copy_to_slice(&mut payload);
                Some(GameMessage::Broadcast { payload })
            }
            Self::CLIENT_INPUT => {
                if data.remaining() < 4 { return None; }
                let client = data.get_u32();
                let mut input = [0u8; 16];
                data.copy_to_slice(&mut input);
                Some(GameMessage::ClientInput {
                    client_id: ClientId(client) ,
                    input
                })
            }
            Self::HANDOFF_REQUEST => {
                // entity_id (4) + pos(8) + vel(8) + state(64) = 84
                if data.remaining() < 4 + 4*4 + 64 { return None; }
                let entity = data.get_u32();
                let px = data.get_f32();
                let py = data.get_f32();
                let vx = data.get_f32();
                let vy = data.get_f32();
                let mut state = [0u8; 64];
                data.copy_to_slice(&mut state);
                Some(GameMessage::HandoffRequest {
                    entity_id: EntityId(entity),
                    pos: Vec2::new(px, py),
                    vel: Vec2::new(vx, vy),
                    state,
                })
            }
            Self::HANDOFF_ACCEPT => {
                if data.remaining() < 4 { return None; }
                let entity = data.get_u32();
                Some(GameMessage::HandoffAccept { entity_id: EntityId(entity) })
            }
            Self::HANDOFF_REJECT => {
                if data.remaining() < 4 { return None; }
                let entity = data.get_u32();
                Some(GameMessage::HandoffReject { entity_id: EntityId(entity) })
            }
            Self::GHOST_UPDATE => {
                if data.remaining() < 4 + 4*4 { return None; }
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
                    state,
                })
            }
            Self::HANDOFF_COMPLETE => {
                if data.remaining() < 4 { return None; }
                let entity = data.get_u32();
                Some(GameMessage::HandoffComplete { entity_id: EntityId(entity) })
            }
            _ => None,
        }
    }
}
