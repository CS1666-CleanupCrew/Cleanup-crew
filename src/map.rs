use bevy::prelude::*;
use bevy::scene::ron::de;

use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

use crate::collidable::{Collidable, Collider};
use crate::player;
use crate::procgen::generate_tables_from_grid;
use crate::room::*; // RoomRes, track_rooms
use crate::table;
use crate::window;
use crate::{BG_WORLD, GameState, MainCamera, GameEntity, TILE_SIZE, WIN_H, WIN_W, Z_FLOOR};
use crate::procgen::{ProcgenSet};


#[derive(Resource, Debug, Clone)]
pub struct LevelToLoad(pub String);

impl Default for LevelToLoad {
    fn default() -> Self {
        Self("assets/rooms/level.txt".to_string())
    }
}


#[derive(Component)]
struct ParallaxBg {
    tile: f32,   // world-units per background tile
}

#[derive(Component)]
struct ParallaxCell {
    ix: i32,
    iy: i32,
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
pub struct BackgroundRes(pub Handle<Image>);

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

#[derive(Resource, Default)]
pub struct BgScroll {
    pub offset: f32,
}

pub struct MapPlugin;
impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<LevelToLoad>()
            .init_resource::<BgScroll>()
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
            .add_systems(Update, scroll_background)
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
    let space_tex = BackgroundRes(asset_server.load("map/space.png"));

    commands.insert_resource(tiles);
    commands.insert_resource(space_tex);

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
    space_tex: Res<BackgroundRes>,
    mut fluid_query: Query<&mut crate::fluiddynamics::FluidGrid>,
    level: Res<LevelRes>,
    _enemies: ResMut<EnemyPosition>,
    _rooms: Res<RoomVec>,
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

    // Parallax background tiling
    let cover_w = map_px_w.max(WIN_W) + BG_WORLD;
    let cover_h = map_px_h.max(WIN_H) + BG_WORLD;
    let nx = (cover_w / BG_WORLD).ceil() as i32;
    let ny = (cover_h / BG_WORLD).ceil() as i32;

    let spawns = EnemySpawnPoints::default();

    let pad: i32 = 3;
    
    // Batch spawn background tiles for better performance
    let mut bg_batch = Vec::new();
    for iy in -pad..(ny + 1) {
        for ix in -pad..(nx + 1) {
            let cx = (ix as f32) * BG_WORLD;
            let cy = (iy as f32) * BG_WORLD;

            let mut bg = Sprite::from_image(space_tex.0.clone());
            bg.custom_size = Some(Vec2::splat(BG_WORLD));

            bg_batch.push((
                bg,
                Transform::from_translation(Vec3::new(cx, cy, Z_FLOOR - 50.0)),
                Visibility::default(),
                ParallaxBg { tile: BG_WORLD },
                ParallaxCell { ix, iy },
                Name::new("SpaceBG"),
                GameEntity,
            ));
        }
    }
    commands.spawn_batch(bg_batch);

    // lets you pick the number of tables and an optional seed
    let generated_tables = generate_tables_from_grid(&level.level, 25, None);
    //generate_enemies_from_grid(&level.level, 15, None, &mut enemies, & rooms);

    // positions we'll mark as breaches in the fluid grid 
    let breach_positions = Vec::new();

    // Pre-collect positions by tile type for batch spawning
    let mut floor_positions = Vec::new();
    let mut wall_positions = Vec::new();
    let mut table_positions = Vec::new();
    let mut glass_positions = Vec::new();
    let mut door_positions = Vec::new();

    // First pass: collect all positions based on tile type
    // tile placement from the generated full map
    for (row_i, row) in level.level.iter().enumerate() {
        for (col_i, ch) in row.chars().enumerate() {
            let x = x0 + col_i as f32 * TILE_SIZE;
            let y = y0 + (map_rows - 1.0 - row_i as f32) * TILE_SIZE;

            let is_generated_table = generated_tables.contains(&(col_i, row_i));
            let is_generated_enemy = false;//enemies.0.contains(&(col_i,row_i));

            // always draw floor under solid/interactive tiles & enemy spawns
            if matches!(ch, '#' | 'T' | 'W' | 'G' | 'E') || is_generated_table || is_generated_enemy {
                floor_positions.push(Vec3::new(x, y, Z_FLOOR));
            }

            match (ch, is_generated_table, is_generated_enemy)  {
                ('T', _, false) | (_, true, false) => {
                    table_positions.push(Vec3::new(x, y, Z_FLOOR + 2.0));
                }

                ('W', _, _) => {
                    wall_positions.push(Vec3::new(x, y, Z_FLOOR + 1.0));
                }

                ('G', _, _) => {
                    glass_positions.push(Vec3::new(x, y, Z_FLOOR + 1.0));
                    
                    // mark this tile as a breach for the fluid sim
                    // let (bx, by) = crate::fluiddynamics::world_to_grid(
                    //     Vec2::new(x, y),
                    //     crate::fluiddynamics::GRID_WIDTH,
                    //     crate::fluiddynamics::GRID_HEIGHT,
                    // );
                    // breach_positions.push((bx, by));
                }

                ('D', _, _) => {
                    door_positions.push(Vec2::new(x, y));
                }

                // ('E', _, _) | (_, _, true) => {
                //     spawns.0.push(Vec3::new(x, y, Z_ENTITIES));
                // }

                _ => {}
            }
        }
    }

    // Batch spawn floors - reuse texture handles
    let floor_batch: Vec<_> = floor_positions.iter().map(|&pos| {
        (
            Sprite::from_image(tiles.floor.clone()),
            Transform::from_translation(pos),
            Name::new("Floor"),
            GameEntity,
        )
    }).collect();
    commands.spawn_batch(floor_batch);

    // Batch spawn walls
    let wall_batch: Vec<_> = wall_positions.iter().map(|&pos| {
        let mut sprite = Sprite::from_image(tiles.wall.clone());
        sprite.custom_size = Some(Vec2::splat(TILE_SIZE));
        (
            sprite,
            Transform::from_translation(pos),
            Collidable,
            Collider { half_extents: Vec2::splat(TILE_SIZE * 0.5) },
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
        sprite.custom_size = Some(Vec2::splat(TILE_SIZE));
        (
            sprite,
            Transform::from_translation(pos),
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
        let mut sprite = Sprite::from_image(tiles.open_door.clone());
        sprite.custom_size = Some(Vec2::splat(TILE_SIZE));
        (
            sprite,
            Transform::from_translation(Vec3::new(pos.x, pos.y, Z_FLOOR + 1.0)),
            Name::new("Door"),
            Door { is_open: false, pos },
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
    commands.insert_resource(spawns);
}

fn scroll_background(
    time: Res<Time>,
    mut scroll: ResMut<BgScroll>,
    mut bg_q: Query<(&ParallaxBg, &ParallaxCell, &mut Transform), With<ParallaxBg>>,
) {
    // World-units per second to move the background to the left
    const BG_SCROLL_SPEED: f32 = 50.0;

    // Advance global scroll offset
    scroll.offset += BG_SCROLL_SPEED * time.delta_secs();

    // Wrap helper into [0, tile)
    let wrap = |v: f32, t: f32| ((v % t) + t) % t;

    for (bg, cell, mut tf) in &mut bg_q {
        let tile = bg.tile;

        // Move everything left by scroll.offset and wrap so it tiles seamlessly
        let ox = wrap(-scroll.offset, tile);

        let base_x = (cell.ix as f32) * tile;
        let base_y = (cell.iy as f32) * tile;

        tf.translation.x = base_x + ox;
        tf.translation.y = base_y;
        // z is unchanged (set when spawned)
    }
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
