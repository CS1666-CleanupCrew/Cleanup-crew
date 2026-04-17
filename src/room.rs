use bevy::prelude::*;
use rand::seq::SliceRandom;
use rand::{SeedableRng};
use rand::rngs::StdRng;
use std::collections::HashSet;
use bevy::time::Time;
use crate::collidable::{Collidable, Collider};
use crate::{GameEntity, GameState, TILE_SIZE, Z_ENTITIES};
use crate::map::{Door, TablePositions};
use crate::map::TileRes;
use crate::player::{NumOfCleared, Player};
use crate::enemies::{EnemyRes, LastKillPos, RangedEnemyRes, spawn_enemy_at, spawn_ranged_enemy_at};
use crate::table;

#[derive(Resource)]
pub struct EnemyPosition(pub HashSet<(usize, usize)>);

#[derive(Resource)]
pub enum LevelState{
    EnteredRoom(usize),
    InRoom(usize, Vec3),
    NotRoom
}

#[derive(Resource)]
pub struct RoomVec(pub Vec<Room>);

pub struct Room{
    pub cleared: bool,
    pub visited: bool,
    pub doors:Vec<Entity>,
    pub numofenemies: usize,
    pub top_left_corner: Vec2,
    pub bot_right_corner: Vec2,
    pub tile_top_left_corner: Vec2,
    pub tile_bot_right_corner: Vec2,
    layout: Vec<String>,
    pub air_pressure: f32,
    pub breaches: Vec<Vec2>,
}

impl Room{
    pub fn new(tlc: Vec2, brc: Vec2, tile_tlc: Vec2, tile_brc: Vec2, room_layout: Vec<String>) -> Self{
        Self{
            cleared: false,
            visited: false,
            doors:Vec::new(),
            numofenemies: 0,
            top_left_corner: tlc.clone(),
            bot_right_corner: brc.clone(),
            tile_top_left_corner: tile_tlc.clone(),
            tile_bot_right_corner: tile_brc.clone(),
            layout: room_layout.clone(),
            air_pressure: 100.0,
            breaches: Vec::new(),
        }
    }

    pub fn bounds_check(&self, pos:Vec2) -> bool{
        self.top_left_corner.x <= pos.x && self.top_left_corner.y >= pos.y && self.bot_right_corner.x >= pos.x && self.bot_right_corner.y <= pos.y
    }

    pub fn within_bounds_check(&self, pos:Vec2) -> bool{
        self.top_left_corner.x+64.0 < pos.x.floor() && self.top_left_corner.y-64.0 > pos.y.floor() && self.bot_right_corner.x-64.0 > pos.x.floor() && self.bot_right_corner.y+64.0 < pos.y.floor()
    }
}

pub struct RoomPlugin;

#[derive(Component)]
pub struct AirPressureUI;

#[derive(Component)]
pub struct AirTankUI;

impl Plugin for RoomPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameState::Loading), setup)
            .add_systems(OnEnter(GameState::Playing), (setup_air_pressure_ui, spawn_all_tables))
            .add_systems(Update, (
                sync_active_room,
                track_rooms,
                entered_room,
                playing_room,
                track_window_breaches,
                update_room_air_pressure,
                apply_breach_forces_to_entities,
                damage_player_from_low_pressure,
                update_air_pressure_ui,
                update_air_tank_ui,
            ).run_if(in_state(GameState::Playing)));
    }
}

fn setup(
    mut commands: Commands,
){
    commands.insert_resource(LevelState::NotRoom);
    commands.insert_resource(EnemyPosition(HashSet::new()));
}

/// Keeps ActiveRoom in sync with LevelState so table physics can filter by room.
fn sync_active_room(
    lvlstate: Res<LevelState>,
    mut active: ResMut<table::ActiveRoom>,
) {
    if !lvlstate.is_changed() { return; }
    match *lvlstate {
        LevelState::EnteredRoom(i) | LevelState::InRoom(i, _) => {
            active.0 = Some(i);
        }
        LevelState::NotRoom => {} // keep last known room so tables settle naturally
    }
}

