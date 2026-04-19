use bevy::prelude::*;
use crate::{GameState, GameMusicVolume, FONT_PATH, settings};

pub struct PausePlugin;

/// Marker resource present only while the game is paused.
#[derive(Resource)]
pub struct IsPaused;

#[derive(Component)]
struct PauseUI;

#[derive(Component)]
enum PauseButton {
    Resume,
    Settings,
    MainMenu,
}

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
                Update,
                handle_escape.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                handle_pause_buttons
                    .run_if(in_state(GameState::Playing))
                    .run_if(resource_exists::<IsPaused>),
            )
            .add_systems(OnExit(GameState::Playing), cleanup_on_exit);
    }
}

fn handle_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    is_paused: Option<Res<IsPaused>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    pause_ui_q: Query<Entity, With<PauseUI>>,
    settings_ui_q: Query<Entity, With<settings::SettingsUI>>,
    settings_open: Option<Res<settings::SettingsOrigin>>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }

    // Escape closes settings first if it's open, then unpauses.
    if settings_open.is_some() {
        commands.remove_resource::<settings::SettingsOrigin>();
        for e in &settings_ui_q {
            commands.entity(e).despawn();
        }
        return;
    }

    if is_paused.is_some() {
        do_resume(&mut commands, &mut virtual_time, &pause_ui_q);
    } else {
        do_pause(&mut commands, &asset_server, &mut virtual_time);
    }
}

fn do_pause(commands: &mut Commands, assets: &AssetServer, time: &mut Time<Virtual>) {
    time.pause();
    commands.insert_resource(IsPaused);

    let font: Handle<Font> = assets.load(FONT_PATH);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
            ZIndex(200),
            PauseUI,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(300.0),
                    padding: UiRect::all(Val::Px(24.0)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(14.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.05, 0.14, 0.97)),
                BorderColor(Color::srgba(0.3, 0.3, 0.6, 0.8)),
                BorderRadius::all(Val::Px(10.0)),
            ))
            .with_children(|panel| {
                panel.spawn((Node::default(),)).with_children(|c| {
                    c.spawn((
                        Text::new("PAUSED"),
                        TextFont { font: font.clone(), font_size: 36.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                });

                spawn_pause_button(panel, font.clone(), "Resume",    PauseButton::Resume,   Color::srgba(0.08, 0.42, 0.08, 0.9));
                spawn_pause_button(panel, font.clone(), "Settings",  PauseButton::Settings, Color::srgba(0.1,  0.1,  0.42, 0.9));
                spawn_pause_button(panel, font.clone(), "Main Menu", PauseButton::MainMenu, Color::srgba(0.38, 0.08, 0.08, 0.9));
            });
        });
}

fn spawn_pause_button(parent: &mut ChildSpawnerCommands, font: Handle<Font>, label: &str, button: PauseButton, bg: Color) {
    parent
        .spawn((
            Button,
            button,
            Node {
                width: Val::Px(240.0),
                height: Val::Px(52.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(bg),
            BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.25)),
            BorderRadius::all(Val::Px(6.0)),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(label),
                TextFont { font, font_size: 24.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });
}

fn do_resume(commands: &mut Commands, time: &mut Time<Virtual>, pause_ui_q: &Query<Entity, With<PauseUI>>) {
    time.unpause();
    commands.remove_resource::<IsPaused>();
    for e in pause_ui_q {
        commands.entity(e).despawn();
    }
}

fn handle_pause_buttons(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    volume: Res<GameMusicVolume>,
    mut interactions: Query<(&Interaction, &PauseButton), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    pause_ui_q: Query<Entity, With<PauseUI>>,
) {
    for (interaction, button) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button {
            PauseButton::Resume => {
                do_resume(&mut commands, &mut virtual_time, &pause_ui_q);
            }
            PauseButton::Settings => {
                settings::open_settings(
                    &mut commands,
                    &asset_server,
                    volume.0,
                    settings::SettingsOrigin::Paused,
                );
            }
            PauseButton::MainMenu => {
                virtual_time.unpause();
                commands.remove_resource::<IsPaused>();
                next_state.set(GameState::Menu);
            }
        }
    }
}

/// Clean up all pause/settings state if we leave Playing by any means.
fn cleanup_on_exit(
    mut commands: Commands,
    pause_ui_q: Query<Entity, With<PauseUI>>,
    settings_ui_q: Query<Entity, With<settings::SettingsUI>>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    virtual_time.unpause();
    commands.remove_resource::<IsPaused>();
    commands.remove_resource::<settings::SettingsOrigin>();
    for e in &pause_ui_q {
        commands.entity(e).despawn();
    }
    for e in &settings_ui_q {
        commands.entity(e).despawn();
    }
}
