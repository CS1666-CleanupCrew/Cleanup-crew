use bevy::{prelude::*};

use crate::collidable::{Collidable, Collider};
use crate::table;
use crate::{ACCEL_RATE, GameState, LEVEL_LEN, PLAYER_SPEED, TILE_SIZE, WIN_H, WIN_W};
use crate::enemy::{Enemy, ENEMY_SIZE};
use crate::enemy::HitAnimation;

const BULLET_SPD: f32 = 500.;

#[derive(Component)]
pub struct Player;

#[derive(Component, Deref, DerefMut)]
pub struct Velocity(Vec2);

#[derive(Resource)]
pub struct PlayerRes{
    up: Handle<Image>,
    right: Handle<Image>,
    down: Handle<Image>,
    left: Handle<Image>,
}

#[derive(Component)]
pub struct Health(pub f32);

#[derive(Component)]
pub struct Bullet;

#[derive(Resource)]
pub struct BulletRes(Handle<Image>, Handle<TextureAtlasLayout>);

#[derive(Resource)]
pub struct ShootTimer(pub Timer);

#[derive(Component, Deref, DerefMut)]
pub struct DamageTimer(pub Timer);

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(Timer);

#[derive(Component, Deref, DerefMut)]
pub struct AnimationFrameCount(usize);

#[derive(Component)]
pub struct Facing(pub FacingDirection);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FacingDirection {
    Up,
    Down,
    Left,
    Right,
}

//Creates an instance of a Velocity
impl Velocity {
    fn new() -> Self {
        Self(Vec2::ZERO)
    }
    fn new_vec(x: f32, y: f32) -> Self {
        Self(Vec2{x, y})
    }
}

//creates a variable of health
impl Health {
    pub fn new(amount: f32) -> Self {
        Self(amount)
    }
}

//Allows for vec2.into() instead of Velocity::from(vec2)
impl From<Vec2> for Velocity {
    fn from(velocity: Vec2) -> Self {
        Self(velocity)
    }
}

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), load_player)
            .add_systems(OnEnter(GameState::Playing), load_bullet)
            .add_systems(OnEnter(GameState::Playing), spawn_player.after(load_player))
            .add_systems(Update, move_player.run_if(in_state(GameState::Playing)))
            .add_systems(Update, update_player_sprite.run_if(in_state(GameState::Playing)))
            .add_systems(Update, move_bullet.run_if(in_state(GameState::Playing)))
            .add_systems(Update, bullet_collision.run_if(in_state(GameState::Playing)))
            .add_systems(Update, animate_bullet.after(move_bullet).run_if(in_state(GameState::Playing)),)
            .add_systems(Update, bullet_hits_enemy.run_if(in_state(GameState::Playing)))
            .add_systems(Update, bullet_hits_table.run_if(in_state(GameState::Playing)))
            .add_systems(Update, enemy_hits_player.run_if(in_state(GameState::Playing)))
            ;
    }
}

fn load_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    let player = PlayerRes {
        up: asset_server.load("Player_Sprite_Up.png"),
        right: asset_server.load("Player_Sprite_Right.png"),
        down: asset_server.load("Player_Sprite_Down.png"),
        left: asset_server.load("Player_Sprite_Left.png"),
    };
    commands.insert_resource(player);

    //Change time for how fast the player can shoot
    commands.insert_resource(ShootTimer(Timer::from_seconds(0.25, TimerMode::Repeating)));
    
}

fn spawn_player(mut commands: Commands, player_sheet: Res<PlayerRes>) {
    commands.spawn((
        Sprite::from_image(player_sheet.down.clone()),
        Transform {
            translation: Vec3::new(0., 0., 0.),
            scale: Vec3::new(0.04, 0.04, 0.04),
            ..Default::default()
        },
        Player,
        Velocity::new(),
        Health::new(100.0),
        DamageTimer::new(1.0),
        Collidable,
        Collider {
            half_extents: Vec2::new(TILE_SIZE * 0.5, TILE_SIZE * 1.0),
        },
        Facing(FacingDirection::Down),
    ));
}

