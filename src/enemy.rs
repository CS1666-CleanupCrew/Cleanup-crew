use crate::collidable::{Collidable, Collider};
use crate::player::Player;
use crate::reaper::Reaper;
use bevy::prelude::*;
use bevy::scene::ron::de;

pub const ENEMY_SIZE: f32 = 32.;
pub const ENEMY_SPEED: f32 = 200.;
pub const ENEMY_ACCEL: f32 = 1800.;

use crate::map::EnemySpawnPoints;
use crate::room::{LevelState, RoomVec};
use crate::table;
use crate::{GameEntity, GameState};
use std::time::Duration;

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

#[derive(Component)]
pub struct MeleeEnemy;

#[derive(Component)]
pub struct RangedEnemy;

// Simple AI for a ranged enemy keeps some distance and periodically shoots
#[derive(Component)]
pub struct RangedEnemyAI {
    // Max distance at which it can shoot
    pub range: f32,
    // Time between shots
    pub fire_cooldown: Timer,
    // Speed to give projectiles when it shoots
    pub projectile_speed: f32,
}

#[derive(Component)]
pub struct RangedEnemyFrames {
    pub right: Vec<Handle<Image>>,
    pub left: Vec<Handle<Image>>,
    pub index: usize,
    pub facing_left: bool,
}

#[derive(Component, Deref, DerefMut)]
pub struct RangedAnimationTimer(pub Timer);

// Animation frames for the ranged enemy
#[derive(Resource)]
pub struct RangedEnemyRes {
    pub right_frames: Vec<Handle<Image>>,
    pub left_frames: Vec<Handle<Image>>,
}

// Event when a ranged enemy wants to shoot.
#[derive(Event)]
pub struct RangedEnemyShootEvent {
    pub origin: Vec3,
    pub direction: Vec2,
    pub speed: f32,
}

pub struct EnemyPlugin;
impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_enemy)
            .add_systems(Startup, load_ranged_enemy)
            .add_event::<RangedEnemyShootEvent>()
            // .add_systems(OnEnter(GameState::Playing), spawn_enemies_from_points)
            .add_systems(Update, animate_enemy.run_if(in_state(GameState::Playing)))
            .add_systems(
                Update,
                (
                    ranged_enemy_ai,
                    move_enemy.after(ranged_enemy_ai),
                    move_reaper_freely.after(ranged_enemy_ai),
                    collide_enemies_with_enemies.after(move_enemy),
                )
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                check_enemy_health.run_if(in_state(GameState::Playing)),
            )
            .add_systems(Update, animate_enemy_hit)
            .add_systems(Update, table_hits_enemy)
            .add_systems(
                Update,
                animate_ranged_enemy.run_if(in_state(GameState::Playing)),
            );
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
    commands.insert_resource(EnemyRes { frames, hit_frames });
}

fn load_ranged_enemy(mut commands: Commands, asset_server: Res<AssetServer>) {
    let right_frames: Vec<Handle<Image>> = vec![
        asset_server.load("ranger/ranger_mob_animation_1.png"),
        asset_server.load("ranger/ranger_mob_animation_1,5.png"),
        asset_server.load("ranger/ranger_mob_animation_2.png"),
        asset_server.load("ranger/ranger_mob_animation_3.png"),
    ];

    let left_frames: Vec<Handle<Image>> = vec![
        asset_server.load("ranger/ranger_mob_animation_1_left.png"),
        asset_server.load("ranger/ranger_mob_animation_1,5_left.png"),
        asset_server.load("ranger/ranger_mob_animation_2_left.png"),
        asset_server.load("ranger/ranger_mob_animation_3_left.png"),
    ];

    commands.insert_resource(RangedEnemyRes {
        right_frames,
        left_frames,
    });
}

