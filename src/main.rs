use crate::collidable::{Collidable, Collider};
use crate::player::{Health, Player};
use bevy::{prelude::*, window::PresentMode};
use bevy::audio::Volume;
use crate::air::{AirGrid, init_air_grid, spawn_pressure_labels};
use crate::room::RoomVec;
use crate::map::MapGridMeta;

pub mod collidable;
pub mod endcredits;
pub mod enemy;
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
pub mod reward;
pub mod heart;
pub mod reaper;
pub mod weapon;



const TITLE: &str = "Cleanup Crew";
const WIN_W: f32 = 1280.;
const WIN_H: f32 = 720.;

const PLAYER_SPEED: f32 = 500.;

const LOW_AIR_THRESHOLD: f32 = 1.0; 
const AIR_DAMAGE_PER_SECOND: f32 = 5.0; 
const AIR_DAMAGE_TICK_RATE: f32 = 0.5;
const ACCEL_RATE: f32 = 5000.;
const TILE_SIZE: f32 = 32.;
const BG_WORLD: f32 = 2048.0;
const LEVEL_LEN: f32 = 1280.;

pub const Z_FLOOR: f32 = -100.0;
pub const Z_ENTITIES: f32 = 0.0;
pub const Z_UI: f32 = 100.0;

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct HealthDisplay;

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
pub struct AirDamageTimer(Timer);

#[derive(Component)]
pub enum EndScreenButtons{
    PlayAgain,
    MainMenu,
    Continue,
}

#[derive(Component)]
pub struct GameEntity;

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
                        resolution: (WIN_W, WIN_H).into(),
                        present_mode: PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                }),
        )
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
            enemy::EnemyPlugin,
            table::TablePlugin,
            fluiddynamics::FluidSimPlugin,
            window::WindowPlugin,
        ))
        .add_plugins((
            menu::MenuPlugin,
            bullet::BulletPlugin,
            room::RoomPlugin,
            broom::BroomPlugin,
            reward::RewardPlugin,
            heart::HeartPlugin,
            reaper::ReaperPlugin,
            weapon::WeaponPlugin,
        ))
        .add_systems(Startup, setup_camera)
        .add_systems(OnEnter(GameState::Menu), log_state_change)
        .add_systems(OnEnter(GameState::Loading), log_state_change)
        .add_systems(OnEnter(GameState::EndCredits), log_state_change)
        .add_systems(OnEnter(GameState::Playing), log_state_change)
        .add_systems(OnEnter(GameState::Playing), setup_air_damage_timer)
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
                damage_on_collision,
            )
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            check_game_over.run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            air_damage_system.run_if(in_state(GameState::Playing)),
        )
        
        .insert_resource(DamageCooldown(Timer::from_seconds(0.5, TimerMode::Once)))
        .insert_resource(GameMusicVolume(0.5)) // .5 volume by default
        .run();
}

fn check_win(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    rooms: Res<RoomVec>,
    player_q: Query<(&Health, &player::MaxHealth, &player::MoveSpeed, &weapon::Weapon, &player::NumOfCleared), With<Player>>,
){
    let mut count = 0;

    for room in rooms.0.iter(){
        if room.cleared{
            count += 1;
        }
    }

    if count == rooms.0.len(){
        // Save player buffs before transitioning (player will be despawned on exit)
        if let Ok((health, max_hp, move_spd, weapon, num_cleared)) = player_q.single() {
            commands.insert_resource(SavedPlayerBuffs {
                max_health: max_hp.0,
                health: health.0,
                move_speed: move_spd.0,
                fire_rate: weapon.fire_rate,
                num_cleared: num_cleared.0,
            });
        }
        next_state.set(GameState::Win);
    }
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

        // Button column
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
            // Continue button (new station, keep buffs)
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

            // Play Again (full restart)
            col.spawn((
                Button,
                EndScreenButtons::PlayAgain,
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
                    Text::new("Restart (New Game)"),
                    TextFont { font: font.clone(), font_size: 28.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });

            // Main Menu
            col.spawn((
                Button,
                EndScreenButtons::MainMenu,
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
                    Text::new("Main Menu"),
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

fn setup_ui_health(mut commands: Commands, asset_server: Res<AssetServer>, station_level: Res<StationLevel>) {
    let font: Handle<Font> = asset_server.load("fonts/BitcountSingleInk-VariableFont_CRSV,ELSH,ELXP,SZP1,SZP2,XPN1,XPN2,YPN1,YPN2,slnt,wght.ttf");
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(12.0),
            ..default()
        },
        Text::new("HP: 100"),
        TextFont {
            font: font.clone(),
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.0, 0.0)),
        ZIndex(10),
        HealthDisplay,
        GameEntity,
    ));

    // Station level display
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(40.0),
            ..default()
        },
        Text::new(format!("Station {}", station_level.0 + 1)),
        TextFont {
            font,
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 1.0)),
        ZIndex(10),
        StationLevelDisplay,
        GameEntity,
    ));
}

fn update_ui_health_text(
    player_q: Query<&Health, With<Player>>,
    mut text_q: Query<&mut Text, With<HealthDisplay>>,
) {
    if let (Ok(health), Ok(mut text)) = (player_q.single(), text_q.single_mut()) {
        *text = Text::new(format!("HP: {}", health.0.round() as i32));
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


fn setup_air_damage_timer(
    mut commands: Commands,
    player_q: Query<Entity, With<Player>>,
) {
    if let Ok(player_entity) = player_q.single() {
        commands.entity(player_entity).insert(AirDamageTimer(
            Timer::from_seconds(AIR_DAMAGE_TICK_RATE, TimerMode::Repeating)
        ));
        info!("Air damage system initialized");
    }
}


fn air_damage_system(
    time: Res<Time>,
    air_grid_q: Query<&AirGrid>,
    grid_meta: Res<MapGridMeta>,
    mut player_q: Query<(&Transform, &mut Health, &mut AirDamageTimer), With<Player>>,
) {
    let Ok(air_grid) = air_grid_q.single() else {
        return;
    };

    let Ok((transform, mut health, mut timer)) = player_q.single_mut() else {
        return;
    };

  
    let player_pos = transform.translation.truncate();
    let grid_x = ((player_pos.x - grid_meta.x0) / TILE_SIZE).clamp(0.0, (grid_meta.cols - 1) as f32) as usize;
    let grid_y = ((player_pos.y - grid_meta.y0) / TILE_SIZE).clamp(0.0, (grid_meta.rows - 1) as f32) as usize;
    let grid_y_flipped = grid_meta.rows.saturating_sub(1).saturating_sub(grid_y);
    let air_pressure = air_grid.get(grid_x, grid_y_flipped);

   
    timer.0.tick(time.delta());

    
    if air_pressure < LOW_AIR_THRESHOLD && timer.0.just_finished() {
        let damage_amount = AIR_DAMAGE_PER_SECOND * AIR_DAMAGE_TICK_RATE;
        health.0 -= damage_amount;
        
        debug!(
            "Player taking air damage! Pressure: {:.2} at ({}, {}) - HP: {:.1}",
            air_pressure, grid_x, grid_y_flipped, health.0
        );
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
                // Increment station level — buffs are already saved in SavedPlayerBuffs
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