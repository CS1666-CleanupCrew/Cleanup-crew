use bevy::prelude::*;
use crate::bullet::aabb_overlap;
use crate::{TILE_SIZE, GameState};
use crate::player::{Player, Facing, FacingDirection};
use crate::collidable::{Collider, Collidable};
use crate::enemy::Enemy;
use crate::window::{Health, GlassState, Window};

#[derive(Component)]
pub struct Broom;

#[derive(Component)]
pub struct BroomSwing {
    pub timer: Timer,
    pub active: bool,
}

use crate::bullet::Bullet;

pub struct BroomPlugin;

impl Plugin for BroomPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, broom_input.run_if(in_state(GameState::Playing)))
           .add_systems(Update, broom_swing_system.run_if(in_state(GameState::Playing)))
           .add_systems(Update, broom_hit_enemies_system.run_if(in_state(GameState::Playing)))
           .add_systems(Update, broom_fix_window.run_if(in_state(GameState::Playing)))
           .add_systems(Update, broom_hit_bullets_system.run_if(in_state(GameState::Playing)));
    }
}

fn distance_point_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let t = (p - a).dot(ab) / ab.length_squared();
    let t = t.clamp(0.0, 1.0);
    let proj = a + ab * t;
    p.distance(proj)
}

pub fn broom_hit_bullets_system(
    mut commands: Commands,
    broom_query: Query<(&Transform, &Collider), With<Broom>>,
    bullet_query: Query<(Entity, &Transform, &Collider), With<Bullet>>,
) {
    let (broom_transform, broom_collider) = match broom_query.get_single() {
        Ok(b) => b,
        Err(_) => return, // No broom active
    };

    let broom_center = broom_transform.translation.truncate();
    let broom_half = broom_collider.half_extents;

    for (bullet_entity, bullet_transform, bullet_collider) in bullet_query.iter() {
        let bullet_center = bullet_transform.translation.truncate();
        let bullet_half = bullet_collider.half_extents;

        let overlap =
            (broom_center.x - bullet_center.x).abs() < (broom_half.x + bullet_half.x) &&
            (broom_center.y - bullet_center.y).abs() < (broom_half.y + bullet_half.y);

        if overlap {
            if let Ok(mut ec) = commands.get_entity(bullet_entity) { ec.despawn(); }
        }
    }
}

fn aabb_capsule_hit(
    aabb_center: Vec2,
    aabb_half: Vec2,
    seg_a: Vec2,
    seg_b: Vec2,
    radius: f32,
) -> bool {
    // Broad phase
    let seg_min = seg_a.min(seg_b) - Vec2::splat(radius) - aabb_half;
    let seg_max = seg_a.max(seg_b) + Vec2::splat(radius) + aabb_half;

    if aabb_center.x < seg_min.x || aabb_center.x > seg_max.x ||
       aabb_center.y < seg_min.y || aabb_center.y > seg_max.y {
        return false;
    }

    // Precise
    let dist = distance_point_to_segment(aabb_center, seg_a, seg_b);
    dist <= radius + aabb_half.length() * 0.5
}



fn broom_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    player_query: Query<(&Transform, &Facing), (With<Player>, Without<Broom>)>,
    broom_q: Query<Entity, (With<Broom>, Without<Player>)>,
) {
    if (keyboard.pressed(KeyCode::KeyC) || keyboard.pressed(KeyCode::KeyB)) && broom_q.is_empty() {
        if let Some((player_tf, facing)) = player_query.iter().next() {

            let broom_length = TILE_SIZE * 2.0;
            let broom_width  = TILE_SIZE * 1.0;

            let broom_pos = player_tf.translation + match facing.0 {
                FacingDirection::Up        => Vec3::new(0.0,  broom_length/2.0, 1.0),
                FacingDirection::Down      => Vec3::new(0.0, -broom_length/2.0, 1.0),
                FacingDirection::Left      => Vec3::new(-broom_length/2.0, 0.0, 1.0),
                FacingDirection::Right     => Vec3::new( broom_length/2.0, 0.0, 1.0),
                FacingDirection::UpRight   => Vec3::new( broom_length/2.0,  broom_length/2.0, 1.0),
                FacingDirection::UpLeft    => Vec3::new(-broom_length/2.0,  broom_length/2.0, 1.0),
                FacingDirection::DownRight => Vec3::new( broom_length/2.0, -broom_length/2.0, 1.0),
                FacingDirection::DownLeft  => Vec3::new(-broom_length/2.0, -broom_length/2.0, 1.0),
            };

            let broom_image: Handle<Image> = asset_server.load("Broom.png");

            commands.spawn((
                Sprite {
                    image: broom_image,
                    custom_size: Some(Vec2::new(broom_length, broom_width)),
                    anchor: bevy::sprite::Anchor::CenterLeft,
                    ..default()
                },
                Transform {
                    translation: broom_pos,
                    ..default()
                },
                Broom,
                BroomSwing {
                    timer: Timer::from_seconds(0.25, TimerMode::Once),
                    active: true,
                },
                Collider::from_size(Vec2::new(broom_length, broom_width)),
                Collidable,
            ));
        }
    }
}


