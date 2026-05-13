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
    player::{
        PlayerTag
    },
};

// -------------------------------------------------------------------------------------------------------------------

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin
{
    fn build(&self, app: &mut App)
    {
        info!("building EnemyPlugin");

        app.init_resource::<QuickRandState>();
        app.add_systems(PreUpdate,
            spawn_enemies.run_if(on_timer(Duration::from_millis(1000)))
        );

        app.add_systems(FixedUpdate, 
            (
                enemies_target_player,
                enemies_set_velocity_to_target
            ).chain()
        );
    }
}

// -------------------------------------------------------------------------------------------------------------------

#[derive(Component)]
#[require(EntityTag)]
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
    const fn new() -> Self {
        EnemyDefaultParams{
            sprite: "todo_enemy.png",
            size: Vec2::splat(18.0),
            speed: 50.0,
            color: Color::Srgba(Srgba { red: 1.000, green: 0.000, blue: 0.000, alpha: 1.0, }),
        }
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

// -------------------------------------------------------------------------------------------------------------------
// no external packages imported
// https://en.wikipedia.org/wiki/Xorshift#xorshiftr+


#[derive(Resource)]
pub struct QuickRandState {
    s: [u64; 2],
}
impl Default for QuickRandState {
    fn default() -> Self {
        Self{s: [0x_63_FE_C5_30_B8_C3_07_95, 0x_0B_11_1B_51_15_83_A7_A2]} // random.org/bytes
    }
}
impl QuickRandState {
    fn next(&mut self) -> u64
    {
        let mut x = self.s[0];
        let y = self.s[1];

        self.s[0] = y;

        x ^= x << 23; // shift & xor
        x ^= x >> 17; // shift & xor
        x ^= y; // xor

        self.s[1] = x.wrapping_add(y);
        return x;
    }

    pub fn rand(&mut self) -> f32
    {
        return self.next() as f32 / u64::MAX as f32;
    }
}

// -------------------------------------------------------------------------------------------------------------------
// "lag" starts at 130_000 enemies...

pub fn spawn_enemies(mut commands: Commands, asset_server: Res<AssetServer>, mut rand: ResMut<QuickRandState>)
{
    const NB_ENEMIES_PER_CALL: u64 = 1_u64;
    for _ in 0..NB_ENEMIES_PER_CALL
    {
        let r = rand.rand();
        // info!("rand : {}", r);

        let rand_start: Vec2 = get_rand_border(r, PLAYABLE_AREA.x as f32, PLAYABLE_AREA.y as f32) - (PLAYABLE_AREA / 2.0);
        let enemy_transform : Transform = Transform::from_xyz(rand_start.x, rand_start.y, ENEMY_Z_ORDER);

        commands.spawn((
            EnemyTag,
            enemy_transform,
            Sprite {
                custom_size: Some(ENEMY_DEFAULT_PARAMS.size),
                image: asset_server.load(ENEMY_DEFAULT_PARAMS.sprite),
                color: ENEMY_DEFAULT_PARAMS.color,
                ..default()
            },
            Target {
                s: ENEMY_DEFAULT_PARAMS.speed,
                ..Default::default()
            },
        ));
    }
    
    // if NB_ENEMIES_PER_CALL <= 1_u64 { info!("enemy spawned"); } else { info!("{} enemies spawned", NB_ENEMIES_PER_CALL); }
    
}

// -------------------------------------------------------------------------------------------------------------------

#[derive(Component)]
pub struct Target
{
    pub t: Vec2,
    pub s: f32,
}
impl Default for Target 
{
    fn default() -> Self { return Self {t: Vec2::NAN, s: 0.0}; }     
}
impl Target {
    pub fn invalid(&self) -> bool { return self.t == Vec2::NAN; }
    pub fn set_invalid(&mut self) { self.t = Vec2::NAN; }
}

// -------------------------------------------------------------------------------------------------------------------

// TODO: on_player_moved, instead of on each tick
pub fn enemies_target_player(player: Single<&Transform, With<PlayerTag>>, enemies: Query<&mut Target, With<EnemyTag>>)
{
    for mut target in enemies
    {
        target.t = player.translation.xy();
    }
}

// -------------------------------------------------------------------------------------------------------------------

pub fn enemies_set_velocity_to_target(mut enemies : Query <(&Transform, &Target, &mut Velocity), With<EnemyTag>>)
{
    for (transform, target, mut velocity) in &mut enemies
    {
        if (!target.invalid()) && (target.s > PLAYABLE_DIST_EPSILON)
        {
            let new_velocity_dir = target.t - transform.translation.xy();
            if new_velocity_dir.norm_squared() > PLAYABLE_DIST_EPSILON
            {
                velocity.v = new_velocity_dir.normalize() * target.s;
            }
            else
            {
                // info!("target reached");
            }
        }
        else
        {
            
        }
    }
}

// -------------------------------------------------------------------------------------------------------------------
