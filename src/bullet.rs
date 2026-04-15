use crate::Player;
use crate::collidable::Collider;
use crate::enemies::RangedEnemyShootEvent;
use crate::player::{Health, MaxHealth, MoveSpeed, Shield};
use crate::room::{LevelState, RoomVec};
use crate::weapon::{BulletDamage, BulletRes, Weapon, WeaponSounds};
use crate::window;
use crate::{GameEntity, GameState, TILE_SIZE};
use crate::table;
use bevy::{prelude::*, window::PrimaryWindow};

#[derive(Component)]
pub struct Bullet;

pub struct BulletPlugin;

#[derive(Component)]
pub enum BulletOwner {
    Player,
    Enemy,
}

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

#[derive(Component, Deref, DerefMut)]
pub struct AnimationFrameCount(pub usize);

#[derive(Component)]
pub struct MarkedForDespawn;

/// Remaining pierce slots — each enemy hit consumes one slot.
/// When it reaches 0 the bullet despawns on the next hit.
#[derive(Component)]
pub struct Piercing(pub u32);

#[derive(Component, Deref, DerefMut)]
pub struct Velocity(pub Vec2);

impl Plugin for BulletPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, shoot_bullet_on_click) // Mouse shooting
            .add_systems(Update, move_bullets.run_if(in_state(GameState::Playing)))
            .add_systems(
                Update,
                bullet_collision.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Last,
                cleanup_marked_bullets.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                animate_bullet
                    .after(move_bullets)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                spawn_bullets_from_ranged.run_if(in_state(GameState::Playing)),
            );
    }
}

fn cursor_to_world(cursor_pos: Vec2, camera: (&Camera, &GlobalTransform)) -> Option<Vec2> {
    camera.0.viewport_to_world_2d(camera.1, cursor_pos).ok()
}

