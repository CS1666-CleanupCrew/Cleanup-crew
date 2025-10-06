use bevy::prelude::*;
use crate::player::Player;
use crate::collidable::{Collidable, Collider};

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


#[derive(Component)]
pub struct Health(pub f32);
impl Health {
    pub fn new(amount: f32) -> Self {
        Self(amount)
    }
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
            .add_systems(Update, animate_enemy.run_if(in_state(GameState::Playing)))
            .add_systems(Update, move_enemy.run_if(in_state(GameState::Playing)))
            .add_systems(Update, check_enemy_health.run_if(in_state(GameState::Playing)));
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


//if enemy's hp = 0, then despawn
fn check_enemy_health(
    mut commands: Commands,
    enemy_query: Query<(Entity, &Health), With<Enemy>>,
) {
    for (entity, health) in enemy_query.iter() {
        if health.0 <= 0.0 {
            commands.entity(entity).despawn();
        }
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
        Health::new(50.0),
        AnimationTimer(Timer::from_seconds(ANIM_TIME, TimerMode::Repeating)),
        EnemyFrames {
            handles: enemy_res.0.clone(),
            index: 0,
        },
    ));
}

fn animate_enemy(
    time: Res<Time>,
    mut query: Query<(&mut Sprite, &mut AnimationTimer, &mut EnemyFrames, &Velocity), With<Enemy>>,
) {
    for (mut sprite, mut timer, mut frames, velocity) in &mut query {
        timer.tick(time.delta());

        if timer.just_finished() {
            frames.index = (frames.index + 1) % frames.handles.len();
            sprite.image = frames.handles[frames.index].clone();
        }

        // Flip the sprite based on the x velocity
        if velocity.x > 0. {
            sprite.flip_x = true;
        } else if velocity.x < 0. {
            sprite.flip_x = false;
        }
    }
}

// moves the enemy towards the player
fn move_enemy(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemy_query: Query<(&mut Transform, &mut Velocity), With<Enemy>>,
) {
    if let Ok(player_transform) = player_query.get_single() {
        let deltat = time.delta_secs();
        let accel = ENEMY_ACCEL * deltat;

        // Iterate over each enemy
        for (mut enemy_transform, mut enemy_velocity) in &mut enemy_query {
            // calculate the direction from the enemy to the player
            let dir_to_player = (player_transform.translation - enemy_transform.translation)
                                .truncate()
                                .normalize_or_zero();

            // using players acceleration to change enemy velocity
            **enemy_velocity = if dir_to_player.length() > 0. {
                (**enemy_velocity + (dir_to_player * accel)).clamp_length_max(ENEMY_SPEED)
            } else if enemy_velocity.length() > accel {
                **enemy_velocity + (enemy_velocity.normalize_or_zero() * -accel)
            } else {
                Vec2::ZERO
            };

            let change = **enemy_velocity * deltat;

            // Update the enemy's position
            enemy_transform.translation.x += change.x;
            enemy_transform.translation.y += change.y;
        }
    }
}

