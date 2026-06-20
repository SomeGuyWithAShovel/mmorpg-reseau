use bevy::prelude::*;
use crate::{
    input::system_input_keyboard,
    ClientEntityTag,
};
use shared::{input::*, entity::*};

// -------------------------------------------------------------------------------------------------------------------

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin
{
    fn build(&self, app: &mut App)
    {
        info!("building PlayerPlugin");

        app.add_message::<MessagePlayerAction>();
        
        app.add_systems(PreUpdate, 
            (
                system_input_keyboard, 
                player_receive_actions,
            ).chain()
        );

    }
}

// -------------------------------------------------------------------------------------------------------------------

#[derive(Message)]
pub struct MessagePlayerAction
{
    pub act: PlayerAction,
}

// -------------------------------------------------------------------------------------------------------------------

pub fn player_receive_actions(mut player_actions: Single<&mut PlayerActionHolder>, 
    mut msgs: MessageReader<MessagePlayerAction>)
{
    player_actions.data = 0;
    for msg in msgs.read()
    {
        player_actions.add_act(msg.act);
    }
}

// -------------------------------------------------------------------------------------------------------------------

#[derive(Bundle)]
struct PlayerBundle(ClientEntityTag, Velocity, Transform, Sprite);

pub fn spawn_player<'a>(id: u32, commands : &'a mut Commands, asset_server : &AssetServer) -> EntityCommands<'a> {
    commands.spawn(PlayerBundle(
        ClientEntityTag(EntityId(id)),
        Velocity::default(),
        PLAYER_DEFAULT_PARAMS.transform,
        Sprite {
            custom_size: Some(PLAYER_DEFAULT_PARAMS.size),
            image: asset_server.load(PLAYER_DEFAULT_PARAMS.sprite),
            color: PLAYER_DEFAULT_PARAMS.color,
            ..default()
        },
    ))
}
