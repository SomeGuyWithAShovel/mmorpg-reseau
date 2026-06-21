use bevy::{
    prelude::*,
};

use crate::{
    common::{
        LASER_Z_ORDER, 
        PLAYABLE_AREA,
    },
    entity::{
        EntityTag,
        Velocity,
    }
};

// -------------------------------------------------------------------------------------------------------------------

pub struct LaserPlugin;

impl Plugin for LaserPlugin
{
    fn build(&self, app: &mut App)
    {
        info!("building LaserPlugin");
        
        app.add_message::<MessageSpawnLaser>();

        app.add_systems(Update,
            laser_spawn_from_msg
        );
        app.add_systems(Last,
            laser_despawn
        );

        app.add_systems(PreUpdate,
            laser_cooldown_tick
        );
    }
}

// -------------------------------------------------------------------------------------------------------------------

#[derive(Component, Default)]
pub struct LaserTag;

// -------------------------------------------------------------------------------------------------------------------

pub struct LaserDefaultParams
{
    pub sprite: &'static str,
    pub size: Vec2,
    pub speed: f32,
    pub color: Color,
    pub cooldown: f32,
}

impl LaserDefaultParams // the Default trait doesn't allow default() to be a const fn
{
    const fn new() -> Self
    {
        LaserDefaultParams{
            sprite: "todo_laser.png",
            size: Vec2 {x: 24_f32, y: 24_f32},
            speed: 500.0,
            color: Color::Srgba(Srgba { red: 0.000, green: 0.750, blue: 1.000, alpha: 1.0, }),
            cooldown: 0.25_f32,
        }
    }
}

const LASER_DEFAULT_PARAMS: LaserDefaultParams = LaserDefaultParams::new();

// -------------------------------------------------------------------------------------------------------------------

#[derive(Message)]
pub struct MessageSpawnLaser
{
    pub pos: Vec2, 
    pub angle: f32,
    // TODO : which "side" is the laser on (shot by the player, or by enemies ?)
}

// -------------------------------------------------------------------------------------------------------------------

pub fn laser_spawn_from_msg(
    mut msgs: MessageReader<MessageSpawnLaser>,
    mut commands: Commands,
    asset_server: Res<AssetServer>
)
{
    for msg in msgs.read()
    {
        let mut laser_spawn_pos : Transform = Transform::from_xyz(msg.pos.x, msg.pos.y, LASER_Z_ORDER);
        laser_spawn_pos.rotation = Quat::from_rotation_z(msg.angle - std::f32::consts::FRAC_PI_2);

        let laser_velocity: Velocity = Velocity { v: Vec2::from_angle(msg.angle) * LASER_DEFAULT_PARAMS.speed };

        commands.spawn(
            (
                LaserTag,

                EntityTag,
                laser_velocity,

                laser_spawn_pos,

                Sprite {
                    custom_size: Some(LASER_DEFAULT_PARAMS.size),
                    image: asset_server.load(LASER_DEFAULT_PARAMS.sprite),
                    color: LASER_DEFAULT_PARAMS.color,
                    ..default()
                },
            )
        );
    }
}

// -------------------------------------------------------------------------------------------------------------------

fn laser_is_oob(translation: &Vec3) -> bool
{
    const OFFSET: f32 = 24_f32;

    const HALF_PLAYABLE_AREA: Vec2 = Vec2 {
        x: (PLAYABLE_AREA.x / 2_f32) + OFFSET,
        y: (PLAYABLE_AREA.y / 2_f32) + OFFSET
    };
    
    return 
        (translation.x < - HALF_PLAYABLE_AREA.x) || 
        (translation.x >   HALF_PLAYABLE_AREA.x) || 
        (translation.y < - HALF_PLAYABLE_AREA.y) || 
        (translation.y >   HALF_PLAYABLE_AREA.y)
    ;
}

pub fn laser_despawn(
    mut commands: Commands,
    lasers: Query<(Entity, &Transform), With<LaserTag>>
)
{
    for (entity, transform) in lasers.iter()
    {
        if laser_is_oob(&transform.translation)
        {
            commands.entity(entity).despawn();
        }
    }
}

// -------------------------------------------------------------------------------------------------------------------

#[derive(Component)]
pub struct LaserCooldown
{
    pub timer: Timer,
}

impl Default for LaserCooldown
{
    fn default() -> Self
    {
        return Self { timer: Timer::from_seconds(LASER_DEFAULT_PARAMS.cooldown, TimerMode::Once) };
    }
}

pub fn laser_cooldown_tick(
    time: Res<Time>,
    mut laser_cooldowns: Query<&mut LaserCooldown>
)
{
    for mut laser_cooldown in laser_cooldowns.iter_mut()
    {
        laser_cooldown.timer.tick(time.delta());
    }
}

// -------------------------------------------------------------------------------------------------------------------
