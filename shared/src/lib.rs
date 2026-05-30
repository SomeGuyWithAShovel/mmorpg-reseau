use std::net::{SocketAddr, IpAddr::{V4, V6}, Ipv4Addr, Ipv6Addr, IpAddr};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use uuid::Uuid;
use bevy::prelude::*;

pub mod game_message;

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
