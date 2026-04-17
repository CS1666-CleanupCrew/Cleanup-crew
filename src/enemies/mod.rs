pub mod chaser;
pub mod ranger;
pub mod reaper;

// Re-export sub-module items so callers can keep using `enemies::X`
// without needing to know which sub-module it lives in.
pub use chaser::{
    AnimationTimer, EnemyFrames, EnemyRes, HitAnimation, MeleeEnemy,
    spawn_enemy_at,
};
pub use ranger::{
    RangedAnimationTimer, RangedEnemy, RangedEnemyAI, RangedEnemyFrames,
    RangedEnemyRes, RangedEnemyShootEvent, spawn_ranged_enemy_at,
};
pub use reaper::Reaper;

use bevy::prelude::*;
use crate::{GameState};
use crate::collidable::{Collider};
use crate::player::Player;
use crate::room::{LevelState, RoomVec};
use crate::table;

// Shared constants

pub const ENEMY_SIZE: f32 = 32.0;
pub const ENEMY_SPEED: f32 = 200.0;
pub const ENEMY_ACCEL: f32 = 1800.0;
pub(super) const ANIM_TIME: f32 = 0.2;

// Shared components

/// Per-entity top speed, set at spawn from base + rooms-cleared bonus.
/// Systems fall back to ENEMY_SPEED when this component is absent.
#[derive(Component)]
pub struct EnemyMoveSpeed(pub f32);

#[derive(Component)]
pub struct Enemy;

#[derive(Component, Deref, DerefMut)]
pub struct Velocity {
    pub velocity: Vec2,
}

impl Velocity {
    pub fn new() -> Self {
        Self { velocity: Vec2::ZERO }
    }
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

#[derive(Resource, Default)]
pub struct LastKillPos(pub Vec2);

// Plugin

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LastKillPos>()
            .add_systems(Startup, chaser::load)
            .add_systems(Startup, ranger::load)
            .add_event::<RangedEnemyShootEvent>()
            .add_systems(Update, chaser::animate.run_if(in_state(GameState::Playing)))
            .add_systems(
                Update,
                (
                    ranger::ai,
                    move_enemy.after(ranger::ai),
                    move_reaper_freely.after(ranger::ai),
                    collide_enemies_with_enemies.after(move_enemy),
                    wall_correction_for_enemies.after(collide_enemies_with_enemies),
                )
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(Update, check_enemy_health.run_if(in_state(GameState::Playing)))
            .add_systems(Update, chaser::animate_hit)
            .add_systems(Update, table_hits_enemy)
            .add_systems(Update, ranger::animate.run_if(in_state(GameState::Playing)));
    }
}

// Shared systems

fn check_enemy_health(
    mut commands: Commands,
    enemy_query: Query<(Entity, &Health, &Transform), With<Enemy>>,
    mut rooms: ResMut<RoomVec>,
    lvlstate: Res<LevelState>,
    mut last_kill_pos: ResMut<LastKillPos>,
) {
    for (entity, health, transform) in enemy_query.iter() {
        if health.0 <= 0.0 {
            if let LevelState::InRoom(index, _) = *lvlstate {
                rooms.0[index].numofenemies -= 1;
            }
            last_kill_pos.0 = transform.translation.truncate();
            commands.entity(entity).despawn();
        }
    }
}

fn move_enemy(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemy_query: Query<
        (
            &mut Transform,
            &mut Velocity,
            Option<&crate::fluiddynamics::PulledByFluid>,
            Option<&ranger::RangedEnemy>,
            Option<&EnemyMoveSpeed>,
        ),
        (With<Enemy>, With<ActiveEnemy>, Without<Reaper>),
    >,
    wall_grid: Res<crate::map::WallGrid>,
    grid_query: Query<&crate::fluiddynamics::FluidGrid>,
) {
    let grid_has_breach = if let Ok(grid) = grid_query.single() {
        !grid.breaches.is_empty()
    } else {
        false
    };

    let Ok(player_transform) = player_query.single() else { return };
    let deltat = time.delta_secs();
    let accel = ENEMY_ACCEL * deltat;
    let enemy_half = Vec2::splat(ENEMY_SIZE * 0.5);

    for (mut enemy_transform, mut enemy_velocity, _pulled_opt, ranged_opt, spd_opt) in &mut enemy_query {
        let max_speed = spd_opt.map_or(ENEMY_SPEED, |s| s.0);
        let mut effective_accel = accel;
        if grid_has_breach {
            effective_accel *= 0.15;
        }

        // Chasers steer toward the player; rangers get their velocity from ranger::ai.
        if ranged_opt.is_none() {
            let dir_to_player = (player_transform.translation - enemy_transform.translation)
                .truncate()
                .normalize_or_zero();

            if dir_to_player.length() > 0.0 {
                **enemy_velocity =
                    (**enemy_velocity + dir_to_player * effective_accel).clamp_length_max(max_speed);
            } else if enemy_velocity.length() > effective_accel {
                let vel = **enemy_velocity;
                **enemy_velocity += vel.normalize_or_zero() * -effective_accel;
            } else {
                **enemy_velocity = Vec2::ZERO;
            }
        }

        let change = **enemy_velocity * deltat;
        let mut pos = enemy_transform.translation;

        if change.x != 0.0 {
            let mut nx = pos.x + change.x;
            for (wall_pos, wall_half) in wall_grid.nearby(Vec2::new(nx, pos.y), 3) {
                if crate::player::aabb_overlap(nx, pos.y, enemy_half, wall_pos.x, wall_pos.y, wall_half) {
                    nx = if change.x > 0.0 {
                        wall_pos.x - (enemy_half.x + wall_half.x)
                    } else {
                        wall_pos.x + (enemy_half.x + wall_half.x)
                    };
                    enemy_velocity.velocity.x = 0.0;
                }
            }
            pos.x = nx;
        }

        if change.y != 0.0 {
            let mut ny = pos.y + change.y;
            for (wall_pos, wall_half) in wall_grid.nearby(Vec2::new(pos.x, ny), 3) {
                if crate::player::aabb_overlap(pos.x, ny, enemy_half, wall_pos.x, wall_pos.y, wall_half) {
                    ny = if change.y > 0.0 {
                        wall_pos.y - (enemy_half.y + wall_half.y)
                    } else {
                        wall_pos.y + (enemy_half.y + wall_half.y)
                    };
                    enemy_velocity.velocity.y = 0.0;
                }
            }
            pos.y = ny;
        }

        enemy_transform.translation = pos;
    }
}

