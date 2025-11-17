use bevy::prelude::*;

use crate::collidable::{Collidable, Collider};
use crate::enemy::{Enemy, ActiveEnemy, Velocity, Health};
use crate::player::Player;
use crate::room::{LevelState, RoomVec};
use crate::{GameState, TILE_SIZE, Z_ENTITIES};

// Marker for the special reaper enemy.
#[derive(Component)]
pub struct Reaper;

// Tracks per-room timer & spawn status for the reaper.
#[derive(Resource)]
pub struct ReaperState {
    // Timer for how long we've been in the current room.
    pub timer: Timer,
    // The room index we are currently timing, if any.
    pub current_room: Option<usize>,
    // Room index where the reaper has already spawned if any.
    pub spawned_in_room: Option<usize>,
}

impl Default for ReaperState {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(15.0, TimerMode::Once),
            current_room: None,
            spawned_in_room: None,
        }
    }
}

pub struct ReaperPlugin;

impl Plugin for ReaperPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<ReaperState>()
            .add_systems(
                Update,
                reaper_room_timer.run_if(in_state(GameState::Playing)),
            );
    }
}

// Spawn a reaper enemy at a given world position.
// No sprite yet just logic (movement & collision come from existing enemy systems).
fn spawn_reaper(commands: &mut Commands, at: Vec3) {
    commands.spawn((
        Transform {
            translation: at,
            ..Default::default()
        },
        Enemy,
        ActiveEnemy,
        Velocity::new(),
        Health::new(200.0),
        Collider {
            half_extents: Vec2::splat(TILE_SIZE * 0.5),
        },
        Collidable,
        Reaper,
    ));
}

// Runs every frame in Playing: if you stay in a room for 15s, spawn the reaper there.
fn reaper_room_timer(
    time: Res<Time>,
    mut state: ResMut<ReaperState>,
    lvlstate: Res<LevelState>,
    mut commands: Commands,
    player_q: Query<&Transform, With<Player>>,
) {
    // Only care while actually inside a room
    let current_idx_opt = match *lvlstate {
        LevelState::InRoom(idx) => Some(idx),
        _ => None,
    };

    match current_idx_opt { Some(idx) => {
            // If we just entered a different room, reset timer & spawn flag
            if state.current_room != Some(idx) {
                state.current_room = Some(idx);
                state.spawned_in_room = None;
                state.timer.reset();
            }

            // Already spawned reaper in this room? nothing to do
            if state.spawned_in_room == Some(idx) {
                return;
            }

            state.timer.tick(time.delta());
            if state.timer.finished() {
                // Spawn the reaper near the player
                if let Ok(player_tf) = player_q.get_single() {
                    let p = player_tf.translation;
                    let spawn_pos = p + Vec3::new(120.0, 0.0, Z_ENTITIES);
                    spawn_reaper(&mut commands, spawn_pos);
                    state.spawned_in_room = Some(idx);
                    info!("Reaper spawned in room {}", idx);
                }
            }
        }
        None => {
            // Not in a room, reset tracking
            if state.current_room.is_some() {
                state.current_room = None;
                state.spawned_in_room = None;
                state.timer.reset();
            }
        }
    }
}
