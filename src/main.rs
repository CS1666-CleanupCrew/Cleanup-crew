use crate::collidable::{Collidable, Collider};
use crate::player::{Health, Player};
use bevy::{prelude::*, window::PresentMode, winit::WinitWindows};
use bevy::audio::Volume;
use crate::air::{init_air_grid, spawn_pressure_labels, update_pressure_labels, update_air_on_window_break};
use crate::room::RoomVec;

pub mod collidable;
pub mod endcredits;
pub mod enemies;
pub mod player;
pub mod table;
pub mod window;
pub mod map;
pub mod procgen;
#[path = "fluid_simulation.rs"]
pub mod fluiddynamics;
pub mod air;
pub mod noise;
pub mod menu;
pub mod room;
pub mod bullet;
pub mod broom;
pub mod rewards;
pub mod heart;
pub mod weapon;
pub mod minimap;
pub mod pause;
pub mod settings;

pub const FONT_PATH: &str = "fonts/BitcountSingleInk-VariableFont_CRSV,ELSH,ELXP,SZP1,SZP2,XPN1,XPN2,YPN1,YPN2,slnt,wght.ttf";



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
struct HealthBarFill;

#[derive(Component)]
struct ShieldBarFill;

/// The whole shield row — hidden when the player has no shield upgrades.
#[derive(Component)]
struct ShieldBarRow;


/// Marker added to every music audio entity so the volume sync system can find them.
#[derive(Component)]
pub struct MusicTrack;

#[derive(Component)]
struct GameMusic;

#[derive(Resource)]
pub struct GameMusicVolume(pub f32);

#[derive(Component)]
pub struct Damage { amount: f32, }

#[derive(Component)]
struct GameOverScreen;

#[derive(Resource)]
struct DamageCooldown(Timer);

#[derive(Resource, Default)]
pub struct ShowAirLabels(pub bool);

#[derive(Component)]
pub enum EndScreenButtons{
    PlayAgain,
    MainMenu,
    Continue,
    Leave,
}

#[derive(Component)]
pub struct GameEntity;

/// Set when all station rooms are cleared and the reaper is dead.
/// The player must physically return to the airlock before the win screen appears.
#[derive(Resource)]
pub struct LevelComplete;

/// Marker for the "Return to your ship" hint banner shown after LevelComplete.
#[derive(Component)]
pub struct ReturnHintUI;

/// Tracks which station iteration the player is on (0 = first station).
/// Each subsequent station has harder enemies.
#[derive(Resource)]
pub struct StationLevel(pub u32);

impl Default for StationLevel {
    fn default() -> Self { Self(0) }
}

/// Saved player buffs carried between stations on "Continue".
#[derive(Resource, Clone)]
pub struct SavedPlayerBuffs {
    pub max_health: f32,
    pub health: f32,
    pub move_speed: f32,
    pub fire_rate: f32,
    pub num_cleared: usize,
    pub armor: f32,
    pub air_tank_max: f32,
    pub air_tank_drain_rate: f32,
    pub weapon_damage: f32,
    pub piercing_pickups: u32,
    pub regen_rate: f32,
    pub shield_max: f32,
    pub vacuum_mass: f32,
}

#[derive(Component)]
pub struct StationLevelDisplay;

