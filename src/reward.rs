use bevy::prelude::*;
use rand::random_range;
use std::time::Duration;
use crate::{TILE_SIZE, GameEntity};
use crate::Player;
use crate::player::{Health, MaxHealth, MoveSpeed, Armor, AirTank, aabb_overlap};
use crate::weapon::Weapon;

#[derive(Component)]
pub struct Reward(pub usize);

#[allow(dead_code)]
#[derive(Resource)]
pub struct RewardRes{
    max_hp: Handle<Image>,
    atk_spd: Handle<Image>,
    mov_spd: Handle<Image>,
    armor: Handle<Image>,
    air_tank: Handle<Image>,
    drain_rate: Handle<Image>,
}

pub struct RewardPlugin;
impl Plugin for RewardPlugin{
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, load_crates);
    }
}

fn load_crates(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
){
    let reward_tiles = RewardRes{
        max_hp: asset_server.load("rewards/HeartBox.png"),
        atk_spd: asset_server.load("rewards/AtkSpdBox.png"),
        mov_spd: asset_server.load("rewards/MoveSpdBox.png"),
        armor: asset_server.load("rewards/ArmorBox.png"),
        air_tank: asset_server.load("rewards/AirTankBox.png"),
        drain_rate: asset_server.load("rewards/DrainRateBox.png"),
    };

    commands.insert_resource(reward_tiles);
}

pub fn spawn_reward(
    commands: &mut Commands,
    pos: Vec3,
    box_sprite: &RewardRes,
){
    let reward_type: usize = random_range(1..=6);
    let reward_img = match reward_type
    {
        1 => box_sprite.max_hp.clone(),
        2 => box_sprite.atk_spd.clone(),
        3 => box_sprite.mov_spd.clone(),
        4 => box_sprite.armor.clone(),
        5 => box_sprite.air_tank.clone(),
        6 => box_sprite.drain_rate.clone(),
        _ => panic!("How did we get here? Reward img error")
    };

    commands.spawn((
        Sprite::from_image(reward_img),
        Transform {
            translation: pos,
            scale: Vec3::new(0.75, 0.75, 1.0),
            ..Default::default()
        },
        Reward(reward_type),
        GameEntity,
    ));
            
}

pub fn player_pickup_reward(
    mut commands: Commands,
    mut player_query: Query<(&Transform, &mut Health, &mut MaxHealth, &mut MoveSpeed, &mut Armor, &mut AirTank), With<Player>>,
    reward_query: Query<(Entity, &Transform, &Reward)>,
    mut player_weapon_q: Query<&mut Weapon, With<Player>>,
) {
    let Ok((player_tf, mut hp, mut maxhp, mut movspd, mut armor, mut tank)) = player_query.single_mut() else {
        return;
    };
    let player_pos = player_tf.translation;
    let player_half = Vec2::splat(TILE_SIZE * 0.5);

    for (reward_entity, reward_tf, reward_type) in &reward_query {
        let reward_pos = reward_tf.translation;
        let reward_half = Vec2::splat(TILE_SIZE * 0.5);
        if aabb_overlap(player_pos.x, player_pos.y, player_half, reward_pos.x, reward_pos.y, reward_half) {
            if let Ok(mut weapon) = player_weapon_q.single_mut() {
                match reward_type.0 {
                    1 => {
                        let increase_hp = random_range(5..=20) as f32;
                        maxhp.0 += increase_hp;
                        hp.0 += increase_hp;
                    }
                    2 => {
                        let new_rate = (weapon.fire_rate - 0.03).max(0.1);
                        weapon.fire_rate = new_rate;
                        weapon.shoot_timer.set_duration(Duration::from_secs_f32(new_rate));
                    }
                    3 => {
                        movspd.0 = (movspd.0 + 20.0).min(600.0);
                    }
                    4 => {
                        armor.0 += 20.0;
                    }
                    5 => {
                        // Larger air tank: +2.5 seconds of grace period
                        tank.max_capacity += 2.5;
                        tank.current = (tank.current + 2.5).min(tank.max_capacity);
                    }
                    6 => {
                        // Slower drain: 20% reduction per pickup (minimum 0.2 units/sec)
                        tank.drain_rate = (tank.drain_rate * 0.8).max(0.2);
                    }
                    _ => panic!("Reward Type Not Found"),
                }
            }
            if let Ok(mut ec) = commands.get_entity(reward_entity) { ec.despawn(); }
        }
    }
}

// Buff	Effect	Fits the theme
// Vacuum Resistance	Increases your PulledByFluid.mass — harder to suck toward breaches	Magnetic boots
// Regen	Slowly regenerates HP over time	Life support module
// Piercing Rounds	Bullets pass through enemies	Plasma cutter
// Damage Up	Increases BulletDamage	Overcharged cell
// Shield Burst	A one-time hit absorber that recharges between rooms	Emergency barrier
// Speed Up	Increases MoveSpeed	Thrusters switch movement tech from broom to thrusters. dodge in direction of mouse
// larger fuel tank for thusters
// make broom go through tables you can take a lot of damage trying to repair through tables
// internal oxygen tank that gives you time to repair windows after they break
// make window generation better
// table need to stop moving when you fix the window
// air need to slowly fill back up
// reaper not going through walls anymore?
// better visual when it
// better feedback in general
// broken tables shouldnt damage you
// the tables are so fucked