/// Spawn every table for every room up-front, tagged with their room index.
/// Physics systems ignore tables outside the active room, so this costs nothing at runtime.
fn spawn_all_tables(
    mut commands: Commands,
    table_positions: Res<TablePositions>,
    tiles: Res<TileRes>,
    rooms: Res<RoomVec>,
) {
    let table_batch: Vec<_> = table_positions.0.iter().filter_map(|&pos| {
        let check = pos.truncate();
        let room_idx = rooms.0.iter().enumerate()
            .find(|(_, r)| r.bounds_check(check))
            .map(|(i, _)| i)?;

        let mut sprite = Sprite::from_image(tiles.table.clone());
        sprite.custom_size = Some(Vec2::splat(TILE_SIZE * 2.0));
        Some((
            sprite,
            Transform {
                translation: pos,
                scale: Vec3::new(0.5, 1.0, 1.0),
                ..Default::default()
            },
            Collidable,
            Collider { half_extents: Vec2::splat(TILE_SIZE * 0.5) },
            Name::new("Table"),
            table::Table,
            table::TableRoom(room_idx),
            table::Health(50.0),
            table::TableState::Intact,
            GameEntity,
        ))
    }).collect();

    commands.spawn_batch(table_batch);
}

pub fn create_room(
    tlc: Vec2,
    brc: Vec2,
    tile_tlc: Vec2,
    tile_brc: Vec2,
    rooms_vec: &mut RoomVec,
    room_layout: Vec<String>,
){
    rooms_vec.0.push(Room::new(tlc, brc, tile_tlc, tile_brc, room_layout));
}

pub fn assign_doors(
    doors: Query<(Entity, &Transform), With<Door>>,
    mut rooms: ResMut<RoomVec>,
){
    for (entity, pos) in doors.iter(){
        for room in rooms.0.iter_mut(){
            if room.bounds_check(Vec2::new(pos.translation.x, pos.translation.y)) {
                room.doors.push(entity);
                break;
            }
        }
    }
}

// pub fn assign_tables(
//     tables: Query<(Entity, &Transform), With<ATABLE>>,
//     mut rooms: ResMut<RoomVec>,
// ){
//     for (entity, pos) in tables.iter(){
//         for room in rooms.0.iter_mut(){
//             if room.bounds_check(Vec2::new(pos.translation.x, pos.translation.y)) {
//                 room.tables.push(entity);
//                 break;
//             }
//         }
//     }
// }

pub fn track_rooms(
    player: Single<&Transform, With<Player>>,
    mut rooms: ResMut<RoomVec>,
    mut lvlstate: ResMut<LevelState>,
){
    let pos = player.into_inner();
    let player_pos = Vec2::new(pos.translation.x, pos.translation.y);

    // Mark rooms visited as soon as the player steps inside their bounds,
    // regardless of current level state (so cleared rooms stay visible on minimap).
    for room in rooms.0.iter_mut() {
        if !room.visited && room.bounds_check(player_pos) {
            room.visited = true;
        }
    }

    // Only look for a new room trigger when we are not already processing one.
    match *lvlstate {
        LevelState::EnteredRoom(_) | LevelState::InRoom(_, _) => {}
        LevelState::NotRoom => {
            for (index, room) in rooms.0.iter_mut().enumerate() {
                if !room.cleared && room.within_bounds_check(player_pos) {
                    *lvlstate = LevelState::EnteredRoom(index);
                    break;
                }
            }
        }
    }
}

pub fn entered_room(
    mut rooms:  ResMut<RoomVec>,
    mut lvlstate: ResMut<LevelState>,
    mut commands: Commands,
    tiles: Res<TileRes>,
    enemy_res: Res<EnemyRes>,
    ranged_res: Res<RangedEnemyRes>,
    play_query: Single<&NumOfCleared, With<Player>>,
    station_level: Res<crate::StationLevel>,
    mut shield_query: Query<&mut crate::player::Shield, With<Player>>,
){
    match *lvlstate
    {
        LevelState::EnteredRoom(index) =>
        {
            // Recharge shield on room entry
            if let Ok(mut shield) = shield_query.single_mut() {
                shield.current = shield.max;
            }

            for door in rooms.0[index].doors.iter(){
                commands.entity(*door).insert(Collidable);
                commands.entity(*door).insert(Collider { half_extents: Vec2::splat(TILE_SIZE * 0.5) },);
                commands.entity(*door).insert(Sprite::from_image(tiles.closed_door.clone()));
            }

            if let Some(pos) = generate_enemies_in_room(1, None, &mut rooms, index, &mut commands, &enemy_res, &ranged_res, &play_query, station_level.0){
                *lvlstate = LevelState::InRoom(index, pos);
            } else {
                // Room is too small/tight to place any enemies — clear it immediately
                // and reopen the doors so the player is never locked in.
                rooms.0[index].cleared = true;
                for door in rooms.0[index].doors.iter() {
                    commands.entity(*door).remove::<Collidable>();
                    commands.entity(*door).remove::<Collider>();
                    commands.entity(*door).insert(Sprite::from_image(tiles.open_door.clone()));
                }
                *lvlstate = LevelState::NotRoom;
            }
        }
        _ => {}
    }
}

