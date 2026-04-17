use bevy::prelude::*;

use crate::bullet::{Bullet, BulletOwner};
use crate::collidable::{Collidable, Collider};
use crate::enemies::{ActiveEnemy, Enemy, Health, RangedEnemy, RangedEnemyAI, Velocity};
use crate::player::Player;
use crate::room::{LevelState, RoomVec};
use crate::table;
use crate::{GameState, TILE_SIZE, Z_ENTITIES};
use crate::GameEntity;

// ── Components & resources ─────────────────────────────────────────────────

#[derive(Component)]
pub struct Reaper;

#[derive(Resource)]
pub struct ReaperState {
    pub timer: Timer,
    pub current_room: Option<usize>,
    pub spawned_in_room: Option<usize>,
    /// True when the reaper spawned in the last uncleared room.
    /// Prevents auto-despawn on room-clear so the player must kill it.
    pub spawned_in_final_room: bool,
}

impl Default for ReaperState {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(7.0, TimerMode::Once),
            current_room: None,
            spawned_in_room: None,
            spawned_in_final_room: false,
        }
    }
}

#[derive(Resource)]
pub struct ReaperRes {
    pub image: Handle<Image>,
}

/// Marker on the UI root node for the on-screen warning banner.
#[derive(Component)]
struct ReaperWarning {
    timer: Timer,
}

// ── Plugin ─────────────────────────────────────────────────────────────────

pub struct ReaperPlugin;

impl Plugin for ReaperPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ReaperState>()
            .add_systems(Startup, load_reaper_assets)
            .add_systems(
                Update,
                (
                    reaper_room_timer,
                    reaper_warning_lifecycle,
                    bullet_hits_reaper,
                    table_hits_reaper,
                    reaper_cleanup_system,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

// ── Asset loading ──────────────────────────────────────────────────────────

fn load_reaper_assets(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(ReaperRes { image: assets.load("reaper/reaper1.png") });
}

// ── Spawn ──────────────────────────────────────────────────────────────────

fn spawn_reaper(commands: &mut Commands, at: Vec3, res: &ReaperRes) {
    commands.spawn((
        Sprite::from_image(res.image.clone()),
        Transform { translation: at, ..Default::default() },
        Enemy,
        ActiveEnemy,
        Reaper,
        RangedEnemy,
        Velocity::new(),
        Health::new(500.0),
        RangedEnemyAI {
            range: 450.0,
            fire_cooldown: Timer::from_seconds(0.5, TimerMode::Repeating),
            projectile_speed: 700.0,
        },
        Collider { half_extents: Vec2::splat(TILE_SIZE * 0.5) },
        Collidable,
        crate::fluiddynamics::PulledByFluid { mass: 20.0 },
        GameEntity,
    ));
}

/// Spawns a full-screen UI banner that appears in the center of the screen
/// (not the world) and auto-despawns after 3 seconds.
fn spawn_reaper_warning(commands: &mut Commands, assets: &AssetServer) {
    let font: Handle<Font> = assets.load(
        "fonts/BitcountSingleInk-VariableFont_CRSV,ELSH,ELXP,SZP1,SZP2,XPN1,XPN2,YPN1,YPN2,slnt,wght.ttf",
    );

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ZIndex(200),
            ReaperWarning { timer: Timer::from_seconds(3.0, TimerMode::Once) },
            GameEntity,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("The Reaper has arrived!"),
                TextFont { font, font_size: 40.0, ..default() },
                TextColor(Color::srgb(1.0, 0.1, 0.1)),
            ));
        });
}

// ── Systems ────────────────────────────────────────────────────────────────

