use bevy::{prelude::*};


use crate::{
    GameState, ACCEL_RATE, LEVEL_LEN, PLAYER_SPEED, TILE_SIZE, WIN_H, WIN_W
};

#[derive(Component)]
pub struct Player;

#[derive(Component, Deref, DerefMut)]
pub struct Velocity(Vec2);

#[derive(Resource)]
pub struct PlayerRes(Handle<Image>);

//Creates an instance of a Velocity
impl Velocity{
    fn new() -> Self{
        Self(Vec2::ZERO)
    }
}

//Allows for vec2.into() instead of Velocity::from(vec2)
impl From<Vec2> for Velocity{
    fn from(velocity: Vec2) -> Self{
        Self(velocity)
    }
}

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin{
    fn build(&self, app: &mut App){
        app
            .add_systems(OnEnter(GameState::Playing), load_player)
            .add_systems(OnEnter(GameState::Playing), spawn_player.after(load_player))
            .add_systems(Update, move_player.run_if(in_state(GameState::Playing)))
            ;
    }
}

fn load_player(
    mut commands: Commands, 
    asset_server: Res<AssetServer>
){
    let player: Handle<Image>= asset_server.load("Player_Sprite.png");

    commands.insert_resource(PlayerRes(
        player.clone(),
    ));
}

fn spawn_player(
    mut commands: Commands,
    player_sheet: Res<PlayerRes>,
){
    commands.spawn((
        Sprite::from_image(
            player_sheet.0.clone()
        ),
        Transform{
            translation: Vec3::new(0., 0., 0.),
            scale: Vec3::new(0.1875, 0.1875, 0.1875),
            ..Default::default()
        },
        Player,
        Velocity::new(),
    ));
}


/**
 * Single is a query for exactly one entity
 * With tells bevy to include entities with the Player component
 * Without is the opposite
*/
fn move_player(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    player: Single<(&mut Transform, &mut Velocity), With<Player>>,
){
    let (mut transform, mut velocity) = player.into_inner();

    let mut dir = Vec2::ZERO;

    if input.pressed(KeyCode::KeyA){
        dir.x -= 1.;
    }
    if input.pressed(KeyCode::KeyD){
        dir.x += 1.;
    }    
    if input.pressed(KeyCode::KeyW){
        dir.y += 1.;
    }    
    if input.pressed(KeyCode::KeyS){
        dir.y -= 1.;
    }

    //Time based on frame to ensure that movement is the same no matter the fps
    let deltat = time.delta_secs();
    let accel = ACCEL_RATE * deltat;

    ** velocity = if dir.length() > 0.{
        (**velocity + (dir.normalize_or_zero() * accel)).clamp_length_max(PLAYER_SPEED)
    } else if velocity.length() > accel{
        **velocity + (velocity.normalize_or_zero() * -accel)
    } else{
        Vec2::ZERO
    };
    let change = **velocity * deltat;

    let min = Vec3::new(
        -WIN_W / 2. + (TILE_SIZE as f32) / 2.,
        -WIN_H / 2. + (TILE_SIZE as f32) * 1.5,
        900.,
    );

    let max = Vec3::new(
        LEVEL_LEN - (WIN_W / 2. + (TILE_SIZE as f32) / 2.),
        WIN_H / 2. - (TILE_SIZE as f32) / 2.,
        900.,
    );

    transform.translation = (transform.translation + change.extend(0.)).clamp(min, max);
}