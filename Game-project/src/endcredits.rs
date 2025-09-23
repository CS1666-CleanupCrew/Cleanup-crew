use bevy::{prelude::*, window::PresentMode, asset::LoadState};

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

pub fn run_slideshow() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Team Slideshow".into(),
                resolution: (1280., 720.).into(),
                present_mode: PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (check_assets_loaded, update_slideshow))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    
    commands.spawn(Camera2dBundle::default());

    
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

fn check_assets_loaded(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut slideshow_state: ResMut<SlideshowState>,
) {
    if slideshow_state.started {
        return;
    }

    
    let all_loaded = slideshow_state.image_handles.iter().all(|handle| {
        matches!(asset_server.load_state(handle.id()), LoadState::Loaded)
    });

    if all_loaded && !slideshow_state.image_handles.is_empty() {
        println!("All assets loaded, starting slideshow");
        slideshow_state.started = true;

        
        commands.spawn((
            SpriteBundle {
                texture: slideshow_state.image_handles[0].clone(),
                transform: Transform::default(),
                ..default()
            },
            CurrentSlide,
            SlideTimer(Timer::from_seconds(2.0, TimerMode::Repeating)),
        ));

        println!("Spawning first image");
    } else {
    
        for handle in &slideshow_state.image_handles {
            match asset_server.load_state(handle.id()) {
                LoadState::Failed => {
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

fn update_slideshow(
    time: Res<Time>,
    mut slideshow_state: ResMut<SlideshowState>,
    mut slide_query: Query<(&mut SlideTimer, &mut Handle<Image>), With<CurrentSlide>>,
) {
    if !slideshow_state.started {
        return;
    }

    for (mut timer, mut texture_handle) in slide_query.iter_mut() {
        timer.0.tick(time.delta());

        if timer.0.just_finished() {
            
            slideshow_state.current_index =
                (slideshow_state.current_index + 1) % slideshow_state.image_handles.len();

          
            *texture_handle = slideshow_state.image_handles[slideshow_state.current_index].clone();

            println!(
                "Showing slide {}/{}",
                slideshow_state.current_index + 1,
                slideshow_state.image_handles.len()
            );
        }
    }
}
