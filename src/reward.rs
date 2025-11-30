use bevy::prelude::*;
use rand::random_range;
use crate::collidable::{Collidable, Collider};
use crate::{TILE_SIZE, GameEntity};

#[derive(Component)]
pub struct Reward(pub usize);

#[allow(dead_code)]
#[derive(Resource)]
pub struct RewardRes{
    max_hp: Handle<Image>,
    atk_spd: Handle<Image>,
    mov_spd: Handle<Image>,
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
    };

    commands.insert_resource(reward_tiles);
}

pub fn spawn_reward(
    commands: &mut Commands,
    pos: Vec3,
    box_sprite: &RewardRes,
){
    let reward_type: usize = random_range(1..=3);
    let reward_img = match reward_type
    {
        1 => {box_sprite.max_hp.clone()},
        2 => {box_sprite.atk_spd.clone()},
        3 => {box_sprite.mov_spd.clone()},
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
        Collidable,
        Collider { half_extents: Vec2::new(TILE_SIZE * 0.75, TILE_SIZE * 0.75) },
        GameEntity,
    ));
            
}