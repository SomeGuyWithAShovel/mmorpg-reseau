use bevy::{
    prelude::*,
};

use crate::{
    sockets::PlayerId,
    entity::{
        EntityTag,
        Velocity,
    },
    area_of_interest::*,
};

// -------------------------------------------------------------------------------------------------------------------

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin
{
    fn build(&self, app: &mut App)
    {
        info!("building PlayerPlugin");
        
        app.add_message::<MessageSpawnPlayer>();

        app.add_systems(PreUpdate, spawn_player);
    }
}

// -------------------------------------------------------------------------------------------------------------------



#[derive(Message)]
pub struct MessageSpawnPlayer
{
    pub id: PlayerId,
}

// -------------------------------------------------------------------------------------------------------------------

#[derive(Component)]
pub struct PlayerTag
{
    pub id: PlayerId,
}

// -------------------------------------------------------------------------------------------------------------------

pub fn spawn_player(mut commands: Commands, mut msgs: MessageReader<MessageSpawnPlayer>)
{
    for msg in msgs.read()
    {
        commands.spawn((
    
            PlayerTag{ id: msg.id.clone() },
    
            EntityTag,
            Velocity::default(),
    
            Transform::from_xyz(0.0, 0.0, 0.0),

            PlayerListOfPlayersInAreaOfInterest::default(),
        ));
    }
}

// -------------------------------------------------------------------------------------------------------------------
