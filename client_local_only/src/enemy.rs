use bevy::{
    prelude::*,
    math::NormedVectorSpace, 
    time::common_conditions::on_timer,
};

use std::time::Duration;

use crate::{
    common::{
        ENEMY_Z_ORDER, 
        PLAYABLE_AREA, 
        PLAYABLE_DIST_EPSILON, 
    },
    entity::*,
    player::PlayerTag,
    rand::PRNG,
};

// -------------------------------------------------------------------------------------------------------------------

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin
{
    fn build(&self, app: &mut App)
    {
        info!("building EnemyPlugin");

        app.init_resource::<EnemySpawnRandState>();
        app.add_systems(
            PreUpdate,
            spawn_enemies.run_if(on_timer(Duration::from_millis(1000)))
        );

        app.add_systems(
            FixedUpdate, 
            (
                enemies_target_player,
                enemies_set_velocity_to_target
            ).chain()
        );
    }
}

// -------------------------------------------------------------------------------------------------------------------

#[derive(Component)]
pub struct EnemyTag;

// -------------------------------------------------------------------------------------------------------------------

pub struct EnemyDefaultParams
{
    pub sprite: &'static str,
    pub size: Vec2,
    pub speed: f32,
    pub color: Color,
}

impl EnemyDefaultParams
{
    const fn new() -> Self
    {
        return EnemyDefaultParams {
            sprite: "todo_enemy.png",
            size: Vec2::splat(18.0),
            speed: 50.0,
            color: Color::Srgba( Srgba { red: 1.000, green: 0.000, blue: 0.000, alpha: 1.0, }),
        };
    }
}

const ENEMY_DEFAULT_PARAMS: EnemyDefaultParams = EnemyDefaultParams::new();

// -------------------------------------------------------------------------------------------------------------------

fn get_rand_border(r: f32, w: f32, h: f32) -> Vec2
{    
    //  '.      0.25      .' <= threshold
    //
    // 0.5                0.0
    //
    //  .'      0.75      '.

    // 0.0 --- threshold ------- 0.25 ------- (0.5-threshold) --- 0.5 --- (0.5+threshold) ------- 0.75 ------- (1-threshold) --- 1.0
    // then, with alpha and Vec2 in each if block :
    // [0            ,     threshold] => (    w , [h/2,   0])
    // [    threshold, 0.5-threshold] => ([w, 0],         0 )
    // [0.5-threshold, 0.5+threshold] => (    0 , [  0,   h])
    // [0.5+threshold, 1.0-threshold] => ([0, w],         h )
    // [1.0-threshold, 1.0          ] => (    w , [h  , h/2])

    // we do all that instead of just having a random int in [0,4[ then a random in [0, w or h] because :
    // - only one call to our prng,
    // - we want to keep the probability distribution evenly spread across the border. If we choose first in [0,4[ evenly, then if w != h, we will get too much generation on the smallest side
    
    // to preserve the w/h ratio, the top left corner may not be 0.125
    let threshold: f32 = h / ((h + w) * 4.0);
    let mut alpha: f32;
    
    if r < threshold // left (top-half)
    {                                                // r: [0; threshold]
        alpha = r;                                   //    [0; threshold]
        alpha = alpha / threshold;                   //    [0; 1]
        alpha = (1.0 - alpha) * 0.5;                 // 0 means top, 1 means bottom, and we need [center, top]
        
        return Vec2{x: w, y: h * alpha};
    }
    
    if r < (0.5 - threshold) // top
    {                                                // r: [threshold, 0.5-threshold]
        alpha = r - threshold;                       //    [0; (0.5-threshold) - threshold]
        alpha = alpha / (0.5 - (2.0 * threshold));   //    [0; 1]
        alpha = 1.0 - alpha;                         // 0 means left, 1 means right
        
        return Vec2 {x: w * alpha, y: 0.0};
    }
    if r < (0.5 + threshold) // right
    {                                                // r: [0.5-threshold, 0.5+threshold]
        alpha = r - (0.5 - threshold);               //    [0; (0.5+threshold) - (0.5-threshold)] = [0; 2*threshold]
        alpha = alpha / (2.0 * threshold);           //    [0; 1]
        // alpha = alpha                             // 0 means top, 1 means bottom
        
        return Vec2 {x: 0.0, y: h * alpha};
    }
    if r < (1.0 - threshold) // bottom
    {                                                // r: [0.5+threshold; 1-threshold]
        alpha = r - (0.5 + threshold);               //    [0; (1-threshold) - (0.5+threshold)] = [0; 0.5 - 2*threshold]
        alpha = alpha / (0.5 - (2.0 * threshold));   //    [0; 1]
        // alpha = alpha                             // 0 means left, 1 means right
        
        return Vec2 {x: w * alpha, y: h };
    }
    if (r >= (1.0 - threshold)) && (r <= 1.0) // left (bottom-half)
    {                                                // r: [1-threshold; 1]
        alpha = r - (1.0 - threshold);               //    [0; threshold]
        alpha = alpha / threshold;                   //    [0; 1]
        alpha = ((1.0 - alpha) + 1.0) * 0.5;         // 0 means top, 1 means bottom, and we need [bottom, center]
        
        return Vec2 {x: w, y: h * alpha}
    }
    // print error : r > 1.0
    return Vec2::ZERO;

}

