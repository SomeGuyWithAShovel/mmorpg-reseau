use bevy::prelude::*;
use bevy::camera::ScalingMode;
use crate::connection::UpdateEntity;
use crate::ClientEntityTag;
use crate::player::spawn_player;
use std::collections::HashMap;
use shared::entity::{EntityState, EntityId};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, init)
            .add_systems(StateTransition, update_states)
            .add_systems(Update, reajust_position)
            .add_systems(Update, show_entities);
    }
}

const CAMERA_AREA : Vec2 = Vec2::new(160.0, 90.0);

fn init(mut commands : Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(
            OrthographicProjection{
                scaling_mode: ScalingMode::AutoMax{max_width: CAMERA_AREA.x as f32, max_height: CAMERA_AREA.y as f32},
                ..OrthographicProjection::default_2d()
            } 
        )
    ));
}

fn update_states(mut msg_reader : MessageReader<UpdateEntity>) {
    let mut map = HashMap::<u32, EntityState>::new();
    for msg in msg_reader.read() {        
        let UpdateEntity{id: EntityId(id), state, ..} = msg;
        map.insert(*id, *state);
    }

    // Mise à jour des états en fonction des besoins...
}

fn reajust_position(mut msg_reader : MessageReader<UpdateEntity>,
                    mut query : Query<(&ClientEntityTag, &mut Transform)>,
                    mut commands : Commands,
                    asset_server : Res<AssetServer>) {
    let mut map = HashMap::<u32, Transform>::new();
    for msg in msg_reader.read() {        
        let UpdateEntity{id: EntityId(id), pos, vel, ..} = msg;
        let rot = Quat::from_rotation_z(vel.to_angle() - std::f32::consts::FRAC_PI_2);
        let transform = Transform::from_xyz(pos.x, pos.y, 0.0)
            .with_rotation(rot);
        map.insert(*id, transform);
    }

    for (ClientEntityTag(EntityId(id)), mut transform) in &mut query {
        if let Some(new_transform) = map.get(id) {
            transform.translation = new_transform.translation;
            transform.rotation = new_transform.rotation;
            map.remove(id);
        }
    }

    // Entitées non encore créees
    for (id, transform) in map {
        spawn_player(id, &mut commands, &asset_server).entry::<Transform>().and_modify(move |mut t| {
            t.translation = transform.translation;
            t.rotation = transform.rotation;
        });
    }
}

fn show_entities() {

}
