use bevy::prelude::*;
use super::{Weapon, WeaponType};

pub fn new() -> Weapon {
    Weapon {
        weapon_type: WeaponType::Zapper,
        fire_rate: 0.7,
        bullet_speed: 700.0,
        damage: 25.0,
        bullet_size: 0.25,
        shoot_timer: Timer::from_seconds(0.5, TimerMode::Once),
        piercing_pickups: 0,
    }
}
