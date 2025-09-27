use bevy::{asset::LoadState, prelude::*};

use crate::{
    GameState,
};

#[derive(Component)]
struct SlideTimer(Timer);

#[derive(Component)]
struct CurrentSlide;

#[derive(Resource)]
struct SlideshowState {
    current_index: usize,
    image_handles: Vec<Handle<Image>>,
    started: bool,
}

//Plugin so that it can be added in main
/**
 * Ok so currently since endcredit runs immediately, it doesnt have enough time to load the assets
 */
pub struct EndCreditPlugin;
impl Plugin for EndCreditPlugin{
    fn build(&self, app: &mut App){
        app
            .add_systems(Startup, load_credits)
            .add_systems(OnEnter(GameState::EndCredits), check_assets_loaded)
            .add_systems(OnEnter(GameState::EndCredits), start_slide)
            .add_systems(Update, update_slideshow);
    }
}

/**
 * Loads in all the images when the app runs
 * Inserts them into a struct called SlideshowState
 */
fn load_credits(mut commands: Commands, asset_server: Res<AssetServer>) {
    
    let handles = vec![
        asset_server.load("vlad.png"),
        asset_server.load("ryan.png"),
        asset_server.load("daniel.png"),
        asset_server.load("sam.png"),
        asset_server.load("will.png"),
        asset_server.load("lucasloepke.png"),
        asset_server.load("aidan.png"),
        asset_server.load("ansel.png"),

        ];

    println!("Loading images from assets folder...");
    println!("Loaded {} image handles", handles.len());

    commands.insert_resource(SlideshowState {
        current_index: 0,
        image_handles: handles,
        started: false,
    });

}

/**
 * Before starting the slideshow, check to ensure all assets were loaded
 * Loads them if not
 * Not sure how important this is but Vlad had it so imma trust
 */
fn check_assets_loaded(
    asset_server: Res<AssetServer>,
    slideshow_state: ResMut<SlideshowState>,
) {
    if slideshow_state.started {
        return;
    }

    let all_loaded = slideshow_state.image_handles.iter().all(|handle| {
        matches!(asset_server.load_state(handle.id()), LoadState::Loaded)
    });

    if !all_loaded && slideshow_state.image_handles.is_empty() {

        for handle in &slideshow_state.image_handles {
            match asset_server.load_state(handle.id()) {
                LoadState::NotLoaded => {
                    println!("Failed to load asset: {:?}", handle);
                }
                LoadState::Loading => {
                    println!("Still loading assets...");
                }
                _ => {}
            }
        }
    }
}

/**
 * Once GameState enters EndCredits, start_slide is called
 * This spawns in the first image
 * This creates a repeating timer
 */
fn start_slide(
    mut commands: Commands,
    slideshow_state: ResMut<SlideshowState>,
){

    commands.spawn((
        Sprite::from_image(slideshow_state.image_handles[0].clone()),
        Transform::from_xyz(0., 0., 0.),
        CurrentSlide,
        SlideTimer(Timer::from_seconds(2.0, TimerMode::Repeating)),
    ));
}

/**
 * Loops through each of the 8 credit images every 2 seconds
 * Not too sure how useful the slideshow_state boolean so I commented it out
 */
fn update_slideshow(
    time: Res<Time>,
    mut slideshow_state: ResMut<SlideshowState>,
    mut slide_query: Query<(&mut SlideTimer, &mut Sprite), With<CurrentSlide>>,
) {
    // slideshow_state.started = true;

    // if !slideshow_state.started {
    //     return;
    // }

    for (mut timer, mut texture_to_change) in slide_query.iter_mut() {
        timer.0.tick(time.delta());

        if timer.0.just_finished() {
            
            slideshow_state.current_index =
                (slideshow_state.current_index + 1) % slideshow_state.image_handles.len();

          
            texture_to_change.image = slideshow_state.image_handles[slideshow_state.current_index].clone();

            println!(
                "Showing slide {}/{}",
                slideshow_state.current_index + 1,
                slideshow_state.image_handles.len()
            );
        }
    }
}