/**
 * States is for the different game states
 * PartialEq and Eq are for comparisons: Allows for == and !=
 * Default allows for faster initializing ..default instead of Default::default()
 *
 * #\[default] sets the GameState below it as the default state
*/
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum GameState {
    #[default]
    Menu,
    Loading,
    Playing,
    GameOver,
    EndCredits,
    Win,
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: TITLE.into(),
                        present_mode: PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(bevy::render::camera::ClearColor(Color::srgb(0.02, 0.02, 0.06)))
        //Initial GameState
        .init_state::<GameState>()
        //Calls the plugin
        .init_resource::<ShowAirLabels>()
        .init_resource::<StationLevel>()
        .add_plugins((
            procgen::ProcGen,
            map::MapPlugin,
            player::PlayerPlugin,
            endcredits::EndCreditPlugin,
            enemies::EnemyPlugin,
            table::TablePlugin,
            fluiddynamics::FluidSimPlugin,
            window::WindowPlugin,
        ))
        .add_plugins((
            menu::MenuPlugin,
            bullet::BulletPlugin,
            room::RoomPlugin,
            broom::BroomPlugin,
            rewards::RewardPlugin,
            heart::HeartPlugin,
            enemies::reaper::ReaperPlugin,
            weapon::WeaponPlugin,
            minimap::MinimapPlugin,
            pause::PausePlugin,
            settings::SettingsPlugin,
        ))
        .add_systems(Startup, (setup_camera, rewards::load_reward_font, maximize_window))
        .add_systems(OnEnter(GameState::Menu), log_state_change)
        .add_systems(OnEnter(GameState::Loading), log_state_change)
        .add_systems(OnEnter(GameState::EndCredits), log_state_change)
        .add_systems(OnEnter(GameState::Playing), log_state_change)
        .add_systems(OnEnter(GameState::Playing), init_air_grid)
        .add_systems(
            OnEnter(GameState::Playing),
            spawn_pressure_labels
                .after(init_air_grid)
                .run_if(|flag: Res<ShowAirLabels>| flag.0),
        )
        .add_systems(OnEnter(GameState::Playing), start_game_music)
        .add_systems(
            Update,
            toggle_game_music.run_if(in_state(GameState::Playing)),
        )

        .add_systems(OnExit(GameState::Playing), clean_game)
        .add_systems(OnExit(GameState::Playing), stop_game_music)
        .add_systems(OnExit(GameState::Playing), remove_level_complete)
        .add_systems(Update, handle_end_screen_buttons.run_if(in_state(GameState::GameOver)))
        .add_systems(Update, handle_end_screen_buttons.run_if(in_state(GameState::Win)))
        .add_systems(OnExit(GameState::GameOver), clean_end_screen)
        .add_systems(OnExit(GameState::Win), clean_end_screen)


        .add_systems(OnEnter(GameState::GameOver), setup_game_over_screen)
        .add_systems(OnEnter(GameState::Win), load_win)


        .add_systems(OnEnter(GameState::Loading), setup_ui_health)
        .add_systems(
            Update,
            update_ui_health_text.run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            (
                damage_on_collision,
                check_game_over,
                check_win,
                check_return_to_airlock,
            )
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            (
                update_air_on_window_break,
                update_pressure_labels,
            )
                .chain()
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            (
                rewards::player_pickup_reward,
                rewards::tick_reward_popups,
            ).run_if(in_state(GameState::Playing)),
        )
        
        .insert_resource(DamageCooldown(Timer::from_seconds(0.5, TimerMode::Once)))
        .insert_resource(GameMusicVolume(0.5)) // .5 volume by default
        .run();
}

fn check_win(
    mut commands: Commands,
    rooms: Res<RoomVec>,
    reaper_q: Query<(), With<enemies::Reaper>>,
    level_complete: Option<Res<LevelComplete>>,
    asset_server: Res<AssetServer>,
){
    // Only fire once.
    if level_complete.is_some() { return; }

    let cleared = rooms.0.iter().filter(|r| r.cleared).count();

    // All rooms cleared AND the reaper is dead (or never spawned).
    // The airlock is pre-cleared so it counts toward both sides equally.
    if cleared == rooms.0.len() && reaper_q.is_empty() {
        commands.insert_resource(LevelComplete);

        // Show a hint banner telling the player to return to their ship.
        let font: Handle<Font> = asset_server.load(
            "fonts/BitcountSingleInk-VariableFont_CRSV,ELSH,ELXP,SZP1,SZP2,XPN1,XPN2,YPN1,YPN2,slnt,wght.ttf",
        );
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                bottom: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            ZIndex(15),
            ReturnHintUI,
            GameEntity,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Station cleared! Return to your ship."),
                TextFont { font, font_size: 30.0, ..default() },
                TextColor(Color::srgb(1.0, 1.0, 0.3)),
            ));
        });
    }
}

