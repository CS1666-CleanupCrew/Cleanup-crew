use bevy::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

use crate::collidable::{Collidable, Collider};
use crate::player;
use crate::procgen::generate_shaped_tables;
use crate::room::*; // RoomRes, track_rooms
use crate::window;
use crate::{GameState, MainCamera, GameEntity, TILE_SIZE, WIN_H, WIN_W, Z_FLOOR};
use crate::procgen::{ProcgenSet};


#[derive(Resource, Debug, Clone)]
pub struct LevelToLoad(pub String);

impl Default for LevelToLoad {
    fn default() -> Self {
        Self("assets/rooms/level.txt".to_string())
    }
}



#[derive(Resource)]
pub struct TileRes {
    pub floor: Handle<Image>,
    pub wall: Handle<Image>,
    pub glass: Handle<Image>,
    pub table: Handle<Image>,
    pub closed_door: Handle<Image>,
    pub open_door: Handle<Image>,
}

#[derive(Resource)]
pub struct TablePositions(pub Vec<Vec3>);

#[derive(Resource)]
pub struct LevelRes {
    pub level: Vec<String>,
}

#[derive(Resource, Default)]
pub struct EnemySpawnPoints(pub Vec<Vec3>);

#[derive(Component)]
pub struct Door {
    pub is_open: bool,
    pub pos: Vec2,
}

#[derive(Resource, Copy, Clone)]
pub struct MapGridMeta {
    pub x0: f32,
    pub y0: f32,
    pub cols: usize,
    pub rows: usize,
}

/// Marker for permanent wall tile entities — used to exclude them from
/// dynamic ECS collision queries (they live in WallGrid instead).
#[derive(Component)]
pub struct WallTile;

/// Spatial hash of permanent wall tile positions built at level load.
/// Replaces O(all_walls) ECS iteration with an O(1) neighbourhood lookup.
#[derive(Resource)]
pub struct WallGrid {
    /// (col, row) in tile-space → half-extents of that wall cell.
    cells: HashMap<(i32, i32), Vec2>,
    pub cell_size: f32,
    pub x0: f32,
    pub y0: f32,
}

impl WallGrid {
    fn world_to_key(&self, pos: Vec2) -> (i32, i32) {
        (
            ((pos.x - self.x0) / self.cell_size).round() as i32,
            ((pos.y - self.y0) / self.cell_size).round() as i32,
        )
    }

    fn key_to_world(&self, col: i32, row: i32) -> Vec2 {
        Vec2::new(
            self.x0 + col as f32 * self.cell_size,
            self.y0 + row as f32 * self.cell_size,
        )
    }

    /// Returns (world_pos, half_extents) for every wall cell within
    /// `radius` tiles of `pos`.  With radius=3 this checks at most 49
    /// cells rather than every wall in the entire level.
    pub fn nearby(&self, pos: Vec2, radius: i32) -> Vec<(Vec2, Vec2)> {
        let (cx, cy) = self.world_to_key(pos);
        let mut out = Vec::with_capacity(((radius * 2 + 1) * (radius * 2 + 1)) as usize);
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let key = (cx + dx, cy + dy);
                if let Some(&half) = self.cells.get(&key) {
                    out.push((self.key_to_world(key.0, key.1), half));
                }
            }
        }
        out
    }

    /// Remove a cell when a breakable tile (e.g. glass) is destroyed.
    pub fn remove(&mut self, pos: Vec2) {
        let key = self.world_to_key(pos);
        self.cells.remove(&key);
    }
}


pub struct MapPlugin;
impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<LevelToLoad>()
            // load_map should run after the full level (which itself runs after load_rooms)
            .add_systems(
                OnEnter(GameState::Loading),
                load_map.after(ProcgenSet::BuildFullLevel),
            )
            // build tilemap after both build_full_level and load_map
            .add_systems(
                OnEnter(GameState::Loading),
                setup_tilemap.after(ProcgenSet::BuildFullLevel).after(load_map),
            )
            .add_systems(OnEnter(GameState::Loading), assign_doors.after(setup_tilemap))
            .add_systems(OnEnter(GameState::Loading), playing_state.after(assign_doors))
            .add_systems(Update, follow_player.run_if(in_state(GameState::Playing)))
            .add_systems(Update, track_rooms.run_if(in_state(GameState::Playing)));
    }
}