fn move_reaper_freely(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Velocity), With<Reaper>>,
) {
    let dt = time.delta_secs();
    for (mut tf, vel) in &mut query {
        tf.translation += (vel.velocity * dt).extend(0.0);
    }
}


fn wall_correction_for_enemies(
    mut enemy_query: Query<&mut Transform, (With<Enemy>, With<ActiveEnemy>)>,
    wall_grid: Res<crate::map::WallGrid>,
) {
    let enemy_half = Vec2::splat(ENEMY_SIZE * 0.5);

    for mut enemy_tf in &mut enemy_query {
        let mut pos = enemy_tf.translation.truncate();
        for (wall_pos, wall_half) in wall_grid.nearby(pos, 3) {
            if crate::player::aabb_overlap(pos.x, pos.y, enemy_half, wall_pos.x, wall_pos.y, wall_half) {
                let overlap_x = (enemy_half.x + wall_half.x) - (pos.x - wall_pos.x).abs();
                let overlap_y = (enemy_half.y + wall_half.y) - (pos.y - wall_pos.y).abs();
                if overlap_x < overlap_y {
                    pos.x += if pos.x > wall_pos.x { overlap_x } else { -overlap_x };
                } else {
                    pos.y += if pos.y > wall_pos.y { overlap_y } else { -overlap_y };
                }
            }
        }
        enemy_tf.translation.x = pos.x;
        enemy_tf.translation.y = pos.y;
    }
}

fn collide_enemies_with_enemies(
    mut enemy_query: Query<&mut Transform, (With<Enemy>, With<ActiveEnemy>)>,
) {
    let enemy_half = Vec2::splat(ENEMY_SIZE * 0.5);
    let max_dist2 = (ENEMY_SIZE * 4.0) * (ENEMY_SIZE * 4.0);

    let mut combinations = enemy_query.iter_combinations_mut();
    while let Some([mut e1, mut e2]) = combinations.fetch_next() {
        let (p1, p2) = (e1.translation.truncate(), e2.translation.truncate());
        if (p1 - p2).length_squared() > max_dist2 { continue; }

        if crate::player::aabb_overlap(p1.x, p1.y, enemy_half, p2.x, p2.y, enemy_half) {
            let overlap_x = (enemy_half.x * 2.0) - (p1.x - p2.x).abs();
            let overlap_y = (enemy_half.y * 2.0) - (p1.y - p2.y).abs();
            if overlap_x < overlap_y {
                let sign = if p1.x > p2.x { 1.0 } else { -1.0 };
                let push = sign * overlap_x * 0.5;
                e1.translation.x += push;
                e2.translation.x -= push;
            } else {
                let sign = if p1.y > p2.y { 1.0 } else { -1.0 };
                let push = sign * overlap_y * 0.5;
                e1.translation.y += push;
                e2.translation.y -= push;
            }
        }
    }
}

fn table_hits_enemy(
    mut enemy_query: Query<
        (&Transform, &mut Health),
        (With<Enemy>, Without<Reaper>),
    >,
    table_query: Query<
        (&Transform, &Collider, Option<&Velocity>, &table::TableRoom),
        With<table::Table>,
    >,
    active_room: Res<table::ActiveRoom>,
) {
    let Some(active) = active_room.0 else { return; };
    let enemy_half = Vec2::splat(ENEMY_SIZE * 0.5);

    for (enemy_tf, mut health) in &mut enemy_query {
        let enemy_pos = enemy_tf.translation.truncate();
        for (table_tf, table_col, vel_opt, room) in &table_query {
            if room.0 != active { continue; }
            let table_pos = table_tf.translation.truncate();
            let table_half = table_col.half_extents + Vec2::new(5.0, 5.0);

            if crate::player::aabb_overlap(
                enemy_pos.x, enemy_pos.y, enemy_half,
                table_pos.x, table_pos.y, table_half,
            ) {
                let speed = vel_opt.map(|v| v.velocity.length()).unwrap_or(0.0);
                if speed > 5.0 {
                    health.0 -= speed * 0.02;
                }
            }
        }
    }
}