/// Once LevelComplete is set, wait for the player to physically walk back
/// into the airlock room before showing the win screen.
fn check_return_to_airlock(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    rooms: Res<RoomVec>,
    player_q: Query<(
        &Health, &player::MaxHealth, &player::MoveSpeed, &weapon::Weapon,
        &player::NumOfCleared, &player::Armor, &player::AirTank,
        &player::Regen, &player::Shield, &fluiddynamics::PulledByFluid,
        &Transform,
    ), With<Player>>,
    level_complete: Option<Res<LevelComplete>>,
) {
    if level_complete.is_none() { return; }

    let Ok((health, max_hp, move_spd, weapon, _num_cleared, armor, tank, regen, shield, pull, transform))
        = player_q.single() else { return; };

    let player_pos = transform.translation.truncate();
    let in_airlock = rooms.0.iter().any(|r| r.is_airlock && r.bounds_check(player_pos));
    if !in_airlock { return; }

    // Player is back on their ship — save buffs and open the win screen.
    commands.insert_resource(SavedPlayerBuffs {
        max_health: max_hp.0,
        health: health.0,
        move_speed: move_spd.0,
        fire_rate: weapon.fire_rate,
        // Reset cleared count so per-station scaling starts fresh.
        num_cleared: 0,
        armor: armor.0,
        air_tank_max: tank.max_capacity,
        air_tank_drain_rate: tank.drain_rate,
        weapon_damage: weapon.damage,
        piercing_pickups: weapon.piercing_pickups,
        regen_rate: regen.0,
        shield_max: shield.max,
        vacuum_mass: pull.mass,
    });
    next_state.set(GameState::Win);
}

fn remove_level_complete(mut commands: Commands) {
    commands.remove_resource::<LevelComplete>();
}

fn load_win(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    station_level: Res<StationLevel>,
){
    let font: Handle<Font> = asset_server.load("fonts/BitcountSingleInk-VariableFont_CRSV,ELSH,ELXP,SZP1,SZP2,XPN1,XPN2,YPN1,YPN2,slnt,wght.ttf");

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ZIndex(20),
        GameOverScreen,
    ))
    .with_children(|root|{

        //Background
        root.spawn((
            Node{
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ImageNode::new(asset_server.load("win.png")),
        ));

        // Station cleared text
        root.spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                top: Val::Px(20.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|r| {
            r.spawn((
                Text::new(format!("Station {} Cleared!", station_level.0 + 1)),
                TextFont { font: font.clone(), font_size: 36.0, ..default() },
                TextColor(Color::srgb(1.0, 1.0, 0.2)),
            ));
        });

        // Button column — just two choices from the airlock
        root.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                margin: UiRect {
                    top: Val::Percent(30.),
                    ..default()
                },
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..default()
            },
        ))
        .with_children(|col|{
            // Continue — board the next station, keep buffs
            col.spawn((
                Button,
                EndScreenButtons::Continue,
                Node {
                    width: Val::Px(420.0),
                    height: Val::Px(60.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.5, 0.1, 0.85)),
                BorderColor(Color::srgba(0.2, 1.0, 0.3, 0.8)),
                BorderRadius::all(Val::Px(6.0)),
            ))
            .with_children(|b| {
                b.spawn((
                    Text::new(format!("Continue to Station {}", station_level.0 + 2)),
                    TextFont { font: font.clone(), font_size: 28.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });

            // Leave — fly home, return to main menu
            col.spawn((
                Button,
                EndScreenButtons::Leave,
                Node {
                    width: Val::Px(420.0),
                    height: Val::Px(60.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.4, 0.05, 0.05, 0.85)),
                BorderColor(Color::srgba(1.0, 0.3, 0.3, 0.8)),
                BorderRadius::all(Val::Px(6.0)),
            ))
            .with_children(|b| {
                b.spawn((
                    Text::new("Leave"),
                    TextFont { font: font.clone(), font_size: 28.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });
        });
    });
}

// Check if player health is < 0
fn check_game_over(
    mut next_state: ResMut<NextState<GameState>>,
    player_q: Query<&Health, With<Player>>,
) {
    if let Ok(health) = player_q.single() {
        if health.0 <= 0.0 {
            debug!("Player health reached 0 — transitioning to GameOver!");
            next_state.set(GameState::GameOver);
        }
    }
}

// Display game over screen
fn setup_game_over_screen(mut commands: Commands, asset_server: Res<AssetServer>) {

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ZIndex(20),
        GameOverScreen,
    ))
    .with_children(|root|{

        //Background
        root.spawn((
            Node{
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ImageNode::new(asset_server.load("game_over.png")),
        ));

        root.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                margin:UiRect {
                    left: Val::Percent(0.),
                    right: Val::Percent(0.),
                    top: Val::Percent(25.),
                    bottom: Val::Percent(0.)
                },
                column_gap: Val::Px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Row,
                ..default()
            },
            ))
            .with_children(|col|{
                col.spawn((
                    Button,
                    EndScreenButtons::PlayAgain,
                    ImageNode::new(asset_server.load("playagain.png")),
                ));
                col.spawn((
                    Button,
                    EndScreenButtons::MainMenu,
                    ImageNode::new(asset_server.load("mainmenu.png")),
                )); 
            });
    });
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, MainCamera));
}

