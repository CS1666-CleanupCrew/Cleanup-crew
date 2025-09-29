use bevy::prelude::*;

pub const ENEMY_SIZE: f32 = 32.;
pub const ENEMY_SPEED: f32 = 200.;
pub const ENEMY_ACCEL: f32 = 1800.;
static mut ENEMY_START_POS: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };

use crate::{GameState, Z_ENTITIES};

const ANIM_TIME: f32 = 0.2;

#[derive(Component)]
pub struct Enemy;

#[derive(Component, Deref, DerefMut)]
pub struct Velocity {
    pub velocity: Vec2,
}

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(Timer);

#[derive(Component)]
pub struct EnemyFrames {
    handles: Vec<Handle<Image>>,
    index: usize,
}

#[derive(Resource)]
pub struct EnemyRes(Vec<Handle<Image>>);

impl Velocity {
    pub fn new() -> Self {
        Self {
            velocity: Vec2::ZERO,
        }
    }
}

pub struct EnemyPlugin;
impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameState::Playing), load_enemy)
            .add_systems(OnEnter(GameState::Playing), spawn_enemy.after(load_enemy))
            .add_systems(Update, animate_enemy.run_if(in_state(GameState::Playing)));
    }
}

fn load_enemy(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Load 3 separate frames
    let frames: Vec<Handle<Image>> = vec![
        asset_server.load("chaser_mob_animation1.png"),
        asset_server.load("chaser_mob_animation2.png"),
        asset_server.load("chaser_mob_animation3.png"),
        asset_server.load("chaser_mob_animation2.png"),
    ];

    commands.insert_resource(EnemyRes(frames));
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

pub fn spawn_enemy(mut commands: Commands, enemy_res: Res<EnemyRes>) {
    commands.spawn((
        Sprite::from_image(enemy_res.0[0].clone()), // start on first frame
        Transform {
            translation: Vec3::new(
                unsafe { ENEMY_START_POS.x },
                unsafe { ENEMY_START_POS.y },
                Z_ENTITIES,
            ),
            ..default()
        },
        Enemy,
        Velocity::new(),
        AnimationTimer(Timer::from_seconds(ANIM_TIME, TimerMode::Repeating)),
        EnemyFrames {
            handles: enemy_res.0.clone(),
            index: 0,
        },
    ));
}

fn animate_enemy(
    time: Res<Time>,
    mut query: Query<(&mut Sprite, &mut AnimationTimer, &mut EnemyFrames), With<Enemy>>,
) {
    for (mut sprite, mut timer, mut frames) in &mut query {
        timer.tick(time.delta());

        if timer.just_finished() {
            frames.index = (frames.index + 1) % frames.handles.len();
            sprite.image = frames.handles[frames.index].clone();
        }
    }
}