pub fn playing_room(
    mut rooms:  ResMut<RoomVec>,
    mut lvlstate: ResMut<LevelState>,
    mut commands: Commands,
    tiles: Res<TileRes>,
    mut player: Single<&mut NumOfCleared, With<Player>>,
    heart_res: Res<crate::heart::HeartRes>,
    reward_res: Res<crate::rewards::RewardRes>,
    last_kill_pos: Res<LastKillPos>,
){
    match *lvlstate
    {
        LevelState::InRoom(index, reward_pos) =>
        {
            if rooms.0[index].numofenemies == 0{
                debug!("All enemies defeated");

                crate::heart::spawn_heart(&mut commands, &heart_res, last_kill_pos.0);
                crate::rewards::spawn_reward(&mut commands, reward_pos, &reward_res);

                for door in rooms.0[index].doors.iter(){
                    commands.entity(*door).remove::<Collidable>();
                    commands.entity(*door).remove::<Collider>();
                    commands.entity(*door).insert(Sprite::from_image(tiles.open_door.clone()));
                }

                rooms.0[index].cleared = true;
                //rooms.0.remove(index);
                player.0 += 1;
                *lvlstate = LevelState::NotRoom;
            }
        }
        _ => {}
    }
}

pub fn generate_enemies_in_room(
    num_of_enemies: usize,
    seed: Option<u64>,
    rooms: &mut RoomVec,
    index: usize,
    mut commands: &mut Commands,
    enemy_res: &EnemyRes,
    ranged_res: &RangedEnemyRes,
    play_query: &NumOfCleared,
    station_level: u32,

) -> Option<Vec3> {
    let rooms_cleared = play_query.0;
    let mut floors: Vec<(f32, f32)> = Vec::new();

    let room = &mut rooms.0[index];
    // Scale enemy count: base + rooms_cleared + station_level bonus
    // Each station adds 2 extra enemies per room
    let station_bonus = (station_level as usize) * 2;
    let scaled_num_enemies = 1 * rooms_cleared + num_of_enemies + station_bonus;
    room.numofenemies = scaled_num_enemies;

    // Health multiplier: each station increases enemy health by 50%
    let health_multiplier = 1.0 + (station_level as f32) * 0.5;

    // Speed bonus: +10 units per room cleared, giving a gradual ramp-up
    let speed_bonus = rooms_cleared as f32 * 10.0;

    let height = room.layout.len() - 6;
    if height <= 0 { return None; }
    
    let width = room.layout[0].len() - 6;

    for ly in 5..height {
        let row = &room.layout[ly];

        for lx in 5..width {
            let ch = row.as_bytes()[lx] as char;
            
            if ch == '#' {
                let world_x = room.top_left_corner.x + lx as f32 * TILE_SIZE;

                let world_y = room.top_left_corner.y - ly as f32 * TILE_SIZE;

                floors.push((world_x, world_y));
            }
        }
    }

    if floors.is_empty() {
        info!("Room {} has zero floor tiles! Cannot spawn enemies.", index);
        return None;
    }

    if let Some(s) = seed {
        let mut seeded = StdRng::seed_from_u64(s);
        floors.shuffle(&mut seeded);
    } else {
        let mut trng = rand::rng();
        floors.shuffle(&mut trng);
    }

    // Iterate over ALL floor candidates (not just scaled_num_enemies of them) so that
    // the adjacent_to_wall filter doesn't silently drop spawns and leave numofenemies
    // higher than the number of enemies that actually exist in the room.
    let mut actually_spawned: usize = 0;
    let mut spawn_idx: usize = 0; // separate counter so ranged ratio stays consistent
    for (x, y) in floors.iter() {
        if actually_spawned >= scaled_num_enemies {
            break;
        }

        let tile_x = ((*x - room.top_left_corner.x) / TILE_SIZE).round() as isize;
        let tile_y = ((room.top_left_corner.y - *y) / TILE_SIZE).round() as isize;

        // Skip tiles adjacent to walls or windows
        let mut adjacent_to_wall = false;
        'adj: for dy in -1..=1isize {
            for dx in -1..=1isize {
                if dx == 0 && dy == 0 { continue; }

                let nx = tile_x + dx;
                let ny = tile_y + dy;

                if nx < 0 || ny < 0
                    || ny as usize >= room.layout.len()
                    || nx as usize >= room.layout[ny as usize].len()
                {
                    continue;
                }

                let ch = room.layout[ny as usize].as_bytes()[nx as usize] as char;
                // '.' = border tile → becomes W via generate_walls
                // ',' = passage tile → also becomes W via generate_walls
                // 'W' = explicit wall, 'G' = glass window
                if matches!(ch, 'W' | 'G' | '.' | ',') {
                    adjacent_to_wall = true;
                    break 'adj;
                }
            }
        }

        if adjacent_to_wall {
            continue;
        }

        // Also skip if the tile itself is outside the room's world bounds (safety net).
        if !room.bounds_check(Vec2::new(*x, *y)) {
            continue;
        }

        let pos = Vec3::new(*x, *y, Z_ENTITIES);

        if spawn_idx % 4 == 2 {
            spawn_ranged_enemy_at(&mut commands, ranged_res, pos, true, health_multiplier, speed_bonus);
        } else {
            spawn_enemy_at(&mut commands, enemy_res, pos, true, health_multiplier, speed_bonus);
        }
        actually_spawned += 1;
        spawn_idx += 1;
    }

    // numofenemies must equal what was actually spawned — a mismatch would permanently
    // lock the room if some tiles were rejected by the wall-adjacency filter.
    room.numofenemies = actually_spawned;

    if actually_spawned == 0 {
        info!("Room {}: all candidate tiles were adjacent to walls, cannot spawn.", index);
        return None;
    }

    if let Some(s) = seed {
        let mut seeded = StdRng::seed_from_u64(s);
        floors.shuffle(&mut seeded);
    } else {
        let mut trng = rand::rng();
        floors.shuffle(&mut trng);
    }

    Some(Vec3::new(floors[0].0, floors[0].1, Z_ENTITIES))

    // debug!("Room {}: spawned {} enemies", index, scaled_num_enemies);
}