fn maximize_window(
    windows: Query<Entity, With<bevy::window::PrimaryWindow>>,
    winit_windows: NonSend<WinitWindows>,
) {
    let Ok(entity) = windows.single() else { return };
    let Some(window) = winit_windows.get_window(entity) else { return };
    window.set_maximized(true);
}

fn setup_ui_health(mut commands: Commands, asset_server: Res<AssetServer>, station_level: Res<StationLevel>) {
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    // HUD column: HP bar, shield bar, station label
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                top: Val::Px(12.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                ..default()
            },
            ZIndex(10),
            GameEntity,
        ))
        .with_children(|col| {
            // ── HP row ──────────────────────────────────────────────────
            col.spawn((Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                ..default()
            },))
            .with_children(|row| {
                row.spawn((Node::default(),)).with_children(|c| {
                    c.spawn((
                        Text::new("HP"),
                        TextFont { font: font.clone(), font_size: 20.0, ..default() },
                        TextColor(Color::srgb(1.0, 0.3, 0.3)),
                    ));
                });
                // Bar background
                row.spawn((
                    Node {
                        width: Val::Px(180.0),
                        height: Val::Px(14.0),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.25, 0.0, 0.0, 0.85)),
                    BorderRadius::all(Val::Px(3.0)),
                ))
                .with_children(|bg| {
                    bg.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.1, 0.9, 0.1)),
                        BorderRadius::all(Val::Px(3.0)),
                        HealthBarFill,
                    ));
                });
            });

            // ── Shield row (hidden until shield pickup collected) ────────
            col.spawn((
                Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    ..default()
                },
                Visibility::Hidden,
                ShieldBarRow,
            ))
            .with_children(|row| {
                row.spawn((Node::default(),)).with_children(|c| {
                    c.spawn((
                        Text::new("SH"),
                        TextFont { font: font.clone(), font_size: 20.0, ..default() },
                        TextColor(Color::srgb(0.3, 0.7, 1.0)),
                    ));
                });
                row.spawn((
                    Node {
                        width: Val::Px(180.0),
                        height: Val::Px(14.0),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.1, 0.3, 0.85)),
                    BorderRadius::all(Val::Px(3.0)),
                ))
                .with_children(|bg| {
                    bg.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.3, 0.8, 1.0)),
                        BorderRadius::all(Val::Px(3.0)),
                        ShieldBarFill,
                    ));
                });
            });

            // ── Station label ────────────────────────────────────────────
            col.spawn((Node::default(),)).with_children(|c| {
                c.spawn((
                    Text::new(format!("Station {}", station_level.0 + 1)),
                    TextFont { font, font_size: 18.0, ..default() },
                    TextColor(Color::srgb(0.8, 0.8, 1.0)),
                    StationLevelDisplay,
                ));
            });
        });
}

fn update_ui_health_text(
    player_q: Query<(&Health, &player::MaxHealth, &player::Shield), With<Player>>,
    mut hp_fill_q: Query<(&mut Node, &mut BackgroundColor), (With<HealthBarFill>, Without<ShieldBarFill>)>,
    mut sh_fill_q: Query<(&mut Node, &mut BackgroundColor), (With<ShieldBarFill>, Without<HealthBarFill>)>,
    mut sh_row_q: Query<&mut Visibility, With<ShieldBarRow>>,
) {
    let Ok((health, max_hp, shield)) = player_q.single() else { return };

    // HP bar width + color
    if let Ok((mut node, mut color)) = hp_fill_q.single_mut() {
        let ratio = (health.0 / max_hp.0).clamp(0.0, 1.0);
        node.width = Val::Percent(ratio * 100.0);
        let r = (1.0 - ratio).min(1.0);
        let g = ratio.min(1.0);
        *color = BackgroundColor(Color::srgb(r, g, 0.0));
    }

    // Shield bar + row visibility
    if let Ok(mut vis) = sh_row_q.single_mut() {
        *vis = if shield.max > 0.0 { Visibility::Visible } else { Visibility::Hidden };
    }
    if shield.max > 0.0 {
        if let Ok((mut node, _)) = sh_fill_q.single_mut() {
            let ratio = (shield.current / shield.max).clamp(0.0, 1.0);
            node.width = Val::Percent(ratio * 100.0);
        }
    }
}