// if enemy's hp = 0, then despawn
fn check_enemy_health(
    mut commands: Commands,
    enemy_query: Query<(Entity, &Health), With<Enemy>>,
    mut rooms: ResMut<RoomVec>,
    lvlstate: Res<LevelState>,
) {
    for (entity, health) in enemy_query.iter() {
        if health.0 <= 0.0 {
            if let LevelState::InRoom(index, _) = *lvlstate {
                rooms.0[index].numofenemies -= 1;
            }
            commands.entity(entity).despawn();
        }
    }
}

pub fn spawn_enemy_at(commands: &mut Commands, enemy_res: &EnemyRes, at: Vec3, active: bool, health_multiplier: f32) {
    let base_health = 50.0;
    let mut e = commands.spawn((
        Sprite::from_image(enemy_res.frames[0].clone()),
        Transform {
            translation: at,
            ..Default::default()
        },
        Enemy,
        Velocity::new(),
        Health::new(base_health * health_multiplier),
        AnimationTimer(Timer::from_seconds(ANIM_TIME, TimerMode::Repeating)),
        EnemyFrames {
            handles: enemy_res.frames.clone(),
            index: 0,
        },
        crate::fluiddynamics::PulledByFluid { mass: 10.0 },
        GameEntity,
        MeleeEnemy,
    ));
    if active {
        e.insert(ActiveEnemy);
    }
}

pub fn spawn_ranged_enemy_at(
    commands: &mut Commands,
    ranged_res: &RangedEnemyRes,
    at: Vec3,
    active: bool,
    health_multiplier: f32,
) {
    let base_health = 40.0;
    let mut e = commands.spawn((
        // start with facing right frame 0
        Sprite::from_image(ranged_res.right_frames[0].clone()),
        Transform {
            translation: at,
            ..Default::default()
        },
        Enemy,
        RangedEnemy,
        Velocity::new(),
        Health::new(base_health * health_multiplier),
        RangedAnimationTimer(Timer::from_seconds(ANIM_TIME, TimerMode::Repeating)),
        RangedEnemyFrames {
            right: ranged_res.right_frames.clone(),
            left: ranged_res.left_frames.clone(),
            index: 0,
            facing_left: false,
        },
        RangedEnemyAI {
            range: 400.0,
            fire_cooldown: Timer::from_seconds(1.5, TimerMode::Repeating),
            projectile_speed: 600.0,
        },
        crate::fluiddynamics::PulledByFluid { mass: 10.0 },
        GameEntity,
    ));

    if active {
        e.insert(ActiveEnemy);
    }
}

fn spawn_enemies_from_points(
    mut commands: Commands,
    enemy_res: Res<EnemyRes>,
    ranged_res: Res<RangedEnemyRes>,
    points: Res<EnemySpawnPoints>,
) {
    for (i, &p) in points.0.iter().enumerate() {
        if i % 3 == 0 {
            // every 3rd enemy is a ranger
            spawn_ranged_enemy_at(&mut commands, &ranged_res, p, true, 1.0);
        } else {
            // others are standard chasers
            spawn_enemy_at(&mut commands, &enemy_res, p, true, 1.0);
        }
    }
}

