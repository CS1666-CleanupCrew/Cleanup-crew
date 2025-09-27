use bevy::{prelude::*, window::PresentMode};

mod endcredits;

const TITLE: &str = "Cleanup Crew";
const WIN_W: f32 = 1280.;
const WIN_H: f32 = 720.;

/**
 * States is for the different game states
 * PartialEq and Eq are for comparisons: Allows for == and !=
 * Default allows for faster initializing ..default instead of Default::default()
*/
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum GameState{
    #[default]
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
        
        .add_systems(Startup, setup_camera)
        .add_systems(OnEnter(GameState::EndCredits), log_state_change)


        .add_plugins((
            endcredits::EndCreditPlugin,
        ))
        .init_state::<GameState>()
        .run();
}

fn setup_camera(mut commands: Commands){
    commands.spawn(Camera2dBundle::default());
}

fn log_state_change(state: Res<State<GameState>>) {
    info!("Just moved to {:?}!", state.get());
}