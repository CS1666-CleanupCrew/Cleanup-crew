use bevy::prelude::*;
use std::time::Duration;
use crate::GameEntity;
use crate::fluiddynamics::PulledByFluid;
use crate::player::Player;
use crate::room::LevelState;
use crate::collidable::Collider;
use crate::bullet::{Bullet, BulletOwner, AnimationTimer, AnimationFrameCount};
use crate::weapon::{BulletDamage, EnemyBulletRes, WeaponSounds};
use super::{Enemy, Velocity, ActiveEnemy, Health, MaxHealth, ENEMY_ACCEL, ENEMY_SPEED, ANIM_TIME, spawn_health_bar_children};

// ── Components ─────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct TurretEnemy;

#[derive(Component)]
pub struct TurretAI {
    pub range: f32,
    pub fire_cooldown: Timer,
    pub projectile_speed: f32,
}

#[derive(Component)]
pub struct TurretFrames {
    pub handles: Vec<Handle<Image>>,
    pub index: usize,
}

#[derive(Component, Deref, DerefMut)]
pub struct TurretAnimationTimer(pub Timer);

// ── Resource ───────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct TurretRes {
    pub frames: Vec<Handle<Image>>,
}

// ── Event ──────────────────────────────────────────────────────────────────

#[derive(Event)]
pub struct TurretShootEvent {
    pub origin: Vec3,
    pub speed: f32,
    /// Current animation frame index (0-7). Odd = 45-degree rotated frame.
    pub frame_index: usize,
}

// ── Asset loading ──────────────────────────────────────────────────────────

pub fn load(mut commands: Commands, asset_server: Res<AssetServer>) {
    let frames = vec![
        asset_server.load("turret/turret_mob_animation1.png"),
        asset_server.load("turret/turret_mob_animation2.png"),
        asset_server.load("turret/turret_mob_animation3.png"),
        asset_server.load("turret/turret_mob_animation4.png"),
        asset_server.load("turret/turret_mob_animation5.png"),
        asset_server.load("turret/turret_mob_animation6.png"),
        asset_server.load("turret/turret_mob_animation7.png"),
        asset_server.load("turret/turret_mob_animation8.png"),
    ];
    commands.insert_resource(TurretRes { frames });
}

// ── Spawn ──────────────────────────────────────────────────────────────────

// Turrets move at 30% of normal enemy speed
const TURRET_SPEED: f32 = ENEMY_SPEED * 0.3;

pub fn spawn_at(
    commands: &mut Commands,
    res: &TurretRes,
    at: Vec3,
    active: bool,
    health_multiplier: f32,
    speed_bonus: f32,
) {
    let hp = 60.0 * health_multiplier;
    let mut e = commands.spawn((
        Sprite::from_image(res.frames[0].clone()),
        Transform { translation: at, ..Default::default() },
        Enemy,
        TurretEnemy,
        Velocity::new(),
        Health::new(hp),
        MaxHealth(hp),
        super::EnemyMoveSpeed(TURRET_SPEED + speed_bonus * 0.5),
        TurretAnimationTimer(Timer::from_seconds(ANIM_TIME, TimerMode::Repeating)),
        TurretFrames { handles: res.frames.clone(), index: 0 },
        TurretAI {
            range: 450.0,
            fire_cooldown: Timer::from_seconds(1.0, TimerMode::Repeating),
            projectile_speed: 500.0,
        },
        super::EnemyPathfinder::new(),
        PulledByFluid { mass: 10.0 },
        GameEntity,
    ));
    e.with_children(|parent| spawn_health_bar_children(parent));
    if active {
        e.insert(ActiveEnemy);
    }
}

pub use spawn_at as spawn_turret_enemy_at;

// ── Systems ────────────────────────────────────────────────────────────────

pub fn animate(
    time: Res<Time>,
    mut query: Query<
        (&mut Sprite, &mut TurretAnimationTimer, &mut TurretFrames),
        With<TurretEnemy>,
    >,
) {
    for (mut sprite, mut timer, mut frames) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() && !frames.handles.is_empty() {
            frames.index = (frames.index + 1) % frames.handles.len();
            sprite.image = frames.handles[frames.index].clone();
        }
    }
}

