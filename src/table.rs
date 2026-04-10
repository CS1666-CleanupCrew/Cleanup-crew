use bevy::prelude::*;
use crate::collidable::{Collidable, Collider};

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
use crate::enemy::Velocity;
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
                    apply_table_velocity,
                    collide_tables_with_tables.after(apply_table_velocity),
                ),
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
                .insert(BrokenTimer(Timer::from_seconds(1.5, TimerMode::Once)))
                .insert(PulledByFluid { mass: 30.0 })
                .insert(Velocity::new());
        }
    }
}

fn animate_broken_tables(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut BrokenTimer), (With<Table>, With<Collidable>)>,
) {
    for (entity, mut timer) in query.iter_mut() {
        timer.0.tick(time.delta());
        commands.entity(entity).remove::<Collidable>();
    }
}

fn apply_table_velocity(
    time: Res<Time>,
    active_room: Res<ActiveRoom>,
    mut table_query: Query<(&mut Transform, &mut Velocity, &Collider, &TableRoom), With<Table>>,
    wall_query: Query<(&Transform, &Collider), (With<Collidable>, Without<Table>)>,
) {
    let Some(active) = active_room.0 else { return; };

    // Cap delta so a lag spike can't cause a huge jump
    let delta = time.delta_secs().min(0.05);

    let walls: Vec<(Vec2, Vec2)> = wall_query
        .iter()
        .map(|(tf, col)| (tf.translation.truncate(), col.half_extents))
        .collect();

    for (mut transform, mut velocity, table_collider, room) in &mut table_query {
        // Only simulate physics for the current active room
        if room.0 != active { continue; }

        if velocity.velocity.length_squared() < 0.01 { continue; }

        // Velocity cap: a table can't move more than its own half-extent in one frame,
        // which guarantees it can never tunnel through a same-size or larger wall.
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
            let py = pos.y;
            for &(wall_pos, wall_half) in &walls {
                let (wx, wy) = (wall_pos.x, wall_pos.y);
                if crate::player::aabb_overlap(nx, py, table_half, wx, wy, wall_half) {
                    nx = if change.x > 0.0 {
                        wx - (table_half.x + wall_half.x)
                    } else {
                        wx + (table_half.x + wall_half.x)
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
            let px = pos.x;
            for &(wall_pos, wall_half) in &walls {
                let (wx, wy) = (wall_pos.x, wall_pos.y);
                if crate::player::aabb_overlap(px, ny, table_half, wx, wy, wall_half) {
                    ny = if change.y > 0.0 {
                        wy - (table_half.y + wall_half.y)
                    } else {
                        wy + (table_half.y + wall_half.y)
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

/// Push `pos` out of any wall it overlaps with.
fn snap_out_of_walls(pos: &mut Vec3, half: Vec2, walls: &[(Vec2, Vec2)]) {
    for &(wp, wh) in walls {
        let dx = pos.x - wp.x;
        let dy = pos.y - wp.y;
        let overlap_x = half.x + wh.x - dx.abs();
        let overlap_y = half.y + wh.y - dy.abs();
        if overlap_x > 0.0 && overlap_y > 0.0 {
            // Resolve along the axis with the smallest penetration depth
            if overlap_x < overlap_y {
                pos.x += if dx >= 0.0 { overlap_x } else { -overlap_x };
            } else {
                pos.y += if dy >= 0.0 { overlap_y } else { -overlap_y };
            }
        }
    }
}

fn collide_tables_with_tables(
    mut table_query: Query<(&mut Transform, &Collider, &Velocity, &TableRoom), With<Table>>,
    wall_query: Query<(&Transform, &Collider), (With<Collidable>, Without<Table>)>,
    active_room: Res<ActiveRoom>,
) {
    let Some(active) = active_room.0 else { return; };

    let walls: Vec<(Vec2, Vec2)> = wall_query
        .iter()
        .map(|(tf, col)| (tf.translation.truncate(), col.half_extents))
        .collect();

    let mut combinations = table_query.iter_combinations_mut();
    while let Some([
        (mut t1_transform, c1, v1, r1),
        (mut t2_transform, c2, v2, r2),
    ]) = combinations.fetch_next()
    {
        // Skip tables outside the active room entirely
        if r1.0 != active || r2.0 != active { continue; }

        let v1_sq = v1.velocity.length_squared();
        let v2_sq = v2.velocity.length_squared();
        if v1_sq < 0.01 && v2_sq < 0.01 { continue; }

        let (p1, h1) = (t1_transform.translation.truncate(), c1.half_extents);
        let (p2, h2) = (t2_transform.translation.truncate(), c2.half_extents);

        let diff = p1 - p2;
        if diff.x.abs() >= h1.x + h2.x || diff.y.abs() >= h1.y + h2.y { continue; }

        if crate::player::aabb_overlap(p1.x, p1.y, h1, p2.x, p2.y, h2) {
            let overlap_x = (h1.x + h2.x) - (p1.x - p2.x).abs();
            let overlap_y = (h1.y + h2.y) - (p1.y - p2.y).abs();

            if overlap_x < overlap_y {
                let sign = if p1.x > p2.x { 1.0 } else { -1.0 };
                // Only push the faster (or only moving) table to prevent
                // launching a stationary table into a wall.
                if v1_sq >= v2_sq {
                    t1_transform.translation.x += sign * overlap_x;
                    snap_out_of_walls(&mut t1_transform.translation, h1, &walls);
                } else {
                    t2_transform.translation.x -= sign * overlap_x;
                    snap_out_of_walls(&mut t2_transform.translation, h2, &walls);
                }
            } else {
                let sign = if p1.y > p2.y { 1.0 } else { -1.0 };
                if v1_sq >= v2_sq {
                    t1_transform.translation.y += sign * overlap_y;
                    snap_out_of_walls(&mut t1_transform.translation, h1, &walls);
                } else {
                    t2_transform.translation.y -= sign * overlap_y;
                    snap_out_of_walls(&mut t2_transform.translation, h2, &walls);
                }
            }
        }
    }
}
