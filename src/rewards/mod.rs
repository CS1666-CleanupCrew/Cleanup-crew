pub mod air_tank;
pub mod armor;
pub mod atk_speed;
pub mod damage_up;
pub mod drain_rate;
pub mod max_hp;
pub mod move_speed;
pub mod piercing;
pub mod regen;
pub mod shield;
pub mod vacuum_res;

use bevy::prelude::*;
use rand::random_range;
use crate::{TILE_SIZE, GameEntity};
use crate::Player;
use crate::player::{Health, MaxHealth, MoveSpeed, Armor, AirTank, Regen, Shield, ThrusterFuel, aabb_overlap};
use crate::fluiddynamics::PulledByFluid;
use crate::weapon::Weapon;

// Popup 

#[derive(Component)]
pub struct RewardPopup {
    timer: Timer,
}

pub fn tick_reward_popups(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Transform, &mut TextColor, &mut RewardPopup)>,
) {
    for (entity, mut tf, mut color, mut popup) in &mut q {
        popup.timer.tick(time.delta());
        let frac = popup.timer.fraction();

        tf.translation.y += 40.0 * time.delta_secs();

        let alpha = if frac < 0.5 { 1.0 } else { 1.0 - (frac - 0.5) * 2.0 };
        color.0 = color.0.with_alpha(alpha);

        if popup.timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}

// Reward component & asset resource 

#[derive(Component)]
pub struct Reward(pub usize);

#[allow(dead_code)]
#[derive(Resource)]
pub struct RewardRes {
    max_hp:     Handle<Image>,
    atk_spd:    Handle<Image>,
    mov_spd:    Handle<Image>,
    armor:      Handle<Image>,
    air_tank:   Handle<Image>,
    drain_rate: Handle<Image>,
    vacuum_res: Handle<Image>,
    regen:      Handle<Image>,
    piercing:   Handle<Image>,
    damage_up:  Handle<Image>,
    shield_burst: Handle<Image>,
}


pub struct RewardPlugin;

impl Plugin for RewardPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_crates);
    }
}


#[derive(Resource)]
pub struct RewardFont(pub Handle<Font>);

pub fn load_reward_font(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load(
        "fonts/BitcountSingleInk-VariableFont_CRSV,ELSH,ELXP,SZP1,SZP2,XPN1,XPN2,YPN1,YPN2,slnt,wght.ttf",
    );
    commands.insert_resource(RewardFont(handle));
}

fn load_crates(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(RewardRes {
        max_hp:       asset_server.load(max_hp::ASSET),
        atk_spd:      asset_server.load(atk_speed::ASSET),
        mov_spd:      asset_server.load(move_speed::ASSET),
        armor:        asset_server.load(armor::ASSET),
        air_tank:     asset_server.load(air_tank::ASSET),
        drain_rate:   asset_server.load(drain_rate::ASSET),
        vacuum_res:   asset_server.load(vacuum_res::ASSET),
        regen:        asset_server.load(regen::ASSET),
        piercing:     asset_server.load(piercing::ASSET),
        damage_up:    asset_server.load(damage_up::ASSET),
        shield_burst: asset_server.load(shield::ASSET),
    });
}

// Spawn

pub fn spawn_reward(commands: &mut Commands, pos: Vec3, box_sprite: &RewardRes) {
    let reward_type: usize = random_range(1..=11);
    let reward_img = match reward_type {
        1  => box_sprite.max_hp.clone(),
        2  => box_sprite.atk_spd.clone(),
        3  => box_sprite.mov_spd.clone(),
        4  => box_sprite.armor.clone(),
        5  => box_sprite.air_tank.clone(),
        6  => box_sprite.drain_rate.clone(),
        7  => box_sprite.vacuum_res.clone(),
        8  => box_sprite.regen.clone(),
        9  => box_sprite.piercing.clone(),
        10 => box_sprite.damage_up.clone(),
        11 => box_sprite.shield_burst.clone(),
        _  => panic!("reward image out of range"),
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

// Pickup

pub fn player_pickup_reward(
    mut commands: Commands,
    mut player_query: Query<(
        Entity, &Transform,
        &mut Health, &mut MaxHealth, &mut MoveSpeed, &mut Armor, &mut AirTank,
        &mut Regen, &mut Shield, &mut PulledByFluid, &mut ThrusterFuel,
    ), With<Player>>,
    reward_query: Query<(Entity, &Transform, &Reward)>,
    mut player_weapon_q: Query<&mut Weapon, With<Player>>,
    font: Res<RewardFont>,
) {
    let Ok((
        _player_entity, player_tf,
        mut hp, mut maxhp, mut movspd, mut arm, mut tank,
        mut reg, mut shld, mut pull, mut fuel,
    )) = player_query.single_mut() else {
        return;
    };
    let player_pos = player_tf.translation;
    let player_half = Vec2::splat(TILE_SIZE * 0.5);

    for (reward_entity, reward_tf, reward_type) in &reward_query {
        let reward_pos = reward_tf.translation;
        let reward_half = Vec2::splat(TILE_SIZE * 0.5);
        if !aabb_overlap(player_pos.x, player_pos.y, player_half, reward_pos.x, reward_pos.y, reward_half) {
            continue;
        }

        if let Ok(mut weapon) = player_weapon_q.single_mut() {
            match reward_type.0 {
                1  => max_hp::apply(&mut hp, &mut maxhp),
                2  => atk_speed::apply(&mut weapon),
                3  => move_speed::apply(&mut movspd, &mut fuel),
                4  => armor::apply(&mut arm),
                5  => air_tank::apply(&mut tank),
                6  => drain_rate::apply(&mut tank),
                7  => vacuum_res::apply(&mut pull),
                8  => regen::apply(&mut reg),
                9  => piercing::apply(&mut weapon),
                10 => damage_up::apply(&mut weapon),
                11 => shield::apply(&mut shld),
                _  => panic!("Reward Type Not Found"),
            }
        }

        if let Ok(mut ec) = commands.get_entity(reward_entity) { ec.despawn(); }

        let name = reward_name(reward_type.0);
        commands.spawn((
            Text2d::new(name),
            TextFont { font: font.0.clone(), font_size: 20.0, ..default() },
            TextColor(Color::srgba(1.0, 1.0, 0.3, 1.0)),
            Transform::from_translation(Vec3::new(reward_pos.x, reward_pos.y + TILE_SIZE, 10.0)),
            RewardPopup { timer: Timer::from_seconds(1.5, TimerMode::Once) },
            GameEntity,
        ));
    }
}

fn reward_name(id: usize) -> &'static str {
    match id {
        1  => max_hp::NAME,
        2  => atk_speed::NAME,
        3  => move_speed::NAME,
        4  => armor::NAME,
        5  => air_tank::NAME,
        6  => drain_rate::NAME,
        7  => vacuum_res::NAME,
        8  => regen::NAME,
        9  => piercing::NAME,
        10 => damage_up::NAME,
        11 => shield::NAME,
        _  => "???",
    }
}
