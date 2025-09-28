use bevy::{prelude::*, window::PresentMode};

mod endcredits;
mod player;
mod enemy;

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

/**
 * States is for the different game states
 * PartialEq and Eq are for comparisons: Allows for == and !=
 * Default allows for faster initializing ..default instead of Default::default()
 * 
 * #\[default] sets the GameState below it as the default state
*/
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum GameState{
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
        .add_systems(Startup, setup_floor)
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

// One char = one 32×32 tile. '#' = draw tile, '.' = empty.
// 40 cols (1280/32), 23 rows (720/32 = 22.5))
const MAP: &[&str] = &[
    "########################################", // bottom of screen
    "########################################",
    "########################################",
    "########################################",
    "########################################",
    "########################################",
    "########################################",
    "########################################",
    "########################################",
    "########################################",
    "########################################",
    "########################################",
    "########################################",
    "########################################",
    "########################################",
    "########################################",
    "########################################",
    "########################################",
    "########################################",
    "########################################",
    "################################.#.#.###",
    "########################################",
    "########################################",
    "########################################", // top of screen
];

fn setup_floor(mut commands: Commands, asset_server: Res<AssetServer>) {
    let tile: Handle<Image> = asset_server.load("floortile.png");

    let map_cols = MAP.first().map(|r| r.len()).unwrap_or(0) as f32;
    let map_rows = MAP.len() as f32;

    // Anchor the map at the bottom-center of the window
    let map_px_w = map_cols * TILE_SIZE;
    let map_px_h = map_rows * TILE_SIZE;
    let x0 = -map_px_w * 0.5 + TILE_SIZE * 0.5; // center horizontally
    let y0 = -WIN_H * 0.5 + TILE_SIZE * 0.5; // sit on bottom

    for (row_i, row) in MAP.iter().enumerate() {
        for (col_i, ch) in row.chars().enumerate() {
            if ch == '#' {
                let x = x0 + col_i as f32 * TILE_SIZE;
                let y = y0 + row_i as f32 * TILE_SIZE;
                commands.spawn((
                    Sprite::from_image(tile.clone()),
                    Transform::from_translation(Vec3::new(x, y, Z_FLOOR)),
                    FloorTile,
                ));
            }
        }
    }
}

fn log_state_change(state: Res<State<GameState>>) {
    info!("Just moved to {:?}!", state.get());
}

// If you have a problem or a question about this code, talk to vlad. 
fn follow_player(
    //these functions are provided directly from bevy
    //finds all entities that are able to transform and are made of the player component
    player_query: Query<&Transform, (With<player::Player>, Without<MainCamera>)>,
    mut camera_query: Query<&mut Transform, (With<MainCamera>, Without<player::Player>)>,
) {
    //players current position. 
    if let Ok(player_transform) = player_query.get_single() {

        
        //This will error out if we would like to have several cameras, this makes the camera mutable
        if let Ok(mut camera_transform) = camera_query.get_single_mut() {



            //level bounds  calculation given 40x23
            let map_cols = MAP.first().map(|r| r.len()).unwrap_or(0) as f32;
            let map_rows = MAP.len() as f32;
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