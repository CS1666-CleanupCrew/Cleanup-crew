use bevy::prelude::*;

use crate::{GameState, ShowAirLabels};
use crate::map::LevelToLoad;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameState::Menu), setup_menu)
            .add_systems(OnEnter(GameState::Menu), start_menu_music)
            .add_systems(Update, handle_buttons.run_if(in_state(GameState::Menu)))
            .add_systems(OnExit(GameState::Menu), cleanup_menu)
            .add_systems(OnExit(GameState::Menu), stop_menu_music);
    }
}

#[derive(Component)]
struct MenuUI;

#[derive(Component)]
enum MenuButton {
    Play,
    PlayTestRoom,
    Credits,
    ToggleAirLabels,
}

#[derive(Component)]
struct AirToggleText;

#[derive(Component)]
struct MenuMusic;

fn setup_menu(
    mut commands: Commands,
    assets: Res<AssetServer>,
    show_labels: Res<ShowAirLabels>, // read initial state for the checkbox
) {
    // Root canvas
    let checked = show_labels.0;
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Px(1280.0),
                height: Val::Px(720.0),
                ..default()
            },
            ZIndex(100), // on top of world
            MenuUI,
        ))
        .with_children(|root| {
            // Background
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(1280.0),
                    height: Val::Px(720.0),
                    ..default()
                },
                ImageNode::new(assets.load("menu/Title_BG.png")),
            ));

            // Cleanup Crew Title
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(1280.0),
                    height: Val::Px(720.0),
                    ..default()
                },
                ImageNode::new(assets.load("menu/Title_Text.png")),
            ));

            root
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(40.0),
                        margin: UiRect {
                            top: Val::Px(100.0),
                            ..default()
                        },
                        ..default()
                    },
                ))
                .with_children(|col| {
                    // Play
                    col.spawn((
                        Button,
                        MenuButton::Play,
                        ImageNode::new(assets.load("menu/Title_Play.png")),
                    ));

                    // Test Room Button (Text-based)
                    col.spawn((
                        Button,
                        MenuButton::PlayTestRoom,
                        Node {
                            width: Val::Px(420.0),
                            height: Val::Px(60.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 0.7)),
                        BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.4)),
                        BorderRadius::all(Val::Px(6.0)),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("Test Room"),
                            TextFont { font_size: 28.0, ..default() },
                        ));
                    });

                    // Credits
                    col.spawn((
                        Button,
                        MenuButton::Credits,
                        ImageNode::new(assets.load("menu/Title_Credits.png")),
                    ));
                    col.spawn((
                        Node {
                            width: Val::Px(420.0),
                            height: Val::Px(60.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(12.0),
                            ..default()
                        },
                    ))
                    .with_children(|row| {
                        // Checkbox button (text-based)
                        row.spawn((
                            Button,
                            MenuButton::ToggleAirLabels,
                            Node {
                                padding: UiRect::all(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 0.7)),
                            BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.4)),
                            BorderRadius::all(Val::Px(6.0)),
                        ))
                        .with_children(|b| {
                            let mark = if checked { "[x]" } else { "[ ]" };
                            b.spawn((
                                Text::new(mark),
                                TextFont { font_size: 28.0, ..default() },
                                AirToggleText,
                            ));
                        });

                        // Static label
                        row.spawn((
                            Text::new("Show air pressure labels"),
                            TextFont { font_size: 28.0, ..default() },
                        ));
                    });
                });
        });
}

fn start_menu_music(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let music_handle = asset_server.load("audio/menu_music.ogg");
    
    commands.spawn((
        AudioPlayer::new(music_handle),
        PlaybackSettings::LOOP,
        MenuMusic,
    ));
    
    debug!("Menu music started");
}

fn stop_menu_music(
    mut commands: Commands,
    music_query: Query<Entity, With<MenuMusic>>,
) {
    for entity in &music_query {
        commands.entity(entity).despawn();
        debug!("Menu music stopped");
    }
}

fn handle_buttons(
    mut interactions: Query<(&Interaction, &MenuButton, Entity), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut show_labels: ResMut<ShowAirLabels>,
    children_q: Query<&Children>,
    mut texts: Query<&mut Text, With<AirToggleText>>,
    mut level_to_load: ResMut<LevelToLoad>,
) {
    for (interaction, which, button_entity) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match which {
            MenuButton::Play => {
                next_state.set(GameState::Loading);
            }
            MenuButton::PlayTestRoom => {
                level_to_load.0 = "assets/rooms/window_room.txt".to_string();
                next_state.set(GameState::Loading);
            }
            MenuButton::Credits => {
                next_state.set(GameState::EndCredits);
            }
            MenuButton::ToggleAirLabels => {
                // Flip the flag
                show_labels.0 = !show_labels.0;


                if let Ok(children) = children_q.get(button_entity) {
                    for child in children.iter() {
                        if let Ok(mut t) = texts.get_mut(child) {
                            *t = Text::new(if show_labels.0 { "[x]" } else { "[ ]" });
                        }
                    }
                }
            }
        }
    }
}

fn cleanup_menu(mut commands: Commands, root_q: Query<Entity, With<MenuUI>>) {
    for e in &root_q {
        commands.entity(e).despawn();
    }
}