fn reaper_room_timer(
    time: Res<Time>,
    mut state: ResMut<ReaperState>,
    lvlstate: Res<LevelState>,
    rooms: Res<RoomVec>,
    mut commands: Commands,
    player_q: Query<&Transform, With<Player>>,
    reaper_res: Res<ReaperRes>,
    assets: Res<AssetServer>,
) {
    let current_idx_opt = match *lvlstate {
        LevelState::InRoom(idx, _) => Some(idx),
        _ => None,
    };

    match current_idx_opt {
        Some(idx) => {
            if state.current_room != Some(idx) {
                state.current_room = Some(idx);
                state.spawned_in_room = None;
                state.timer.reset();
            }
            if state.spawned_in_room == Some(idx) {
                return;
            }

            state.timer.tick(time.delta());
            if state.timer.finished() {
                if let Ok(player_tf) = player_q.single() {
                    let p = player_tf.translation;
                    let spawn_pos = p + Vec3::new(120.0, 0.0, Z_ENTITIES);
                    // Mark whether this is the last uncleared room so cleanup
                    // knows not to auto-despawn the reaper when the room clears.
                    let uncleared = rooms.0.iter().filter(|r| !r.cleared).count();
                    state.spawned_in_final_room = uncleared <= 1;
                    spawn_reaper(&mut commands, spawn_pos, &reaper_res);
                    spawn_reaper_warning(&mut commands, &assets);
                    state.spawned_in_room = Some(idx);
                }
            }
        }
        None => {
            if state.current_room.is_some() {
                state.current_room = None;
                state.spawned_in_room = None;
                state.timer.reset();
            }
        }
    }
}

fn reaper_warning_lifecycle(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut ReaperWarning)>,
) {
    for (entity, mut warn) in &mut q {
        warn.timer.tick(time.delta());
        if warn.timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn bullet_hits_reaper(
    mut commands: Commands,
    bullet_query: Query<(&Transform, Entity, &BulletOwner), With<Bullet>>,
    mut reaper_query: Query<(&Transform, &mut Health), With<Reaper>>,
    state: Res<ReaperState>,
) {
    if !state.spawned_in_final_room {
        return;
    }

    let bullet_half = Vec2::splat(TILE_SIZE * 0.5);
    let reaper_half = Vec2::splat(TILE_SIZE * 0.5);

    for (bullet_tf, bullet_entity, owner) in &bullet_query {
        if !matches!(owner, &BulletOwner::Player) {
            continue;
        }
        let bullet_pos = bullet_tf.translation;
        for (reaper_tf, mut health) in &mut reaper_query {
            let reaper_pos = reaper_tf.translation;
            if crate::bullet::aabb_overlap(
                bullet_pos.x, bullet_pos.y, bullet_half,
                reaper_pos.x, reaper_pos.y, reaper_half,
            ) {
                health.0 -= 25.0;
                if let Ok(mut entity) = commands.get_entity(bullet_entity) {
                    entity.despawn();
                }
            }
        }
    }
}

fn table_hits_reaper(
    mut reaper_query: Query<(&Transform, &mut Health), With<Reaper>>,
    table_query: Query<
        (&Transform, &Collider, Option<&crate::enemies::Velocity>),
        With<table::Table>,
    >,
    state: Res<ReaperState>,
) {
    if !state.spawned_in_final_room {
        return;
    }

    let reaper_half = Vec2::splat(TILE_SIZE * 0.5);

    for (reaper_tf, mut health) in &mut reaper_query {
        let reaper_pos = reaper_tf.translation.truncate();
        for (table_tf, table_col, vel_opt) in &table_query {
            let table_pos = table_tf.translation.truncate();
            let table_half = table_col.half_extents + Vec2::new(5.0, 5.0);
            if crate::player::aabb_overlap(
                reaper_pos.x, reaper_pos.y, reaper_half,
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

fn reaper_cleanup_system(
    mut commands: Commands,
    lvlstate: Res<LevelState>,
    rooms: Res<RoomVec>,
    mut state: ResMut<ReaperState>,
    reaper_q: Query<(Entity, &Health), With<Reaper>>,
) {
    let current_idx = if let LevelState::InRoom(idx, _) = *lvlstate {
        Some(idx)
    } else {
        None
    };

    let room_cleared = current_idx
        .and_then(|idx| rooms.0.get(idx))
        .map(|r| r.cleared)
        .unwrap_or(false);

    // Left the room (lvlstate became NotRoom after room cleared or player exited)
    let left_room = current_idx.is_none();

    for (entity, health) in &reaper_q {
        let is_dead = health.0 <= 0.0;

        // Final-room reaper only dies when the player kills it — never auto-despawn.
        // Non-final reapers auto-despawn when the room clears or the player leaves.
        let should_despawn = is_dead
            || (!state.spawned_in_final_room && (room_cleared || left_room));

        if should_despawn {
            commands.entity(entity).despawn();
            state.spawned_in_room = None;
            if is_dead {
                state.spawned_in_final_room = false;
            }
        }
    }

    // Reset room tracking when leaving a non-final room
    if left_room && !state.spawned_in_final_room {
        state.current_room = None;
        state.spawned_in_room = None;
        state.timer.reset();
    }
}