/**
 * Single is a query for exactly one entity
 * With tells bevy to include entities with the Player component
 * Without is the opposite
*/
fn move_player(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    player: Single<(&mut Transform, &mut Velocity, &mut Facing), With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
    colliders: Query<(&Transform, &Collider), (With<Collidable>, Without<Player>, Without<Bullet>)>,
    commands: Commands,
    bullet_animate: Res<BulletRes>,
    mut shoot_timer: ResMut<ShootTimer>,
) {
    let (mut transform, mut velocity, mut facing) = player.into_inner();

    let mut dir: Vec2 = Vec2::ZERO;

    if input.just_pressed(KeyCode::KeyT) {
        next_state.set(GameState::EndCredits);
    }
    if input.pressed(KeyCode::KeyA) {
        dir.x -= 1.;
        facing.0 = FacingDirection::Left;
    }
    if input.pressed(KeyCode::KeyD) {
        dir.x += 1.;
        facing.0 = FacingDirection::Right;
    }
    if input.pressed(KeyCode::KeyW) {
        dir.y += 1.;
        facing.0 = FacingDirection::Up;
    }
    if input.pressed(KeyCode::KeyS) {
        dir.y -= 1.;
        facing.0 = FacingDirection::Down;
    }

    if input.pressed(KeyCode::Space) && shoot_timer.0.tick(time.delta()).finished() {
        let bullet_dir = match facing.0 {
            FacingDirection::Up => Vec2::new(0.0, 1.0),
            FacingDirection::Down => Vec2::new(0.0, -1.0),
            FacingDirection::Left => Vec2::new(-1.0, 0.0),
            FacingDirection::Right => Vec2::new(1.0, 0.0),
        };
        spawn_bullet(
            commands,
            bullet_animate,
            Vec2 { x: transform.translation.x, y: transform.translation.y },
            bullet_dir,
        );
        shoot_timer.0.reset();
    }

    //Time based on frame to ensure that movement is the same no matter the fps
    let deltat = time.delta_secs();
    let accel = ACCEL_RATE * deltat;

    **velocity = if dir.length() > 0. {
        (**velocity + (dir.normalize_or_zero() * accel)).clamp_length_max(PLAYER_SPEED)
    } else if velocity.length() > accel {
        **velocity + (velocity.normalize_or_zero() * -accel)
    } else {
        Vec2::ZERO
    };
    let change = **velocity * deltat;

    let min = Vec3::new(
        -WIN_W / 2. + (TILE_SIZE as f32) / 2.,
        -WIN_H / 2. + (TILE_SIZE as f32) * 1.5,
        900.,
    );

    let max = Vec3::new(
        LEVEL_LEN - (WIN_W / 2. + (TILE_SIZE as f32) / 2.),
        WIN_H / 2. - (TILE_SIZE as f32) / 2.,
        900.,
    );

    let mut pos = transform.translation;
    let delta = change; // Vec2
    let player_half = Vec2::new(TILE_SIZE * 0.5, TILE_SIZE * 1.0);

    // ---- X axis ----
    if delta.x != 0.0 {
        let mut nx = pos.x + delta.x;
        let px = nx;
        let py = pos.y;

        for (ct, c) in &colliders {
            let (cx, cy) = (ct.translation.x, ct.translation.y);
            if aabb_overlap(px, py, player_half, cx, cy, c.half_extents) {
                if delta.x > 0.0 {
                    nx = cx - (player_half.x + c.half_extents.x);
                } else {
                    nx = cx + (player_half.x + c.half_extents.x);
                }
                **velocity = Vec2::new(0.0, velocity.y);
            }
        }
        pos.x = nx;
    }

    // ---- Y axis ----
    if delta.y != 0.0 {
        let mut ny = pos.y + delta.y;
        let px = pos.x;
        let py = ny;

        for (ct, c) in &colliders {
            let (cx, cy) = (ct.translation.x, ct.translation.y);
            if aabb_overlap(px, py, player_half, cx, cy, c.half_extents) {
                if delta.y > 0.0 {
                    ny = cy - (player_half.y + c.half_extents.y);
                } else {
                    ny = cy + (player_half.y + c.half_extents.y);
                }
                **velocity = Vec2::new(velocity.x, 0.0);
            }
        }
        pos.y = ny;
    }

    // Apply the resolved position
    transform.translation = pos;
}


