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

#[derive(Resource)]
struct TableGraphics {
    broken: Handle<Image>,
}

pub struct TablePlugin;
use crate::enemy::Velocity;

impl Plugin for TablePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_table_graphics)
            .add_systems(
                Update,
                (check_for_broken_tables, animate_broken_tables, apply_table_velocity, collide_tables_with_tables.after(apply_table_velocity)),
            );
    }
}

fn load_table_graphics(mut commands: Commands, asset_server: Res<AssetServer>) {
    let broken_handle = asset_server.load("map/table_broken.png");
    commands.insert_resource(TableGraphics {
        broken: broken_handle,
    });
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
                .insert(crate::fluiddynamics::PulledByFluid { mass: 30.0 })
                .insert(Velocity::new());
        }
    }
}

fn animate_broken_tables(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Visibility, &mut BrokenTimer), With<Table>>,
) {
    for (entity, mut visibility, mut timer) in query.iter_mut() {
        timer.0.tick(time.delta());

        //if timer.0.just_finished() {
            //*visibility = Visibility::Hidden;
            commands.entity(entity).remove::<Collidable>();
        //}
    }
}

fn apply_table_velocity(
    time: Res<Time>,
    // Updated query to get mutable velocity and the table's collider
    mut table_query: Query<(&mut Transform, &mut crate::enemy::Velocity, &Collider), With<Table>>,
    // Added query for walls, just like in move_enemy
    wall_query: Query<(&Transform, &Collider), (With<Collidable>, Without<Table>)>,
) {
    let deltat = time.delta_secs();

    for (mut transform, mut velocity, table_collider) in &mut table_query {
        
//colliision detection from enemey.rs
        let change = velocity.velocity * deltat;
        let mut pos = transform.translation;
        let table_half = table_collider.half_extents;

        // ---- X axis ----
        if change.x != 0.0 {
            let mut nx = pos.x + change.x;
            let px = nx;
            let py = pos.y;
            for (wall_tf, wall_collider) in &wall_query {
                let (wx, wy) = (wall_tf.translation.x, wall_tf.translation.y);
                if crate::player::aabb_overlap(px, py, table_half, wx, wy, wall_collider.half_extents) {
                    if change.x > 0.0 {
                        nx = wx - (table_half.x + wall_collider.half_extents.x);
                    } else {
                        nx = wx + (table_half.x + wall_collider.half_extents.x);
                    }

                    //wall friction
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
            let py = ny;
            for (wall_tf, wall_collider) in &wall_query {
                let (wx, wy) = (wall_tf.translation.x, wall_tf.translation.y);
                if crate::player::aabb_overlap(px, py, table_half, wx, wy, wall_collider.half_extents) {
                    if change.y > 0.0 {
                        ny = wy - (table_half.y + wall_collider.half_extents.y);
                    } else {
                        ny = wy + (table_half.y + wall_collider.half_extents.y);
                    }

                    //wall friction
                    if velocity.velocity.x.abs() > 0.01 {
                        velocity.velocity.x *= WALL_SLIDE_FRICTION_MULTIPLIER;
                    }

                    velocity.velocity.y = 0.0;
                }
            }
            pos.y = ny;
        }

        transform.translation = pos; // Apply the final, collision-checked position
        // --- End of copied logic ---
        
        // Debug - show which tables are moving
        if velocity.velocity.length() > 1.0 {
            //info!("Table at ({:.0}, {:.0}) moving with velocity ({:.1}, {:.1})", 
            //   transform.translation.x, transform.translation.y,
            //   velocity.velocity.x, velocity.velocity.y);
        }
    }
}

fn collide_tables_with_tables(
    mut table_query: Query<(&mut Transform, &Collider), (With<Table>, With<Velocity>)>,
) {
    let mut combinations = table_query.iter_combinations_mut();
    while let Some([(mut t1_transform, c1), (mut t2_transform, c2)]) =
        combinations.fetch_next()
    {
        let (p1, h1) = (t1_transform.translation.truncate(), c1.half_extents);
        let (p2, h2) = (t2_transform.translation.truncate(), c2.half_extents);

        //check if they overlap and push them apart if they do
        if crate::player::aabb_overlap(p1.x, p1.y, h1, p2.x, p2.y, h2) {
            let overlap_x = (h1.x + h2.x) - (p1.x - p2.x).abs();
            let overlap_y = (h1.y + h2.y) - (p1.y - p2.y).abs();

            if overlap_x < overlap_y {
                let sign = if p1.x > p2.x { 1.0 } else { -1.0 };
                let push = sign * overlap_x * 0.5; 
                t1_transform.translation.x += push;
                t2_transform.translation.x -= push;
            } else {
                let sign = if p1.y > p2.y { 1.0 } else { -1.0 };
                let push = sign * overlap_y * 0.5; 
                t1_transform.translation.y += push;
                t2_transform.translation.y -= push;
            }
        }
    }
}