pub fn generate_enemies_for_all_rooms(
    num_of_enemies: usize,
    seed: Option<u64>,
    rooms: &RoomVec,
    enemy_hash: &mut EnemyPosition,
    grid: &Vec<String>
){  
    for (_i, room) in rooms.0.iter().enumerate()
    {
        let mut floors: Vec<(usize, usize)> = Vec::new();
        let top = room.tile_top_left_corner.y as usize;
        let bot = room.tile_bot_right_corner.y as usize;

        for y in bot..top
        { 
            let row = &grid[y];
            for (x, ch) in row.chars().enumerate()
            {
                if x > room.tile_top_left_corner.x as usize && x < room.tile_bot_right_corner.x as usize
                {
                    if ch == '#' 
                    {
                        floors.push((x, y));
                    }
                }
            }
        }

        if let Some(s) = seed 
        {
            let mut seeded = StdRng::seed_from_u64(s);
            floors.shuffle(&mut seeded);
        } else {
            let mut trng = rand::rng();
            floors.shuffle(&mut trng);
        }

        enemy_hash.0.extend(floors.into_iter().take(num_of_enemies));
    }
}

pub fn update_room_air_pressure(
    time: Res<Time>,
    mut rooms: ResMut<RoomVec>,
) {
    let refill_rate = 5.0; // %/sec when all windows are sealed

    for (idx, room) in rooms.0.iter_mut().enumerate() {
        if room.breaches.is_empty() {
            if room.air_pressure < 100.0 {
                let old_pressure = room.air_pressure;
                room.air_pressure = (room.air_pressure + refill_rate * time.delta_secs()).min(100.0);
                if (old_pressure / 10.0).floor() != (room.air_pressure / 10.0).floor() {
                    debug!("Room {} refilling: {:.1}%", idx, room.air_pressure);
                }
            }
            continue;
        }

        let base_escape_rate = 2.5;
        let total_escape_rate = base_escape_rate * room.breaches.len() as f32;

        let old_pressure = room.air_pressure;

        room.air_pressure -= total_escape_rate * time.delta_secs();
        room.air_pressure = room.air_pressure.max(0.0);

        if (old_pressure / 10.0).floor() != (room.air_pressure / 10.0).floor() {
            debug!("Room {} pressure: {:.1}% (escaping at {:.1}%/sec)",
                  idx, room.air_pressure, total_escape_rate);
        }
    }
}