// Mouse shooting - uses weapon stats from player
pub fn shoot_bullet_on_click(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    mut q_player: Query<(&Transform, &mut Weapon), With<crate::player::Player>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    bullet_res: Res<BulletRes>,
    weapon_sounds: Res<WeaponSounds>,
) {
    let Ok((player_transform, mut weapon)) = q_player.single_mut() else {
        return;
    };

    if buttons.pressed(MouseButton::Left) && weapon.can_shoot() {
        let window = match q_window.single() {
            Ok(win) => win,
            Err(_) => return,
        };

        let Some(cursor_pos) = window.cursor_position() else {
            return;
        };

        let (camera, cam_transform) = match q_camera.single() {
            Ok(c) => c,
            Err(_) => return,
        };

        let Some(world_pos) = cursor_to_world(cursor_pos, (camera, cam_transform)) else {
            return;
        };

        let player_pos = player_transform.translation.truncate();

        let dir_vec = (world_pos - player_pos).normalize_or_zero();
        if dir_vec == Vec2::ZERO {
            return;
        }

        let shoot_offset = 16.0;
        let spawn_pos = player_pos + dir_vec * shoot_offset;

        // Spawn bullet using weapon stats
        let mut bullet_entity = commands.spawn((
            Sprite::from_atlas_image(
                bullet_res.0.clone(),
                TextureAtlas {
                    layout: bullet_res.1.clone(),
                    index: 0,
                },
            ),
            Transform {
                translation: Vec3::new(spawn_pos.x, spawn_pos.y, 5.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::splat(weapon.bullet_size),
            },
            Velocity(dir_vec * weapon.bullet_speed),
            Bullet,
            BulletOwner::Player,
            Collider {
                half_extents: Vec2::splat(5.0),
            },
            BulletDamage(weapon.damage),
            AnimationTimer(Timer::from_seconds(0.2, TimerMode::Repeating)),
            AnimationFrameCount(3),
            GameEntity,
        ));
        let pierce = weapon.effective_pierce_count();
        if pierce > 0 {
            bullet_entity.insert(Piercing(pierce));
        }

        commands.spawn((
            AudioPlayer::new(weapon_sounds.laser.clone()),
            PlaybackSettings::DESPAWN,
        ));

        weapon.reset_timer();
    }
}

// Enemy shooting
pub fn spawn_bullets_from_ranged(
    mut commands: Commands,
    mut events: EventReader<RangedEnemyShootEvent>,
    bullet_res: Res<BulletRes>,
    weapon_sounds: Res<WeaponSounds>,
) {
    for ev in events.read() {
        let origin = ev.origin;
        let dir = ev.direction.normalize_or_zero();
        if dir == Vec2::ZERO {
            continue;
        }

        let spawn_pos = origin.truncate() + dir * 16.0;

        commands.spawn((
            Sprite::from_atlas_image(
                bullet_res.0.clone(),
                TextureAtlas {
                    layout: bullet_res.1.clone(),
                    index: 0,
                },
            ),
            Transform {
                translation: Vec3::new(spawn_pos.x, spawn_pos.y, 5.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::splat(0.25),
            },
            Velocity(dir * ev.speed),
            Bullet,
            BulletOwner::Enemy,
            Collider {
                half_extents: Vec2::splat(5.0),
            },
            BulletDamage(10.0), // Enemy bullet damage
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

pub fn move_bullets(
    mut commands: Commands,
    mut bullet_q: Query<
        (Entity, &mut Transform, &Velocity),
        (With<Bullet>, Without<MarkedForDespawn>),
    >,
    time: Res<Time>,
) {
    for (entity, mut transform, vel) in bullet_q.iter_mut() {
        transform.translation += (vel.0 * time.delta_secs()).extend(0.0);

        let p = transform.translation;
        if p.x.abs() > 4000.0 || p.y.abs() > 4000.0 {
            commands.entity(entity).try_insert(MarkedForDespawn);
        }
    }
}

fn animate_bullet(
    time: Res<Time>,
    mut bullet: Query<(&mut Sprite, &mut AnimationTimer, &AnimationFrameCount), With<Bullet>>,
) {
    for (mut sprite, mut timer, frame_count) in &mut bullet {
        timer.tick(time.delta());

        if timer.just_finished() {
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = (atlas.index + 1) % **frame_count;
            }
        }
    }
}

pub fn bullet_collision(
    mut commands: Commands,
    mut bullet_query: Query<
        (Entity, &Transform, &BulletOwner, &BulletDamage, Option<&mut Piercing>),
        (With<Bullet>, Without<MarkedForDespawn>),
    >,
    mut enemy_query: Query<
        (&Transform, &mut crate::enemies::Health),
        (With<crate::enemies::Enemy>, Without<crate::enemies::Reaper>),
    >,
    mut player_query: Query<
        (&Transform, &mut Health, &mut MaxHealth, &mut MoveSpeed, &mut crate::player::Armor, &mut Shield),
        With<Player>,
    >,
    mut table_query: Query<
        (&Transform, &mut table::Health, &table::TableState),
        With<table::Table>,
    >,
    mut window_query: Query<
        (&Transform, &mut window::Health, &window::GlassState),
        With<window::Window>,
    >,
    wall_grid: Res<crate::map::WallGrid>,
    lvlstate: Res<LevelState>,
    rooms: Res<RoomVec>,
) {
    let bullet_half = Vec2::splat(8.0);

    let Ok((player_tf, mut hp, _maxhp, _movspd, armor, mut shield)) = player_query.single_mut() else {
        return;
    };

    let _final_room = matches!(*lvlstate, LevelState::InRoom(_, _)) && rooms.0.len() == 1;

    'bullet_loop: for (bullet_entity, bullet_tf, owner, damage, mut piercing) in &mut bullet_query {
        let bullet_pos = bullet_tf.translation;

        // Bullet hits enemy
        if matches!(owner, BulletOwner::Player) {
            for (enemy_tf, mut health) in &mut enemy_query {
                let enemy_pos = enemy_tf.translation;
                let enemy_half = Vec2::splat(crate::enemies::ENEMY_SIZE * 0.5);
                if aabb_overlap(
                    bullet_pos.x,
                    bullet_pos.y,
                    bullet_half,
                    enemy_pos.x,
                    enemy_pos.y,
                    enemy_half,
                ) {
                    health.0 -= damage.0;
                    match &mut piercing {
                        None => {
                            // Non-piercing bullet: stop on first hit
                            commands.entity(bullet_entity).try_insert(MarkedForDespawn);
                            continue 'bullet_loop;
                        }
                        Some(p) if p.0 == 0 => {
                            // Used up all pierce slots: stop on this hit
                            commands.entity(bullet_entity).try_insert(MarkedForDespawn);
                            continue 'bullet_loop;
                        }
                        Some(p) => {
                            // Consume one pierce slot and keep going
                            p.0 -= 1;
                        }
                    }
                }
            }
        }

        // Bullet hits player
        if matches!(owner, BulletOwner::Enemy) {
            let player_pos = player_tf.translation;
            let player_half = Vec2::splat(TILE_SIZE);
            if aabb_overlap(
                bullet_pos.x,
                bullet_pos.y,
                bullet_half,
                player_pos.x,
                player_pos.y,
                player_half,
            ) {
                if shield.current >= 1.0 {
                    shield.current -= 1.0;
                } else {
                    hp.0 -= damage.0 * crate::player::armor_factor(armor.0);
                }
                commands.entity(bullet_entity).try_insert(MarkedForDespawn);
                continue 'bullet_loop;
            }
        }

        // Bullet hits table
        if matches!(owner, BulletOwner::Player) {
            for (table_tf, mut table_health, state) in &mut table_query {
                if *state != table::TableState::Intact {
                    continue;
                }
                let table_pos = table_tf.translation;
                let table_half = Vec2::splat(TILE_SIZE * 0.5);
                if aabb_overlap(
                    bullet_pos.x,
                    bullet_pos.y,
                    bullet_half,
                    table_pos.x,
                    table_pos.y,
                    table_half,
                ) {
                    table_health.0 -= damage.0; // Use bullet damage
                    commands.entity(bullet_entity).try_insert(MarkedForDespawn);
                    continue 'bullet_loop;
                }
            }
        }

        // Bullet hits window
        if matches!(owner, BulletOwner::Player) {
            for (window_tf, mut window_health, state) in &mut window_query {
                if *state != window::GlassState::Intact {
                    continue;
                }
                let window_pos = window_tf.translation;
                let window_half = Vec2::splat(TILE_SIZE * 0.5);
                if aabb_overlap(
                    bullet_pos.x,
                    bullet_pos.y,
                    bullet_half,
                    window_pos.x,
                    window_pos.y,
                    window_half,
                ) {
                    window_health.0 -= damage.0; // Use bullet damage
                    commands.entity(bullet_entity).try_insert(MarkedForDespawn);
                    continue 'bullet_loop;
                }
            }
        }


        for (wall_pos, wall_half) in wall_grid.nearby(bullet_pos.truncate(), 2) {
            if aabb_overlap(
                bullet_pos.x,
                bullet_pos.y,
                bullet_half,
                wall_pos.x,
                wall_pos.y,
                wall_half,
            ) {
                commands.entity(bullet_entity).try_insert(MarkedForDespawn);
                continue 'bullet_loop;
            }
        }
    }
}

fn cleanup_marked_bullets(world: &mut World) {
    let mut to_despawn = Vec::new();

    let mut query = world.query_filtered::<Entity, (With<Bullet>, With<MarkedForDespawn>)>();
    for entity in query.iter(world) {
        to_despawn.push(entity);
    }

    for entity in to_despawn {
        if let Ok(entity_mut) = world.get_entity_mut(entity) {
            entity_mut.despawn();
        }
    }
}

pub fn aabb_overlap(ax: f32, ay: f32, a_half: Vec2, bx: f32, by: f32, b_half: Vec2) -> bool {
    (ax - bx).abs() < (a_half.x + b_half.x) && (ay - by).abs() < (a_half.y + b_half.y)
}

