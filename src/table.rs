use bevy::prelude::*;
use crate::collidable::{Collidable, Collider};
use crate::GameState;

const WALL_SLIDE_FRICTION_MULTIPLIER: f32 = 0.7;

#[derive(Component)]
pub struct Table;

#[derive(Component)]
pub struct Health(pub f32);

#[derive(Component, PartialEq, Debug)]
pub enum TableState {
    Intact,
    Broken,
}

#[derive(Component)]
struct BrokenTimer(Timer);

/// Which room index this table belongs to — used to filter physics to the active room only.
#[derive(Component)]
pub struct TableRoom(pub usize);

/// Updated by room.rs whenever LevelState changes. Table physics only runs for this room.
#[derive(Resource, Default)]
pub struct ActiveRoom(pub Option<usize>);

#[derive(Resource)]
struct TableGraphics {
    broken: Handle<Image>,
}

pub struct TablePlugin;
use crate::enemies::Velocity;
use crate::fluiddynamics::PulledByFluid;

impl Plugin for TablePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ActiveRoom::default())
            .add_systems(Startup, load_table_graphics)
            .add_systems(
                Update,
                (
                    ensure_tables_have_pull_components,
                    check_for_broken_tables,
                    animate_broken_tables,
                ),
            )
            .add_systems(
                Update,
                (
                    apply_table_velocity,
                    collide_tables_with_tables.after(apply_table_velocity),
                ).run_if(in_state(GameState::Playing)),
            );
    }
}

fn load_table_graphics(mut commands: Commands, asset_server: Res<AssetServer>) {
    let broken_handle = asset_server.load("map/table_broken.png");
    commands.insert_resource(TableGraphics { broken: broken_handle });
}

fn ensure_tables_have_pull_components(
    mut commands: Commands,
    query_missing_pull: Query<Entity, (With<Table>, Without<PulledByFluid>)>,
    query_missing_vel: Query<Entity, (With<Table>, Without<Velocity>)>,
) {
    const INTACT_TABLE_MASS: f32 = 120.0;
    for entity in query_missing_pull.iter() {
        commands.entity(entity).insert(PulledByFluid { mass: INTACT_TABLE_MASS });
    }
    for entity in query_missing_vel.iter() {
        commands.entity(entity).insert(Velocity::new());
    }
}

fn check_for_broken_tables(
    mut commands: Commands,
    mut query: Query<(Entity, &Health, &mut Sprite, &mut TableState), With<Table>>,
    table_graphics: Res<TableGraphics>,
) {
    for (entity, health, mut sprite, mut state) in query.iter_mut() {
        if health.0 <= 0.0 && *state == TableState::Intact {
            *state = TableState::Broken;
            sprite.image = table_graphics.broken.clone();
            commands
                .entity(entity)
                .remove::<Collidable>()
                .insert(BrokenTimer(Timer::from_seconds(1.5, TimerMode::Once)))
                .insert(PulledByFluid { mass: 30.0 })
                .insert(Velocity::new());
        }
    }
}

