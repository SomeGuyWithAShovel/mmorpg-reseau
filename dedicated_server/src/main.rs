use bevy::prelude::*;
use std::str::FromStr;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::collections::HashMap;
use std::fmt;

mod common;
use crate::common::*;
mod game_sockets;
use crate::game_sockets::*

pub struct PlayerInfo(u32);

#[derive(Resource, Default)]
pub struct PlayerRegistry {
    pub players: HashMap<SocketAddr, PlayerInfo>,
}

#[derive(Resource, Deref, DerefMut)]
pub struct HeartbeatResource(Heartbeat);

#[derive(Resource)]
struct HeartbeatTimer(Timer);

// main.rs
fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .insert_resource(ServerConfig::from_env())
        .insert_resource(HeartbeatTimer(Timer::from_seconds(SECONDS_BETWEEN_HEARTBEATS , TimerMode::Repeating)))
        .add_systems(Startup, bind_socket)
        .add_systems(Update, (receive_packets, send_heartbeat).chain())
        .run();
}

fn bind_socket(mut commands : Commands) {

}

fn send_heartbeat(time: Res<Time>, mut timer: ResMut<HeartbeatTimer>, heartbeat : Res<HeartbeatResource>) {
    if timer.0.tick(time.delta()).just_finished() {
        
    }
}

fn receive_packets() {

}
