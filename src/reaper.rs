use bevy::prelude::*;

use crate::bullet::{Bullet, BulletOwner};
use crate::collidable::{Collidable, Collider};
use crate::enemy::{ActiveEnemy, Enemy, Health, RangedEnemy, RangedEnemyAI, Velocity};
use crate::player::Player;
use crate::room::{LevelState, RoomVec};
use crate::table;
use crate::{GameState, TILE_SIZE, Z_ENTITIES};
use crate::GameEntity;



#[derive(Component)]
pub struct Reaper;

// Tracks per-room timer & spawn status for the reaper.
#[derive(Resource)]
pub struct ReaperState {
    pub timer: Timer,
    pub current_room: Option<usize>,
    pub spawned_in_room: Option<usize>,
}

impl Default for ReaperState {
    fn default() -> Self {
        Self {
            // spawn after 7 seconds in a room
            timer: Timer::from_seconds(7.0, TimerMode::Once),
            current_room: None,
            spawned_in_room: None,
        }
    }
}

// Sprite for the reaper.
#[derive(Resource)]
pub struct ReaperRes {
    pub image: Handle<Image>,
}

// On screen warning when the reaper appears.
#[derive(Component)]
struct ReaperWarning {
    timer: Timer,
}


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

// Setup

fn load_reaper_assets(mut commands: Commands, assets: Res<AssetServer>) {
    let image: Handle<Image> = assets.load("reaper/reaper1.png");
    commands.insert_resource(ReaperRes { image });
}

// Spawning logic

// Spawn a reaper enemy at a given world position.
// Movement & melee damage use existing Enemy + RangedEnemy systems.
fn spawn_reaper(commands: &mut Commands, at: Vec3, res: &ReaperRes) {
    commands.spawn((
        Sprite::from_image(res.image.clone()),
        Transform {
            translation: at,
            ..Default::default()
        },
        Enemy,
        ActiveEnemy,
        Reaper,
        // treat it like a ranged enemy so it can shoot + keep some distance
        RangedEnemy,
        Velocity::new(),
        Health::new(500.0),
        RangedEnemyAI {
            range: 450.0,
            fire_cooldown: Timer::from_seconds(0.5, TimerMode::Repeating),
            projectile_speed: 700.0,
        },
        Collider {
            half_extents: Vec2::splat(TILE_SIZE * 0.5),
        },
        Collidable,
        crate::fluiddynamics::PulledByFluid { mass: 20.0 },
        GameEntity,
    ));

    //info!("Reaper spawned at {:?}", at);
}


