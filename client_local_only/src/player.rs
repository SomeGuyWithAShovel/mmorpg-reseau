use std::fmt;

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
    laser::{
        MessageSpawnLaser,
        LaserCooldown, 
    },
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
    const fn new() -> Self
    {
        return PlayerDefaultParams {
            sprite: "todo_texture_atlas.png",
            size: Vec2::splat(24.0),
            speed: 100.0,
            boost_mult: 1.5,
            transform: Transform::from_xyz(0.0, 0.0, PLAYER_Z_ORDER),
            color: Color::Srgba(Srgba { red: 0.000, green: 0.750, blue: 1.000, alpha: 1.0, }),
        };
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
        PlayerAction::Extra,
    ];
    pub const NAMES: [&'static str; 8] = [
        "Forward",
        "Backward",
        "Left",
        "Right",
        "Boost",
        "Shoot",
        "Dodge",
        "Extra",
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
        };
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
    
    #[allow(unused)]
    pub fn has_any_act(&self) -> bool
    {
        return self.data != 0_u8;
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

impl fmt::Debug for PlayerActionHolder
{   // https://doc.rust-lang.org/std/fmt/trait.Debug.html
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        let mut out = f.debug_struct("PlayerActionHolder");

        for i in 0_usize..8
        {
            let act_name = PlayerAction::NAMES[i];
            let act: u8 = if self.check_act(PlayerAction::ALL[i]) { 1_u8 } else { 0_u8 };
            out.field(act_name, &act);
        }
        return out.finish();
    }
}

// -------------------------------------------------------------------------------------------------------------------

#[derive(Message)]
pub struct MessagePlayerAction
{
    pub act: PlayerAction,
}

// -------------------------------------------------------------------------------------------------------------------

pub fn player_receive_actions(
    mut msgs: MessageReader<MessagePlayerAction>,
    mut player_actions: Single<&mut PlayerActionHolder, With<PlayerTag>>,
)
{
    for msg in msgs.read()
    {
        player_actions.add_act(msg.act);
    }
}

// -------------------------------------------------------------------------------------------------------------------

pub fn player_process_actions(
    player: Single<(&mut PlayerActionHolder, &mut Velocity, &Transform, &mut LaserCooldown), With<PlayerTag>>,
    mut laser_msg_writer : MessageWriter<MessageSpawnLaser>,
)
{
    let (mut actions, mut velocity, transform, mut laser_cooldown) = player.into_inner();

    // info!("\n{:?}", actions);

    if (laser_cooldown.timer.is_finished()) && actions.check_act(PlayerAction::Shoot)
    {
        let laser_angle: f32 = transform.rotation.to_euler(EulerRot::YXZ).2 + std::f32::consts::FRAC_PI_2; // YXZ = EulerRot::default()
        let laser_offset: Vec2 = Vec2::from_angle(laser_angle) * 8_f32;
        let laser_start_pos: Vec2 = transform.translation.xy() + laser_offset;
        
        laser_cooldown.timer.reset();
        laser_msg_writer.write(MessageSpawnLaser { pos: laser_start_pos, angle: laser_angle });
    }

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

pub fn spawn_player(
    mut commands: Commands, 
    asset_server: Res<AssetServer>
)
{
    // camera
    commands.spawn(
        (
            Camera2d,
            Projection::Orthographic(
                OrthographicProjection {
                    scaling_mode: ScalingMode::AutoMax{max_width: PLAYABLE_AREA.x as f32, max_height: PLAYABLE_AREA.y as f32},
                    ..OrthographicProjection::default_2d()
                } 
            )
        )
);

    // player
    commands.spawn(
        (
            PlayerTag,

            EntityTag,
            Velocity::default(),

            PLAYER_DEFAULT_PARAMS.transform,

            Sprite {
                custom_size: Some(PLAYER_DEFAULT_PARAMS.size),
                image: asset_server.load(PLAYER_DEFAULT_PARAMS.sprite),
                color: PLAYER_DEFAULT_PARAMS.color,
                ..default()
            },

            PlayerActionHolder::default(),

            LaserCooldown::default(),
        )
    );
}

// -------------------------------------------------------------------------------------------------------------------
