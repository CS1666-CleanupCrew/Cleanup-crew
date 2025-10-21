use crate::collidable::{Collidable, Collider};
use crate::player::{Health, Player};
use bevy::{prelude::*, window::PresentMode};
use crate::air::init_air_grid;

pub mod collidable;
pub mod endcredits;
pub mod enemy;
pub mod player;
pub mod table;
pub mod window;
pub mod map;
pub mod procgen;
pub mod air;
pub mod noise;
pub mod menu;

const TITLE: &str = "Cleanup Crew";
const WIN_W: f32 = 1280.;
const WIN_H: f32 = 720.;

const PLAYER_SPEED: f32 = 500.;
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
struct Damage { amount: f32, }

#[derive(Resource)]
struct DamageCooldown(Timer);

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
    EndCredits,
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
        .add_plugins((
            map::MapPlugin,
            player::PlayerPlugin,
            endcredits::EndCreditPlugin,
            enemy::EnemyPlugin,
            table::TablePlugin,
            window::WindowPlugin,
            procgen::ProcGen,
            menu::MenuPlugin,
        ))
        .add_systems(Startup, setup_camera)
        .add_systems(OnEnter(GameState::Menu), log_state_change)
        .add_systems(OnEnter(GameState::Loading), log_state_change)
        .add_systems(OnEnter(GameState::EndCredits), log_state_change)
        .add_systems(OnEnter(GameState::Playing), log_state_change)
        .add_systems(Startup, setup_ui_health)
        .add_systems(
            Update,
            update_ui_health_text.run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            damage_on_collision.run_if(in_state(GameState::Playing)),
        )
        .insert_resource(DamageCooldown(Timer::from_seconds(0.5, TimerMode::Once)))
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, MainCamera));
}

fn setup_ui_health(mut commands: Commands, asset_server: Res<AssetServer>) {
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
            font,
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.0, 0.0)),
        ZIndex(10),
        HealthDisplay,
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
                info!(" Player took {} damage! HP now = {}", dmg.amount, health.0);
                cooldown.0.reset();
                break;
            }
        }
    }
}

fn log_state_change(state: Res<State<GameState>>) {
    info!("Just moved to {:?}!", state.get());
}