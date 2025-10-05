use crate::collidable::{Collidable, Collider};
use bevy::{prelude::*, window::PresentMode};

mod collidable;
mod endcredits;
mod enemy;
mod player;

const TITLE: &str = "Cleanup Crew";
const WIN_W: f32 = 1280.;
const WIN_H: f32 = 720.;

const PLAYER_SPEED: f32 = 500.;
const ACCEL_RATE: f32 = 5000.;
const TILE_SIZE: f32 = 32.;
const LEVEL_LEN: f32 = 1280.;

pub const Z_FLOOR: f32 = -100.0;
pub const Z_ENTITIES: f32 = 0.0;
pub const Z_UI: f32 = 100.0;

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct FloorTile;
#[derive(Component)]
struct Wall;

/**
 * States is for the different game states
 * PartialEq and Eq are for comparisons: Allows for == and !=
 * Default allows for faster initializing ..default instead of Default::default()
 *
 * #\[default] sets the GameState below it as the default state
*/
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum GameState {
    #[default]
    Playing,
    EndCredits,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: TITLE.into(),
                resolution: (WIN_W, WIN_H).into(),
                present_mode: PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        //Initial GameState
        .init_state::<GameState>()
        //Calls the plugin
        .add_plugins((
            player::PlayerPlugin,
            endcredits::EndCreditPlugin,
            enemy::EnemyPlugin,
        ))
        .add_systems(Startup, setup_camera)
        .add_systems(Startup, setup_tilemap)
        .add_systems(OnEnter(GameState::EndCredits), log_state_change)
        .add_systems(OnEnter(GameState::Playing), log_state_change)
        .add_systems(Update, follow_player.run_if(in_state(GameState::Playing)))

        .run();
}


fn setup_camera(mut commands: Commands){
    commands.spawn((
        Camera2d,
        MainCamera,
    ));
}

// One char = one 32×32 tile.
// Legend:
//  '#' = floor tile
//  '.' = empty
//  'T' = table (floor renders underneath)
//  'W' = wall (floor renders underneath + collidable wall sprite)
// 40 cols (1280/32), 23 rows (720/32 = 22.5))
const MAP: &[&str] = &[ // top of screen
    "WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "WWWWWWWWWWWWWWWWWWW###WWWWWWWWWWWWWWWWWW",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "W#####T########################.#.#.###W",
    "W######################################W",
    "W######################################W",
    "W##############################T#######W",
    "W######################################W",
    "W######T###############################W",
    "WWWWWWWWWWWWWWWWWWW###WWWWWWWWWWWWWWWWWW",
    "W######################################W",
    "W######################################W",
    "W######################################W",
    "WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW",
];

/// Unified loader: draws floor and spawns items/walls from MAP symbols.
///   '#' => floor
///   'T' => table (spawns a floor tile under it + a collidable table sprite)
///   'W' => wall  (spawns a floor tile under it + a collidable wall sprite)
fn setup_tilemap(mut commands: Commands, asset_server: Res<AssetServer>) {
    let floor_tex: Handle<Image> = asset_server.load("floortile.png");
    let table_tex: Handle<Image> = asset_server.load("table.png");
    let wall_tex: Handle<Image>  = asset_server.load("window.png");

    let map_cols = MAP.first().map(|r| r.len()).unwrap_or(0) as f32;
    let map_rows = MAP.len() as f32;

    // Center the map in world space (origin = middle of map)
    let map_px_w = map_cols * TILE_SIZE;
    let map_px_h = map_rows * TILE_SIZE;
    let x0 = -map_px_w * 0.5 + TILE_SIZE * 0.5;
    let y0 = -map_px_h * 0.5 + TILE_SIZE * 0.5;

    for (row_i, row) in MAP.iter().enumerate() {
    for (col_i, ch) in row.chars().enumerate() {
        let x = x0 + col_i as f32 * TILE_SIZE;
        let y = y0 + (map_rows - 1.0 - row_i as f32) * TILE_SIZE; // invert the vertical draw order

            // Floor under '#', 'T', and 'W' so the world stays visually continuous
            if ch == '#' || ch == 'T' || ch == 'W' {
                commands.spawn((
                    Sprite::from_image(floor_tex.clone()),
                    Transform::from_translation(Vec3::new(x, y, Z_FLOOR)),
                    FloorTile,
                ));
            }

            // Items and walls
            match ch {
                'T' => {
                    // place a 32x32 table with a collider
                    let mut sprite = Sprite::from_image(table_tex.clone());
                    sprite.custom_size = Some(Vec2::splat(TILE_SIZE));
                    commands.spawn((
                        sprite,
                        Transform::from_translation(Vec3::new(x, y, Z_FLOOR + 1.0)),
                        Visibility::default(),
                        Collidable,
                        Collider { half_extents: Vec2::splat(TILE_SIZE * 0.5) },
                        Name::new("Table"),
                    ));
                }
                'W' => {
                    // place a 32x32 wall tile with a collider
                    let mut sprite = Sprite::from_image(wall_tex.clone());
                    sprite.custom_size = Some(Vec2::splat(TILE_SIZE));
                    commands.spawn((
                        sprite,
                        Transform::from_translation(Vec3::new(x, y, Z_FLOOR + 1.0)),
                        Visibility::default(),
                        Wall,
                        Collidable,
                        Collider { half_extents: Vec2::splat(TILE_SIZE * 0.5) },
                        Name::new("Wall"),
                    ));
                }
                _ => {}
            }
        }
    }
}

fn log_state_change(state: Res<State<GameState>>) {
    info!("Just moved to {:?}!", state.get());
}