//what a lot of games use for collision detection I found
#[inline]
pub fn aabb_overlap(
    ax: f32, ay: f32, a_half: Vec2,
    bx: f32, by: f32, b_half: Vec2
) -> bool {
    (ax - bx).abs() < (a_half.x + b_half.x) &&
    (ay - by).abs() < (a_half.y + b_half.y)
}

//enemy collision with player
//-------------------------------------------------------------------------------------------------------------
impl DamageTimer {
    pub fn new(seconds: f32) -> Self {
        Self(Timer::from_seconds(seconds, TimerMode::Once))
}
}

fn enemy_hits_player(
    time: Res<Time>,
    mut player_query: Query<(&Transform, &mut crate::player::Health, &mut DamageTimer), With<crate::player::Player>>,
    mut enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    mut commands: Commands,
) {
    let player_half = Vec2::splat(32.0);
    let enemy_half = Vec2::splat(ENEMY_SIZE * 0.5);
    for (player_tf, mut health, mut damage_timer) in &mut player_query {
        
        damage_timer.0.tick(time.delta());

        let player_pos = player_tf.translation.truncate();

        for (enemy_entity, enemy_tf) in &mut enemy_query {
            let enemy_pos = enemy_tf.translation.truncate();
            if aabb_overlap(
                player_pos.x, 
                player_pos.y, 
                player_half,
                enemy_pos.x, 
                enemy_pos.y, 
                enemy_half,
            ) {
                if damage_timer.0.finished() {
                    health.0 -= 15.0;
                    damage_timer.0.reset();
                    commands.entity(enemy_entity).insert(HitAnimation {
                        timer: Timer::from_seconds(0.3, TimerMode::Once),
                    });
                }
            }
        }
    }
}
//-------------------------------------------------------------------------------------------------------------

/**
 * Updates player sprite while changing directions
 * Eventually use a sprite sheet for all of the animation and direction changes
 */

fn update_player_sprite(
    mut query: Query<&mut Sprite, With<Player>>,
    player_res: Res<PlayerRes>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for mut sprite in &mut query {
        let new_handle = if input.pressed(KeyCode::KeyW) {
            &player_res.up
        } else if input.pressed(KeyCode::KeyS) {
            &player_res.down
        } else if input.pressed(KeyCode::KeyA) {
            &player_res.left
        } else if input.pressed(KeyCode::KeyD) {
            &player_res.right
        } else {
            continue;
        };

        sprite.custom_size = None;
        sprite.image = new_handle.clone(); // now works
    }
}

/**
 * BULLET SECTION
 */

fn load_bullet(
    mut commands: Commands, 
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
){  
    //Bullet look
    let bullet_animate_image: Handle<Image> = asset_server.load("bullet_animation.png");

    //Bullet size within image and layout
    let bullet_animate_layout = TextureAtlasLayout::from_grid(UVec2::splat(100), 3, 1, None, None);
    let bullet_animate_handle = texture_atlases.add(bullet_animate_layout);

    commands.insert_resource(BulletRes(bullet_animate_image, bullet_animate_handle));
}

fn spawn_bullet(
    mut commands: Commands,
    bullet_animate: Res<BulletRes>,
    pos: Vec2,
    dir: Vec2,
){

    commands.spawn((
        Sprite::from_atlas_image(
            bullet_animate.0.clone(),
            TextureAtlas { 
                layout: bullet_animate.1.clone(),
                index: 0, 
            },
        ),
        Transform{
            translation: Vec3::new(pos.x, pos.y, 910.),
            scale: Vec3::splat(0.25),
            ..Default::default()
        },
        AnimationTimer(Timer::from_seconds(0.2, TimerMode::Repeating)),
        AnimationFrameCount(3),
        Velocity::new_vec(dir.x, dir.y),
        Bullet,
        Collidable,
        Collider {
            half_extents: Vec2::splat(5.0), // adjust to bullet size
        },
    ));
}

