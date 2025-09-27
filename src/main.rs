use bevy::{prelude::*, window::PresentMode};

mod endcredits;
mod player;
mod enemy;

const TITLE: &str = "Cleanup Crew";
const WIN_W: f32 = 1280.;
const WIN_H: f32 = 720.;

const PLAYER_SPEED: f32 = 500.;
const ACCEL_RATE: f32 = 5000.;

const TILE_SIZE: f32 = 100.;

const LEVEL_LEN: f32 = 1280.;

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
        .add_systems(OnEnter(GameState::EndCredits), log_state_change)
        .add_systems(OnEnter(GameState::Playing), log_state_change)

        .run();
}


fn setup_camera(mut commands: Commands){
    commands.spawn(Camera2d);
}

fn log_state_change(state: Res<State<GameState>>) {
    info!("Just moved to {:?}!", state.get());
}
