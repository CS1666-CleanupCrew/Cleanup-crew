use bevy::{prelude::*, window::PrimaryWindow};

use crate::collidable::{Collidable, Collider};
use crate::table;
use crate::broom::Broom;
use crate::{ACCEL_RATE, GameState, GameEntity, LEVEL_LEN, PLAYER_SPEED, TILE_SIZE, WIN_H, WIN_W};
use crate::enemies::{Enemy, ENEMY_SIZE};
use crate::enemies::HitAnimation;
use crate::map::{LevelRes, MapGridMeta};
use crate::fluiddynamics::PulledByFluid;
use crate::bullet::{Bullet, Velocity};
use crate::weapons::{Weapon, WeaponType, WeaponInventory, fire_weapon, BulletRes, WeaponSounds, BeamRifleRes};
const WALL_SLIDE_FRICTION_MULTIPLIER: f32 = 0.92; // lower is more friction

// #[derive(Resource)]
// pub struct PlayerLaserSound(Handle<AudioSource>);

#[derive(Component)]
pub struct Player;           

#[derive(Component)]
pub struct NumOfCleared(pub usize);  

// #[derive(Component, Deref, DerefMut)]
// pub struct Velocity(Vec2);

#[derive(Resource)]
pub struct PlayerRes{
    up: (Handle<Image>, Handle<TextureAtlasLayout>),
    right: (Handle<Image>, Handle<TextureAtlasLayout>),
    down: (Handle<Image>, Handle<TextureAtlasLayout>),
    left: (Handle<Image>, Handle<TextureAtlasLayout>),
}

#[derive(Component)]
pub struct Health(pub f32);

#[derive(Component)]
pub struct MaxHealth(pub f32);

#[derive(Component)]
pub struct MoveSpeed(pub f32);

/// Flat armor value. Damage is multiplied by 100 / (100 + armor).
/// 0 = no reduction, 100 = 50% reduction, 200 = 66% reduction (RoR2 formula).
#[derive(Component)]
pub struct Armor(pub f32);

/// Internal oxygen reserve. Drains when room air pressure is low, giving the
/// player a grace period before they start taking damage.
#[derive(Component)]
pub struct AirTank {
    pub current: f32,      // current oxygen [0..max_capacity]
    pub max_capacity: f32, // maximum oxygen
    pub drain_rate: f32,   // units consumed per second in low-air environments
}

impl AirTank {
    pub fn new(max_capacity: f32, drain_rate: f32) -> Self {
        Self { current: max_capacity, max_capacity, drain_rate }
    }
}

/// HP regenerated per second. Stacks with multiple pickups.
#[derive(Component)]
pub struct Regen(pub f32);

/// One-time hit absorber. Each charge blocks one hit fully; recharges on room entry.
#[derive(Component)]
pub struct Shield {
    pub current: f32,
    pub max: f32,
}

impl Shield {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }
}

/// Thruster fuel for the dodge dash. Gained via Speed Up pickups.
#[derive(Component)]
pub struct ThrusterFuel {
    pub current: f32,
    pub max: f32,
}

// #[derive(Resource)]
// pub struct BulletRes(Handle<Image>, Handle<TextureAtlasLayout>);

// #[derive(Resource)]
// pub struct ShootTimer(pub Timer);

#[derive(Component, Deref, DerefMut)]
pub struct DamageTimer(pub Timer);

// #[derive(Component, Deref, DerefMut)]
// pub struct AnimationTimer(Timer);

// #[derive(Component, Deref, DerefMut)]
// pub struct AnimationFrameCount(usize);

#[derive(Component)]
pub struct Facing(pub FacingDirection);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FacingDirection {
    Up,
    UpRight,
    UpLeft,
    Down,
    DownRight,
    DownLeft,
    Left,
    Right,
}

//Creates an instance of a Velocity
// impl Velocity {
//     fn new() -> Self {
//         Self(Vec2::ZERO)
//     }
//     fn new_vec(x: f32, y: f32) -> Self {
//         Self(Vec2{x, y})
//     }
// }

