use bevy::{prelude::*, window::PrimaryWindow};
use crate::collidable::{Collidable, Collider};
use crate::table;
use crate::window;
use crate::Player;
use crate::{GameState, TILE_SIZE};
use crate::enemy::RangedEnemyShootEvent;




const BULLET_SPEED: f32 = 600.0;

#[derive(Resource)]
pub struct BulletRes(Handle<Image>, Handle<TextureAtlasLayout>);

#[derive(Component)]
pub struct Bullet;
pub struct BulletPlugin;

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(Timer);

#[derive(Component, Deref, DerefMut)]
pub struct AnimationFrameCount(usize);

impl Plugin for BulletPlugin {
    fn build(&self, app:&mut App) {
        app.add_systems(Startup, load_bullet)
            .add_systems(Update, shoot_bullet_on_click)
            .add_systems(Update,spawn_bullets_from_ranged.run_if(in_state(GameState::Playing)),)
            .add_systems(Update, move_bullets.run_if(in_state(GameState::Playing)))
            .add_systems(Update, bullet_collision.run_if(in_state(GameState::Playing)))
            .add_systems(Update, animate_bullet.after(move_bullets).run_if(in_state(GameState::Playing)),)
            .add_systems(Update, bullet_hits_enemy.run_if(in_state(GameState::Playing)))
            .add_systems(Update, bullet_hits_table.run_if(in_state(GameState::Playing)))
            .add_systems(Update, bullet_hits_window.run_if(in_state(GameState::Playing)));
    }
}


fn load_bullet(
    mut commands: Commands, 
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
){  
    let bullet_animate_image: Handle<Image> = asset_server.load("bullet_animation.png");

    let bullet_animate_layout = TextureAtlasLayout::from_grid(UVec2::splat(100), 3, 1, None, None);
    let bullet_animate_handle = texture_atlases.add(bullet_animate_layout);

    commands.insert_resource(BulletRes(bullet_animate_image, bullet_animate_handle));
}


fn cursor_to_world(
    cursor_pos: Vec2,
    camera: (&Camera, &GlobalTransform),
) -> Option<Vec2> {
    camera.0.viewport_to_world_2d(camera.1, cursor_pos).ok()
}


pub fn shoot_bullet_on_click(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    q_player: Query<&Transform, With<crate::player::Player>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    bullet_animate: Res<BulletRes>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let window = match q_window.single() {
        Ok(win) => win,
        Err(_) => return,
    };

    let Some(cursor_pos) = window.cursor_position() else { return; };

    let (camera, cam_transform) = match q_camera.single() {
        Ok(c) => c,
        Err(_) => return,
    };

    let Some(world_pos) = cursor_to_world(cursor_pos, (camera, cam_transform)) else { return; };

    let Ok(player_transform) = q_player.single() else { return; };
    let player_pos = player_transform.translation.truncate();

    let dir_vec = (world_pos - player_pos).normalize_or_zero();
    if dir_vec == Vec2::ZERO {
        return;
    }

    let shoot_offset = 16.0;
    let spawn_pos = player_pos + dir_vec * shoot_offset;

    commands.spawn((
        Sprite::from_atlas_image(
            bullet_animate.0.clone(),
            TextureAtlas {
                layout: bullet_animate.1.clone(),
                index: 0,
            },
        ),
        Transform {
            translation: Vec3::new(spawn_pos.x, spawn_pos.y, 5.0),
            scale: Vec3::splat(0.25),
            ..Default::default()
        },
        Velocity(dir_vec * BULLET_SPEED),
        Bullet,
        Collider { half_extents: Vec2::splat(5.0) },
    ));
}

pub fn spawn_bullets_from_ranged(
    mut commands: Commands,
    mut events: EventReader<RangedEnemyShootEvent>,
    bullet_animate: Res<BulletRes>,
) {
    for ev in events.read() {
        let origin = ev.origin;
        let dir = ev.direction.normalize_or_zero();
        if dir == Vec2::ZERO {
            continue;
        }

        // Small offset so the bullet isn't inside the ranger sprite
        let spawn_pos = origin.truncate() + dir * 16.0;

        commands.spawn((
            Sprite::from_atlas_image(
                bullet_animate.0.clone(),
                TextureAtlas {
                    layout: bullet_animate.1.clone(),
                    index: 0,
                },
            ),
            Transform {
                translation: Vec3::new(spawn_pos.x, spawn_pos.y, 5.0),
                scale: Vec3::splat(0.25),
                ..Default::default()
            },
            Velocity(dir * ev.speed),            // bullet.rs's Velocity
            Bullet,
            Collider { half_extents: Vec2::splat(5.0) },
        ));
    }
}





#[derive(Component, Deref, DerefMut)]
pub struct Velocity(pub Vec2);

