use bevy::prelude::Transform;
use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::game_message::{Topic, ClientId, GameMessage};
use crate::entity::{EntityId, Velocity, EntityState};
use crate::input::PlayerActionHolder;
use bevy::prelude::Vec3;

pub enum TopicContent {
    EntityInfo {
        entity_id: EntityId,
        transform : Transform,
        velocity : Velocity,
        state: EntityState,
    },
    PlayerInput {
        client_id: ClientId,
        input: PlayerActionHolder,
    }
}

impl TopicContent {

    fn append_entity_bytes(
        transform : &Transform,
        velocity : &Velocity,
        state: &EntityState,
        out : &mut BytesMut) {
        let pos = transform.translation;

        let compressed_pos_x = (pos.x * 1024.0).round() as i32;
        let compressed_pos_y = (pos.y * 1024.0).round() as i32;

        let compressed_vel_x = (velocity.v.x * 1024.0).round() as i32;
        let compressed_vel_y = (velocity.v.y * 1024.0).round() as i32;
        
        out.put_i32(compressed_pos_x);
        out.put_i32(compressed_pos_y);
        out.put_i32(compressed_vel_x);
        out.put_i32(compressed_vel_y);
        state.append_bytes(out);
    }

    fn entity_from_bytes(id: EntityId, data : &mut Bytes) -> Option<Self> {
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

        let state = EntityState::from_bytes(data)?;
        
        return Some(Self::EntityInfo{
            entity_id: id,
            transform: Transform::from_translation(pos),
            velocity: vel,
            state,
        });
    }


    fn entity_transform_topic(id : u32) -> String {
        format!("player/{}", id.to_string())
    }

    fn player_input_topic(id: u128) -> String {
        format!("input/{}", id.to_string())
    }
    
    pub fn from_publish(topic : &Topic, payload : Vec<u8>) -> Option<Self> {
        
        fn entity_id_from_string(id_str : &str) -> Option<EntityId> {            
            let id = id_str
                .parse::<u32>()
                .ok()?;
            return Some(EntityId(id));
        }

        fn client_id_from_string(id_str : &str) -> Option<ClientId> {
            let id = id_str
                .parse::<u128>()
                .ok()?;
            return Some(ClientId::of_player(id));
        }
        
        // L'id contenu dans le topic ne sert qu'au broker
        // J'aurai adoré faire ça avec un match, mais ça a pas l'air possible...
        const ENTITY : &'static str = "entity/";
        const INPUT  : &'static str = "input/";
        
        if topic.0.starts_with(ENTITY) {
            let entity_id = entity_id_from_string(&topic.0[..ENTITY.len()])?;
            return Self::entity_from_bytes(entity_id, &mut Bytes::from(payload));
        }
        else if topic.0.starts_with(INPUT) {
            let client_id = client_id_from_string(&topic.0[..INPUT.len()])?;
            let input = PlayerActionHolder{data: payload[0]};
            return Some(Self::PlayerInput{client_id, input});
        }
        else {
            return None;
        }
    }
    
    pub fn to_publish(&self) -> GameMessage {

        let mut bytes = BytesMut::new();
        
        match self {
            Self::EntityInfo{entity_id, transform, velocity, state} => {
                Self::append_entity_bytes(&transform, &velocity, &state, &mut bytes);
                let mut payload = vec![0u8; bytes.len()];
                bytes.copy_to_slice(&mut payload);
                GameMessage::Publish {
                    topic: Topic(Self::entity_transform_topic(entity_id.0)),
                    payload
                }
            }
            Self::PlayerInput{client_id, input} => {
                let payload = vec![input.data];
                GameMessage::Publish {
                    topic: Topic(Self::player_input_topic(client_id.value)),
                    payload,
                }
            }
        }
    }
}
