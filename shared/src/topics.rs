use crate::Topic;

use bevy::prelude::Transform;
use bytes::{Buf, BufMut, Bytes, BytesMut};

pub enum TopicContent {
    PlayerInfo(pub Transform, pub Velocity),
}

impl TopicContent {

    fn append_player_bytes(transform : &Transform, velocity : &Velocity, out : &mut BytesMut) {
        let pos = transform.translation;
        let rot = transform.rotation;

        let compressed_pos_x = round(pos.x * 1024) as i32;
        let compressed_pos_y = round(pos.y * 1024) as i32;

        let compressed_vel_x = round(velocity.x * 1024) as i32;
        let compressed_vel_y = round(velocity.y * 1024) as i32;
        
        out.put_i32(compressed_pos_x);
        out.put_i32(compressed_pos_y);
        out.put_i32(compressed_vel_x);
        out.put_i32(compressed_vel_y);
    }

    fn player_from_bytes(data : &mut Bytes) -> Option<Self> {
        if data.remaining() < 4*size_of::<i32>() { return None; }
        let compressed_pos_x = data.get_i32();
        let compressed_pos_y = data.get_i32();

        let compressed_vel_x = data.get_i32();
        let compressed_vel_y = data.get_i32();

        let pos = Vec3::new(compressed_pos_x as f32, compressed_pos_y as f32, 0.0) / 1024.0;
        let vel = Velocity::new(
            compressed_vel_x as f32 / 1024.0,
            compressed_vel_y as f32 / 1024.0
        );        
        
        return Self::PlayerInfo(Transform::from_translation(pos), vel);
    }
    
    const PLAYER_TRANSFORM : str = "/player";
    
    pub fn from_publish(topic : &Topic, payload : Vec<u8>) -> Option<Self> {
        match topic.0.as_str() {
            PLAYER_TRANSFORM => {
                return player_from_bytes(Bytes::from_bytes(payload))?;
            }
        }
    }
    
    pub fn to_publish(&self) -> GameMessage {

        let mut bytes = BytesMut::new();
        
        match self {
            Self::PlayerInfo(transform, velocity) => {
                Self::append_player_bytes(&transform, &velocity, &mut bytes);
                let payload = vec![0u8; bytes.len()];
                bytes.copy_to_slice(&mut payload);
                GameMessage::Publish {
                    topic: Topic(PLAYER_TRANSFORM),
                    payload
                }
            }
        }
    }
}
