use bevy::prelude::*;

use crate::GameState;

#[derive(Component)]
struct SlideTimer(Timer);

#[derive(Component)]
struct CurrentSlide;

#[derive(Component)]
struct CreditsUI;

#[derive(Component)]
struct BackToMenuButton;

#[derive(Resource)]
struct SlideshowState {
    current_index: usize,
    image_handles: Vec<Handle<Image>>,
}

pub struct EndCreditPlugin;
impl Plugin for EndCreditPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, load_credits)
            .add_systems(OnEnter(GameState::EndCredits), start_slide)
            .add_systems(OnExit(GameState::EndCredits), cleanup_credits)
            .add_systems(
                Update,
                (update_slideshow, handle_back_button)
                    .run_if(in_state(GameState::EndCredits)),
            );
    }
}

fn load_credits(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handles = vec![
        asset_server.load("credits/vlad.png"),
        asset_server.load("credits/ryan.png"),
        asset_server.load("credits/daniel.png"),
        asset_server.load("credits/sam.png"),
        asset_server.load("credits/will.png"),
        asset_server.load("credits/lucas.png"),
        asset_server.load("credits/aidan.png"),
        asset_server.load("credits/ansel.png"),
    ];
    commands.insert_resource(SlideshowState {
        current_index: 0,
        image_handles: handles,
    });
}

fn start_slide(
    mut commands: Commands,
    slideshow_state: Res<SlideshowState>,
) {
    // Slide image
    commands.spawn((
        Sprite::from_image(slideshow_state.image_handles[0].clone()),
        Transform::from_xyz(0.0, 0.0, 0.0),
        CurrentSlide,
        SlideTimer(Timer::from_seconds(2.0, TimerMode::Repeating)),
        CreditsUI,
    ));

    // Back to Menu button (UI overlay)
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(24.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-100.0)),
                width: Val::Px(200.0),
                height: Val::Px(52.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Button,
            BackgroundColor(Color::srgba(0.1, 0.1, 0.3, 0.85)),
            BorderColor(Color::srgba(0.4, 0.4, 1.0, 0.6)),
            BorderRadius::all(Val::Px(6.0)),
            ZIndex(10),
            BackToMenuButton,
            CreditsUI,
        ))
        .with_children(|b| {
            b.spawn((
                Text::new("Back to Menu"),
                TextFont { font_size: 22.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });
}

fn update_slideshow(
    time: Res<Time>,
    mut slideshow_state: ResMut<SlideshowState>,
    mut slide_query: Query<(&mut SlideTimer, &mut Sprite), With<CurrentSlide>>,
) {
    for (mut timer, mut sprite) in slide_query.iter_mut() {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            slideshow_state.current_index =
                (slideshow_state.current_index + 1) % slideshow_state.image_handles.len();
            sprite.image = slideshow_state.image_handles[slideshow_state.current_index].clone();
        }
    }
}

fn handle_back_button(
    interactions: Query<&Interaction, (Changed<Interaction>, With<BackToMenuButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            next_state.set(GameState::Menu);
        }
    }
}

fn cleanup_credits(
    mut commands: Commands,
    q: Query<Entity, With<CreditsUI>>,
    mut slideshow_state: ResMut<SlideshowState>,
) {
    for entity in &q {
        commands.entity(entity).despawn();
    }
    slideshow_state.current_index = 0;
}
