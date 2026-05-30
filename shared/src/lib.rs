use std::net::{SocketAddr, IpAddr::{V4, V6}, Ipv4Addr, Ipv6Addr, IpAddr};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use uuid::Uuid;
use bevy::prelude::*;
pub const SECONDS_BETWEEN_HEARTBEATS : f32 = 5.0;

#[derive(Debug)]
pub struct Heartbeat {
    pub id : Uuid,
    pub addr : SocketAddr,
    pub zone : String,
    pub player_count : usize,
    pub is_full : bool,
}

impl Heartbeat {
    pub fn to_bytes(&self) -> Bytes {
        // id (uuid v4): 16 octets
        // addr est de taille 4 octets ou 16 octets. On ajoute un booléen (1 octet) pour dire si on est ipv4 ou non
        // port (u16): 2 octets
        // zone : arbitraire, on range length de longueur 8 puis la donnée
        // player_count, 8 octets
        // On vérifie quand même

        let Self{id, addr, zone, player_count, is_full} = self;
        
        let len = 16 + 1 + 16 + 2 + 8 + zone.len() + 8;
        let mut res = BytesMut::with_capacity(len);

        res.put_slice(id.as_bytes());
        let bools = (addr.is_ipv4() as u8) | (*is_full as u8) << 1;
        res.put_u8(bools);

        match addr.ip() {
            V4(ipv4) => { res.put_slice(&ipv4.octets()); }
            V6(ipv6) => { res.put_slice(&ipv6.octets()); }
        }
        res.put_u16(addr.port());

        res.put_u64(zone.len() as u64);
        res.put_slice(zone.as_bytes());
        res.put_u64(*player_count as u64);
        
        res.freeze()
    }

    pub fn from_bytes(mut data : Bytes) -> Option<Self> {
        let mut id_bytes = [0; 16];
        data.copy_to_slice(&mut id_bytes);
        let id = Uuid::from_bytes(id_bytes);

        let bools = data.get_u8();
        let is_ipv4 = bools & 1 == 1;
        let is_full = (bools >> 1) == 1;
        let addr : SocketAddr;
        if is_ipv4 {
            let mut ipv4_bytes = [0; 4];
            data.copy_to_slice(&mut ipv4_bytes);
            let port = data.get_u16();

            addr = SocketAddr::new(IpAddr::V4(
                Ipv4Addr::new(ipv4_bytes[0], ipv4_bytes[1], ipv4_bytes[2], ipv4_bytes[3])
            ), port);
        }
        else {
            let mut ipv6_bytes = [0; 16];
            data.copy_to_slice(&mut ipv6_bytes);
            let port = data.get_u16();

            addr = SocketAddr::new(IpAddr::V6(
                Ipv6Addr::from_bits(u128::from_be_bytes(ipv6_bytes))
            ), port);
        }

        let str_len = data.get_u64();
        let mut bytes_vec : Vec<u8> = vec![0; str_len as usize];
        data.copy_to_slice(&mut bytes_vec[..]);

        let res_zone = String::from_utf8(bytes_vec);
        let Ok(zone) = res_zone else { return None; };
        let player_count = data.get_u64();

        Some(Self {id, addr, zone, player_count: player_count as usize, is_full})
    }
}

// Message client/serveur:
// Elem : BinaryDataType + octets de contenu
// contenu List : u64 de longueur + longueur*[Elem]
// contenu Join : "JOIN { username }" en ascii
pub enum BinaryDataType {
    List,
    Transform2d,
    Join,
    Welcome,
}

impl BinaryDataType {
    pub fn as_byte(self) -> u8 {
        match self {
            BinaryDataType::Join => 0,
            BinaryDataType::Welcome => 1,
            BinaryDataType::List => 10,
            BinaryDataType::Transform2d => 11,
        }
    }

    pub fn from_byte(byte : u8) -> Option<BinaryDataType> {
        match byte {
            0 => Some(BinaryDataType::Join),
            1 => Some(BinaryDataType::Welcome),
            10 => Some(BinaryDataType::List),
            11 => Some(BinaryDataType::Transform2d),
            _ => None,
        }
    }
}

pub fn unscaled_transform_2d_as_bytes(transform : Transform) -> Bytes {
    let rotation_vector = transform.rotation.mul_vec3(Vec3::X); 
    let mut rotation_angle = rotation_vector.angle_between(Vec3::X);
    if rotation_vector.y < 0.0 {
        rotation_angle = std::f32::consts::TAU - rotation_angle;
    }
    
    // [-0.5, 255,5]
    let angle_norm = ((rotation_angle / std::f32::consts::PI) * 128.0) - 0.5;
    let angle_int = angle_norm.round().clamp(0.0, 255.0) as u8;
    let x_int = (transform.translation.x*1024.0) as i32;
    let y_int = (transform.translation.y*1024.0) as i32;
    
    let mut buf = BytesMut::with_capacity(4 + 4 + 1);
    buf.put_i32(x_int);
    buf.put_i32(y_int);
    buf.put_u8(angle_int);

    buf.freeze()
}

pub fn bytes_as_unscaled_transform_2d(mut bytes : Bytes) -> Transform {
    let pos_x = (bytes.get_i32() as f32) /1024.0;
    let pos_y = (bytes.get_i32() as f32) /1024.0;
    let angle_int = bytes.get_u8() as f32;
    let angle = ((angle_int + 0.5) / 128.0) * std::f32::consts::PI;

    Transform::IDENTITY
        .with_translation(Vec3::new(pos_x, pos_y, 0.0))
        .with_rotation(Quat::from_rotation_z(angle))
}

// TODO : Mettre dans shared
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest
{
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginSuccess
{
    pub player_id: Uuid,
    pub server: ServerInfo,
}

#[derive(Serialize, Deserialize)]
pub struct ServerInfo
{
    pub ip: Ipv4Addr,
    pub port: u16,
    pub zone: String,
}

pub enum EntityState {
    Owned,
    PendingHandoff,
    Ghost,
}

#[derive(Debug, Clone, Copy)]
pub struct ClientId(pub u32);
#[derive(Debug, Clone, Copy)]
pub struct EntityId(pub u32);

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
            GameMessage::ClientInput { client_id } => {
                out.put_u8(Self::CLIENT_INPUT);
                out.put_u32(client_id.0);
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
            GameMessage::GhostUpdate { entity_id, pos, vel } => {
                out.put_u8(Self::GHOST_UPDATE);
                out.put_u32(entity_id.0);
                out.put_f32(pos.x);
                out.put_f32(pos.y);
                out.put_f32(vel.x);
                out.put_f32(vel.y);
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
                Some(GameMessage::ClientInput { client_id: ClientId(client) })
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
                Some(GameMessage::GhostUpdate {
                    entity_id: EntityId(entity),
                    pos: Vec2::new(px, py),
                    vel: Vec2::new(vx, vy),
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
