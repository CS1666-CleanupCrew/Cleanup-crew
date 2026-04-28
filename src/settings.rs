use bevy::prelude::*;
use bevy::audio::Volume;
use bevy::window::{WindowMode, PrimaryWindow, MonitorSelection, VideoModeSelection};
use crate::{GameMusicVolume, MusicTrack, FONT_PATH};

pub struct SettingsPlugin;

/// Tracks whether the settings panel is open, and what context to return to.
#[derive(Resource, PartialEq, Eq)]
pub enum SettingsOrigin {
    MainMenu,
    Paused,
}

/// The player's chosen window mode, kept as a resource so it persists across settings opens.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameWindowMode {
    Windowed,
    #[default]
    BorderlessFullscreen,
    Fullscreen,
}

impl GameWindowMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Windowed => "Windowed",
            Self::BorderlessFullscreen => "Borderless FS",
            Self::Fullscreen => "Fullscreen",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Windowed => Self::BorderlessFullscreen,
            Self::BorderlessFullscreen => Self::Fullscreen,
            Self::Fullscreen => Self::Windowed,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Windowed => Self::Fullscreen,
            Self::BorderlessFullscreen => Self::Windowed,
            Self::Fullscreen => Self::BorderlessFullscreen,
        }
    }

    pub fn to_bevy(self) -> WindowMode {
        match self {
            Self::Windowed => WindowMode::Windowed,
            Self::BorderlessFullscreen => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
            Self::Fullscreen => WindowMode::Fullscreen(MonitorSelection::Current, VideoModeSelection::Current),
        }
    }
}

/// Root node of the settings overlay.
#[derive(Component)]
pub struct SettingsUI;

#[derive(Component)]
struct VolumeDisplay;

#[derive(Component)]
struct WindowModeDisplay;

#[derive(Component)]
enum SettingsButton {
    VolumeDown,
    VolumeUp,
    WindowModeLeft,
    WindowModeRight,
    Back,
}

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
                Update,
                handle_settings_buttons.run_if(resource_exists::<SettingsOrigin>),
            )
            .add_systems(Update, update_volume_display)
            .add_systems(Update, update_window_mode_display)
            .add_systems(Update, sync_volume_to_sinks)
            .add_systems(Update, sync_window_mode);
    }
}