pub fn track_window_breaches(
    mut rooms: ResMut<RoomVec>,
    windows: Query<(&Transform, &crate::window::GlassState), With<crate::window::Window>>,
) {
    for (window_transform, glass_state) in windows.iter() {
        let window_pos = window_transform.translation.truncate();

        for (_idx, room) in rooms.0.iter_mut().enumerate() {
            let expanded_tlc = Vec2::new(room.top_left_corner.x - 64.0, room.top_left_corner.y + 64.0);
            let expanded_brc = Vec2::new(room.bot_right_corner.x + 64.0, room.bot_right_corner.y - 64.0);

            let in_expanded_bounds = expanded_tlc.x <= window_pos.x
                && expanded_tlc.y >= window_pos.y
                && expanded_brc.x >= window_pos.x
                && expanded_brc.y <= window_pos.y;

            if !in_expanded_bounds {
                continue;
            }

            match glass_state {
                crate::window::GlassState::Broken => {
                    if !room.breaches.iter().any(|&b| b.distance(window_pos) < 1.0) {
                        room.breaches.push(window_pos);
                        // debug!("Breach added to room {} at {:?}", idx, window_pos);
                    }
                }
                crate::window::GlassState::Intact => {
                    let before = room.breaches.len();
                    room.breaches.retain(|&b| b.distance(window_pos) >= 1.0);
                    if room.breaches.len() != before {
                        // debug!("Breach removed from room {} at {:?}", idx, window_pos);
                    }
                }
            }
            break;
        }
    }
}


pub fn apply_breach_forces_to_entities(
    time: Res<Time>,
    rooms: Res<RoomVec>,
    mut tables: Query<(&Transform, &mut crate::enemies::Velocity, &crate::fluiddynamics::PulledByFluid), With<crate::table::Table>>,
    mut player: Query<(&Transform, &mut crate::bullet::Velocity, &crate::fluiddynamics::PulledByFluid), (With<crate::player::Player>, Without<crate::table::Table>)>,  // Changed to bullet::Velocity
    mut enemies: Query<(&Transform, &mut crate::enemies::Velocity, &crate::fluiddynamics::PulledByFluid), (With<crate::enemies::Enemy>, Without<crate::player::Player>, Without<crate::table::Table>)>,
) {
    // Determine which room the player is in
    let player_room = if let Ok((player_transform, _, _)) = player.single() {
        let player_pos = player_transform.translation.truncate();
        rooms.0.iter().find(|room| room.bounds_check(player_pos))
    } else {
        None
    };

    let Some(room) = player_room else {
        //println!("Player is not inside any room. Physics deactivated for all rooms.");
        return;
    };

    if room.breaches.is_empty() {
        //println!("Player entered room, but it has no breaches. Physics deactivated for this room.");
        return;
    }

    //println!("Player entered room. Physics for this room is activated. Other rooms are deactivated.");

    // Helper closure to apply suction toward the room's breaches
    let apply_suction = |world_pos: Vec2, mass: f32, velocity: &mut Vec2| {
        let mut total_force = Vec2::ZERO;
        for &breach_world_pos in &room.breaches {
            let to_breach = breach_world_pos - world_pos;
            let distance = to_breach.length();
            if distance > 1.0 {
                total_force += to_breach.normalize() * 25000.0; // force magnitude
            }
        }
        let acceleration = total_force / mass;
        *velocity += acceleration * time.delta().as_secs_f32();

        // Clamp maximum speed
        let max_velocity = 200.0;
        if velocity.length() > max_velocity {
            *velocity = velocity.normalize() * max_velocity;
        }
    };

    // Apply only to player in the room (always in that room)
    if let Ok((transform, mut velocity, pulled_by_fluid)) = player.single_mut() {
        apply_suction(transform.translation.truncate(), pulled_by_fluid.mass, &mut **velocity);
    }

    // Apply only to enemies in the room
    for (transform, mut velocity, pulled_by_fluid) in enemies.iter_mut() {
        let pos = transform.translation.truncate();
        if room.bounds_check(pos) {
            apply_suction(pos, pulled_by_fluid.mass, &mut velocity.velocity);
        }
    }

    // Apply only to tables inside the player's room
    for (transform, mut velocity, pulled_by_fluid) in tables.iter_mut() {
        let pos = transform.translation.truncate();
        if room.bounds_check(pos) {
            apply_suction(pos, pulled_by_fluid.mass, &mut velocity.velocity);
        }
    }
}

