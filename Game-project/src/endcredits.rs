use bevy::{prelude::*, window::PresentMode, time::common_conditions::on_timer};
use std::time::Duration;

const TITLE: &str = "EndCredits";
const WIN_W: f32 = 1280.;
const WIN_H: f32 = 720.;

#[derive(Resource)]
struct Slideshow {
    handles: Vec<Handle<Image>>,
    current_index: usize,
}

#[derive(Component)]
struct SlideshowImage;

pub fn run_slideshow() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.0, 0.0, 0.0)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: TITLE.into(),
                resolution: (WIN_W, WIN_H).into(),
                present_mode: PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, change_image.run_if(on_timer(Duration::from_secs(2))))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let handles = vec![
        asset_server.load("assets/vlad.png"),
        asset_server.load("assets/ryan.png"),
        asset_server.load("assets/daniel.png"),
        asset_server.load("assets/sam.png"),
        asset_server.load("assets/will.png"),
        asset_server.load("assets/lucasloepke.png"),
    ];

    commands.spawn((
        SpriteBundle {
            texture: handles[0].clone(),
            transform: Transform::from_scale(Vec3::splat(0.8)), // resize if needed
            ..default()
        },
        SlideshowImage,
    ));

    commands.insert_resource(Slideshow {
        handles,
        current_index: 0,
    });
}

fn change_image(
    mut slideshow: ResMut<Slideshow>,
    mut query: Query<&mut Handle<Image>, With<SlideshowImage>>,
) {
    slideshow.current_index = (slideshow.current_index + 1) % slideshow.handles.len();
    let new_handle = slideshow.handles[slideshow.current_index].clone();

    if let Ok(mut texture) = query.get_single_mut() {
        *texture = new_handle;
    }
}
