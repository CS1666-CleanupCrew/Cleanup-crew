mod endcredits;
mod motion;
use motion::motionStruct;

const TITLE: &str = "bv05 Better Motion";
const WIN_W: f32 = 1280.;
const WIN_H: f32 = 720.;
const PLAYER_SIZE: f32 = 32.;
const PLAYER_SPEED: f32 = 300.;
const ACCEL_RATE: f32 = 3600.;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::Srgba(Srgba::gray(0.25))))
        .add_plugins(DefaultPlugins)
        .add_plugins(BetterMotionPlugin::new(
            WIN_W,
            WIN_H,
            PLAYER_SIZE,
            PLAYER_SPEED,
            ACCEL_RATE,
            "player sprite.png", // <-- path to your sprite file in assets/
        ))
        .run();
}