/// RoR2-style armor formula: returns the fraction of damage that gets through.
/// armor=0 → 1.0 (full damage), armor=100 → 0.5, armor=200 → 0.33, etc.
pub fn armor_factor(armor: f32) -> f32 {
    100.0 / (100.0 + armor.max(0.0))
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
        app.add_systems(Startup, load_player)
            // .add_systems(Startup, load_bullet)
            .add_systems(OnEnter(GameState::Playing), spawn_player.after(load_player))
            .add_systems(Update, regen_system.run_if(in_state(GameState::Playing)))
            .add_systems(Update, thruster_regen_system.run_if(in_state(GameState::Playing)))
            .add_systems(Update, thruster_dodge_system.run_if(in_state(GameState::Playing)).run_if(not(resource_exists::<crate::pause::IsPaused>)))
            .add_systems(Update, move_player.run_if(in_state(GameState::Playing)).run_if(not(resource_exists::<crate::pause::IsPaused>)))
            .add_systems(Update, update_player_sprite.run_if(in_state(GameState::Playing)))
            .add_systems(Update, apply_breach_force_to_player.after(move_player).run_if(in_state(GameState::Playing)))
            // .add_systems(Update, move_bullet.run_if(in_state(GameState::Playing)))
            // .add_systems(Update, bullet_collision.run_if(in_state(GameState::Playing)))
            // .add_systems(Update, animate_bullet.after(move_bullet).run_if(in_state(GameState::Playing)),)
            .add_systems(Update, enemy_hits_player.run_if(in_state(GameState::Playing)))
            .add_systems(Update, player_deflects_tables
                .after(table::collide_tables_with_tables)
                .run_if(in_state(GameState::Playing)))
            .add_systems(Update, table_hits_player
                .after(player_deflects_tables)
                .run_if(in_state(GameState::Playing)))
            .add_systems(Update, wall_collision_correction
                .after(apply_breach_force_to_player)
                .after(table::collide_tables_with_tables)
                .run_if(in_state(GameState::Playing)))

            ;
    }
}

fn load_player(mut commands: Commands, asset_server: Res<AssetServer>, mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,) {
    let frame_size = UVec2::new(650, 1560);

    let up_image = asset_server.load("player/PlayerUp.png");
    let up_layout = TextureAtlasLayout::from_grid(frame_size, 8, 1, None, None);
    let up_handle = texture_atlases.add(up_layout);

    let right_image = asset_server.load("player/PlayerRight.png");
    let right_layout = TextureAtlasLayout::from_grid(frame_size, 8, 1, None, None);
    let right_handle = texture_atlases.add(right_layout);

    let down_image = asset_server.load("player/PlayerDown.png");
    let down_layout = TextureAtlasLayout::from_grid(frame_size, 8, 1, None, None);
    let down_handle = texture_atlases.add(down_layout);

    let left_image = asset_server.load("player/PlayerLeft.png");
    let left_layout = TextureAtlasLayout::from_grid(frame_size, 8, 1, None, None);
    let left_handle = texture_atlases.add(left_layout);

    let player = PlayerRes {
        up: (up_image, up_handle),
        right: (right_image, right_handle),
        down: (down_image, down_handle),
        left: (left_image, left_handle),
    };
    commands.insert_resource(player);

    // let laser_sound: Handle<AudioSource> = asset_server.load("audio/laser_zap.ogg");
    // commands.insert_resource(PlayerLaserSound(laser_sound));

    // //Change time for how fast the player can shoot
    // commands.insert_resource(ShootTimer(Timer::from_seconds(0.5, TimerMode::Once)));
    
}

