pub mod chaser;
pub mod ranger;
pub mod reaper;
pub mod turret;

// Re-export sub-module items so callers can keep using `enemies::X`
// without needing to know which sub-module it lives in.
pub use chaser::{
    AnimationTimer, EnemyFrames, EnemyRes, HitAnimation, MeleeEnemy,
    spawn_enemy_at,
};
pub use ranger::{
    RangedAnimationTimer, RangedEnemy, RangedEnemyAI, RangedEnemyFrames,
    RangedEnemyRes, RangerShootEvent, spawn_ranged_enemy_at,
};
pub use reaper::Reaper;
pub use turret::{TurretEnemy, TurretRes, TurretShootEvent, spawn_turret_enemy_at};

use bevy::prelude::*;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Reverse;
use crate::{GameState, TILE_SIZE};
use crate::collidable::{Collider, Collidable};
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

#[derive(Component)]
pub struct MaxHealth(pub f32);

/// Marker on the foreground fill sprite of an enemy's world-space health bar.
#[derive(Component)]
pub struct EnemyHealthBarFg;

const BAR_WIDTH: f32 = 34.0;
const BAR_HEIGHT: f32 = 4.0;
const BAR_Y_OFFSET: f32 = 62.0;

/// Spawns the two bar sprites (background + fill) as children of an enemy entity.
pub fn spawn_health_bar_children(parent: &mut ChildSpawnerCommands) {
    // Background
    parent.spawn((
        Sprite {
            color: Color::srgba(0.1, 0.0, 0.0, 0.85),
            custom_size: Some(Vec2::new(BAR_WIDTH, BAR_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(0.0, BAR_Y_OFFSET, 1.0),
    ));
    // Fill (starts full width, updated each frame)
    parent.spawn((
        Sprite {
            color: Color::srgb(0.1, 0.9, 0.1),
            custom_size: Some(Vec2::new(BAR_WIDTH, BAR_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(0.0, BAR_Y_OFFSET, 2.0),
        EnemyHealthBarFg,
    ));
}

#[derive(Resource, Default)]
pub struct LastKillPos(pub Vec2);

// Pathfinding

const PATH_RECOMPUTE_SECS: f32 = 0.5;
const PATH_MAX_NODES: usize = 150;
const PATH_SEARCH_PAD: i32 = 16;
/// Only run pathfinding for enemies within this world-space distance of the player.
const PATH_MAX_DIST: f32 = 900.0;

/// Cached set of table-occupied tiles, rebuilt every ~0.3 s so `compute_enemy_paths`
/// doesn't allocate a new HashSet every frame.
#[derive(Resource)]
pub struct TableBlockedTiles {
    pub tiles: HashSet<(i32, i32)>,
    timer: Timer,
}

impl Default for TableBlockedTiles {
    fn default() -> Self {
        Self {
            tiles: HashSet::new(),
            timer: Timer::from_seconds(0.3, TimerMode::Repeating),
        }
    }
}

fn update_table_blocked_tiles(
    time: Res<Time>,
    mut cache: ResMut<TableBlockedTiles>,
    table_q: Query<&Transform, (With<table::Table>, With<Collidable>)>,
    wall_grid: Res<crate::map::WallGrid>,
) {
    cache.timer.tick(time.delta());
    if !cache.timer.just_finished() { return; }
    cache.tiles = table_q
        .iter()
        .map(|tf| wall_grid.world_to_tile(tf.translation.truncate()))
        .collect();
}

/// A* pathfinder attached to every non-reaper enemy.
/// Stores the current list of world-space waypoints to follow when the
/// direct path to the player is blocked by a wall or table.
#[derive(Component)]
pub struct EnemyPathfinder {
    pub waypoints: Vec<Vec2>,
    timer: Timer,
}

impl EnemyPathfinder {
    pub fn new() -> Self {
        // Stagger recompute timers so enemies don't all run A* the same frame.
        let offset = rand::random::<f32>() * PATH_RECOMPUTE_SECS;
        let mut timer = Timer::from_seconds(PATH_RECOMPUTE_SECS, TimerMode::Repeating);
        timer.set_elapsed(std::time::Duration::from_secs_f32(offset));
        Self { waypoints: Vec::new(), timer }
    }
}

// Plugin

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LastKillPos>()
            .init_resource::<TableBlockedTiles>()
            .add_systems(Startup, chaser::load)
            .add_systems(Startup, ranger::load)
            .add_systems(Startup, turret::load)
            .add_event::<RangerShootEvent>()
            .add_event::<TurretShootEvent>()
            .add_systems(Update, chaser::animate.run_if(in_state(GameState::Playing)))
            .add_systems(
                Update,
                (
                    update_table_blocked_tiles,
                    compute_enemy_paths.after(update_table_blocked_tiles),
                    ranger::ai.after(compute_enemy_paths),
                    turret::ai.after(compute_enemy_paths),
                    ranger::spawn_ranger_bullets.after(ranger::ai),
                    turret::spawn_turret_bullets.after(turret::ai),
                    move_enemy.after(ranger::ai).after(turret::ai),
                    move_reaper_freely.after(ranger::ai),
                    collide_enemies_with_enemies.after(move_enemy),
                    wall_correction_for_enemies.after(collide_enemies_with_enemies),
                    enemies_collide_with_tables.after(wall_correction_for_enemies),
                )
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(Update, kill_enemies_outside_station.run_if(in_state(GameState::Playing)))
            .add_systems(Update, check_enemy_health.run_if(in_state(GameState::Playing)))
        .add_systems(Update, update_enemy_health_bars.run_if(in_state(GameState::Playing)))
            .add_systems(Update, chaser::animate_hit)
            .add_systems(Update, table_hits_enemy)
            .add_systems(Update, ranger::animate.run_if(in_state(GameState::Playing)))
            .add_systems(Update, turret::animate.run_if(in_state(GameState::Playing)));
    }
}

// Shared systems

fn update_enemy_health_bars(
    enemy_q: Query<(&Health, &MaxHealth, &Children), With<Enemy>>,
    mut fg_q: Query<(&mut Sprite, &mut Transform), With<EnemyHealthBarFg>>,
) {
    for (health, max_health, children) in &enemy_q {
        let ratio = (health.0 / max_health.0).clamp(0.0, 1.0);
        let fill_w = ratio * BAR_WIDTH;
        // Anchor left edge: shift left by half the missing width
        let fill_x = -(1.0 - ratio) * BAR_WIDTH * 0.5;
        let r = (1.0 - ratio).min(1.0);
        let g = ratio.min(1.0);

        for child in children.iter() {
            if let Ok((mut sprite, mut tf)) = fg_q.get_mut(child) {
                sprite.custom_size = Some(Vec2::new(fill_w.max(0.0), BAR_HEIGHT));
                sprite.color = Color::srgb(r, g, 0.0);
                tf.translation.x = fill_x;
            }
        }
    }
}

fn kill_enemies_outside_station(
    grid_meta: Res<crate::map::MapGridMeta>,
    mut enemy_query: Query<(&Transform, &mut Health), (With<Enemy>, Without<Reaper>)>,
) {
    let tile = crate::TILE_SIZE;
    let x_min = grid_meta.x0 - tile * 0.5;
    let x_max = grid_meta.x0 + grid_meta.cols as f32 * tile - tile * 0.5;
    let y_min = grid_meta.y0 - tile * 0.5;
    let y_max = grid_meta.y0 + grid_meta.rows as f32 * tile - tile * 0.5;

    for (tf, mut hp) in &mut enemy_query {
        let p = tf.translation;
        if p.x < x_min || p.x > x_max || p.y < y_min || p.y > y_max {
            hp.0 = 0.0;
        }
    }
}

// ── Pathfinding ────────────────────────────────────────────────────────────

/// Bresenham line-of-sight check on the tile grid.
/// Returns true if the straight line from `from` to `to` passes through no wall or blocked tile.
fn has_los(
    from: (i32, i32),
    to: (i32, i32),
    wall_grid: &crate::map::WallGrid,
    blocked: &HashSet<(i32, i32)>,
) -> bool {
    let (mut x, mut y) = from;
    let (tx, ty) = to;
    let dx = (tx - x).abs();
    let dy = (ty - y).abs();
    let sx = if x < tx { 1i32 } else { -1 };
    let sy = if y < ty { 1i32 } else { -1 };
    let mut err = dx - dy;
    loop {
        if (x, y) != from && (x, y) != to {
            if wall_grid.is_wall_tile(x, y) || blocked.contains(&(x, y)) {
                return false;
            }
        }
        if x == tx && y == ty { break; }
        let e2 = 2 * err;
        if e2 > -dy { err -= dy; x += sx; }
        if e2 < dx { err += dx; y += sy; }
    }
    true
}

/// A* pathfinder on the tile grid with 8-directional movement.
/// Returns a list of tile (col, row) pairs from the step after `start` to `goal`.
/// Returns empty vec if no path is found within `max_nodes` expansions.
fn a_star(
    start: (i32, i32),
    goal: (i32, i32),
    wall_grid: &crate::map::WallGrid,
    blocked: &HashSet<(i32, i32)>,
    max_nodes: usize,
    padding: i32,
) -> Vec<(i32, i32)> {
    if start == goal { return Vec::new(); }

    let min_c = start.0.min(goal.0) - padding;
    let max_c = start.0.max(goal.0) + padding;
    let min_r = start.1.min(goal.1) - padding;
    let max_r = start.1.max(goal.1) + padding;

    let h = |(c, r): (i32, i32)| -> i32 {
        10 * ((c - goal.0).abs() + (r - goal.1).abs())
    };

    let mut open: BinaryHeap<Reverse<(i32, (i32, i32))>> = BinaryHeap::new();
    let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
    let mut g: HashMap<(i32, i32), i32> = HashMap::new();
    let mut expanded = 0usize;

    g.insert(start, 0);
    open.push(Reverse((h(start), start)));

    const DIRS: [(i32, i32, i32); 8] = [
        (1, 0, 10), (-1, 0, 10), (0, 1, 10), (0, -1, 10),
        (1, 1, 14), (1, -1, 14), (-1, 1, 14), (-1, -1, 14),
    ];

    while let Some(Reverse((_, cur))) = open.pop() {
        if cur == goal {
            let mut path = Vec::new();
            let mut node = goal;
            while node != start {
                path.push(node);
                node = *came_from.get(&node).unwrap();
            }
            path.reverse();
            return path;
        }

        expanded += 1;
        if expanded >= max_nodes { break; }

        let cur_g = *g.get(&cur).unwrap_or(&i32::MAX);

        for &(dc, dr, cost) in &DIRS {
            let nb = (cur.0 + dc, cur.1 + dr);

            if nb.0 < min_c || nb.0 > max_c || nb.1 < min_r || nb.1 > max_r { continue; }
            if nb != goal && wall_grid.is_wall_tile(nb.0, nb.1) { continue; }
            if nb != goal && blocked.contains(&nb) { continue; }

            // Prevent cutting through wall corners diagonally.
            if dc != 0 && dr != 0 {
                if wall_grid.is_wall_tile(cur.0 + dc, cur.1) ||
                   wall_grid.is_wall_tile(cur.0, cur.1 + dr) { continue; }
            }

            let tg = cur_g.saturating_add(cost);
            if tg < *g.get(&nb).unwrap_or(&i32::MAX) {
                came_from.insert(nb, cur);
                g.insert(nb, tg);
                open.push(Reverse((tg + h(nb), nb)));
            }
        }
    }

    Vec::new()
}

/// Recomputes paths for enemies that have no line-of-sight to the player.
/// Also advances waypoints as the enemy moves through them.
fn compute_enemy_paths(
    time: Res<Time>,
    player_q: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemy_q: Query<
        (&Transform, &mut EnemyPathfinder),
        (With<Enemy>, With<ActiveEnemy>, Without<Reaper>),
    >,
    wall_grid: Res<crate::map::WallGrid>,
    blocked_cache: Res<TableBlockedTiles>,
) {
    let Ok(player_tf) = player_q.single() else { return };
    let player_pos = player_tf.translation.truncate();
    let goal = wall_grid.world_to_tile(player_pos);
    let blocked = &blocked_cache.tiles;

    for (enemy_tf, mut pathfinder) in &mut enemy_q {
        let pos = enemy_tf.translation.truncate();

        // Advance past waypoints the enemy has already reached.
        while let Some(&wp) = pathfinder.waypoints.first() {
            if pos.distance(wp) < TILE_SIZE * 0.8 {
                pathfinder.waypoints.remove(0);
            } else {
                break;
            }
        }

        pathfinder.timer.tick(time.delta());
        if !pathfinder.timer.just_finished() {
            continue;
        }

        // Skip A* for enemies too far away — they'll head straight until closer.
        if pos.distance_squared(player_pos) > PATH_MAX_DIST * PATH_MAX_DIST {
            pathfinder.waypoints.clear();
            continue;
        }

        let start = wall_grid.world_to_tile(pos);

        if has_los(start, goal, &wall_grid, blocked) {
            pathfinder.waypoints.clear();
        } else {
            let tiles = a_star(start, goal, &wall_grid, blocked, PATH_MAX_NODES, PATH_SEARCH_PAD);
            pathfinder.waypoints = tiles
                .into_iter()
                .map(|(c, r)| wall_grid.tile_to_world(c, r))
                .collect();
        }
    }
}

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
            Option<&turret::TurretEnemy>,
            Option<&EnemyMoveSpeed>,
            Option<&EnemyPathfinder>,
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

    let player_pos = player_transform.translation.truncate();

    for (mut enemy_transform, mut enemy_velocity, _pulled_opt, ranged_opt, turret_opt, spd_opt, pathfinder_opt) in &mut enemy_query {
        let max_speed = spd_opt.map_or(ENEMY_SPEED, |s| s.0);
        let mut effective_accel = accel;
        if grid_has_breach {
            effective_accel *= 0.15;
        }

        // Chasers steer toward the player (or a path waypoint if blocked).
        // Rangers get their velocity from ranger::ai.
        if ranged_opt.is_none() && turret_opt.is_none() {
            let target = pathfinder_opt
                .and_then(|pf| pf.waypoints.first().copied())
                .unwrap_or(player_pos);

            let dir = (target - enemy_transform.translation.truncate()).normalize_or_zero();

            if dir.length() > 0.0 {
                **enemy_velocity =
                    (**enemy_velocity + dir * effective_accel).clamp_length_max(max_speed);
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
    mut enemy_query: Query<&mut Transform, (With<Enemy>, With<ActiveEnemy>, Without<Reaper>)>,
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

fn enemies_collide_with_tables(
    mut enemy_query: Query<(&mut Transform, &mut Velocity), (With<Enemy>, With<ActiveEnemy>, Without<Reaper>)>,
    table_query: Query<(&Transform, &Collider, &table::TableRoom), (With<table::Table>, With<Collidable>, Without<Enemy>)>,
    active_room: Res<table::ActiveRoom>,
) {
    let Some(active) = active_room.0 else { return; };
    let enemy_half = Vec2::splat(ENEMY_SIZE * 0.5);

    for (mut enemy_tf, mut enemy_vel) in &mut enemy_query {
        let ep = enemy_tf.translation.truncate();
        for (table_tf, table_col, room) in &table_query {
            if room.0 != active { continue; }
            let tp = table_tf.translation.truncate();
            let th = table_col.half_extents;

            if !crate::player::aabb_overlap(ep.x, ep.y, enemy_half, tp.x, tp.y, th) {
                continue;
            }

            let overlap_x = (enemy_half.x + th.x) - (ep.x - tp.x).abs();
            let overlap_y = (enemy_half.y + th.y) - (ep.y - tp.y).abs();

            if overlap_x < overlap_y {
                if ep.x > tp.x {
                    enemy_tf.translation.x += overlap_x;
                    enemy_vel.velocity.x = enemy_vel.velocity.x.max(0.0);
                } else {
                    enemy_tf.translation.x -= overlap_x;
                    enemy_vel.velocity.x = enemy_vel.velocity.x.min(0.0);
                }
            } else {
                if ep.y > tp.y {
                    enemy_tf.translation.y += overlap_y;
                    enemy_vel.velocity.y = enemy_vel.velocity.y.max(0.0);
                } else {
                    enemy_tf.translation.y -= overlap_y;
                    enemy_vel.velocity.y = enemy_vel.velocity.y.min(0.0);
                }
            }
        }
    }
}

fn collide_enemies_with_enemies(
    mut enemy_query: Query<(&mut Transform, &mut Velocity), (With<Enemy>, With<ActiveEnemy>, Without<Reaper>)>,
) {
    // Use a circle distance slightly larger than the sprite so enemies don't stack.
    let sep = ENEMY_SIZE * 1.15;
    let sep2 = sep * sep;
    let max_check2 = (ENEMY_SIZE * 5.0) * (ENEMY_SIZE * 5.0);

    let mut combinations = enemy_query.iter_combinations_mut();
    while let Some([(mut tf1, mut vel1), (mut tf2, mut vel2)]) = combinations.fetch_next() {
        let p1 = tf1.translation.truncate();
        let p2 = tf2.translation.truncate();
        let diff = p1 - p2;
        let dist2 = diff.length_squared();
        if dist2 >= max_check2 || dist2 < 0.001 { continue; }

        if dist2 < sep2 {
            let dist = dist2.sqrt();
            let push_dir = diff / dist;
            let overlap = sep - dist;
            let push = push_dir * overlap * 0.5;

            tf1.translation.x += push.x;
            tf1.translation.y += push.y;
            tf2.translation.x -= push.x;
            tf2.translation.y -= push.y;

            // Cancel velocity components that are pushing the two enemies into each other
            // so they don't immediately re-penetrate next frame.
            let approach = (vel1.velocity - vel2.velocity).dot(push_dir);
            if approach < 0.0 {
                let correction = push_dir * approach * 0.5;
                vel1.velocity -= correction;
                vel2.velocity += correction;
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