fn animate_broken_tables(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut BrokenTimer), With<Table>>,
) {
    for (entity, mut timer) in query.iter_mut() {
        timer.0.tick(time.delta());
        if timer.0.finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn apply_table_velocity(
    time: Res<Time>,
    active_room: Res<ActiveRoom>,
    mut table_query: Query<(&mut Transform, &mut Velocity, &Collider, &TableRoom), With<Table>>,
    wall_grid: Res<crate::map::WallGrid>,
) {
    let Some(active) = active_room.0 else { return; };

    // Cap delta so a lag spike can't cause a huge jump
    let delta = time.delta_secs().min(0.05);

    for (mut transform, mut velocity, table_collider, room) in &mut table_query {
        if room.0 != active { continue; }
        if velocity.velocity.length_squared() < 0.01 { continue; }

        let max_speed = table_collider.half_extents.x.min(table_collider.half_extents.y) / delta;
        let speed = velocity.velocity.length();
        if speed > max_speed {
            velocity.velocity = velocity.velocity * (max_speed / speed);
        }

        let change = velocity.velocity * delta;
        let mut pos = transform.translation;
        let table_half = table_collider.half_extents;

        // ---- X axis ----
        if change.x != 0.0 {
            let mut nx = pos.x + change.x;
            for (wall_pos, wall_half) in wall_grid.nearby(Vec2::new(nx, pos.y), 3) {
                if crate::player::aabb_overlap(nx, pos.y, table_half, wall_pos.x, wall_pos.y, wall_half) {
                    nx = if change.x > 0.0 {
                        wall_pos.x - (table_half.x + wall_half.x)
                    } else {
                        wall_pos.x + (table_half.x + wall_half.x)
                    };
                    if velocity.velocity.y.abs() > 0.01 {
                        velocity.velocity.y *= WALL_SLIDE_FRICTION_MULTIPLIER;
                    }
                    velocity.velocity.x = 0.0;
                }
            }
            pos.x = nx;
        }

        // ---- Y axis ----
        if change.y != 0.0 {
            let mut ny = pos.y + change.y;
            for (wall_pos, wall_half) in wall_grid.nearby(Vec2::new(pos.x, ny), 3) {
                if crate::player::aabb_overlap(pos.x, ny, table_half, wall_pos.x, wall_pos.y, wall_half) {
                    ny = if change.y > 0.0 {
                        wall_pos.y - (table_half.y + wall_half.y)
                    } else {
                        wall_pos.y + (table_half.y + wall_half.y)
                    };
                    if velocity.velocity.x.abs() > 0.01 {
                        velocity.velocity.x *= WALL_SLIDE_FRICTION_MULTIPLIER;
                    }
                    velocity.velocity.y = 0.0;
                }
            }
            pos.y = ny;
        }

        transform.translation = pos;
    }
}

/// Push `pos` out of nearby walls using the spatial hash.
fn snap_out_of_walls(pos: &mut Vec3, half: Vec2, wall_grid: &crate::map::WallGrid) {
    for (wp, wh) in wall_grid.nearby(pos.truncate(), 3) {
        let dx = pos.x - wp.x;
        let dy = pos.y - wp.y;
        let overlap_x = half.x + wh.x - dx.abs();
        let overlap_y = half.y + wh.y - dy.abs();
        if overlap_x > 0.0 && overlap_y > 0.0 {
            if overlap_x < overlap_y {
                pos.x += if dx >= 0.0 { overlap_x } else { -overlap_x };
            } else {
                pos.y += if dy >= 0.0 { overlap_y } else { -overlap_y };
            }
        }
    }
}

pub fn collide_tables_with_tables(
    mut table_query: Query<(Entity, &mut Transform, &Collider, &Velocity, &TableRoom), With<Table>>,
    wall_grid: Res<crate::map::WallGrid>,
    active_room: Res<ActiveRoom>,
) {
    let Some(active) = active_room.0 else { return; };

    // Collect only the active-room entities to avoid O(all_tables²) pair iteration.
    let active_entities: Vec<Entity> = table_query.iter()
        .filter_map(|(e, _, _, _, room)| if room.0 == active { Some(e) } else { None })
        .collect();

    for i in 0..active_entities.len() {
        for j in (i + 1)..active_entities.len() {
            let Ok([(mut t1_tf, c1, v1, _), (mut t2_tf, c2, v2, _)]) =
                table_query.get_many_mut([active_entities[i], active_entities[j]])
                    .map(|arr| arr.map(|(_, tf, c, v, r)| (tf, c, v, r)))
            else { continue; };

            let v1_sq = v1.velocity.length_squared();
            let v2_sq = v2.velocity.length_squared();
            if v1_sq < 0.01 && v2_sq < 0.01 { continue; }

            let (p1, h1) = (t1_tf.translation.truncate(), c1.half_extents);
            let (p2, h2) = (t2_tf.translation.truncate(), c2.half_extents);

            let diff = p1 - p2;
            if diff.x.abs() >= h1.x + h2.x || diff.y.abs() >= h1.y + h2.y { continue; }

            if crate::player::aabb_overlap(p1.x, p1.y, h1, p2.x, p2.y, h2) {
                let overlap_x = (h1.x + h2.x) - (p1.x - p2.x).abs();
                let overlap_y = (h1.y + h2.y) - (p1.y - p2.y).abs();

                // Cap per-frame push to one tile so a fast table can't compress
                // a player through a wall in a single step.
                let max_push = crate::TILE_SIZE;
                if overlap_x < overlap_y {
                    let sign = if p1.x > p2.x { 1.0 } else { -1.0 };
                    let push = overlap_x.min(max_push);
                    if v1_sq >= v2_sq {
                        t1_tf.translation.x += sign * push;
                        snap_out_of_walls(&mut t1_tf.translation, h1, &wall_grid);
                    } else {
                        t2_tf.translation.x -= sign * push;
                        snap_out_of_walls(&mut t2_tf.translation, h2, &wall_grid);
                    }
                } else {
                    let sign = if p1.y > p2.y { 1.0 } else { -1.0 };
                    let push = overlap_y.min(max_push);
                    if v1_sq >= v2_sq {
                        t1_tf.translation.y += sign * push;
                        snap_out_of_walls(&mut t1_tf.translation, h1, &wall_grid);
                    } else {
                        t2_tf.translation.y -= sign * push;
                        snap_out_of_walls(&mut t2_tf.translation, h2, &wall_grid);
                    }
                }
            }
        }
    }
}