pub fn ai(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemies: Query<
        (
            &Transform,
            &mut Velocity,
            &mut TurretAI,
            &TurretFrames,
            Option<&super::EnemyMoveSpeed>,
            Option<&super::EnemyPathfinder>,
        ),
        With<TurretEnemy>,
    >,
    mut shoot_writer: EventWriter<TurretShootEvent>,
    lvlstate: Res<LevelState>,
) {
    let Ok(player_tf) = player_query.single() else { return };
    let player_pos = player_tf.translation.truncate();

    let difficulty_mult: f32 = match *lvlstate {
        LevelState::InRoom(idx, _, _) | LevelState::EnteredRoom(idx) => 1.0 + (idx as f32 * 0.10),
        LevelState::NotRoom => 1.0,
    };

    for (enemy_tf, mut vel, mut ai, frames, spd_opt, pathfinder_opt) in &mut enemies {
        let max_speed = spd_opt.map_or(TURRET_SPEED, |s| s.0);
        let scaled_dt = time.delta_secs() * difficulty_mult;
        ai.fire_cooldown.tick(Duration::from_secs_f32(scaled_dt));

        let enemy_pos = enemy_tf.translation.truncate();
        let diff = player_pos - enemy_pos;
        let dist = diff.length();
        if dist == 0.0 { continue; }

        let to_player = diff / dist;
        let accel = ENEMY_ACCEL * 0.4 * time.delta_secs();

        let has_waypoints = pathfinder_opt.map_or(false, |pf| !pf.waypoints.is_empty());

        if has_waypoints {
            let wp = pathfinder_opt.unwrap().waypoints[0];
            let dir = (wp - enemy_pos).normalize_or_zero();
            vel.velocity = (vel.velocity + dir * accel).clamp_length_max(max_speed);
        } else {
            let desired = ai.range * 0.75;
            let delta = dist - desired;
            let move_dir = if delta > 20.0 {
                to_player
            } else if delta < -20.0 {
                -to_player
            } else {
                Vec2::ZERO
            };
            vel.velocity = (vel.velocity + move_dir * accel).clamp_length_max(max_speed);

            if ai.fire_cooldown.finished() && dist <= ai.range {
                shoot_writer.write(TurretShootEvent {
                    origin: enemy_tf.translation,
                    speed: ai.projectile_speed,
                    frame_index: frames.index,
                });
                ai.fire_cooldown.reset();
            }
        }
    }
}

// ── Bullet stats ───────────────────────────────────────────────────────────

const TURRET_BULLET_DAMAGE: f32 = 12.0;
const TURRET_BULLET_SCALE: f32 = 0.3;

pub fn spawn_turret_bullets(
    mut commands: Commands,
    mut events: EventReader<TurretShootEvent>,
    bullet_res: Res<EnemyBulletRes>,
    weapon_sounds: Res<WeaponSounds>,
) {
    for ev in events.read() {
        // Odd frame indices (files 2,4,6,8) are 45-degree rotated — fire diagonally.
        let dirs: [Vec2; 4] = if ev.frame_index % 2 == 1 {
            let d = std::f32::consts::FRAC_1_SQRT_2;
            [Vec2::new(d, d), Vec2::new(-d, d), Vec2::new(d, -d), Vec2::new(-d, -d)]
        } else {
            [Vec2::Y, Vec2::NEG_Y, Vec2::X, Vec2::NEG_X]
        };
        for dir in dirs {
            let spawn_pos = ev.origin.truncate() + dir * 16.0;
            commands.spawn((
                Sprite::from_atlas_image(
                    bullet_res.0.clone(),
                    TextureAtlas { layout: bullet_res.1.clone(), index: 0 },
                ),
                Transform {
                    translation: Vec3::new(spawn_pos.x, spawn_pos.y, 5.0),
                    scale: Vec3::splat(TURRET_BULLET_SCALE),
                    ..Default::default()
                },
                crate::bullet::Velocity(dir * ev.speed),
                Bullet,
                BulletOwner::Enemy,
                Collider { half_extents: Vec2::splat(5.0) },
                BulletDamage(TURRET_BULLET_DAMAGE),
                AnimationTimer(Timer::from_seconds(0.2, TimerMode::Repeating)),
                AnimationFrameCount(3),
                GameEntity,
            ));
        }

        commands.spawn((
            AudioPlayer::new(weapon_sounds.laser.clone()),
            PlaybackSettings::DESPAWN,
        ));
    }
}
