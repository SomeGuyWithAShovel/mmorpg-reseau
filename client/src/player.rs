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
};

// -------------------------------------------------------------------------------------------------------------------

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin
{
    fn build(&self, app: &mut App)
    {
        info!("building PlayerPlugin");

        app.add_message::<MessagePlayerAction>();
        
        app.add_systems(Startup, 
            spawn_player
        );

        app.add_systems(PreUpdate, 
            (
                system_input_keyboard, 
                player_receive_actions,
                player_process_actions
            ).chain()
        );

    }
}

// -------------------------------------------------------------------------------------------------------------------

#[derive(Component)]
#[require(EntityTag, PlayerActionHolder)]
pub struct PlayerTag;

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

// #[allow(unused)]
#[derive(Debug, Clone, Copy)] // Debug so it can be printed with {:?}
pub enum PlayerAction
{
    Forward,
    Backward,
    Left,
    Right,
    Boost,
    Shoot,
    Dodge,
    Extra
}

impl PlayerAction 
{  
    pub const ALL: [Self; 8] = [ // NO WARNINGS WHEN ADDING A NEW VALUE
        PlayerAction::Forward,
        PlayerAction::Backward,
        PlayerAction::Left,
        PlayerAction::Right,
        PlayerAction::Boost,
        PlayerAction::Shoot,
        PlayerAction::Dodge,
        PlayerAction::Extra
    ];
}

// -------------------------------------------------------------------------------------------------------------------

#[derive(Component, Default)] // Default, so it can initialize itself using u8::default()
pub struct PlayerActionHolder
{
    data: u8,
}

impl PlayerActionHolder // so I don't start to do bitwise operations everywhere
{
    fn get_from_act(act: PlayerAction) -> u8
    {
        match act
        {
            PlayerAction::Forward  => { return 0x01_u8; }
            PlayerAction::Backward => { return 0x02_u8; }
            PlayerAction::Left     => { return 0x04_u8; }
            PlayerAction::Right    => { return 0x08_u8; }
            PlayerAction::Boost    => { return 0x10_u8; }
            PlayerAction::Shoot    => { return 0x20_u8; }
            PlayerAction::Dodge    => { return 0x40_u8; }
            PlayerAction::Extra    => { return 0x80_u8; }
        }
    }
    
    // ---------------------------------------------------------------------------------------------------------------

    pub fn clear_acts(&mut self) { self.data = 0u8; }

    pub fn add_act(&mut self, act: PlayerAction)
    {
        self.data |= Self::get_from_act(act);
    }
    
    // ---------------------------------------------------------------------------------------------------------------
    
    pub fn check_act(&self, act: PlayerAction) -> bool
    {
        return (self.data & Self::get_from_act(act)) != 0u8;
    }
    
    // ---------------------------------------------------------------------------------------------------------------
    
    pub fn get_move_dir(&self) -> Vec2
    {
        let mut move_dir = Vec2 {x: 0.0, y: 0.0};

        if self.check_act(PlayerAction::Forward)
        {
            move_dir.y +=  1.0;    
        }
        if self.check_act(PlayerAction::Backward)
        {
            move_dir.y += -1.0;
        }
        if self.check_act(PlayerAction::Left)
        {
            move_dir.x += -1.0;
        }
        if self.check_act(PlayerAction::Right)
        {
            move_dir.x +=  1.0;
        }
        return move_dir.normalize();
    }
    
    // ---------------------------------------------------------------------------------------------------------------

}

// -------------------------------------------------------------------------------------------------------------------

#[derive(Message)]
pub struct MessagePlayerAction
{
    pub act: PlayerAction,
}

// -------------------------------------------------------------------------------------------------------------------

pub fn player_receive_actions(mut player_actions: Single<&mut PlayerActionHolder, With<PlayerTag>>, 
    mut msgs: MessageReader<MessagePlayerAction>)
{
    for msg in msgs.read()
    {
        player_actions.add_act(msg.act);
    }
}

// -------------------------------------------------------------------------------------------------------------------

pub fn player_process_actions(mut player: Single<(&mut PlayerActionHolder, &mut Velocity), With<PlayerTag>>)
{
    // info!("player_process_actions({:08b})", player.0.data);
    let (actions, velocity) = &mut *player;

    let mut speed: f32 = PLAYER_DEFAULT_PARAMS.speed;
    speed *= if actions.check_act(PlayerAction::Boost) { PLAYER_DEFAULT_PARAMS.boost_mult } else { 1.0 };
    
    let move_dir = actions.get_move_dir();
    if move_dir.length_squared() > PLAYABLE_DIST_EPSILON
    {
        velocity.v += move_dir * speed;
    }

    actions.clear_acts();
}

// -------------------------------------------------------------------------------------------------------------------

pub fn spawn_player(mut commands: Commands, asset_server: Res<AssetServer>)
{
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

    // player
    commands.spawn((
        PlayerTag,
        PLAYER_DEFAULT_PARAMS.transform,
        Sprite {
            custom_size: Some(PLAYER_DEFAULT_PARAMS.size),
            image: asset_server.load(PLAYER_DEFAULT_PARAMS.sprite),
            color: PLAYER_DEFAULT_PARAMS.color,
            ..default()
        },
    ));
}

// -------------------------------------------------------------------------------------------------------------------
