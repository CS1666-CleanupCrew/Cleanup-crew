use bevy::log::Level;
use bevy::prelude::*;

use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use crate::procgen::generate_tables_from_grid;

use crate::collidable::{Collidable, Collider};
use crate::player;
use crate::enemy::Enemy;
use crate::table;
use crate::window;
use crate::{BG_WORLD, Damage, GameState, MainCamera, TILE_SIZE, WIN_H, WIN_W, Z_FLOOR, Z_ENTITIES};
use crate::procgen::{load_rooms, build_full_level};

#[derive(Component)]
struct ParallaxBg {
    factor: f32, // 0.0 = static, 1.0 = locks to camera
    tile: f32,   // world-units per background tile
}

#[derive(Component)]
struct ParallaxCell {
    ix: i32,
    iy: i32,
}

#[derive(Resource)]
pub struct TileRes {
    floor: Handle<Image>,
    wall: Handle<Image>,
    glass: Handle<Image>,
    table: Handle<Image>,
    closed_door: Handle<Image>,
    open_door: Handle<Image>,
}

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
}

pub struct MapPlugin;
impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app

            .add_systems(OnEnter(GameState::Loading), load_map.after(build_full_level))
            .add_systems(OnEnter(GameState::Loading), setup_tilemap.after(load_map))
            .add_systems(
                OnEnter(GameState::Loading),
                playing_state.after(setup_tilemap),
            )
            .add_systems(Update, follow_player.run_if(in_state(GameState::Playing)))
            .add_systems(Update, parallax_scroll)
            //.add_systems(Update, open_doors_when_clear.run_if(in_state(GameState::Playing)))
            ;
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

fn load_map(mut commands: Commands, asset_server: Res<AssetServer>) {
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
    let f = File::open("assets/rooms/level.txt").expect("file don't exist");
    let reader = BufReader::new(f);

    for line_result in reader.lines() {
        let line = line_result.unwrap();
        level.level.push(line);
    }
    commands.insert_resource(level);
}

pub fn setup_tilemap(
    mut commands: Commands, 
    level: Res<LevelRes>,
    tiles: Res<TileRes>,
    space_tex:Res<BackgroundRes>,
) {
    let floor_tex: Handle<Image> = tiles.floor.clone();
    let wall_tex: Handle<Image>  = tiles.wall.clone();
    let table_tex: Handle<Image> = tiles.table.clone();
    let glass_tex: Handle<Image> = tiles.glass.clone();
    let closed_door_tex: Handle<Image> = tiles.closed_door.clone();
    let open_door_tex: Handle<Image> = tiles.open_door.clone();


    let map_cols = level.level.first().map(|r| r.len()).unwrap_or(0) as f32;
    let map_rows = level.level.len() as f32;

    let map_px_w = map_cols * TILE_SIZE;
    let map_px_h = map_rows * TILE_SIZE;
    let x0 = -map_px_w * 0.5 + TILE_SIZE * 0.5;
    let y0 = -map_px_h * 0.5 + TILE_SIZE * 0.5;

    let cover_w = map_px_w.max(WIN_W) + BG_WORLD;
    let cover_h = map_px_h.max(WIN_H) + BG_WORLD;
    let nx = (cover_w / BG_WORLD).ceil() as i32;
    let ny = (cover_h / BG_WORLD).ceil() as i32;

    let mut spawns = EnemySpawnPoints::default();


    let pad: i32 = 3;
    for iy in -pad..(ny + 1) {
        for ix in -pad..(nx + 1) {
            let cx = (ix as f32) * BG_WORLD;
            let cy = (iy as f32) * BG_WORLD;

            let mut bg = Sprite::from_image(space_tex.0.clone());
            bg.custom_size = Some(Vec2::splat(BG_WORLD));

            commands.spawn((
                bg,
                Transform::from_translation(Vec3::new(cx, cy, Z_FLOOR - 50.0)), // behind floor
                Visibility::default(),
                ParallaxBg { factor: 0.9, tile: BG_WORLD },
                ParallaxCell { ix, iy },
                Name::new("SpaceBG"),
            ));
        }
    }

    // lets you pick the number of tables and an optional seed
    let generated_tables = generate_tables_from_grid(&level.level, 25, None);

    // Loop through the room grid
    for (row_i, row) in level.level.iter().enumerate() {
        for (col_i, ch) in row.chars().enumerate() {
            let x = x0 + col_i as f32 * TILE_SIZE;
            let y = y0 + (map_rows - 1.0 - row_i as f32) * TILE_SIZE;

            let is_generated_table = generated_tables.contains(&(col_i, row_i));

            // Always draw a floor tile under walls/tables/spawns
            if ch == '#' || ch == 'T' || ch == 'W' || ch == 'E' || is_generated_table {
                commands.spawn((
                    Sprite::from_image(floor_tex.clone()),
                    Transform::from_translation(Vec3::new(x, y, Z_FLOOR)),
                    Name::new("Floor"),
                ));
            }

            match (ch, is_generated_table) {
                // Spawn either authored ('T') or generated tables
                ('T', _) | ('#', true) => {
                    let mut sprite = Sprite::from_image(table_tex.clone());
                    sprite.custom_size = Some(Vec2::splat(TILE_SIZE * 2.0));
                    commands.spawn((
                        sprite,
                        Transform {
                            translation: Vec3::new(x, y, Z_FLOOR + 2.0),
                            scale: Vec3::new(0.5, 1.0, 1.0),
                            ..Default::default()
                        },
                        Collidable,
                        Collider { half_extents: Vec2::splat(TILE_SIZE * 0.5) },
                        Name::new("Table"),
                        table::Table,
                        table::Health(50.0),
                        table::TableState::Intact,
                    ));
                }

                ('D', _) => {
                    let mut sprite = Sprite::from_image(closed_door_tex.clone());
                    sprite.custom_size = Some(Vec2::splat(TILE_SIZE));
                    commands.spawn((
                        sprite,
                        Transform::from_translation(Vec3::new(x, y, Z_FLOOR + 1.0)),
                        Collidable,
                        Collider { half_extents: Vec2::splat(TILE_SIZE * 0.5) },
                        Name::new("Door"),
                        Door { is_open: false },
                    ));
                }

                // Spawn walls
                ('W', _) => {
                    let mut sprite = Sprite::from_image(wall_tex.clone());
                    sprite.custom_size = Some(Vec2::splat(TILE_SIZE));
                    commands.spawn((
                        sprite,
                        Transform::from_translation(Vec3::new(x, y, Z_FLOOR + 3.0)),
                        Collidable,
                        Collider { half_extents: Vec2::splat(TILE_SIZE * 0.5) },
                        Name::new("Wall"),
                    ));
                }

                ('G', _) => {
                    let mut sprite = Sprite::from_image(glass_tex.clone());
                    sprite.custom_size = Some(Vec2::splat(TILE_SIZE));
                    commands.spawn((
                        sprite,
                        Transform::from_translation(Vec3::new(x, y, Z_FLOOR + 3.0)),
                        Collidable,
                        Collider { half_extents: Vec2::splat(TILE_SIZE * 0.5) },
                        Name::new("Glass"),
                        window::Window,
                        window::Health(50.0),
                        window::GlassState::Intact,
                    ));
                }


                // Spawn enemies
                ('E', _) => {
                    spawns.0.push(Vec3::new(x, y, Z_ENTITIES));
                }

                _ => {}
            }
        }
    }
    commands.insert_resource(spawns);
}