fn move_bullet(
    time: Res<Time>,
    mut bullet: Query<(&mut Transform, &mut Velocity), With<Bullet>>,
){

    for (mut transform, b) in &mut bullet {
        transform.translation.x += b.x * BULLET_SPD * time.delta_secs();
        transform.translation.y += b.y * BULLET_SPD * time.delta_secs();
    }
}

fn bullet_collision(
    mut commands: Commands,
    bullet_query: Query<(Entity, &Transform, &Collider), With<Bullet>>,
    colliders: Query<(&Transform, &Collider), (With<Collidable>, Without<Player>, Without<Bullet>, Without<crate::enemy::Enemy>, Without<table::Table>,)>,
) {
    for (bullet_entity, bullet_transform, bullet_collider) in &bullet_query {
        let bx = bullet_transform.translation.x;
        let by = bullet_transform.translation.y;
        let b_half = bullet_collider.half_extents;

        // Check collision with all collidable entities
        for (collider_transform, collider) in &colliders {
            let cx = collider_transform.translation.x;
            let cy = collider_transform.translation.y;
            let c_half = collider.half_extents;

            if aabb_overlap(bx, by, b_half, cx, cy, c_half) {
                commands.entity(bullet_entity).despawn();
                break;
            }
        }
    }
}

fn animate_bullet(
    time: Res<Time>,
    mut bullet: Query<
        (
            &mut Sprite,
            &mut AnimationTimer,
            &AnimationFrameCount,
        ),
        With<Bullet>,
    >,
) {
    for (mut sprite, mut timer, frame_count) in &mut bullet{
        timer.tick(time.delta());

        if timer.just_finished() {
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = (atlas.index + 1) % **frame_count;
            }
        }
    }
}

/**
 * This handles bullet enemy collision
 * 
 * Right now the enemy will typically die despite having a health
 * of 50 and the bullet dealing 25 damage. This probably is happening
 * because this detection is happening every frame. 
*/
fn bullet_hits_enemy(
    mut enemy_query: Query<(&Transform, &mut crate::enemy::Health), With<crate::enemy::Enemy>>,
    bullet_query: Query<(&Transform, Entity), With<Bullet>>,
    mut commands: Commands,
) {
    let bullet_half = Vec2::splat(8.0);
    let enemy_half = Vec2::splat(crate::enemy::ENEMY_SIZE * 0.5);
    for (bullet_tf, bullet_entity) in &bullet_query {
        let bullet_pos = bullet_tf.translation;
        for (enemy_tf, mut health) in &mut enemy_query {
            let enemy_pos = enemy_tf.translation;
            if aabb_overlap(
                bullet_pos.x, bullet_pos.y, bullet_half,
                enemy_pos.x, enemy_pos.y, enemy_half,
            ) {
                health.0 -= 25.0;
                commands.entity(bullet_entity).despawn();
            }
        }
    }
}

fn bullet_hits_table(
    mut commands: Commands,
    mut table_query: Query<(&Transform, &mut table::Health), With<table::Table>>,
    bullet_query: Query<(Entity, &Transform), With<Bullet>>,
) {
    let bullet_half = Vec2::splat(8.0); // Bullet's collider size
    let table_half = Vec2::splat(TILE_SIZE * 0.5); // Table's collider size

    'bullet_loop: for (bullet_entity, bullet_tf) in &bullet_query {
        let bullet_pos = bullet_tf.translation;
        for (table_tf, mut health) in &mut table_query {
            let table_pos = table_tf.translation;
            if aabb_overlap(
                bullet_pos.x,
                bullet_pos.y,
                bullet_half,
                table_pos.x,
                table_pos.y,
                table_half,
            ) {
                health.0 -= 25.0; // Deal 25 damage
                commands.entity(bullet_entity).despawn(); // Despawn bullet on hit
                continue 'bullet_loop; // Move to the next bullet
            }
        }
    }
}