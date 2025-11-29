use bevy::prelude::*;
// use crate::player::Player;
// use crate::collidable::{Collidable, Collider};

#[derive(Component)]
struct Reward;

#[allow(dead_code)]
#[derive(Resource)]
struct RewardTiles{
    med: Handle<Image>,
    atk_spd: Handle<Image>,
    mov_spd: Handle<Image>,
}

pub struct RewardPlugin;
impl Plugin for RewardPlugin{
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, load_crate);
    }
}

fn load_crate(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
){
    let reward_tiles = RewardTiles{
        med: asset_server.load("map/crate.png"),
        atk_spd: asset_server.load("map/crate.png"),
        mov_spd: asset_server.load("map/crate.png"),
    };

    commands.insert_resource(reward_tiles);
}

pub fn spawn_reward(
    _pos: Vec3,
){

}