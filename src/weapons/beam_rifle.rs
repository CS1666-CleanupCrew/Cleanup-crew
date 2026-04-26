use bevy::prelude::*;
use super::{Weapon, WeaponType, BulletDamage};
use crate::bullet::{Bullet, BulletOwner, HitEnemies};
use crate::collidable::Collider;
use crate::GameEntity;

#[derive(Resource)]
pub struct BeamRifleRes {
    pub bullet: Handle<Image>,
}

pub fn new() -> Weapon {
    Weapon {
        weapon_type: WeaponType::BeamRifle,
        fire_rate: 0.008,
        bullet_speed: 1800.0,
        damage: 8.0,
        bullet_size: 0.5,
        shoot_timer: Timer::from_seconds(0.08, TimerMode::Once),
        piercing_pickups: 0,
    }
}

pub fn spawn_bullet(commands: &mut Commands, res: &BeamRifleRes, weapon: &Weapon, pos: Vec2, dir: Vec2) {
    let angle = dir.y.atan2(dir.x);
    commands.spawn((
        Sprite::from_image(res.bullet.clone()),
        Transform {
            translation: Vec3::new(pos.x, pos.y, 910.0),
            rotation: Quat::from_rotation_z(angle),
            scale: Vec3::splat(weapon.bullet_size),
        },
        crate::bullet::Velocity(dir.normalize_or_zero() * weapon.bullet_speed),
        Bullet,
        BulletOwner::Player,
        Collider { half_extents: Vec2::new(15.0, 2.0) },
        BulletDamage(weapon.damage),
        HitEnemies::default(),
        GameEntity,
    ));
}
