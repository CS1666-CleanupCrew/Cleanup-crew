use bevy::prelude::*;
use crate::{TILE_SIZE, GameState};
use crate::player::{Player, Facing, FacingDirection};
use crate::collidable::{Collider, Collidable};
use crate::enemy::Enemy;

#[derive(Component)]
pub struct Broom;

#[derive(Component)]
pub struct BroomSwing {
    pub timer: Timer,
    pub active: bool,
}

pub struct BroomPlugin;

impl Plugin for BroomPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, broom_input.run_if(in_state(GameState::Playing)))
           .add_systems(Update, broom_swing_system.run_if(in_state(GameState::Playing)))
           .add_systems(Update, broom_hit_enemies_system.run_if(in_state(GameState::Playing)));
    }
}

fn broom_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    player_query: Query<(&Transform, &Facing), With<Player>>,
    broom_q: Query<Entity, (With<Broom>, Without<Player>)>,
) {
    if keyboard.just_pressed(KeyCode::KeyB) {
        if broom_q.is_empty() {
            if let Some((player_tf, facing)) = player_query.iter().next() {
                let broom_length = TILE_SIZE * 2.0;
                let broom_width = TILE_SIZE * 1.0;

                let broom_pos = player_tf.translation + match facing.0 {
                    FacingDirection::Up => Vec3::new(0.0, broom_length / 2.0, 1.0),
                    FacingDirection::Down => Vec3::new(0.0, -broom_length / 2.0, 1.0),
                    FacingDirection::Left => Vec3::new(-broom_length / 2.0, 0.0, 1.0),
                    FacingDirection::Right => Vec3::new(broom_length / 2.0, 0.0, 1.0),
                    FacingDirection::UpRight => Vec3::new(broom_length / 2.0, broom_length / 2.0, 1.0),
                    FacingDirection::UpLeft => Vec3::new(-broom_length / 2.0, broom_length / 2.0, 1.0),
                    FacingDirection::DownRight => Vec3::new(broom_length / 2.0, -broom_length / 2.0, 1.0),
                    FacingDirection::DownLeft => Vec3::new(-broom_length / 2.0, -broom_length / 2.0, 1.0),
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
}

fn broom_swing_system(
    time: Res<Time>,
    mut commands: Commands,
    player_query: Query<(&Transform, &Facing), (With<Player>, Without<Broom>)>,
    mut broom_query: Query<(Entity, &mut Transform, &mut BroomSwing), (With<Broom>, Without<Player>)>,
) {
    if let Some((player_tf, facing)) = player_query.iter().next() {
        for (broom_entity, mut broom_tf, mut swing) in &mut broom_query {
            swing.timer.tick(time.delta());
            if swing.active {
                let broom_length = TILE_SIZE * 2.0;
                let sweep = (-90.0_f32).to_radians() + swing.timer.elapsed_secs() / swing.timer.duration().as_secs_f32() * (180.0_f32).to_radians();
                let base_angle = match facing.0 {
                    FacingDirection::Up => std::f32::consts::FRAC_PI_2,
                    FacingDirection::Down => -std::f32::consts::FRAC_PI_2,
                    FacingDirection::Left => std::f32::consts::PI,
                    FacingDirection::Right => 0.0,
                    FacingDirection::UpRight => std::f32::consts::FRAC_PI_4,
                    FacingDirection::UpLeft => 3.0 * std::f32::consts::FRAC_PI_4,
                    FacingDirection::DownRight => -std::f32::consts::FRAC_PI_4,
                    FacingDirection::DownLeft => -3.0 * std::f32::consts::FRAC_PI_4,
                };
                broom_tf.rotation = Quat::from_rotation_z(base_angle + sweep);
                broom_tf.translation = player_tf.translation + broom_tf.rotation * Vec3::new(broom_length / 2.0, 0.0, 0.0);

                if swing.timer.finished() {
                    commands.entity(broom_entity).despawn();
                }
            }
        }
    }
}

fn broom_hit_enemies_system(
    player_query: Query<&Transform, With<Player>>,
    broom_query: Query<(&Transform, &Collider), (With<Broom>, Without<Enemy>, Without<Player>)>,
    mut enemies: Query<&mut Transform, (With<Enemy>, Without<Player>, Without<Broom>)>,
) {
    let player_tf = if let Some(tf) = player_query.iter().next() {
        tf
    } else { return; };

    let player_pos = player_tf.translation.truncate();

    for (broom_tf, broom_col) in &broom_query {
        let broom_center = broom_tf.translation.truncate();

        let half_extents = broom_col.half_extents;

        let inv_rot = broom_tf.rotation.conjugate();

        for mut enemy_tf in &mut enemies {
            let enemy_pos = enemy_tf.translation.truncate();

            let to_enemy_world = Vec3::new(enemy_pos.x - broom_center.x, enemy_pos.y - broom_center.y, 0.0);

            let to_enemy_local = inv_rot * to_enemy_world;
            let to_enemy_local2 = to_enemy_local.truncate();

            if to_enemy_local2.x.abs() < half_extents.x && to_enemy_local2.y.abs() < half_extents.y {
                info!("Enemy hit by broom!");

                let mut knockback_dir: Vec2 = (enemy_pos - broom_center).normalize_or_zero();

                if knockback_dir.length_squared() == 0.0 {
                    knockback_dir = (enemy_pos - player_pos).normalize_or_zero();
                }

                if knockback_dir.length_squared() == 0.0 {
                    knockback_dir = Vec2::Y;
                }

                let knockback_distance = TILE_SIZE * 10.0;
                enemy_tf.translation.x += knockback_dir.x * knockback_distance;
                enemy_tf.translation.y += knockback_dir.y * knockback_distance;
            }
        }
    }
}
