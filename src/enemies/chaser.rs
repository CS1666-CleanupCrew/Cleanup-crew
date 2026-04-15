use bevy::prelude::*;
use crate::GameEntity;
use crate::fluiddynamics::PulledByFluid;
use super::{Enemy, Velocity, ActiveEnemy, Health, ANIM_TIME};

// ── Components ─────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct MeleeEnemy;

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

#[derive(Component)]
pub struct EnemyFrames {
    pub handles: Vec<Handle<Image>>,
    pub index: usize,
}

#[derive(Component)]
pub struct HitAnimation {
    pub timer: Timer,
}

// ── Resource ───────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct EnemyRes {
    pub frames: Vec<Handle<Image>>,
    pub hit_frames: Vec<Handle<Image>>,
}

// ── Asset loading ──────────────────────────────────────────────────────────

pub fn load(mut commands: Commands, asset_server: Res<AssetServer>) {
    let frames = vec![
        asset_server.load("chaser/chaser_mob_animation1.png"),
        asset_server.load("chaser/chaser_mob_animation2.png"),
        asset_server.load("chaser/chaser_mob_animation3.png"),
        asset_server.load("chaser/chaser_mob_animation2.png"),
    ];
    let hit_frames = vec![
        asset_server.load("chaser/chaser_mob_bite1.png"),
        asset_server.load("chaser/chaser_mob_bite2.png"),
    ];
    commands.insert_resource(EnemyRes { frames, hit_frames });
}

// ── Spawn ──────────────────────────────────────────────────────────────────

pub fn spawn_at(
    commands: &mut Commands,
    res: &EnemyRes,
    at: Vec3,
    active: bool,
    health_multiplier: f32,
) {
    let mut e = commands.spawn((
        Sprite::from_image(res.frames[0].clone()),
        Transform { translation: at, ..Default::default() },
        Enemy,
        Velocity::new(),
        Health::new(50.0 * health_multiplier),
        AnimationTimer(Timer::from_seconds(ANIM_TIME, TimerMode::Repeating)),
        EnemyFrames { handles: res.frames.clone(), index: 0 },
        PulledByFluid { mass: 10.0 },
        MeleeEnemy,
        GameEntity,
    ));
    if active {
        e.insert(ActiveEnemy);
    }
}

// ── Systems ────────────────────────────────────────────────────────────────

pub fn animate(
    time: Res<Time>,
    mut query: Query<
        (&mut Sprite, &mut AnimationTimer, &mut EnemyFrames, &Velocity),
        (With<Enemy>, With<ActiveEnemy>),
    >,
) {
    for (mut sprite, mut timer, mut frames, velocity) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() {
            frames.index = (frames.index + 1) % frames.handles.len();
            sprite.image = frames.handles[frames.index].clone();
        }
        if velocity.x > 0.0 {
            sprite.flip_x = true;
        } else if velocity.x < 0.0 {
            sprite.flip_x = false;
        }
    }
}

pub fn animate_hit(
    time: Res<Time>,
    mut commands: Commands,
    mut enemies: Query<
        (Entity, &mut Sprite, &mut HitAnimation),
        (Without<super::ranger::RangedEnemy>, Without<crate::enemies::Reaper>),
    >,
    enemy_res: Res<EnemyRes>,
) {
    for (entity, mut sprite, mut hit) in &mut enemies {
        hit.timer.tick(time.delta());
        if hit.timer.elapsed_secs() < 1.0 {
            sprite.image = enemy_res.hit_frames[0].clone();
        } else {
            sprite.image = enemy_res.hit_frames[1].clone();
        }
        if hit.timer.finished() {
            commands.entity(entity).remove::<HitAnimation>();
            sprite.image = enemy_res.frames[0].clone();
        }
    }
}

// Re-export `spawn_at` under the old name used in room.rs so callers can use
// either `chaser::spawn_at` or the mod-level alias `spawn_enemy_at`.
pub use spawn_at as spawn_enemy_at;
