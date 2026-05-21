use bevy::{
    prelude::*,
};

use game_sockets::GameConnection;

use crate::{
    entity::{
        EntityTag,
        Velocity,
    },
    area_of_interest::AreaOfInterestEntities,
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
    pub connection: GameConnection,
}

// -------------------------------------------------------------------------------------------------------------------

#[derive(Component)]
pub struct PlayerTag
{
    pub connection: GameConnection,
}

// -------------------------------------------------------------------------------------------------------------------

pub fn spawn_player(mut commands: Commands, mut msgs: MessageReader<MessageSpawnPlayer>)
{
    for msg in msgs.read()
    {
        commands.spawn((
    
            PlayerTag{ connection: msg.connection },
    
            EntityTag::new(),
            Velocity::default(),
    
            Transform::from_xyz(0.0, 0.0, 0.0),

            AreaOfInterestEntities::default(),
        ));
    }
}

// -------------------------------------------------------------------------------------------------------------------