fn reaper_room_timer(
    time: Res<Time>,
    mut state: ResMut<ReaperState>,
    lvlstate: Res<LevelState>,
    mut commands: Commands,
    player_q: Query<&Transform, With<Player>>,
    rooms: Res<RoomVec>,
    reaper_res: Res<ReaperRes>,
    assets: Res<AssetServer>,
) {
    // Only care while actually inside a room
    let current_idx_opt = match *lvlstate {
        LevelState::InRoom(idx, _) => Some(idx),
        _ => None,
    };

    match current_idx_opt {
        Some(idx) => {
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
                if let Ok(player_tf) = player_q.single() {
                    let p = player_tf.translation;
                    let spawn_pos = p + Vec3::new(120.0, 0.0, Z_ENTITIES);

                    spawn_reaper(&mut commands, spawn_pos, &reaper_res);
                    spawn_reaper_warning(&mut commands, &assets);
                    state.spawned_in_room = Some(idx);

                    // info!(
                    //     "Reaper spawned in room {} (rooms left: {})",
                    //     idx,
                    //     rooms.0.len()
                    // );
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


fn spawn_reaper_warning(commands: &mut Commands, assets: &AssetServer) {
    let font: Handle<Font> = assets.load(
        "fonts/BitcountSingleInk-VariableFont_CRSV,ELSH,ELXP,SZP1,SZP2,XPN1,XPN2,YPN1,YPN2,slnt,wght.ttf",
    );

    commands.spawn((
        Text2d::new("The Reaper has arrived!"),
        TextFont {
            font,
            font_size: 32.0,
            ..Default::default()
        },
        TextColor(Color::srgb(1.0, 0.1, 0.1)),
        Transform::from_xyz(0.0, 200.0, Z_ENTITIES + 200.0),
        ReaperWarning {
            timer: Timer::from_seconds(3.0, TimerMode::Once),
        },
    ));
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

fn is_final_room(lvlstate: &LevelState, rooms: &RoomVec) -> bool {
    matches!(lvlstate, LevelState::InRoom(_, _)) && rooms.0.len() == 1
}

//  Damage gating only in final room
// Player bullets can hit Reaper ONLY in the final room.
fn bullet_hits_reaper(
    mut commands: Commands,
    bullet_query: Query<(&Transform, Entity, &BulletOwner), With<Bullet>>,
    mut reaper_query: Query<(&Transform, &mut Health), With<Reaper>>,
    lvlstate: Res<LevelState>,
    rooms: Res<RoomVec>,
) {
    if !is_final_room(&lvlstate, &rooms) {
        return;
    }

    let bullet_half = Vec2::splat(TILE_SIZE * 0.5);
    let reaper_half = Vec2::splat(TILE_SIZE * 0.5);

    for (bullet_tf, bullet_entity, owner) in &bullet_query {
        // Only PLAYER bullets may hurt the Reaper
        if !matches!(owner, &BulletOwner::Player) {
            continue;
        }
        let bullet_pos = bullet_tf.translation;

        for (reaper_tf, mut health) in &mut reaper_query {
            let reaper_pos = reaper_tf.translation;
            if crate::bullet::aabb_overlap(
                bullet_pos.x,
                bullet_pos.y,
                bullet_half,
                reaper_pos.x,
                reaper_pos.y,
                reaper_half,
            ) {
                health.0 -= 25.0;
                if let Ok(mut entity) = commands.get_entity(bullet_entity) { entity.despawn(); }
            }
        }
    }
}

/// Tables can damage Reaper ONLY in the final room.
fn table_hits_reaper(
    _time: Res<Time>,
    mut reaper_query: Query<(&Transform, &mut Health), With<Reaper>>,
    table_query: Query<
        (&Transform, &Collider, Option<&crate::enemy::Velocity>),
        With<table::Table>,
    >,
    lvlstate: Res<LevelState>,
    rooms: Res<RoomVec>,
) {
    if !is_final_room(&lvlstate, &rooms) {
        return;
    }

    let reaper_half = Vec2::splat(TILE_SIZE * 0.5);

    for (reaper_tf, mut health) in &mut reaper_query {
        let reaper_pos = reaper_tf.translation.truncate();

        for (table_tf, table_col, vel_opt) in &table_query {
            let table_pos = table_tf.translation.truncate();

            let extra = Vec2::new(5.0, 5.0);
            let table_half = table_col.half_extents + extra;

            if crate::player::aabb_overlap(
                reaper_pos.x,
                reaper_pos.y,
                reaper_half,
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

    // If we’re not in any room, just clean up any stray Reapers and reset state
    if current_idx.is_none() {
        for (entity, _) in &reaper_q {
            commands.entity(entity).despawn();
        }
        state.current_room = None;
        state.spawned_in_room = None;
        state.timer.reset();
        return;
    }

    let idx = current_idx.unwrap();

    let in_final_room = is_final_room(&lvlstate, &rooms);

    // Check if this room is marked cleared
    let room_cleared = rooms
        .0
        .get(idx)
        .map(|r| r.cleared)
        .unwrap_or(false);

    for (entity, health) in &reaper_q {
        // In non-final rooms: despawn on room clear or death
        // In final room: ONLY despawn on death
        let should_despawn =
            (!in_final_room && room_cleared) || health.0 <= 0.0;

        if should_despawn {
            commands.entity(entity).despawn();
            state.spawned_in_room = None;
        }
    }
}