// One char = one 32×32 tile.
// Legend:
//  '#' = floor tile
//  '.' = empty
//  'T' = table (floor renders underneath)
//  'W' = wall (floor renders underneath + collidable wall sprite)
//   'G' = glass window
// Minimum of 40 cols (1280/32), 23 rows (720/32 = 22.5))

fn playing_state(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Playing);
}

// Makes lower walls spawn above higher walls
fn z_from_y(y: f32) -> f32 {
    Z_FLOOR + 10.0 - y * 0.001
}

fn load_map(mut commands: Commands, asset_server: Res<AssetServer>,
    level_to_load: ResMut<LevelToLoad>,) {
    let mut level = LevelRes {
        level: Vec::new(),
    };
    let tiles = TileRes {
        floor: asset_server.load("map/floortile.png"),
        wall: asset_server.load("map/walls.png"),
        glass: asset_server.load("map/window.png"),
        table: asset_server.load("map/table.png"),
        closed_door: asset_server.load("map/closed_door.png"),
        open_door: asset_server.load("map/open_door.png"),
    };
    commands.insert_resource(tiles);

    //Change this path for a different map
    //info!("Loading map: {}", level_to_load.0);
    let f = File::open(level_to_load.0.clone()).expect("file don't exist");
    let reader = BufReader::new(f);

    for line_result in reader.lines() {
        let line = line_result.unwrap();
        level.level.push(line);
    }
    commands.insert_resource(level);
}