pub fn move_bullets(
    mut commands: Commands,
    mut bullet_q: Query<(Entity, &mut Transform, &Velocity), With<Bullet>>,
    time: Res<Time>,
) {
    for (entity, mut transform, vel) in bullet_q.iter_mut() {
        transform.translation += (vel.0 * time.delta_secs()).extend(0.0);

        // Despawn off-screen bullets (optional)
        if transform.translation.length() > 5000.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn bullet_collision(
    mut commands: Commands,
    bullet_query: Query<(Entity, &Transform, &Collider), With<Bullet>>,
    colliders: Query<(&Transform, &Collider), (With<Collidable>, Without<Player>, Without<Bullet>, Without<crate::enemy::Enemy>, Without<table::Table>,)>,
) {
    for (bullet_entity, bullet_transform, bullet_collider) in &bullet_query {
        let bx = bullet_transform.translation.x;
        let by = bullet_transform.translation.y;
        let b_half = bullet_collider.half_extents;

        // Check collision with all collidable entities
        for (collider_transform, collider) in &colliders {
            let cx = collider_transform.translation.x;
            let cy = collider_transform.translation.y;
            let c_half = collider.half_extents;

            if aabb_overlap(bx, by, b_half, cx, cy, c_half) {
                commands.entity(bullet_entity).despawn();
                break;
            }
        }
    }
}

fn animate_bullet(
    time: Res<Time>,
    mut bullet: Query<
        (
            &mut Sprite,
            &mut AnimationTimer,
            &AnimationFrameCount,
        ),
        With<Bullet>,
    >,
) {
    for (mut sprite, mut timer, frame_count) in &mut bullet{
        timer.tick(time.delta());

        if timer.just_finished() {
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = (atlas.index + 1) % **frame_count;
            }
        }
    }
}

fn bullet_hits_enemy(
    mut enemy_query: Query<(&Transform, &mut crate::enemy::Health), With<crate::enemy::Enemy>>,
    bullet_query: Query<(&Transform, Entity), With<Bullet>>,
    mut commands: Commands,
) {
    let bullet_half = Vec2::splat(TILE_SIZE * 0.5);
    let enemy_half = Vec2::splat(crate::enemy::ENEMY_SIZE * 0.5);
    for (bullet_tf, bullet_entity) in &bullet_query {
        let bullet_pos = bullet_tf.translation;
        for (enemy_tf, mut health) in &mut enemy_query {
            let enemy_pos = enemy_tf.translation;
            if aabb_overlap(
                bullet_pos.x, bullet_pos.y, bullet_half,
                enemy_pos.x, enemy_pos.y, enemy_half,
            ) {
                health.0 -= 25.0;
                commands.entity(bullet_entity).despawn();
            }
        }
    }
}

fn bullet_hits_table(
    mut commands: Commands,
    mut table_query: Query<(&Transform, &mut table::Health, &table::TableState), With<table::Table>>,
    bullet_query: Query<(Entity, &Transform), With<Bullet>>,
) {
    let bullet_half = Vec2::splat(8.0); // Bullet's collider size
    let table_half = Vec2::splat(TILE_SIZE * 0.5); // Table's collider size

    'bullet_loop: for (bullet_entity, bullet_tf) in &bullet_query {
        let bullet_pos = bullet_tf.translation;
        for (table_tf, mut health, state) in &mut table_query {
            if *state == table::TableState::Intact{
                let table_pos = table_tf.translation;
                if aabb_overlap(
                    bullet_pos.x,
                    bullet_pos.y,
                    bullet_half,
                    table_pos.x,
                    table_pos.y,
                    table_half,
                ) {
                    health.0 -= 25.0; // Deal 25 damage
                    commands.entity(bullet_entity).despawn(); // Despawn bullet on hit
                    continue 'bullet_loop; // Move to the next bullet
                }
            }
        }
    }
}

fn bullet_hits_window(
    mut commands: Commands,
    mut window_query: Query<(&Transform, &mut window::Health, &window::GlassState), With<window::Window>>,
    bullet_query: Query<(Entity, &Transform), With<Bullet>>,
) {
    let bullet_half = Vec2::splat(8.0); // Bullet's collider size
    let window_half = Vec2::splat(TILE_SIZE * 0.5); // window's collider size

    'bullet_loop: for (bullet_entity, bullet_tf) in &bullet_query {
        let bullet_pos = bullet_tf.translation;
        for (window_tf, mut health, state) in &mut window_query {
            if *state == window::GlassState::Intact{
                let window_pos = window_tf.translation;
                if aabb_overlap(
                    bullet_pos.x,
                    bullet_pos.y,
                    bullet_half,
                    window_pos.x,
                    window_pos.y,
                    window_half,
                ) {
                    health.0 -= 25.0; // Deal 25 damage
                    commands.entity(bullet_entity).despawn(); // Despawn bullet on hit
                    continue 'bullet_loop; // Move to the next bullet
                }
            }
        }
    }
}

pub fn aabb_overlap(
    ax: f32, ay: f32, a_half: Vec2,
    bx: f32, by: f32, b_half: Vec2
) -> bool {
    (ax - bx).abs() < (a_half.x + b_half.x) &&
    (ay - by).abs() < (a_half.y + b_half.y)
}