pub fn damage_player_from_low_pressure(
    time: Res<Time>,
    rooms: Res<RoomVec>,
    mut player: Query<(&Transform, &mut crate::player::Health, &mut crate::player::DamageTimer, &mut crate::player::AirTank), With<crate::player::Player>>,
) {
    let Ok((transform, mut health, mut damage_timer, mut tank)) = player.single_mut() else {
        return;
    };

    let player_pos = transform.translation.truncate();
    let mut current_room: Option<&Room> = None;

    for room in rooms.0.iter() {
        if room.bounds_check(player_pos) {
            current_room = Some(room);
            break;
        }
    }

    let Some(room) = current_room else {
        return;
    };

    let pressure_threshold = 20.0;

    if room.air_pressure < pressure_threshold {
        // Low air: drain the tank
        tank.current = (tank.current - tank.drain_rate * time.delta_secs()).max(0.0);

        // Only damage the player once the tank is fully depleted
        if tank.current <= 0.0 {
            damage_timer.tick(time.delta());

            if damage_timer.finished() {
                let damage = 5.0;
                health.0 -= damage;
                damage_timer.reset();

                debug!(
                    "Player taking pressure damage! Room pressure: {:.1}% - HP: {:.1}",
                    room.air_pressure, health.0
                );
            }
        }
    }
}

fn setup_air_pressure_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let font: Handle<Font> = asset_server.load(
        "fonts/BitcountSingleInk-VariableFont_CRSV,ELSH,ELXP,SZP1,SZP2,XPN1,XPN2,YPN1,YPN2,slnt,wght.ttf"
    );

    commands.spawn((
        Text::new("Air: 100%"),
        TextFont {
            font: font.clone(),
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgb(0.0, 1.0, 0.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            ..default()
        },
        AirPressureUI,
        GameEntity,
    ));

    commands.spawn((
        Text::new("Tank: 100%"),
        TextFont {
            font,
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgb(0.2, 1.0, 0.5)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(38.0),
            right: Val::Px(10.0),
            ..default()
        },
        AirTankUI,
        GameEntity,
    ));
}

fn update_air_pressure_ui(
    rooms: Res<RoomVec>,
    player: Query<&Transform, With<Player>>,
    mut ui_query: Query<(&mut Text, &mut TextColor), With<AirPressureUI>>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };

    let Ok((mut text, mut color)) = ui_query.single_mut() else {
        return;
    };

    let player_pos = player_transform.translation.truncate();
    let mut current_pressure = 100.0;

    for room in rooms.0.iter() {
        if room.bounds_check(player_pos) {
            current_pressure = room.air_pressure;
            break;
        }
    }

    **text = format!("Air: {:.0}%", current_pressure);

    color.0 = if current_pressure < 20.0 {
        Color::srgb(1.0, 0.0, 0.0)
    } else if current_pressure < 50.0 {
        Color::srgb(1.0, 1.0, 0.0)
    } else {
        Color::srgb(0.0, 1.0, 0.0)
    };
}

fn update_air_tank_ui(
    player: Query<(&Transform, &crate::player::AirTank), With<Player>>,
    rooms: Res<RoomVec>,
    mut ui_query: Query<(&mut Text, &mut TextColor), With<AirTankUI>>,
) {
    let Ok((transform, tank)) = player.single() else {
        return;
    };
    let Ok((mut text, mut color)) = ui_query.single_mut() else {
        return;
    };

    let player_pos = transform.translation.truncate();
    let in_low_air_room = rooms.0.iter()
        .find(|r| r.bounds_check(player_pos))
        .map(|r| r.air_pressure < 20.0)
        .unwrap_or(false);

    let pct = (tank.current / tank.max_capacity * 100.0).clamp(0.0, 100.0);
    **text = format!("Tank: {:.0}%", pct);

    color.0 = if tank.current <= 0.0 {
        Color::srgb(1.0, 0.1, 0.1)
    } else if in_low_air_room {
        // Draining — fade orange to red as tank depletes
        let t = pct / 100.0;
        Color::srgb(1.0, t * 0.6, 0.1)
    } else {
        Color::srgb(0.2, 1.0, 0.5)
    };
}
