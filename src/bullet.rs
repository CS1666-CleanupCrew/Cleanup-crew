use bevy::{prelude::*, window::PrimaryWindow};
use crate::collidable::{Collidable, Collider};
use crate::{WIN_W,WIN_H};


const BULLET_SPEED: f32 = 600.0;

#[derive(Resource)]
pub struct BulletRes(Handle<Image>, Handle<TextureAtlasLayout>);

#[derive(Component)]
pub struct Bullet;
pub struct BulletPlugin;

impl Plugin for BulletPlugin {
    fn build(&self, app:&mut App) {
        app
            .add_systems(Startup, load_bullet)
            .add_systems(Update, shoot_bullet_on_click)
            .add_systems(Update, move_bullets);
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
    window: &Window,
    camera_transform: &GlobalTransform,
) -> Vec2 {
    // Convert screen position [0..width]x[0..height] to NDC [-1..1]
    let ndc = Vec2::new(
        (cursor_pos.x / window.width()) * 2.0 - 1.0,
        ((window.height() - cursor_pos.y) / window.height()) * 2.0 - 1.0,
    );

    // Project NDC to world space
    let world_pos_3: Vec3 =
        (camera_transform.compute_matrix() * ndc.extend(0.0).extend(1.0)).truncate();
    world_pos_3.truncate() // Vec3 -> Vec2
}

pub fn shoot_bullet_on_click(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    q_player: Query<&Transform, With<crate::player::Player>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<&GlobalTransform, With<Camera>>,
    bullet_animate: Res<BulletRes>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    // Get window
    let window = match q_window.single() {
        Ok(win) => win,
        Err(_) => return,
    };

    // Get cursor position
    let Some(cursor_pos) = window.cursor_position() else { return };

    // Get camera transform
    let camera_transform = match q_camera.single() {
        Ok(cam) => cam,
        Err(_) => return,
    };

    // Convert cursor to world coordinates
    let world_pos = cursor_to_world(cursor_pos, window, camera_transform);

    // Get player position
    let Ok(player_transform) = q_player.single() else { return };
    let player_pos = player_transform.translation.truncate();

    // Compute direction vector
    let dir_vec = (world_pos - player_pos).normalize();

    // Spawn position slightly in front of player (muzzle offset)
    let shoot_offset = 16.0; // adjust based on your sprite size
    let spawn_pos = player_pos + dir_vec * shoot_offset;

    // Spawn the bullet
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