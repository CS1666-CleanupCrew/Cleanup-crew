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


pub fn shoot_bullet_on_click(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    q_player: Query<&Transform, With<crate::player::Player>>,
    // asset_server: Res<AssetServer>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    bullet_animate: Res<BulletRes>,

) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let window = q_window.single().expect("Primary window not found");
    let screen_pos: Vec2 = window.cursor_position().unwrap_or(Vec2::ZERO);
    let world_pos = Vec2::new(
        screen_pos.x - WIN_W / 2.0,
        -1.0*(screen_pos.y - WIN_H / 2.0)
    );


    let Ok(player) = q_player.single() else { return; };
    let player_pos = player.translation.truncate();

    
    let dir = BULLET_SPEED * (world_pos - player_pos).normalize_or_zero();


    // Spawn the bullet
    commands.spawn((
        Sprite::from_atlas_image(
            bullet_animate.0.clone(),
            TextureAtlas { 
                layout: bullet_animate.1.clone(),
                index: 0, 
            },
        ),
        Transform{
            translation: Vec3::new(player_pos.x, player_pos.y, 5.),
            scale: Vec3::splat(0.25),
            ..Default::default()
        },
        // AnimationTimer(Timer::from_seconds(0.2, TimerMode::Repeating)),
        // AnimationFrameCount(3),
        Velocity(dir),
        Bullet,
        // Collidable,
        Collider {
            half_extents: Vec2::splat(5.0), // adjust to bullet size
        },
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