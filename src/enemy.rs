use bevy::prelude::*;
use crate::player::Player;
use crate::collidable::{Collidable, Collider};

pub const ENEMY_SIZE: f32 = 32.;
pub const ENEMY_SPEED: f32 = 200.;
pub const ENEMY_ACCEL: f32 = 1800.;

use crate::map::EnemySpawnPoints;
use crate::GameState;

const ANIM_TIME: f32 = 0.2;

#[derive(Component)]
pub struct Enemy;

#[derive(Component, Deref, DerefMut)]
pub struct Velocity {
    pub velocity: Vec2,
}

#[derive(Component)]
pub struct ActiveEnemy;

#[derive(Component)]
pub struct Health(pub f32);
impl Health {
    pub fn new(amount: f32) -> Self {
        Self(amount)
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(Timer);

#[derive(Component)]
pub struct EnemyFrames {
    handles: Vec<Handle<Image>>,
    index: usize,
}

#[derive(Component)]
pub struct HitAnimation {
    pub timer: Timer,
}

#[derive(Resource)]
pub struct EnemyRes {
    pub frames: Vec<Handle<Image>>,
    pub hit_frames: Vec<Handle<Image>>,
}

impl Velocity {
    pub fn new() -> Self {
        Self {
            velocity: Vec2::ZERO,
        }
    }
}

pub struct EnemyPlugin;
impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, load_enemy)
            .add_systems(OnEnter(GameState::Playing), spawn_enemies_from_points)
            .add_systems(Update, animate_enemy.run_if(in_state(GameState::Playing)))
            .add_systems(Update, (move_enemy, collide_enemies_with_enemies.after(move_enemy)).run_if(in_state(GameState::Playing)))
            .add_systems(Update, check_enemy_health.run_if(in_state(GameState::Playing)))
            .add_systems(Update, animate_enemy_hit);
    }
}

fn load_enemy(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Load 3 separate frames
    let frames: Vec<Handle<Image>> = vec![
        asset_server.load("chaser/chaser_mob_animation1.png"),
        asset_server.load("chaser/chaser_mob_animation2.png"),
        asset_server.load("chaser/chaser_mob_animation3.png"),
        asset_server.load("chaser/chaser_mob_animation2.png"),
    ];
    
    let hit_frames: Vec<Handle<Image>> = vec![
    asset_server.load("chaser/chaser_mob_bite1.png"),
    asset_server.load("chaser/chaser_mob_bite2.png"),
    ];
    commands.insert_resource(EnemyRes{
        frames,
        hit_frames,
    });

}

//if enemy's hp = 0, then despawn
fn check_enemy_health(
    mut commands: Commands,
    enemy_query: Query<(Entity, &Health), With<Enemy>>,
) {
    for (entity, health) in enemy_query.iter() {
        if health.0 <= 0.0 {
            
            commands.entity(entity).despawn();
        }
    }
}

pub fn spawn_enemy_at(
    commands: &mut Commands,
    enemy_res: &EnemyRes,
    at: Vec3,
    active: bool,
) {
    let mut e = commands.spawn((
        Sprite::from_image(enemy_res.frames[0].clone()),
        Transform { translation: at, ..Default::default() },
        Enemy,
        Velocity::new(),
        Health::new(50.0),
        AnimationTimer(Timer::from_seconds(ANIM_TIME, TimerMode::Repeating)),
        EnemyFrames {
            handles: enemy_res.frames.clone(),
            index: 0,
        },
        crate::fluiddynamics::PulledByFluid { mass: 50.0 },
    ));
    if active { e.insert(ActiveEnemy); }
}

fn spawn_enemies_from_points(
    mut commands: Commands,
    enemy_res: Res<EnemyRes>,
    points: Res<EnemySpawnPoints>,
) {
    for (i, &p) in points.0.iter().enumerate(){
        spawn_enemy_at(&mut commands, &enemy_res, p, true); // active now
    }
}

fn animate_enemy(
    time: Res<Time>,
    mut query: Query<(&mut Sprite, &mut AnimationTimer, &mut EnemyFrames, &Velocity), (With<Enemy>, With<ActiveEnemy>)>,
) {
    for (mut sprite, mut timer, mut frames, velocity) in &mut query {
        timer.tick(time.delta());

        if timer.just_finished() {
            frames.index = (frames.index + 1) % frames.handles.len();
            sprite.image = frames.handles[frames.index].clone();
        }

        // Flip the sprite based on the x velocity
        if velocity.x > 0. {
            sprite.flip_x = true;
        } else if velocity.x < 0. {
            sprite.flip_x = false;
        }
    }
}

