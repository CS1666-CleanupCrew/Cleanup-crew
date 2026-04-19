use bevy::prelude::*;
use std::time::Duration;
use crate::GameEntity;
use crate::fluiddynamics::PulledByFluid;
use crate::player::Player;
use crate::room::LevelState;
use crate::collidable::Collider;
use crate::bullet::{Bullet, BulletOwner, AnimationTimer, AnimationFrameCount};
use crate::weapon::{BulletDamage, EnemyBulletRes, WeaponSounds};
use super::{Enemy, Velocity, ActiveEnemy, Health, MaxHealth, ENEMY_ACCEL, ENEMY_SPEED, ANIM_TIME, spawn_health_bar_children, Reaper};

// ── Components ─────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct RangedEnemy;

#[derive(Component)]
pub struct RangedEnemyAI {
    pub range: f32,
    pub fire_cooldown: Timer,
    pub projectile_speed: f32,
}

#[derive(Component)]
pub struct RangedEnemyFrames {
    pub right: Vec<Handle<Image>>,
    pub left: Vec<Handle<Image>>,
    pub index: usize,
    pub facing_left: bool,
}

#[derive(Component, Deref, DerefMut)]
pub struct RangedAnimationTimer(pub Timer);

// ── Resource ───────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct RangedEnemyRes {
    pub right_frames: Vec<Handle<Image>>,
    pub left_frames: Vec<Handle<Image>>,
}

// ── Event ──────────────────────────────────────────────────────────────────

#[derive(Event)]
pub struct RangerShootEvent {
    pub origin: Vec3,
    pub direction: Vec2,
    pub speed: f32,
}

// ── Asset loading ──────────────────────────────────────────────────────────

pub fn load(mut commands: Commands, asset_server: Res<AssetServer>) {
    let right_frames = vec![
        asset_server.load("ranger/ranger_mob_animation_1.png"),
        asset_server.load("ranger/ranger_mob_animation_1,5.png"),
        asset_server.load("ranger/ranger_mob_animation_2.png"),
        asset_server.load("ranger/ranger_mob_animation_3.png"),
    ];
    let left_frames = vec![
        asset_server.load("ranger/ranger_mob_animation_1_left.png"),
        asset_server.load("ranger/ranger_mob_animation_1,5_left.png"),
        asset_server.load("ranger/ranger_mob_animation_2_left.png"),
        asset_server.load("ranger/ranger_mob_animation_3_left.png"),
    ];
    commands.insert_resource(RangedEnemyRes { right_frames, left_frames });
}

// ── Spawn ──────────────────────────────────────────────────────────────────

pub fn spawn_at(
    commands: &mut Commands,
    res: &RangedEnemyRes,
    at: Vec3,
    active: bool,
    health_multiplier: f32,
    speed_bonus: f32,
) {
    let hp = 40.0 * health_multiplier;
    let mut e = commands.spawn((
        Sprite::from_image(res.right_frames[0].clone()),
        Transform { translation: at, ..Default::default() },
        Enemy,
        RangedEnemy,
        Velocity::new(),
        Health::new(hp),
        MaxHealth(hp),
        super::EnemyMoveSpeed(ENEMY_SPEED + speed_bonus),
        RangedAnimationTimer(Timer::from_seconds(ANIM_TIME, TimerMode::Repeating)),
        RangedEnemyFrames {
            right: res.right_frames.clone(),
            left: res.left_frames.clone(),
            index: 0,
            facing_left: false,
        },
        RangedEnemyAI {
            range: 400.0,
            fire_cooldown: Timer::from_seconds(1.0, TimerMode::Repeating),
            projectile_speed: 600.0,
        },
        PulledByFluid { mass: 10.0 },
        GameEntity,
    ));
    e.with_children(|parent| spawn_health_bar_children(parent));
    if active {
        e.insert(ActiveEnemy);
    }
}

// ── Systems ────────────────────────────────────────────────────────────────

