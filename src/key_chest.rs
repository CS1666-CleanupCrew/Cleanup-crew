use bevy::prelude::*;
use rand::random_range;
use crate::collidable::{Collidable, Collider};
use crate::{GameEntity, GameState, TILE_SIZE, Z_ENTITIES};
use crate::player::{Player, aabb_overlap};
use crate::enemies::Enemy;
use crate::room::LevelState;

// ─── Components ──────────────────────────────────────────────────────────────

/// Marker added to one random enemy per level; their death spawns the KeyPickup.
#[derive(Component)]
pub struct KeyHolder;

/// The key sprite on the floor waiting to be collected.
#[derive(Component)]
pub struct KeyPickup;

/// The chest entity in the world.
#[derive(Component)]
pub struct Chest;

/// Marker for the bottom-right HUD key icon.
#[derive(Component)]
pub struct KeyHudIcon;

// ─── Resources ───────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct KeyChestRes {
    pub key_img:   Handle<Image>,
    pub chest_img: Handle<Image>,
}

#[derive(Resource)]
pub struct LevelKeyState {
    /// Index (0..6, non-airlock) of the room whose enemies include the KeyHolder.
    pub key_holder_room: usize,
    /// True once `KeyHolder` has been inserted on an enemy entity.
    pub key_assigned: bool,
    /// True once the player has collected the key off the floor.
    pub has_key: bool,
    /// Index (0..6, non-airlock) of the room that contains the chest.
    pub chest_room: usize,
    /// True once the chest entity has been spawned.
    pub chest_spawned: bool,
}

impl LevelKeyState {
    fn new() -> Self {
        let key_room = random_range(0..6usize);
        // Ensure chest is always in a different room than the key holder.
        let r = random_range(0..5usize);
        let chest_room = if r >= key_room { r + 1 } else { r };
        Self {
            key_holder_room: key_room,
            key_assigned: false,
            has_key: false,
            chest_room,
            chest_spawned: false,
        }
    }
}

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct KeyChestPlugin;

impl Plugin for KeyChestPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, load_assets)
            .add_systems(
                OnEnter(GameState::Loading),
                (init_level_key_state, setup_key_hud),
            )
            .add_systems(
                Update,
                (
                    assign_key_holder,
                    spawn_chest_on_room_entry,
                    pickup_key,
                    interact_with_chest,
                    update_key_hud,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

// ─── Systems ─────────────────────────────────────────────────────────────────

fn load_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(KeyChestRes {
        key_img:   asset_server.load("items/key.png"),
        chest_img: asset_server.load("chests/chest_closed.png"),
    });
}

fn init_level_key_state(mut commands: Commands) {
    commands.insert_resource(LevelKeyState::new());
}

fn setup_key_hud(mut commands: Commands, res: Res<KeyChestRes>) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(20.0),
            bottom: Val::Px(20.0),
            width: Val::Px(48.0),
            height: Val::Px(48.0),
            ..default()
        },
        ImageNode::new(res.key_img.clone()),
        Visibility::Hidden,
        ZIndex(10),
        KeyHudIcon,
        GameEntity,
    ));
}

/// Tags one random enemy in the key-holder room with `KeyHolder`.
/// Enemies spawned by `entered_room` may not appear in queries until the
/// following frame, so this retries each frame until it finds at least one.
fn assign_key_holder(
    mut commands: Commands,
    mut key_state: ResMut<LevelKeyState>,
    lvl_state: Res<LevelState>,
    enemy_q: Query<Entity, With<Enemy>>,
) {
    if key_state.key_assigned { return; }
    let LevelState::InRoom(idx, _, _) = *lvl_state else { return };
    if idx != key_state.key_holder_room { return; }

    let enemies: Vec<Entity> = enemy_q.iter().collect();
    if enemies.is_empty() { return; }

    let pick = enemies[random_range(0..enemies.len())];
    commands.entity(pick).insert(KeyHolder);
    key_state.key_assigned = true;
}

/// Spawns the chest the first time the player enters the designated chest room.
/// Uses the pre-computed reward floor position so the chest always lands on
/// a valid floor tile.
fn spawn_chest_on_room_entry(
    mut commands: Commands,
    lvl_state: Res<LevelState>,
    mut key_state: ResMut<LevelKeyState>,
    res: Res<KeyChestRes>,
) {
    if key_state.chest_spawned { return; }
    let LevelState::InRoom(idx, _, chest_pos) = *lvl_state else { return };
    if idx != key_state.chest_room { return; }

    commands.spawn((
        Sprite::from_image(res.chest_img.clone()),
        Transform::from_xyz(chest_pos.x, chest_pos.y, Z_ENTITIES),
        Chest,
        Collidable,
        Collider { half_extents: Vec2::splat(TILE_SIZE * 0.5) },
        GameEntity,
    ));
    key_state.chest_spawned = true;
}

/// Auto-collect the key when the player walks over it.
fn pickup_key(
    mut commands: Commands,
    mut key_state: ResMut<LevelKeyState>,
    player_q: Query<&Transform, With<Player>>,
    key_q: Query<(Entity, &Transform), With<KeyPickup>>,
) {
    let Ok(player_tf) = player_q.single() else { return };
    let pp = player_tf.translation;
    let half = Vec2::splat(TILE_SIZE * 0.6);

    for (entity, key_tf) in &key_q {
        let kp = key_tf.translation;
        if aabb_overlap(pp.x, pp.y, half, kp.x, kp.y, half) {
            commands.entity(entity).despawn();
            key_state.has_key = true;
        }
    }
}

/// Press E near a chest while holding the key to open it.
fn interact_with_chest(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    mut key_state: ResMut<LevelKeyState>,
    player_q: Query<&Transform, With<Player>>,
    chest_q: Query<(Entity, &Transform), With<Chest>>,
    mut inventory_q: Query<&mut crate::weapons::WeaponInventory, With<Player>>,
) {
    if !input.just_pressed(KeyCode::KeyE) { return; }
    if !key_state.has_key { return; }

    let Ok(player_tf) = player_q.single() else { return };
    let pp = player_tf.translation;
    let interact_half = Vec2::splat(TILE_SIZE * 1.5);
    let chest_half = Vec2::splat(TILE_SIZE * 0.5);

    for (entity, chest_tf) in &chest_q {
        let cp = chest_tf.translation;
        if aabb_overlap(pp.x, pp.y, interact_half, cp.x, cp.y, chest_half) {
            commands.entity(entity).despawn();
            key_state.has_key = false;
            if let Ok(mut inv) = inventory_q.single_mut() {
                inv.weapons.push(crate::weapons::Weapon::new(crate::weapons::WeaponType::BeamRifle));
            }
            break;
        }
    }
}

/// Shows or hides the HUD key icon based on whether the player holds the key.
fn update_key_hud(
    key_state: Res<LevelKeyState>,
    mut hud_q: Query<&mut Visibility, With<KeyHudIcon>>,
) {
    let Ok(mut vis) = hud_q.single_mut() else { return };
    *vis = if key_state.has_key { Visibility::Visible } else { Visibility::Hidden };
}

// ─── Public helpers (called from other modules) ───────────────────────────────

/// Spawns a key pickup at the given world position.
/// Called by `enemies::check_enemy_health` when a `KeyHolder` enemy is killed.
pub fn drop_key(commands: &mut Commands, res: &KeyChestRes, pos: Vec3) {
    commands.spawn((
        Sprite::from_image(res.key_img.clone()),
        Transform::from_xyz(pos.x, pos.y, Z_ENTITIES),
        KeyPickup,
        GameEntity,
    ));
}