#[derive(Resource, Default)]
pub struct EnemySpawnRandState
{
    prng: PRNG,
}

impl EnemySpawnRandState
{
    pub fn rand_01(&mut self) -> f64
    {
        return self.prng.rand_01();
    }
}

// -------------------------------------------------------------------------------------------------------------------
// "lag" starts at 130_000 enemies...

pub fn spawn_enemies(
    mut commands: Commands,
    asset_server: Res<AssetServer>, 
    mut rand: ResMut<EnemySpawnRandState>
)
{
    const NB_ENEMIES_PER_CALL: u64 = 1_u64;
    for _ in 0..NB_ENEMIES_PER_CALL
    {
        let r: f32 = rand.rand_01() as f32;
        // info!("rand : {}", r);

        let rand_start: Vec2 = get_rand_border(r, PLAYABLE_AREA.x as f32, PLAYABLE_AREA.y as f32) - (PLAYABLE_AREA / 2.0);
        let enemy_transform : Transform = Transform::from_xyz(rand_start.x, rand_start.y, ENEMY_Z_ORDER);

        commands.spawn(
            (
                EnemyTag,

                EntityTag,
                Velocity::default(),

                enemy_transform,

                Sprite {
                    custom_size: Some(ENEMY_DEFAULT_PARAMS.size),
                    image: asset_server.load(ENEMY_DEFAULT_PARAMS.sprite),
                    color: ENEMY_DEFAULT_PARAMS.color,
                    ..default()
                },

                Target::default(),
            )
        );
    }
    
    // if NB_ENEMIES_PER_CALL <= 1_u64 { info!("enemy spawned"); } else { info!("{} enemies spawned", NB_ENEMIES_PER_CALL); }
    
}

// -------------------------------------------------------------------------------------------------------------------

#[derive(Component)]
pub struct Target
{
    pub pos: Vec2,
}

impl Default for Target 
{
    fn default() -> Self { return Self {pos: Vec2::NAN}; }     
}

impl Target
{
    pub fn invalid(&self) -> bool { return self.pos == Vec2::NAN; }
    
    #[allow(unused)]
    pub fn set_invalid(&mut self) { self.pos = Vec2::NAN; }
}

// -------------------------------------------------------------------------------------------------------------------

// TODO: on_player_moved, instead of on each tick
pub fn enemies_target_player(
    player: Single<&Transform, With<PlayerTag>>, 
    enemies: Query<&mut Target, With<EnemyTag>>
)
{
    for mut target in enemies
    {
        target.pos = player.translation.xy();
    }
}

// -------------------------------------------------------------------------------------------------------------------

pub fn enemies_set_velocity_to_target(
    mut enemies : Query <(&Transform, &Target, &mut Velocity), With<EnemyTag>>
)
{
    for (transform, target, mut velocity) in &mut enemies
    {
        if target.invalid() == false
        {
            let new_velocity_dir = target.pos - transform.translation.xy();
            if new_velocity_dir.norm_squared() > PLAYABLE_DIST_EPSILON
            {
                velocity.v = new_velocity_dir.normalize() * ENEMY_DEFAULT_PARAMS.speed;
                // info!("ennemy speed : {}", velocity.v.length());
            }
            else
            {
                // info!("target reached");
            }
        }
        else
        {
            // ?
        }
    }
}

// -------------------------------------------------------------------------------------------------------------------
