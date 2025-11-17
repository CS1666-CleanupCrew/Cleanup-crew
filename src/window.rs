use bevy::prelude::*;
use crate::collidable::Collidable;

#[derive(Component)]
pub struct Window;

#[derive(Component)]
pub struct Health(pub f32);

#[derive(Component, PartialEq, Debug)]
pub enum GlassState {
    Intact,
    Broken,
}

#[derive(Component)]
struct WindowAnimation {
    frame_index: usize,
    timer: Timer,
}

#[derive(Component)]
struct BrokenTimer(Timer);

#[derive(Resource)]
struct WindowGraphics {
    broken: Vec<Handle<Image>>,
}

pub struct WindowPlugin;

impl Plugin for WindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_window_graphics)
            .add_systems(
                Update,
                (check_for_broken_windows, animate_broken_windows),
            )
            .add_systems(Update, animate_broken_windows);
    }
}

fn load_window_graphics(mut commands: Commands, asset_server: Res<AssetServer>) {
    let broken_handle = vec![
        asset_server.load("map/broken_window_1.png"),
        asset_server.load("map/broken_window_2.png"),
        asset_server.load("map/broken_window_3.png"),
    ];
    commands.insert_resource(WindowGraphics {
        broken: broken_handle,
    });
}

fn check_for_broken_windows(
    mut commands: Commands,
    mut query: Query<(Entity, &Health, &mut Sprite, &mut GlassState, &Transform), With<Window>>,
    mut fluid_query: Query<&mut crate::fluiddynamics::FluidGrid>,
    window_graphics: Res<WindowGraphics>,
) {
    for (entity, health, mut sprite, mut state, transform) in query.iter_mut() {
        if health.0 <= 0.0 && *state == GlassState::Intact {
            *state = GlassState::Broken;

            commands
                .entity(entity)
                .insert(WindowAnimation {
                        frame_index: 0,
                        timer: Timer::from_seconds(0.30, TimerMode::Repeating),
                });

            sprite.image = window_graphics.broken[0].clone();

            let mut breach_positions = Vec::new();

            // mark this tile as a breach for the fluid sim
            let world_pos = transform.translation.truncate();
            let (bx, by) = crate::fluiddynamics::world_to_grid(
                world_pos,
                crate::fluiddynamics::GRID_WIDTH,
                crate::fluiddynamics::GRID_HEIGHT,
            );
            breach_positions.push((bx, by));

            // Push any recorded breach positions into the fluid grid
            if let Ok(mut grid) = fluid_query.get_single_mut() {
                for &(bx, by) in &breach_positions {
                    grid.add_breach(bx, by);
                }
            }

            commands
                .entity(entity)
                .insert(BrokenTimer(Timer::from_seconds(1.5, TimerMode::Once)));
        }
    }
}


fn animate_broken_windows(
    time: Res<Time>,
    window_graphics: Res<WindowGraphics>,
    mut query: Query<(&mut Sprite, &mut WindowAnimation)>,
) {
    for (mut sprite, mut animation) in query.iter_mut() {
        animation.timer.tick(time.delta());

        if animation.timer.just_finished() {
            animation.frame_index = (animation.frame_index + 1) % window_graphics.broken.len();
            sprite.image = window_graphics.broken[animation.frame_index].clone();
        }
    }
}