fn animate_enemy(
    time: Res<Time>,
    mut query: Query<
        (
            &mut Sprite,
            &mut AnimationTimer,
            &mut EnemyFrames,
            &Velocity,
        ),
        (With<Enemy>, With<ActiveEnemy>),
    >,
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

fn animate_ranged_enemy(
    time: Res<Time>,
    mut query: Query<
        (
            &mut Sprite,
            &mut RangedAnimationTimer,
            &mut RangedEnemyFrames,
        ),
        With<RangedEnemy>,
    >,
) {
    for (mut sprite, mut timer, mut frames) in &mut query {
        timer.tick(time.delta());

        // Prepare the next frame image while only holding an immutable borrow of `frames`.
        // This keeps the immutable borrow from overlapping with the later mutable borrow.
        let mut new_image: Option<Handle<Image>> = None;
        let new_index = {
            let frame_list = if frames.facing_left {
                &frames.left
            } else {
                &frames.right
            };

            if timer.just_finished() && !frame_list.is_empty() {
                let idx = (frames.index + 1) % frame_list.len();
                new_image = Some(frame_list[idx].clone());
                idx
            } else {
                frames.index
            }
        };

        // Immutable borrow ended; now it's safe to mutate `frames`.
        if let Some(img) = new_image {
            frames.index = new_index;
            sprite.image = img;
        }
    }
}

pub fn animate_enemy_hit(
    time: Res<Time>,
    mut commands: Commands,
    mut enemies: Query<
        (Entity, &mut Sprite, &mut HitAnimation),
        (Without<RangedEnemy>, Without<Reaper>),
    >,
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
    mut enemy_query: Query<
        (
            &mut Transform,
            &mut Velocity,
            Option<&crate::fluiddynamics::PulledByFluid>,
            Option<&RangedEnemy>,
        ),
        (
            With<Enemy>,
            With<ActiveEnemy>,
            Without<crate::reaper::Reaper>,
        ),
    >,
    wall_query: Query<(&Transform, &Collider), (With<Collidable>, Without<Enemy>, Without<Player>)>,
    grid_query: Query<&crate::fluiddynamics::FluidGrid>,
) {
    let grid_has_breach = if let Ok(grid) = grid_query.single() {
        !grid.breaches.is_empty()
    } else {
        false
    };

    if let Ok(player_transform) = player_query.single() {
        let deltat = time.delta_secs();
        let accel = ENEMY_ACCEL * deltat;

        for (mut enemy_transform, mut enemy_velocity, _pulled_opt, ranged_opt) in &mut enemy_query {
            let mut effective_accel = accel;

            if grid_has_breach {
                effective_accel *= 0.15;
            }

            // Steering:
            // Chasers: compute velocity toward player.
            // Rangers: skip steering, their velocity comes from ranged_enemy_ai.
            if ranged_opt.is_none() {
                let dir_to_player = (player_transform.translation - enemy_transform.translation)
                    .truncate()
                    .normalize_or_zero();

                if dir_to_player.length() > 0.0 {
                    **enemy_velocity = (**enemy_velocity + dir_to_player * effective_accel)
                        .clamp_length_max(ENEMY_SPEED);
                } else if enemy_velocity.length() > effective_accel {
                    let vel = **enemy_velocity;
                    **enemy_velocity += vel.normalize_or_zero() * -effective_accel;
                } else {
                    **enemy_velocity = Vec2::ZERO;
                }
            }

            let change = **enemy_velocity * deltat;
            let mut pos = enemy_transform.translation;
            let enemy_half = Vec2::splat(ENEMY_SIZE * 0.5);

            // X axis
            if change.x != 0.0 {
                let mut nx = pos.x + change.x;
                let px = nx;
                let py = pos.y;
                for (wall_tf, wall_collider) in &wall_query {
                    let (wx, wy) = (wall_tf.translation.x, wall_tf.translation.y);
                    if crate::player::aabb_overlap(
                        px,
                        py,
                        enemy_half,
                        wx,
                        wy,
                        wall_collider.half_extents,
                    ) {
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

            // Y axis
            if change.y != 0.0 {
                let mut ny = pos.y + change.y;
                let px = pos.x;
                let py = ny;
                for (wall_tf, wall_collider) in &wall_query {
                    let (wx, wy) = (wall_tf.translation.x, wall_tf.translation.y);
                    if crate::player::aabb_overlap(
                        px,
                        py,
                        enemy_half,
                        wx,
                        wy,
                        wall_collider.half_extents,
                    ) {
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

fn move_reaper_freely(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Velocity), With<crate::reaper::Reaper>>,
) {
    let dt = time.delta_secs();
    for (mut tf, vel) in &mut query {
        tf.translation += (vel.velocity * dt).extend(0.0);
    }
}

// collide enemies with each other
fn collide_enemies_with_enemies(
    mut enemy_query: Query<&mut Transform, (With<Enemy>, With<ActiveEnemy>)>,
) {
    let enemy_half = Vec2::splat(ENEMY_SIZE * 0.5);
    let max_interaction_dist = ENEMY_SIZE * 4.0; // or something
    let max_interaction_dist2 = max_interaction_dist * max_interaction_dist;

    // get all combinations of 2 enemies
    let mut combinations = enemy_query.iter_combinations_mut();
    while let Some([mut e1_transform, mut e2_transform]) = combinations.fetch_next() {
        let (p1, h1) = (e1_transform.translation.truncate(), enemy_half);
        let (p2, h2) = (e2_transform.translation.truncate(), enemy_half);
        let d2 = (p1 - p2).length_squared();
        if d2 > max_interaction_dist2 {
            continue;
        }

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

fn table_hits_enemy(
    _time: Res<Time>,
    mut enemy_query: Query<
        (&Transform, &mut Health),
        (With<Enemy>, Without<crate::reaper::Reaper>),
    >,
    table_query: Query<
        (&Transform, &Collider, Option<&crate::enemy::Velocity>),
        With<table::Table>,
    >,
) {
    let enemy_half = Vec2::splat(ENEMY_SIZE * 0.5);

    for (enemy_tf, mut health) in &mut enemy_query {
        let enemy_pos = enemy_tf.translation.truncate();

        for (table_tf, table_col, vel_opt) in &table_query {
            let table_pos = table_tf.translation.truncate();

            // Small hitbox expansion
            let extra = Vec2::new(5.0, 5.0);
            let table_half = table_col.half_extents + extra;

            if crate::player::aabb_overlap(
                enemy_pos.x,
                enemy_pos.y,
                enemy_half,
                table_pos.x,
                table_pos.y,
                table_half,
            ) {
                let speed = vel_opt.map(|v| v.velocity.length()).unwrap_or(0.0);

                let threshold = 5.0;
                if speed > threshold {
                    let dmg = speed * 0.02;
                    health.0 -= dmg;
                }
            }
        }
    }
}

fn ranged_enemy_ai(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemies: Query<(&Transform, &mut Velocity, &mut RangedEnemyAI), With<RangedEnemy>>,
    mut shoot_writer: EventWriter<RangedEnemyShootEvent>,
    lvlstate: Res<LevelState>,
) {
    let Ok(player_tf) = player_query.single() else {
        return;
    };
    let player_pos = player_tf.translation.truncate();

    // Difficulty scaling per room as f32
    let difficulty_mult: f32 = match *lvlstate {
        LevelState::InRoom(idx, _) => 1.0 + (idx as f32 * 0.10),
        LevelState::EnteredRoom(idx) => 1.0 + (idx as f32 * 0.10),
        LevelState::NotRoom => 1.0,
    };

    for (enemy_tf, mut vel, mut ai) in &mut enemies {
        // scale cooldown tick by difficulty multiplier (faster in deeper rooms)
        let scaled_dt = time.delta_secs() * difficulty_mult;
        ai.fire_cooldown.tick(Duration::from_secs_f32(scaled_dt));

        let enemy_pos = enemy_tf.translation.truncate();
        let diff = player_pos - enemy_pos;
        let dist = diff.length();
        if dist == 0.0 {
            continue;
        }

        let dir = diff / dist;

        // hover around some distance
        let desired = ai.range * 0.75;
        let delta = dist - desired;
        let move_dir = if delta > 20.0 {
            dir
        } else if delta < -20.0 {
            -dir
        } else {
            Vec2::ZERO
        };

        let accel = ENEMY_ACCEL * time.delta_secs();
        vel.velocity = (vel.velocity + move_dir * accel).clamp_length_max(ENEMY_SPEED);

        // shoot if in range + cooldown finished
        if ai.fire_cooldown.finished() && dist <= ai.range {
            shoot_writer.write(RangedEnemyShootEvent {
                origin: enemy_tf.translation,
                direction: dir,
                speed: ai.projectile_speed,
            });
            ai.fire_cooldown.reset();
        }
    }
}