/// Spawn the settings overlay. Caller decides the origin context.
pub fn open_settings(commands: &mut Commands, assets: &AssetServer, current_volume: f32, current_window_mode: GameWindowMode, origin: SettingsOrigin) {
    commands.insert_resource(origin);

    let font: Handle<Font> = assets.load(FONT_PATH);
    let vol_text = format!("{}%", (current_volume * 100.0).round() as u32);

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
            ZIndex(300),
            SettingsUI,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(380.0),
                    padding: UiRect::all(Val::Px(28.0)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(22.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.05, 0.14, 0.97)),
                BorderColor(Color::srgba(0.3, 0.3, 0.7, 0.8)),
                BorderRadius::all(Val::Px(10.0)),
            ))
            .with_children(|panel| {
                // Title
                panel.spawn((Node::default(),)).with_children(|c| {
                    c.spawn((
                        Text::new("SETTINGS"),
                        TextFont { font: font.clone(), font_size: 34.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                });

                // Music Volume row
                panel
                    .spawn((Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    },))
                    .with_children(|row| {
                        row.spawn((Node::default(),)).with_children(|c| {
                            c.spawn((
                                Text::new("Music Volume"),
                                TextFont { font: font.clone(), font_size: 22.0, ..default() },
                                TextColor(Color::srgb(0.85, 0.85, 0.85)),
                            ));
                        });

                        row.spawn((Node {
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(10.0),
                            ..default()
                        },))
                        .with_children(|ctrl| {
                            // Minus button
                            spawn_small_button(ctrl, font.clone(), "-", SettingsButton::VolumeDown);

                            // Volume % display
                            ctrl.spawn((Node {
                                width: Val::Px(58.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },))
                            .with_children(|c| {
                                c.spawn((
                                    Text::new(vol_text),
                                    TextFont { font: font.clone(), font_size: 22.0, ..default() },
                                    TextColor(Color::WHITE),
                                    VolumeDisplay,
                                ));
                            });

                            // Plus button
                            spawn_small_button(ctrl, font.clone(), "+", SettingsButton::VolumeUp);
                        });
                    });

                // Window Mode row
                panel
                    .spawn((Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    },))
                    .with_children(|row| {
                        row.spawn((Node::default(),)).with_children(|c| {
                            c.spawn((
                                Text::new("Window Mode"),
                                TextFont { font: font.clone(), font_size: 22.0, ..default() },
                                TextColor(Color::srgb(0.85, 0.85, 0.85)),
                            ));
                        });

                        row.spawn((Node {
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(10.0),
                            ..default()
                        },))
                        .with_children(|ctrl| {
                            spawn_small_button(ctrl, font.clone(), "<", SettingsButton::WindowModeLeft);

                            ctrl.spawn((Node {
                                width: Val::Px(110.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },))
                            .with_children(|c| {
                                c.spawn((
                                    Text::new(current_window_mode.label()),
                                    TextFont { font: font.clone(), font_size: 18.0, ..default() },
                                    TextColor(Color::WHITE),
                                    WindowModeDisplay,
                                ));
                            });

                            spawn_small_button(ctrl, font.clone(), ">", SettingsButton::WindowModeRight);
                        });
                    });

                // Back button
                panel
                    .spawn((
                        Button,
                        SettingsButton::Back,
                        Node {
                            width: Val::Px(240.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect { top: Val::Px(8.0), ..default() },
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.25, 0.1, 0.1, 0.9)),
                        BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
                        BorderRadius::all(Val::Px(6.0)),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("Back"),
                            TextFont { font: font.clone(), font_size: 24.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });
            });
        });
}

fn spawn_small_button(parent: &mut ChildSpawnerCommands, font: Handle<Font>, label: &str, button: SettingsButton) {
    parent
        .spawn((
            Button,
            button,
            Node {
                width: Val::Px(38.0),
                height: Val::Px(38.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.2, 0.2, 0.35, 0.9)),
            BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(label),
                TextFont { font, font_size: 26.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });
}

fn handle_settings_buttons(
    mut commands: Commands,
    mut interactions: Query<(&Interaction, &SettingsButton), (Changed<Interaction>, With<Button>)>,
    mut volume: ResMut<GameMusicVolume>,
    mut window_mode: ResMut<GameWindowMode>,
    ui_q: Query<Entity, With<SettingsUI>>,
) {
    for (interaction, button) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button {
            SettingsButton::VolumeDown => {
                volume.0 = (volume.0 - 0.1).max(0.0);
            }
            SettingsButton::VolumeUp => {
                volume.0 = (volume.0 + 0.1).min(1.0);
            }
            SettingsButton::WindowModeLeft => {
                *window_mode = window_mode.prev();
            }
            SettingsButton::WindowModeRight => {
                *window_mode = window_mode.next();
            }
            SettingsButton::Back => {
                commands.remove_resource::<SettingsOrigin>();
                for e in &ui_q {
                    commands.entity(e).despawn();
                }
            }
        }
    }
}

fn update_volume_display(
    volume: Res<GameMusicVolume>,
    mut text_q: Query<&mut Text, With<VolumeDisplay>>,
) {
    if !volume.is_changed() {
        return;
    }
    let label = format!("{}%", (volume.0 * 100.0).round() as u32);
    for mut t in &mut text_q {
        *t = Text::new(&label);
    }
}

fn sync_volume_to_sinks(
    volume: Res<GameMusicVolume>,
    mut sinks: Query<&mut AudioSink, With<MusicTrack>>,
) {
    if !volume.is_changed() {
        return;
    }
    for mut sink in &mut sinks {
        sink.set_volume(Volume::Linear(volume.0));
    }
}

fn update_window_mode_display(
    mode: Res<GameWindowMode>,
    mut text_q: Query<&mut Text, With<WindowModeDisplay>>,
) {
    if !mode.is_changed() {
        return;
    }
    for mut t in &mut text_q {
        *t = Text::new(mode.label());
    }
}

fn sync_window_mode(
    mode: Res<GameWindowMode>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !mode.is_changed() {
        return;
    }
    if let Ok(mut window) = windows.single_mut() {
        window.mode = mode.to_bevy();
    }
}
