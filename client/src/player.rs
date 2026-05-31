use bevy::{
    prelude::*,
    camera::ScalingMode,
};
use crate::{
    common::{
        PLAYABLE_AREA, 
        PLAYABLE_DIST_EPSILON, 
        PLAYER_Z_ORDER
    },
    entity::*,
    input::system_input_keyboard,
    connection::*,    
};
use shared::input::*;

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

#[derive(Component)]
pub struct LocalPlayerTag;

// -------------------------------------------------------------------------------------------------------------------

pub struct PlayerDefaultParams
{
    pub sprite: &'static str,
    pub size: Vec2,
    pub speed: f32,
    pub boost_mult: f32,
    pub transform: Transform,
    pub color: Color,
}
impl PlayerDefaultParams // the Default trait doesn't allow default() to be a const fn
{
    const fn new() -> Self {
        PlayerDefaultParams{
            sprite: "todo_texture_atlas.png",
            size: Vec2::splat(24.0),
            speed: 100.0,
            boost_mult: 1.5,
            transform: Transform::from_xyz(0.0, 0.0, PLAYER_Z_ORDER),
            color: Color::Srgba(Srgba { red: 0.000, green: 0.750, blue: 1.000, alpha: 1.0, }),
        }
    }
}

const PLAYER_DEFAULT_PARAMS: PlayerDefaultParams = PlayerDefaultParams::new();


// -------------------------------------------------------------------------------------------------------------------

#[derive(Message)]
pub struct MessagePlayerAction
{
    pub act: PlayerAction,
}

// -------------------------------------------------------------------------------------------------------------------

pub fn player_receive_actions(mut player_actions: Single<&mut PlayerActionHolder, With<LocalPlayerTag>>, 
    mut msgs: MessageReader<MessagePlayerAction>)
{
    for msg in msgs.read()
    {
        player_actions.add_act(msg.act);
    }
}

// -------------------------------------------------------------------------------------------------------------------

pub fn player_process_actions(mut player: Single<(&mut PlayerActionHolder, &mut Velocity), With<LocalPlayerTag>>)
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

#[derive(Bundle, Default)]
struct PlayerBundle(EntityTag, Velocity, Transform, Sprite, PlayerActionHolder);

pub fn spawn_player<'a>(id: u64, commands : &'a mut Commands, asset_server : &AssetServer) -> EntityCommands<'a> {
    commands.spawn(PlayerBundle(
        EntityTag(id),
        Velocity::default(),
        PLAYER_DEFAULT_PARAMS.transform,
        Sprite {
            custom_size: Some(PLAYER_DEFAULT_PARAMS.size),
            image: asset_server.load(PLAYER_DEFAULT_PARAMS.sprite),
            color: PLAYER_DEFAULT_PARAMS.color,
            ..default()
        },
        PlayerActionHolder::default(),
    ))
}

pub fn spawn_local_player(msg : On<WelcomeEvent>, mut commands: Commands, asset_server: Res<AssetServer>) {
    // camera
    commands.spawn((
        Camera2d,
        Projection::Orthographic(
            OrthographicProjection{
                scaling_mode: ScalingMode::AutoMax{max_width: PLAYABLE_AREA.x as f32, max_height: PLAYABLE_AREA.y as f32},
                ..OrthographicProjection::default_2d()
            } 
        )
    ));
    spawn_player(msg.id, &mut commands, asset_server.into_inner()).insert(LocalPlayerTag);
}

// -------------------------------------------------------------------------------------------------------------------