pub fn setup_tilemap(
    mut commands: Commands,
    tiles: Res<TileRes>,
    mut fluid_query: Query<&mut crate::fluiddynamics::FluidGrid>,
    level: Res<LevelRes>,
    _enemies: ResMut<EnemyPosition>,
    rooms: Res<RoomVec>,
) {
    // Map dimensions are taken from the generated level we actually spawn
    let map_cols = level.level.first().map(|r| r.len()).unwrap_or(0) as f32;
    let map_rows = level.level.len() as f32;

    //info!("Spawning level: {} cols × {} rows", map_cols as usize, map_rows as usize);

    let map_px_w = map_cols * TILE_SIZE;
    let map_px_h = map_rows * TILE_SIZE;
    let x0 = -map_px_w * 0.5 + TILE_SIZE * 0.5;
    let y0 = -map_px_h * 0.5 + TILE_SIZE * 0.5;
    
    commands.insert_resource(MapGridMeta {
        x0,
        y0,
        cols: map_cols as usize,
        rows: map_rows as usize,
    });


    // lets you pick the number of tables and an optional seed
    let generated_tables = generate_shaped_tables(&rooms, &level.level, None);
    //generate_enemies_from_grid(&level.level, 15, None, &mut enemies, & rooms);
    // let enemy_spawns = generate_enemy_spawns_from_grid(&level.level, 15, &_rooms, None);
    // commands.insert_resource(EnemySpawnPoints(enemy_spawns));
    commands.insert_resource(EnemySpawnPoints(Vec::new()));

    // positions we'll mark as breaches in the fluid grid 
    let breach_positions = Vec::new();

    // Collect wall/table/glass/door positions and floor strips in one pass.
    // Floor tiles are grouped into contiguous horizontal strips (one entity per run)
    // instead of one entity per tile, cutting floor entity count by ~room_width times.
    let mut wall_positions = Vec::new();
    let mut table_positions = Vec::new();
    let mut glass_positions = Vec::new();
    let mut door_positions = Vec::new();
    let mut floor_strips: Vec<(Vec3, Vec2)> = Vec::new(); // (center, size)

    for (row_i, row) in level.level.iter().enumerate() {
        let y = y0 + (map_rows - 1.0 - row_i as f32) * TILE_SIZE;
        let chars: Vec<char> = row.chars().collect();
        let row_len = chars.len();

        let mut strip_start: Option<usize> = None;
        let mut strip_len = 0usize;

        let flush_strip = |start: usize, len: usize, strips: &mut Vec<(Vec3, Vec2)>| {
            if len == 0 { return; }
            let strip_w = len as f32 * TILE_SIZE;
            let cx = x0 + start as f32 * TILE_SIZE + (len - 1) as f32 * TILE_SIZE * 0.5;
            strips.push((Vec3::new(cx, y, Z_FLOOR), Vec2::new(strip_w, TILE_SIZE)));
        };

        for col_i in 0..=row_len {
            let is_floor = if col_i < row_len {
                let ch = chars[col_i];
                let is_gen_table = generated_tables.contains(&(col_i, row_i));
                matches!(ch, '#' | 'S' | 'T' | 'W' | 'G' | 'E' | 'D') || is_gen_table
            } else {
                false // sentinel to flush the last strip
            };

            if is_floor {
                if strip_start.is_none() {
                    strip_start = Some(col_i);
                    strip_len = 0;
                }
                strip_len += 1;
            } else if let Some(start) = strip_start.take() {
                flush_strip(start, strip_len, &mut floor_strips);
                strip_len = 0;
            }

            if col_i == row_len { break; }

            let ch = chars[col_i];
            let x = x0 + col_i as f32 * TILE_SIZE;
            let is_generated_table = generated_tables.contains(&(col_i, row_i));
            let is_generated_enemy = false;

            match (ch, is_generated_table, is_generated_enemy) {
                ('T', _, false) | (_, true, false) => {
                    table_positions.push(Vec3::new(x, y, Z_FLOOR + 2.0));
                }
                ('W', _, _) => {
                    wall_positions.push(Vec3::new(x, y, Z_FLOOR + 1.0));
                }
                ('G', _, _) => {
                    glass_positions.push(Vec3::new(x, y, Z_FLOOR + 1.0));
                }
                ('D', _, _) => {
                    door_positions.push(Vec2::new(x, y));
                }
                _ => {}
            }
        }
    }

    // Batch spawn floor strips — one entity per contiguous horizontal run
    let floor_batch: Vec<_> = floor_strips.iter().map(|&(pos, size)| {
        let mut sprite = Sprite::from_image(tiles.floor.clone());
        sprite.custom_size = Some(size);
        sprite.image_mode = bevy::sprite::SpriteImageMode::Tiled {
            tile_x: true,
            tile_y: false,
            stretch_value: 1.0,
        };
        (
            sprite,
            Transform::from_translation(pos),
            Name::new("Floor"),
            GameEntity,
        )
    }).collect();
    commands.spawn_batch(floor_batch);

    // Build wall spatial hash — O(1) neighbourhood lookup replaces
    // the O(n_walls) linear scan done every frame in collision systems.
    let mut wall_cells = HashMap::new();
    for &pos in &wall_positions {
        let key = (
            ((pos.x - x0) / TILE_SIZE).round() as i32,
            ((pos.y - y0) / TILE_SIZE).round() as i32,
        );
        wall_cells.insert(key, Vec2::splat(TILE_SIZE * 0.5));
    }
    commands.insert_resource(WallGrid {
        cells: wall_cells,
        cell_size: TILE_SIZE,
        x0,
        y0,
    });

    // Batch spawn walls
    let wall_batch: Vec<_> = wall_positions.iter().map(|&pos| {
        let mut sprite = Sprite::from_image(tiles.wall.clone());
        sprite.custom_size = Some(Vec2::new(TILE_SIZE,TILE_SIZE*1.5625));
        (
            sprite,
            Transform{
                translation: Vec3::new(pos.x, pos.y, z_from_y(pos.y)),
                scale: Vec3::new(1.0, 1.31, 1.0),
                ..Default::default()
            },
            Collidable,
            Collider { half_extents: Vec2::splat(TILE_SIZE * 0.5) },
            WallTile,
            Name::new("Wall"),
            GameEntity,
        )
    }).collect();
    commands.spawn_batch(wall_batch);

    commands.insert_resource(TablePositions(table_positions));

    // Batch spawn tables
    // let table_batch: Vec<_> = table_positions.iter().map(|&pos| {
    //     let mut sprite = Sprite::from_image(tiles.table.clone());
    //     sprite.custom_size = Some(Vec2::splat(TILE_SIZE * 2.0));
    //     (
    //         sprite,
    //         Transform {
    //             translation: pos,
    //             scale: Vec3::new(0.5, 1.0, 1.0),
    //             ..Default::default()
    //         },
    //         Collidable,
    //         Collider { half_extents: Vec2::splat(TILE_SIZE * 0.5) },
    //         Name::new("Table"),
    //         table::Table,
    //         table::Health(50.0),
    //         table::TableState::Intact,
    //         ATABLE,
    //         GameEntity,
    //     )
    // }).collect();
    // commands.spawn_batch(table_batch);

    // Batch spawn glass windows
    let glass_batch: Vec<_> = glass_positions.iter().map(|&pos| {
        let mut sprite = Sprite::from_image(tiles.glass.clone());
        sprite.custom_size = Some(Vec2::new(TILE_SIZE,TILE_SIZE*1.5625));
        (
            sprite,
            Transform{
                translation: Vec3::new(pos.x, pos.y, z_from_y(pos.y)),
                scale: Vec3::new(1.0, 1.31, 1.0),
                ..Default::default()
            },
            Name::new("Glass"),
            Collidable,
            Collider { half_extents: Vec2::splat(TILE_SIZE * 0.5) },
            window::Window,
            window::Health(50.0),
            window::GlassState::Intact,
            GameEntity,
        )
    }).collect();
    commands.spawn_batch(glass_batch);

    // Batch spawn doors
    let door_batch: Vec<_> = door_positions.iter().map(|&pos| {
        let sprite = Sprite::from_image(tiles.open_door.clone());
        (
            sprite,
            Transform{
                translation: Vec3::new(pos.x, pos.y, z_from_y(pos.y)),
                scale: Vec3::new(1.0, 1.0, 1.0),
                ..Default::default()
            },
            Name::new("Door"),
            Door { is_open: true, pos },
            GameEntity,
        )
    }).collect();
    commands.spawn_batch(door_batch);

    // Push any recorded breach positions into the fluid grid
    if let Ok(mut grid) = fluid_query.single_mut() {
        for &(bx, by) in &breach_positions {
            grid.add_breach(bx, by);
        }
    }

    // info!("Spawned {} enemy spawn points", spawns.0.len());
}