fn broom_swing_system(
    time: Res<Time>,
    mut commands: Commands,
    player_query: Query<(&Transform, &Facing), (With<Player>, Without<Broom>)>,
    mut broom_query: Query<(Entity, &mut Transform, &mut BroomSwing), (With<Broom>, Without<Player>)>,
) {
    if let Some((player_tf, facing)) = player_query.iter().next() {
        for (entity, mut broom_tf, mut swing) in &mut broom_query {
            swing.timer.tick(time.delta());

            if swing.active {
                let broom_length = TILE_SIZE * 2.0;

                let sweep = (-90.0_f32).to_radians()
                    + (swing.timer.elapsed_secs() / swing.timer.duration().as_secs_f32()) 
                    * (180.0_f32).to_radians();

                let base_angle = match facing.0 {
                    FacingDirection::Up        => std::f32::consts::FRAC_PI_2,
                    FacingDirection::Down      => -std::f32::consts::FRAC_PI_2,
                    FacingDirection::Left      => std::f32::consts::PI,
                    FacingDirection::Right     => 0.0,
                    FacingDirection::UpRight   => std::f32::consts::FRAC_PI_4,
                    FacingDirection::UpLeft    => 3.0 * std::f32::consts::FRAC_PI_4,
                    FacingDirection::DownRight => -std::f32::consts::FRAC_PI_4,
                    FacingDirection::DownLeft  => -3.0 * std::f32::consts::FRAC_PI_4,
                };

                broom_tf.rotation = Quat::from_rotation_z(base_angle + sweep);
                broom_tf.translation =
                    player_tf.translation + broom_tf.rotation * Vec3::new(broom_length / 2.0, 0.0, 0.0);

                if swing.timer.finished() {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}


pub fn broom_hit_enemies_system(
    mut enemies: Query<(&mut Health, &Transform, &Sprite), (With<Enemy>, Without<Broom>)>,
    broom_query: Query<(&Transform, &Sprite), (With<Broom>, Without<Enemy>)>,
) {
    if let Some((broom_tf, broom_sprite)) = broom_query.iter().next() {
        let broom_size = broom_sprite.custom_size.unwrap();

        for (mut health, enemy_tf, enemy_sprite) in enemies.iter_mut() {
            let enemy_size = enemy_sprite.custom_size.unwrap();

            if aabb_overlap(
                broom_tf.translation.x,
                broom_tf.translation.y,
                broom_size,
                enemy_tf.translation.x,
                enemy_tf.translation.y,
                enemy_size,
            ) {
                health.0 -= 10.0;
                info!("Enemy hit by broom at {:?}", enemy_tf.translation);
            }
        }
    }
}






pub fn broom_fix_window(
    mut window_query: Query<(&mut Health, &mut GlassState, &Transform, &Sprite), (With<Window>, Without<Broom>)>,
    broom_query: Query<(&Transform, &Sprite), (With<Broom>, Without<Window>)>,
) {
    if let Some((broom_tf, broom_sprite)) = broom_query.iter().next() {
        let broom_size = broom_sprite.custom_size.unwrap();

        for (mut health, state, window_tf, window_sprite) in window_query.iter_mut() {
            let window_size = window_sprite.custom_size.unwrap();

            if aabb_overlap(
                broom_tf.translation.x,
                broom_tf.translation.y,
                broom_size,
                window_tf.translation.x,
                window_tf.translation.y,
                window_size,
            ) {
                if *state == GlassState::Broken {
                    health.0 += 20.0;
                    info!("Broom repaired window at {:?}", window_tf.translation);
                }
            }
        }
    }
}