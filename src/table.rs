use bevy::prelude::*;
use crate::collidable::Collidable;

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
                (check_for_broken_tables, animate_broken_tables, apply_table_velocity),
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
    mut query: Query<(&mut Transform, &crate::enemy::Velocity), With<Table>>,
) {
    for (mut transform, velocity) in &mut query {
        let delta = velocity.velocity * time.delta_secs();
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
        
        // Debug - show which tables are moving
        if velocity.velocity.length() > 1.0 {
            info!("Table at ({:.0}, {:.0}) moving with velocity ({:.1}, {:.1})", 
                  transform.translation.x, transform.translation.y,
                  velocity.velocity.x, velocity.velocity.y);
        }
    }
}