pub fn animate_enemy_hit(
    time: Res<Time>,
    mut commands: Commands,
    mut enemies: Query<(Entity, &mut Sprite, &mut HitAnimation)>,
    enemy_res: Res<EnemyRes>,
) {
    for (entity, mut sprite, mut hit) in &mut enemies {
        hit.timer.tick(time.delta());

        if hit.timer.elapsed_secs() < 1.0 {
            sprite.image = enemy_res.hit_frames[0].clone();
        } else {
            sprite.image = enemy_res.hit_frames[1].clone();
        }

        if hit.timer.finished() {
            commands.entity(entity).remove::<HitAnimation>();
            sprite.image = enemy_res.frames[0].clone();
        }
    }    
}

// moves the enemy towards the player
fn move_enemy(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemy_query: Query<(&mut Transform, &mut Velocity), (With<Enemy>, With<ActiveEnemy>)>,
    wall_query: Query<(&Transform, &Collider), (With<Collidable>, Without<Enemy>, Without<Player>)>,
) {
    if let Ok(player_transform) = player_query.single() {
        let deltat = time.delta_secs();
        let accel = ENEMY_ACCEL * deltat;

        // Iterate over each enemy
        for (mut enemy_transform, mut enemy_velocity) in &mut enemy_query {
            // calculate the direction from the enemy to the player
            let dir_to_player = (player_transform.translation - enemy_transform.translation)
                                .truncate()
                                .normalize_or_zero();

            // using players acceleration to change enemy velocity
            **enemy_velocity = if dir_to_player.length() > 0. {
                (**enemy_velocity + (dir_to_player * accel)).clamp_length_max(ENEMY_SPEED)
            } else if enemy_velocity.length() > accel {
                **enemy_velocity + (enemy_velocity.normalize_or_zero() * -accel)
            } else {
                Vec2::ZERO
            };

            let change = **enemy_velocity * deltat;
            let mut pos = enemy_transform.translation;
            let enemy_half = Vec2::splat(ENEMY_SIZE * 0.5);

            // ---- X axis ----
            if change.x != 0.0 {
                let mut nx = pos.x + change.x;
                let px = nx;
                let py = pos.y;
                for (wall_tf, wall_collider) in &wall_query {
                    let (wx, wy) = (wall_tf.translation.x, wall_tf.translation.y);
                    if crate::player::aabb_overlap(px, py, enemy_half, wx, wy, wall_collider.half_extents) {
                        if change.x > 0.0 {
                            nx = wx - (enemy_half.x + wall_collider.half_extents.x);
                        } else {
                            nx = wx + (enemy_half.x + wall_collider.half_extents.x);
                        }
                        enemy_velocity.velocity.x = 0.0;
                    }
                }
                pos.x = nx;
            }

            // ---- Y axis ----
            if change.y != 0.0 {
                let mut ny = pos.y + change.y;
                let px = pos.x;
                let py = ny;
                for (wall_tf, wall_collider) in &wall_query {
                    let (wx, wy) = (wall_tf.translation.x, wall_tf.translation.y);
                    if crate::player::aabb_overlap(px, py, enemy_half, wx, wy, wall_collider.half_extents) {
                        if change.y > 0.0 {
                            ny = wy - (enemy_half.y + wall_collider.half_extents.y);
                        } else {
                            ny = wy + (enemy_half.y + wall_collider.half_extents.y);
                        }
                        enemy_velocity.velocity.y = 0.0;
                    }
                }
                pos.y = ny;
            }

            enemy_transform.translation = pos;
        }
    }
}

//collide enemies with each other
fn collide_enemies_with_enemies(
    mut enemy_query: Query<&mut Transform, (With<Enemy>, With<ActiveEnemy>)>,
) {
    let enemy_half = Vec2::splat(ENEMY_SIZE * 0.5);

    // get all combinations of 2 enemies
    let mut combinations = enemy_query.iter_combinations_mut();
    while let Some([(mut e1_transform), (mut e2_transform)]) =
        combinations.fetch_next()
    {
        let (p1, h1) = (e1_transform.translation.truncate(), enemy_half);
        let (p2, h2) = (e2_transform.translation.truncate(), enemy_half);

        // check if they overlap
        if crate::player::aabb_overlap(p1.x, p1.y, h1, p2.x, p2.y, h2) {

            let overlap_x = (h1.x + h2.x) - (p1.x - p2.x).abs();
            let overlap_y = (h1.y + h2.y) - (p1.y - p2.y).abs();

            if overlap_x < overlap_y {
                let sign = if p1.x > p2.x { 1.0 } else { -1.0 };
                let push = sign * overlap_x * 0.5; 
                e1_transform.translation.x += push;
                e2_transform.translation.x -= push;
            } else {
                let sign = if p1.y > p2.y { 1.0 } else { -1.0 };
                let push = sign * overlap_y * 0.5; 
                e1_transform.translation.y += push;
                e2_transform.translation.y -= push;
            }
        }
    }
}




