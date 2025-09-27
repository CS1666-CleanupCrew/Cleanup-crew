use bevy::prelude::*;

pub const ENEMY_SIZE: f32 = 32.;
pub const ENEMY_SPEED: f32 = 200.;
pub const ENEMY_ACCEL: f32 = 1800.;
static mut ENEMY_START_POS: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };

use crate::{
    GameState
};

#[derive(Component)]
pub struct Enemy;

#[derive(Component, Deref, DerefMut)]
pub struct Velocity {
    pub velocity: Vec2,
}

#[derive(Resource)]
pub struct EnemyRes(Handle<Image>);

impl Velocity {
    pub fn new() -> Self {
        Self {
            velocity: Vec2::ZERO,
        }
    }
}

pub struct EnemyPlugin;
impl Plugin for EnemyPlugin{
    fn build(&self, app: &mut App){
        app
            .add_systems(OnEnter(GameState::Playing), load_enemy)
            .add_systems(OnEnter(GameState::Playing), spawn_enemy.after(load_enemy))
            // .add_systems(Update, move_player.run_if(in_state(GameState::Playing)))
            ;
    }
}

fn load_enemy(
    mut commands: Commands, 
    asset_server: Res<AssetServer>
){
    let enemy: Handle<Image>= asset_server.load("enemy.png");

    commands.insert_resource(EnemyRes(
        enemy.clone(),
    ));
}


// Getter
pub fn enemy_start_pos() -> Vec3 {
    unsafe { ENEMY_START_POS }
}

// Setter
pub fn set_enemy_start_pos(new_pos: Vec3) {
    unsafe {
        ENEMY_START_POS = new_pos;
    }
}

    

pub fn spawn_enemy(
    mut commands: Commands,
    enemy_sheet: Res<EnemyRes>,
) {

    unsafe {let spawn_pos = ENEMY_START_POS;}
    

    commands.spawn((
        Sprite::from_image(
            enemy_sheet.0.clone()
        ),
        Transform {
            translation: unsafe{ENEMY_START_POS},
            scale: Vec3::splat(1.0),
            ..default()
        },
        Enemy,
        Velocity::new(),
    ));
}