fn spawn_player(
    mut commands: Commands,
    player_sheet: Res<PlayerRes>,
    level: Res<LevelRes>,
    grid: Res<MapGridMeta>,
    saved_buffs: Option<Res<crate::SavedPlayerBuffs>>,
) {
    let (image, layout) = &player_sheet.down;

    // 1) Try to find an 'S' (explicit spawn) in the ASCII level
    let mut spawn_grid: Option<(usize, usize)> = None;
    'outer: for (y, row) in level.level.iter().enumerate() {
        if let Some(x) = row.chars().position(|c| c == 'S') {
            spawn_grid = Some((x, y));
            break 'outer;
        }
    }

    // 2) Fallback: pick the first '#'
    if spawn_grid.is_none() {
        for (y, row) in level.level.iter().enumerate() {
            if let Some(x) = row.chars().position(|c| c == '#') {
                spawn_grid = Some((x, y));
                break;
            }
        }
    }

    let (gx, gy) = spawn_grid.unwrap_or((0, 0));

    let x_player_spawn_offset = TILE_SIZE * 2.0;
    let y_player_spawn_offset = -TILE_SIZE * 2.0;

    let world_x = grid.x0 + gx as f32 * TILE_SIZE + x_player_spawn_offset;
    let world_y = grid.y0 + (grid.rows as f32 - 1.0 - gy as f32) * TILE_SIZE + y_player_spawn_offset;

    // Apply saved buffs from previous station if continuing, otherwise use defaults
    let (hp, max_hp, move_speed, fire_rate, num_cleared, armor, tank_max, tank_drain,
         weapon_damage, piercing, regen_rate, shield_max, vacuum_mass) =
        if let Some(buffs) = &saved_buffs {
            info!(
                "Applying saved buffs: max_hp={}, hp={}, move_spd={}, fire_rate={}, cleared={}, armor={}, tank_max={}, tank_drain={}",
                buffs.max_health, buffs.health, buffs.move_speed, buffs.fire_rate,
                buffs.num_cleared, buffs.armor, buffs.air_tank_max, buffs.air_tank_drain_rate
            );
            (
                buffs.health, buffs.max_health, buffs.move_speed, buffs.fire_rate,
                buffs.num_cleared, buffs.armor, buffs.air_tank_max, buffs.air_tank_drain_rate,
                buffs.weapon_damage, buffs.piercing_pickups, buffs.regen_rate, buffs.shield_max, buffs.vacuum_mass,
            )
        } else {
            (100.0, 100.0, 1.0, 0.5, 0, 0.0, 5.0, 1.0, 25.0, 0u32, 0.0, 0.0, 50.0)
        };

    let mut weapon = Weapon::new(WeaponType::Zapper);
    weapon.fire_rate = fire_rate;
    weapon.shoot_timer = Timer::from_seconds(fire_rate, TimerMode::Once);
    weapon.damage = weapon_damage;
    weapon.piercing_pickups = piercing;
    let mut inventory = WeaponInventory::new(weapon);
    if let Some(buffs) = &saved_buffs {
        for &wtype in &buffs.extra_weapons {
            inventory.weapons.push(Weapon::new(wtype));
        }
    }

    commands.spawn((
        Sprite::from_atlas_image(
            image.clone(),
            TextureAtlas { layout: layout.clone(), index: 0 },
        ),
        Transform {
            translation: Vec3::new(world_x, world_y, 0.0),
            scale: Vec3::new(0.04, 0.04, 0.04),
            ..Default::default()
        },
        Player,
        Velocity(Vec2::ZERO),
        Health::new(hp),
        MaxHealth(max_hp),
        DamageTimer::new(1.0),
        // grouped into nested tuples to stay within Bevy's 15-element Bundle limit
        (MoveSpeed(move_speed), Armor(armor), Collidable, Regen(regen_rate), Shield::new(shield_max)),
        Collider { half_extents: Vec2::new(TILE_SIZE * 0.5, TILE_SIZE * 1.0) },
        Facing(FacingDirection::Down),
        NumOfCleared(num_cleared),
        (PulledByFluid{mass: vacuum_mass}, AirTank::new(tank_max, tank_drain), ThrusterFuel { current: 0.0, max: 0.0 }),
        inventory,
        GameEntity,
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
    mut player: Query<(&mut Transform, &mut Velocity, &mut Facing, &MoveSpeed, &mut WeaponInventory), With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
    // Excludes permanent wall tiles and tables — tables are handled by player_deflects_tables.
    colliders: Query<(&Transform, &Collider), (With<Collidable>, Without<Player>, Without<Bullet>, Without<Broom>, Without<crate::map::WallTile>, Without<table::Table>)>,
    wall_grid: Res<crate::map::WallGrid>,
    mut commands: Commands,
    bullet_res: Res<BulletRes>,
    beam_res: Res<BeamRifleRes>,
    grid_query: Query<&crate::fluiddynamics::FluidGrid>,
    buttons: Res<ButtonInput<MouseButton>>,
    weapon_sounds: Res<WeaponSounds>,
) {
    let Ok(grid) = grid_query.single() else {
        return;
    };
    let Ok((mut transform, mut velocity, mut facing, spd, mut inventory)) = player.single_mut() else {
        return;
    };

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

    // decide what direction the player is facing if is diagonal
    if dir == vec2(1.0,1.0){
        facing.0 = FacingDirection::UpRight;
    }
    if dir == vec2(-1.0,1.0){
        facing.0 = FacingDirection::UpLeft;
    }
    if dir == vec2(1.0,-1.0){
        facing.0 = FacingDirection::DownRight;
    }
    if dir == vec2(-1.0,-1.0){
        facing.0 = FacingDirection::DownLeft;
    }


    if input.pressed(KeyCode::Space) && inventory.current().can_shoot() && !buttons.pressed(MouseButton::Left) {
        let bullet_dir = match facing.0 {
            FacingDirection::Up => Vec2::new(0.0, 1.0),
            FacingDirection::UpRight => Vec2::new(1.0, 1.0),
            FacingDirection::UpLeft => Vec2::new(-1.0, 1.0),
            FacingDirection::Down => Vec2::new(0.0, -1.0),
            FacingDirection::DownRight => Vec2::new(1.0, -1.0),
            FacingDirection::DownLeft => Vec2::new(-1.0, -1.0),
            FacingDirection::Left => Vec2::new(-1.0, 0.0),
            FacingDirection::Right => Vec2::new(1.0, 0.0),
        };

        fire_weapon(
            &mut commands,
            &mut inventory,
            &bullet_res,
            &beam_res,
            &weapon_sounds,
            transform.translation.truncate(),
            bullet_dir,
        );
    }

    //Time based on frame to ensure that movement is the same no matter the fps
    let deltat = time.delta_secs();
    let accel = ACCEL_RATE * deltat;

    **velocity = if dir.length() > 0. {
        (**velocity + (dir.normalize_or_zero() * accel)).clamp_length_max(PLAYER_SPEED + spd.0)
    // allows the player to be moved if the breaches are open
    // the drag helps stop the player so it doesn't feel like they are on ice
    } else if !grid.breaches.is_empty() {
        let drag = 0.80;
        **velocity * drag
    
    } else if velocity.length() > accel {
        **velocity + (velocity.normalize_or_zero() * -accel)
    } else {
        Vec2::ZERO
    };
    let change = **velocity * deltat;

    let _min = Vec3::new(
        -WIN_W / 2. + (TILE_SIZE as f32) / 2.,
        -WIN_H / 2. + (TILE_SIZE as f32) * 1.5,
        900.,
    );

    let _max = Vec3::new(
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
        let py = pos.y;
        let mut hit_x = false;

        // Check permanent wall tiles via spatial hash (O(1) neighbourhood).
        for (wall_pos, wall_half) in wall_grid.nearby(Vec2::new(nx, py), 3) {
            if aabb_overlap(nx, py, player_half, wall_pos.x, wall_pos.y, wall_half) {
                let candidate = if delta.x > 0.0 {
                    wall_pos.x - (player_half.x + wall_half.x)
                } else {
                    wall_pos.x + (player_half.x + wall_half.x)
                };
                if delta.x > 0.0 { nx = nx.min(candidate); } else { nx = nx.max(candidate); }
                hit_x = true;
            }
        }
        // Check dynamic collidables (doors, glass — tables excluded, handled by player_deflects_tables).
        for (ct, c) in &colliders {
            let (cx, cy) = (ct.translation.x, ct.translation.y);
            if aabb_overlap(nx, py, player_half, cx, cy, c.half_extents) {
                let candidate = if delta.x > 0.0 {
                    cx - (player_half.x + c.half_extents.x)
                } else {
                    cx + (player_half.x + c.half_extents.x)
                };
                if delta.x > 0.0 { nx = nx.min(candidate); } else { nx = nx.max(candidate); }
                hit_x = true;
            }
        }
        if hit_x {
            if dir.y != 0.0 { velocity.y *= WALL_SLIDE_FRICTION_MULTIPLIER; }
            velocity.x = 0.0;
        }
        pos.x = nx;
    }

    // ---- Y axis ----
    if delta.y != 0.0 {
        let mut ny = pos.y + delta.y;
        let px = pos.x;
        let mut hit_y = false;

        // Check permanent wall tiles via spatial hash.
        for (wall_pos, wall_half) in wall_grid.nearby(Vec2::new(px, ny), 3) {
            if aabb_overlap(px, ny, player_half, wall_pos.x, wall_pos.y, wall_half) {
                let candidate = if delta.y > 0.0 {
                    wall_pos.y - (player_half.y + wall_half.y)
                } else {
                    wall_pos.y + (player_half.y + wall_half.y)
                };
                if delta.y > 0.0 { ny = ny.min(candidate); } else { ny = ny.max(candidate); }
                hit_y = true;
            }
        }
        // Check dynamic collidables (doors, glass — tables excluded).
        for (ct, c) in &colliders {
            let (cx, cy) = (ct.translation.x, ct.translation.y);
            if aabb_overlap(px, ny, player_half, cx, cy, c.half_extents) {
                let candidate = if delta.y > 0.0 {
                    cy - (player_half.y + c.half_extents.y)
                } else {
                    cy + (player_half.y + c.half_extents.y)
                };
                if delta.y > 0.0 { ny = ny.min(candidate); } else { ny = ny.max(candidate); }
                hit_y = true;
            }
        }
        if hit_y {
            if dir.x != 0.0 { velocity.x *= WALL_SLIDE_FRICTION_MULTIPLIER; }
            velocity.y = 0.0;
        }
        pos.y = ny;
    }

    // Apply the resolved position
    transform.translation = pos;
}


//what a lot of games use for collision detection I found
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
    mut player_query: Query<(&Transform, &mut crate::player::Health, &mut DamageTimer, &Armor, &mut Shield), With<crate::player::Player>>,
    enemy_query: Query<(Entity, &Transform, &crate::enemies::Health), With<Enemy>>,
    mut commands: Commands,
) {
    let player_half = Vec2::splat(32.0);
    let enemy_half = Vec2::splat(ENEMY_SIZE * 0.5);
    for (player_tf, mut health, mut damage_timer, armor, mut shield) in &mut player_query {

        damage_timer.0.tick(time.delta());

        let player_pos = player_tf.translation.truncate();

        for (enemy_entity, enemy_tf, enemy_health) in &enemy_query {
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
                    debug!(
                        "Player hit by entity {:?} at position {:?}",
                        enemy_entity, enemy_pos
                    );
                    if shield.current >= 1.0 {
                        shield.current -= 1.0;
                    } else {
                        health.0 -= 15.0 * armor_factor(armor.0);
                    }
                    damage_timer.0.reset();
                    
               
                    if enemy_health.0 > 0.0 {
                        commands.entity(enemy_entity).insert(HitAnimation {
                            timer: Timer::from_seconds(0.3, TimerMode::Once),
                        });
                    }
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
    time: Res<Time>,
    mut query: Query<&mut Sprite, With<Player>>,
    player_res: Res<PlayerRes>,
    input: Res<ButtonInput<KeyCode>>,
    mut frame_timer: Local<f32>,
) {
    *frame_timer += time.delta_secs();

    let frame = ((*frame_timer / 0.1) as usize) % 8;


    for mut sprite in &mut query {
        // Select the current sprite sheet based on input
        let (image, layout_handle) = if input.pressed(KeyCode::KeyW) {
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
        
        sprite.texture_atlas = Some(TextureAtlas {
            layout: layout_handle.clone(),
            index: frame,
        });
        sprite.image = image.clone();
    }
}
//-------------------------------------------------------------------------------------------------------------

fn regen_system(
    time: Res<Time>,
    mut player_query: Query<(&mut Health, &MaxHealth, &Regen), With<Player>>,
) {
    let Ok((mut hp, max_hp, regen)) = player_query.single_mut() else { return; };
    if regen.0 > 0.0 {
        hp.0 = (hp.0 + regen.0 * time.delta_secs()).min(max_hp.0);
    }
}


/// Soft impulse: tables that are moving fast enough give the player a small nudge
/// away from the table's center. Direction is always outward from the table, so
/// many tables surrounding the player cancel each other out rather than stacking.
fn table_hits_player(
    mut player_query: Query<(&Transform, &mut Velocity, &mut Health, &mut DamageTimer, &Armor, &mut Shield), With<Player>>,
    table_query: Query<(&Transform, &Collider, Option<&crate::enemies::Velocity>), With<table::Table>>,
) {
    let player_half = Vec2::new(TILE_SIZE * 0.5, TILE_SIZE * 1.0);

    for (player_tf, mut player_vel, mut health, mut dmg_timer, armor, mut shield) in &mut player_query {
        let player_pos = player_tf.translation.truncate();

        let mut total_impulse = Vec2::ZERO;
        let mut fastest_speed = 0.0_f32;

        for (table_tf, table_col, vel_opt) in &table_query {
            let table_pos = table_tf.translation.truncate();
            let table_half = table_col.half_extents + Vec2::splat(5.0);

            if !aabb_overlap(player_pos.x, player_pos.y, player_half, table_pos.x, table_pos.y, table_half) {
                continue;
            }

            let speed = vel_opt.map(|v| v.velocity.length()).unwrap_or(0.0);
            if speed > 5.0 {
                // Push direction is always away from the table center, not in the table's travel
                // direction. Opposite pushes from tables on both sides cancel out naturally.
                let push_dir = (player_pos - table_pos).normalize_or_zero();
                total_impulse += push_dir * speed * 0.06;
                if speed > fastest_speed { fastest_speed = speed; }
            }
        }

        if total_impulse != Vec2::ZERO {
            let capped = total_impulse.clamp_length_max(PLAYER_SPEED * 0.8);
            **player_vel = (**player_vel + capped).clamp_length_max(PLAYER_SPEED * 2.0);

            if dmg_timer.0.finished() {
                if shield.current >= 1.0 {
                    shield.current -= 1.0;
                } else {
                    health.0 -= fastest_speed * 0.02 * armor_factor(armor.0);
                }
                dmg_timer.0.reset();
            }
        }
    }
}

/// Fraction of the overlap pushed onto the table when the player walks into it.
/// The remainder is pushed back onto the player (keeping them from sinking in).
/// 0.0 = player cannot move tables at all by walking; 1.0 = tables move at full player speed.
const PLAYER_WALK_TABLE_PUSH: f32 = 0.45;

/// Tables treat the player as a solid obstacle: when a table overlaps the player,
/// push the table out and redirect its velocity to flow around (remove the component
/// heading into the player, keep the perpendicular component).
fn player_deflects_tables(
    mut player_query: Query<&mut Transform, With<Player>>,
    mut table_query: Query<(&mut Transform, &Collider, &mut crate::enemies::Velocity, &table::TableRoom), (With<table::Table>, Without<Player>)>,
    wall_grid: Res<crate::map::WallGrid>,
    active_room: Res<table::ActiveRoom>,
) {
    let Some(active) = active_room.0 else { return; };
    let Ok(mut player_tf) = player_query.single_mut() else { return; };
    let player_pos = player_tf.translation.truncate();
    let player_half = Vec2::new(TILE_SIZE * 0.5, TILE_SIZE * 1.0);

    // Accumulate all player pushback from tables so opposite tables cancel out.
    let mut player_correction = Vec2::ZERO;

    for (mut table_tf, table_col, mut table_vel, room) in &mut table_query {
        if room.0 != active { continue; }

        let table_pos = table_tf.translation.truncate();
        let table_half = table_col.half_extents;

        if !aabb_overlap(player_pos.x, player_pos.y, player_half, table_pos.x, table_pos.y, table_half) {
            continue;
        }

        let dx = table_pos.x - player_pos.x;
        let dy = table_pos.y - player_pos.y;
        let overlap_x = (player_half.x + table_half.x) - dx.abs();
        let overlap_y = (player_half.y + table_half.y) - dy.abs();

        // Split the overlap: table gets PLAYER_WALK_TABLE_PUSH of it, player gets pushed back the rest.
        // Also zero the table's velocity component heading into the player so it deflects sideways.
        if overlap_x < overlap_y {
            let sign = if dx >= 0.0 { 1.0_f32 } else { -1.0_f32 };
            table_tf.translation.x += sign * overlap_x * PLAYER_WALK_TABLE_PUSH;
            player_correction.x  -= sign * overlap_x * (1.0 - PLAYER_WALK_TABLE_PUSH);
            if table_vel.velocity.x * sign < 0.0 {
                table_vel.velocity.x = 0.0;
            }
        } else {
            let sign = if dy >= 0.0 { 1.0_f32 } else { -1.0_f32 };
            table_tf.translation.y += sign * overlap_y * PLAYER_WALK_TABLE_PUSH;
            player_correction.y  -= sign * overlap_y * (1.0 - PLAYER_WALK_TABLE_PUSH);
            if table_vel.velocity.y * sign < 0.0 {
                table_vel.velocity.y = 0.0;
            }
        }

        let mut pos = table_tf.translation;
        table::snap_out_of_walls(&mut pos, table_half, &wall_grid);
        table_tf.translation = pos;
    }

    player_tf.translation.x += player_correction.x;
    player_tf.translation.y += player_correction.y;
}

fn apply_breach_force_to_player(
    time: Res<Time>,
    grid_query: Query<&crate::fluiddynamics::FluidGrid>,
    mut player_query: Query<(&Transform, &mut Velocity, &PulledByFluid), With<Player>>,
) {
    let Ok(grid) = grid_query.single() else {
        return;
    };
    
    if grid.breaches.is_empty() {
        return;
    }
    
    let cell_size = crate::TILE_SIZE;
    let grid_origin_x = -(grid.width as f32 * cell_size) / 2.0;
    let grid_origin_y = -(grid.height as f32 * cell_size) / 2.0;
    
    for (transform, mut velocity, pulled) in &mut player_query {
        let world_pos = transform.translation.truncate();
        
        let grid_x = ((world_pos.x - grid_origin_x) / cell_size) as usize;
        let grid_y = ((world_pos.y - grid_origin_y) / cell_size) as usize;
        
        if grid_x >= grid.width || grid_y >= grid.height {
            continue;
        }
        
        // checks the macroscopic variables (velocity and pressure) at player loc
        let (rho, fluid_vx, fluid_vy) = grid.compute_macroscopic(grid_x, grid_y);
        
        let normal_density = 1.0;
        let pressure_diff = normal_density - rho;
        
        // the threshold you have to get over for the vaccuum forces to actually affect the player
        let pressure_threshold = 0.15;
        
        
        let scaled_pressure_diff = (pressure_diff - pressure_threshold).max(0.0);
        
        let fluid_velocity = Vec2::new(fluid_vx, fluid_vy);

        
        // the strength of the forces that you can tweak to get more visible results
         let pressure_force_strength = 500000.0;
        let velocity_force_strength = 300000.0;
        
        let pressure_force = fluid_velocity.normalize_or_zero()  * scaled_pressure_diff  * pressure_force_strength;
        let velocity_force = fluid_velocity * velocity_force_strength;
        
        let total_force = pressure_force + velocity_force;
        
        let acceleration = total_force / pulled.mass;
        let deltat = time.delta_secs();
        velocity.0 += acceleration * deltat;

        // Cap speed so the player can't tunnel through walls from breach suction.
        // Anything above ~one tile per frame (32 / 0.016s ≈ 2000) causes tunneling;
        // 900 is fast enough to feel pulled while staying safely below that threshold.
        velocity.0 = velocity.0.clamp_length_max(900.0);
    }
}

// Prevents player from being inside walls (e.g., when pushed by tables)
fn wall_correction(pos: &mut Vec2, player_half: Vec2, walls: &[(Vec2, Vec2)]) {
    for &(wall_pos, wall_half) in walls {
        if aabb_overlap(pos.x, pos.y, player_half, wall_pos.x, wall_pos.y, wall_half) {
            let overlap_x = (player_half.x + wall_half.x) - (pos.x - wall_pos.x).abs();
            let overlap_y = (player_half.y + wall_half.y) - (pos.y - wall_pos.y).abs();
            if overlap_x < overlap_y {
                pos.x += if pos.x > wall_pos.x { overlap_x } else { -overlap_x };
            } else {
                pos.y += if pos.y > wall_pos.y { overlap_y } else { -overlap_y };
            }
        }
    }
}

fn wall_collision_correction(
    mut player_q: Query<&mut Transform, With<Player>>,
    wall_grid: Res<crate::map::WallGrid>,
    dynamic_q: Query<(&Transform, &Collider), (With<Collidable>, Without<Player>, Without<crate::map::WallTile>, Without<table::Table>)>,
) {
    let Ok(mut player_tf) = player_q.single_mut() else { return };

    let player_half = Vec2::new(TILE_SIZE * 0.5, TILE_SIZE * 1.0);
    let mut player_pos = player_tf.translation.truncate();

    // Collect only nearby walls from the spatial hash + any dynamic obstacles (not tables).
    let mut walls: Vec<(Vec2, Vec2)> = wall_grid.nearby(player_pos, 4);
    for (tf, col) in &dynamic_q {
        walls.push((tf.translation.truncate(), col.half_extents));
    }

    // Two passes handle corner cases where pushing out of wall A moves into wall B.
    wall_correction(&mut player_pos, player_half, &walls);
    wall_correction(&mut player_pos, player_half, &walls);

    player_tf.translation.x = player_pos.x;
    player_tf.translation.y = player_pos.y;
}

fn thruster_regen_system(
    time: Res<Time>,
    mut q: Query<&mut ThrusterFuel, With<Player>>,
) {
    let Ok(mut fuel) = q.single_mut() else { return; };
    if fuel.max > 0.0 {
        fuel.current = (fuel.current + 0.5 * time.delta_secs()).min(fuel.max);
    }
}

fn thruster_dodge_system(
    input: Res<ButtonInput<KeyCode>>,
    mut q_player: Query<(&Transform, &mut Velocity, &mut ThrusterFuel), With<Player>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
) {
    if !input.just_pressed(KeyCode::ShiftLeft) { return; }

    let Ok((player_tf, mut velocity, mut fuel)) = q_player.single_mut() else { return; };
    if fuel.max <= 0.0 || fuel.current < 1.0 { return; }

    let Ok(window) = q_window.single() else { return; };
    let Some(cursor_pos) = window.cursor_position() else { return; };
    let Ok((camera, cam_transform)) = q_camera.single() else { return; };
    let Some(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos).ok() else { return; };

    let player_pos = player_tf.translation.truncate();
    let dir = (world_pos - player_pos).normalize_or_zero();
    if dir == Vec2::ZERO { return; }

    velocity.0 = dir * 1000.0;
    fuel.current -= 1.0;
}