// fn open_doors_when_clear(
//         enemies: Query<Entity, With<Enemy>>,
//         mut doors: Query<(Entity, &mut Sprite, &mut Door)>,
//         tiles: Res<TileRes>,
//         mut commands: Commands,
//     ) {
//         // If any enemies exist, do nothing
//         if !enemies.is_empty() {
//             return;
//         }

//         // Open any closed doors if no enemies
//         for (entity, mut sprite, mut door) in &mut doors {
//             if !door.is_open {
//                 sprite.image = tiles.open_door.clone();
//                 door.is_open = true;

//                 // Remove collision
//                 commands.entity(entity).remove::<Collidable>();
//                 commands.entity(entity).remove::<Collider>();
//             }
//         }
//     }

fn parallax_scroll(
    cam_q: Query<&Transform, (With<MainCamera>, Without<ParallaxBg>)>,
    mut bg_q: Query<(&ParallaxBg, &ParallaxCell, &mut Transform), With<ParallaxBg>>,
) {
    let Ok(cam_tf) = cam_q.single() else {
        return;
    };
    let cam = cam_tf.translation;

    for (bg, cell, mut tf) in &mut bg_q {
        // continuous parallax offset
        let off_x = -cam.x * (1.0 - bg.factor);
        let off_y = -cam.y * (1.0 - bg.factor);

        // wrap offset into [0, tile)
        let wrap = |v: f32, t: f32| ((v % t) + t) % t;
        let ox = wrap(off_x, bg.tile);
        let oy = wrap(off_y, bg.tile);

        // base so the grid stays centered around origin
        let base_x = (cell.ix as f32) * bg.tile;
        let base_y = (cell.iy as f32) * bg.tile;

        tf.translation.x = base_x + ox;
        tf.translation.y = base_y + oy;
    }
}

// If you have a problem or a question about this code, talk to vlad.
fn follow_player(
    //these functions are provided directly from bevy
    //finds all entities that are able to transform and are made of the player component
    player_query: Query<&Transform, (With<player::Player>, Without<MainCamera>)>,
    mut camera_query: Query<&mut Transform, (With<MainCamera>, Without<player::Player>)>,
    level: Res<LevelRes>,
) {
    //players current position.
    if let Ok(player_transform) = player_query.single() {
        //This will error out if we would like to have several cameras, this makes the camera mutable
        if let Ok(mut camera_transform) = camera_query.single_mut() {
            //level bounds  calculation given 40x23
            let map_cols = level.level.first().map(|r| r.len()).unwrap_or(0) as f32;
            let map_rows = level.level.len() as f32;
            let level_width = map_cols * TILE_SIZE;
            let level_height = map_rows * TILE_SIZE;

            //these are the bounds for the camera, but it will not move horizontally because we have an exact match between the window and tile width
            let max_x = (level_width - WIN_W) * 0.5;
            let min_x = -(level_width - WIN_W) * 0.5;
            let max_y = (level_height - WIN_H) * 0.5;
            let min_y = -(level_height - WIN_H) * 0.5;

            //camera following the player given the bounds
            let target_x = player_transform.translation.x.clamp(min_x, max_x);
            let target_y = player_transform.translation.y.clamp(min_y, max_y);
            camera_transform.translation.x = target_x;
            camera_transform.translation.y = target_y;
        }
    }
}
