use bevy::prelude::*;
use crate::GameEntity;
use crate::bullet::{Bullet, BulletOwner, Velocity, AnimationTimer, AnimationFrameCount, Piercing};
use crate::collidable::Collider;

#[derive(Component, Clone)]
pub struct Weapon {
    pub weapon_type: WeaponType,
    pub fire_rate: f32,           // seconds between shots
    pub bullet_speed: f32,
    pub damage: f32,
    pub bullet_size: f32,
    pub shoot_timer: Timer,
    pub piercing: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WeaponType {
    BasicLaser,

}

impl Weapon {
    pub fn new(weapon_type: WeaponType) -> Self {
        match weapon_type {
            WeaponType::BasicLaser => Self {
                weapon_type,
                fire_rate: 0.5,
                bullet_speed: 700.0,
                damage: 25.0,
                bullet_size: 0.25,
                shoot_timer: Timer::from_seconds(0.5, TimerMode::Once),
                piercing: false,
            },
            // Add more weapon types here:
            // WeaponType::RapidFire => Self { ... },
        }
    }

    pub fn can_shoot(&self) -> bool {
        self.shoot_timer.finished()
    }

    pub fn reset_timer(&mut self) {
        self.shoot_timer.reset();
    }

    pub fn tick(&mut self, delta: std::time::Duration) {
        self.shoot_timer.tick(delta);
    }
}

#[derive(Resource)]
pub struct BulletRes(pub Handle<Image>, pub Handle<TextureAtlasLayout>);

#[derive(Resource)]
pub struct WeaponSounds {
    pub laser: Handle<AudioSource>,
}



pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_weapon_assets)
            .add_systems(Update, update_weapon_timers);
    }
}

fn load_weapon_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Load bullet graphics
    let bullet_animate_image: Handle<Image> = asset_server.load("bullet_animation.png");
    let bullet_animate_layout = TextureAtlasLayout::from_grid(UVec2::splat(100), 3, 1, None, None);
    let bullet_animate_handle = texture_atlases.add(bullet_animate_layout);
    commands.insert_resource(BulletRes(bullet_animate_image, bullet_animate_handle));

    // Load weapon sounds
    let laser_sound: Handle<AudioSource> = asset_server.load("audio/laser_zap.ogg");
    commands.insert_resource(WeaponSounds { laser: laser_sound });
}

fn update_weapon_timers(
    time: Res<Time>,
    mut weapons: Query<&mut Weapon>,
) {
    for mut weapon in &mut weapons {
        weapon.tick(time.delta());
    }
}

// Spawn bullet based on weapon stats
pub fn spawn_bullet(
    commands: &mut Commands,
    bullet_res: &BulletRes,
    weapon: &Weapon,
    pos: Vec2,
    dir: Vec2,
) {
    let normalized_dir = dir.normalize_or_zero();

    let mut bullet = commands.spawn((
        Sprite::from_atlas_image(
            bullet_res.0.clone(),
            TextureAtlas {
                layout: bullet_res.1.clone(),
                index: 0,
            },
        ),
        Transform {
            translation: Vec3::new(pos.x, pos.y, 910.0),
            scale: Vec3::splat(weapon.bullet_size),
            ..Default::default()
        },
        AnimationTimer(Timer::from_seconds(0.2, TimerMode::Repeating)),
        AnimationFrameCount(3),
        Velocity(normalized_dir * weapon.bullet_speed),
        Bullet,
        BulletOwner::Player,
        Collider {
            half_extents: Vec2::splat(5.0),
        },
        BulletDamage(weapon.damage),
        GameEntity,
    ));
    if weapon.piercing {
        bullet.insert(Piercing);
    }
}

// New component to track bullet damage
#[derive(Component)]
pub struct BulletDamage(pub f32);