// If you have a problem or a question about this code, talk to vlad.
fn follow_player(
    //these functions are provided directly from bevy
    //finds all entities that are able to transform and are made of the player component
    player_query: Query<&Transform, (With<player::Player>, Without<MainCamera>)>,
    mut camera_query: Query<&mut Transform, (With<MainCamera>, Without<player::Player>)>,
    grid_meta: Res<MapGridMeta>,
) {
    //players current position.
    if let Ok(player_transform) = player_query.single() {
        //This will error out if we would like to have several cameras, this makes the camera mutable
        if let Ok(mut camera_transform) = camera_query.single_mut() {
            let level_width  = grid_meta.cols as f32 * TILE_SIZE;
            let level_height = grid_meta.rows as f32 * TILE_SIZE;

            //these are the bounds for the camera, but it will not move horizontally because we have an exact match between the window and tile width
            let max_x = (level_width - WIN_W) * 0.5;
            let min_x = -(level_width - WIN_W) * 0.5;
            let max_y = (level_height - WIN_H) * 0.5;
            let min_y = -(level_height - WIN_H) * 0.5;

            //camera following the player given the bounds
            let target_x = player_transform.translation.x.clamp(min_x, max_x);
            let target_y = player_transform.translation.y.clamp(min_y, max_y);
            camera_transform.translation.x = target_x;
            camera_transform. translation.y = target_y;
        }
    }
}


pub fn generate_enemy_spawns_from_grid(
    grid: &[String],
    max_enemies: usize,
    room_vec: &RoomVec,
    seed: Option<u64>,
) -> Vec<Vec3> {
    let rows = grid.len();
    if rows == 0 {
        return Vec::new();
    }
    
    let map_center_x = (grid[0].len() / 2) as f32;
    let map_center_y = (rows / 2) as f32;
    
    // Collect all floor cells ('#') that are NOT on room edges
    let mut valid_spawns: Vec<Vec3> = Vec::new();
    
    for room in &room_vec.0 {
        let x1 = room.tile_top_left_corner.x as usize;
        let y1 = room.tile_top_left_corner.y as usize;
        let x2 = room.tile_bot_right_corner.x as usize;
        let y2 = room.tile_bot_right_corner.y as usize;
        
        // Only spawn in interior of rooms (not on edges)
        for y in (y1 + 2)..=(y2.saturating_sub(2)) {
            for x in (x1 + 2)..=(x2.saturating_sub(2)) {
                if y < rows && x < grid[0].len() && grid[y].chars().nth(x) == Some('#') {
                    // Convert to world coordinates
                    let world_x = (x as f32 - map_center_x) * TILE_SIZE;
                    let world_y = -(y as f32 - map_center_y) * TILE_SIZE;
                    valid_spawns.push(Vec3::new(world_x, world_y, crate::Z_ENTITIES));
                }
            }
        }
    }
    
    // Shuffle and take max_enemies positions
    use rand::seq::SliceRandom;
    if let Some(s) = seed {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(s);
        valid_spawns.shuffle(&mut rng);
    } else {
        let mut rng = rand::rng();
        valid_spawns.shuffle(&mut rng);
    }
    
    valid_spawns.into_iter().take(max_enemies).collect()
}