pub fn animate(
    time: Res<Time>,
    mut query: Query<
        (&mut Sprite, &mut RangedAnimationTimer, &mut RangedEnemyFrames),
        With<RangedEnemy>,
    >,
) {
    for (mut sprite, mut timer, mut frames) in &mut query {
        timer.tick(time.delta());

        let mut new_image: Option<Handle<Image>> = None;
        let new_index = {
            let frame_list = if frames.facing_left { &frames.left } else { &frames.right };
            if timer.just_finished() && !frame_list.is_empty() {
                let idx = (frames.index + 1) % frame_list.len();
                new_image = Some(frame_list[idx].clone());
                idx
            } else {
                frames.index
            }
        };
        if let Some(img) = new_image {
            frames.index = new_index;
            sprite.image = img;
        }
    }
}

pub fn ai(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemies: Query<(&Transform, &mut Velocity, &mut RangedEnemyAI, Option<&super::EnemyMoveSpeed>), (With<RangedEnemy>, Without<Reaper>)>,
    mut shoot_writer: EventWriter<RangerShootEvent>,
    lvlstate: Res<LevelState>,
) {
    let Ok(player_tf) = player_query.single() else { return };
    let player_pos = player_tf.translation.truncate();

    let difficulty_mult: f32 = match *lvlstate {
        LevelState::InRoom(idx, _) | LevelState::EnteredRoom(idx) => 1.0 + (idx as f32 * 0.10),
        LevelState::NotRoom => 1.0,
    };

    for (enemy_tf, mut vel, mut enemy_ai, spd_opt) in &mut enemies {
        let max_speed = spd_opt.map_or(ENEMY_SPEED, |s| s.0);
        let scaled_dt = time.delta_secs() * difficulty_mult;
        enemy_ai.fire_cooldown.tick(Duration::from_secs_f32(scaled_dt));

        let enemy_pos = enemy_tf.translation.truncate();
        let diff = player_pos - enemy_pos;
        let dist = diff.length();
        if dist == 0.0 { continue; }

        let dir = diff / dist;
        let desired = enemy_ai.range * 0.75;
        let delta = dist - desired;
        let move_dir = if delta > 20.0 { dir } else if delta < -20.0 { -dir } else { Vec2::ZERO };

        let accel = ENEMY_ACCEL * time.delta_secs();
        vel.velocity = (vel.velocity + move_dir * accel).clamp_length_max(max_speed);

        if enemy_ai.fire_cooldown.finished() && dist <= enemy_ai.range {
            shoot_writer.write(RangerShootEvent {
                origin: enemy_tf.translation,
                direction: dir,
                speed: enemy_ai.projectile_speed,
            });
            enemy_ai.fire_cooldown.reset();
        }
    }
}

// Alias for room.rs callers
pub use spawn_at as spawn_ranged_enemy_at;

// ── Bullet stats ───────────────────────────────────────────────────────────

const RANGER_BULLET_DAMAGE: f32 = 10.0;
const RANGER_BULLET_SCALE: f32 = 0.25;

pub fn spawn_ranger_bullets(
    mut commands: Commands,
    mut events: EventReader<RangerShootEvent>,
    bullet_res: Res<EnemyBulletRes>,
    weapon_sounds: Res<WeaponSounds>,
) {
    for ev in events.read() {
        let dir = ev.direction.normalize_or_zero();
        if dir == Vec2::ZERO { continue; }
        let spawn_pos = ev.origin.truncate() + dir * 16.0;

        commands.spawn((
            Sprite::from_atlas_image(
                bullet_res.0.clone(),
                TextureAtlas { layout: bullet_res.1.clone(), index: 0 },
            ),
            Transform {
                translation: Vec3::new(spawn_pos.x, spawn_pos.y, 5.0),
                scale: Vec3::splat(RANGER_BULLET_SCALE),
                ..Default::default()
            },
            crate::bullet::Velocity(dir * ev.speed),
            Bullet,
            BulletOwner::Enemy,
            Collider { half_extents: Vec2::splat(5.0) },
            BulletDamage(RANGER_BULLET_DAMAGE),
            AnimationTimer(Timer::from_seconds(0.2, TimerMode::Repeating)),
            AnimationFrameCount(3),
            GameEntity,
        ));

        commands.spawn((
            AudioPlayer::new(weapon_sounds.laser.clone()),
            PlaybackSettings::DESPAWN,
        ));
    }
}
