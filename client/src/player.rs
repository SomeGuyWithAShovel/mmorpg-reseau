use bevy::{
    prelude::*,
    camera::ScalingMode,
};
use crate::{
    input::system_input_keyboard,
    connection::*,
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
                player_process_actions
            ).chain()
        );
        app.add_observer(spawn_local_player);

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

pub fn player_process_actions(mut player: Single<(&mut PlayerActionHolder, &mut Velocity)>)
{
    // info!("player_process_actions({:08b})", player.0.data);
    let (actions, velocity) = &mut *player;

    let mut speed: f32 = PLAYER_DEFAULT_PARAMS.speed;
    speed *= if actions.check_act(PlayerAction::Boost) { PLAYER_DEFAULT_PARAMS.boost_mult } else { 1.0 };
    
    let move_dir = actions.get_move_dir();
    if move_dir.length_squared() > PLAYABLE_DIST_EPSILON
    {
        velocity.v = move_dir * speed;
        // info!("player_speed : {}", velocity.v.length());
    }
    else
    {
        velocity.reset();
    }

    actions.clear_acts();
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

const CAMERA_AREA : Vec2 = Vec2::new(160.0, 90.0);

pub fn spawn_local_player(msg : On<WelcomeEvent>, mut commands: Commands, asset_server: Res<AssetServer>) {
    // camera
    commands.spawn((
        Camera2d,
        Projection::Orthographic(
            OrthographicProjection{
                scaling_mode: ScalingMode::AutoMax{max_width: CAMERA_AREA.x as f32, max_height: CAMERA_AREA.y as f32},
                ..OrthographicProjection::default_2d()
            } 
        )
    ));
    spawn_player(msg.id, &mut commands, asset_server.into_inner()).insert(PlayerActionHolder::default());
}

// -------------------------------------------------------------------------------------------------------------------