fn damage_on_collision(
    time: Res<Time>,
    mut cooldown: ResMut<DamageCooldown>,
    mut player_q: Query<(&mut Health, &Transform), With<Player>>,
    damaging_q: Query<(&Transform, &Collider, &Damage), With<Collidable>>,
) {
    cooldown.0.tick(time.delta());

    if let Ok((mut health, p_tf)) = player_q.single_mut() {
        if !cooldown.0.finished() { return; }

        let player_half = Vec2::splat(TILE_SIZE * 0.5);
        let px = p_tf.translation.x;
        let py = p_tf.translation.y;

        for (tf, col, dmg) in &damaging_q {
            let (cx, cy) = (tf.translation.x, tf.translation.y);
            let overlap_x = (px - cx).abs() <= (player_half.x + col.half_extents.x);
            let overlap_y = (py - cy).abs() <= (player_half.y + col.half_extents.y);

            if overlap_x && overlap_y {
                health.0 -= dmg.amount;
                debug!(" Player took {} damage! HP now = {}", dmg.amount, health.0);
                cooldown.0.reset();
                break;
            }
        }
    }
}


fn start_game_music(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    volume: Res<GameMusicVolume>,
) {
    let music_handle = asset_server.load("audio/game_music_maybe.ogg");

    commands.spawn((
        AudioPlayer::new(music_handle),
        PlaybackSettings {
            mode: bevy::audio::PlaybackMode::Loop,
            volume: Volume::Linear(volume.0),
            ..default()
        },
        GameMusic,
        MusicTrack,
    ));

    debug!("Game music started");
}

fn stop_game_music(
    mut commands: Commands,
    music_query: Query<Entity, With<GameMusic>>,
) {
    for entity in &music_query {
        commands.entity(entity).despawn();
        debug!("Game music stopped");
    }
}

fn toggle_game_music(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    music_query: Query<Entity, With<GameMusic>>,
    volume: Res<GameMusicVolume>,
) {
    if !keys.just_pressed(KeyCode::KeyM) {
        return;
    }

    if music_query.is_empty() {
        let music_handle = asset_server.load("audio/game_music_maybe.ogg");

        commands.spawn((
            AudioPlayer::new(music_handle),
            PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Loop,
                volume: Volume::Linear(volume.0),
                ..default()
            },
            GameMusic,
            MusicTrack,
        ));

        debug!("Game music toggled ON");
    } else {
        for e in &music_query {
            commands.entity(e).despawn();
        }
        debug!("Game music toggled OFF");
    }
}

fn log_state_change(state: Res<State<GameState>>) {
    info!("Just moved to {:?}!", state.get());
}

fn handle_end_screen_buttons(
    mut commands: Commands,
    mut interactions: Query<(&Interaction, &EndScreenButtons), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut station_level: ResMut<StationLevel>,
) {
    for (interaction, which) in &mut interactions {
        
        if *interaction != Interaction::Pressed {
            continue;
        }
        match which {
            EndScreenButtons::Continue => {
                // Increment station level — buffs are already saved in SavedPlayerBuffs.
                station_level.0 += 1;
                info!("Continuing to station {} (difficulty increased)", station_level.0 + 1);
                next_state.set(GameState::Loading);
            }
            EndScreenButtons::PlayAgain => {
                // Full reset
                station_level.0 = 0;
                commands.remove_resource::<SavedPlayerBuffs>();
                next_state.set(GameState::Loading);
            }
            EndScreenButtons::MainMenu => {
                // Full reset
                station_level.0 = 0;
                commands.remove_resource::<SavedPlayerBuffs>();
                next_state.set(GameState::Menu);
            }
            EndScreenButtons::Leave => {
                // Player chose to leave after clearing a station — full reset, back to menu.
                station_level.0 = 0;
                commands.remove_resource::<SavedPlayerBuffs>();
                next_state.set(GameState::Menu);
            }
        }
    }
}

fn clean_end_screen(mut commands: Commands, root_q: Query<Entity, With<GameOverScreen>>) {
    for e in &root_q {
        commands.entity(e).despawn();
    }
}

fn clean_game(mut commands: Commands, root_q: Query<Entity, With<GameEntity>>) {
    for e in &root_q {
        if let Ok(mut entity_commands) = commands.get_entity(e) {
            entity_commands.despawn();
        }
    }
}