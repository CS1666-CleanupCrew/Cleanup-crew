use bevy::prelude::*;
use crate::player::{MaxHealth, Player};
use crate::{TILE_SIZE, GameEntity};

#[derive(Component)]
pub struct Heart;

#[derive(Resource)]
pub struct HeartRes {
    pub image: Handle<Image>,
}

pub struct HeartPlugin;

impl Plugin for HeartPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, load_heart)
            .add_systems(Update, collect_heart.run_if(in_state(crate::GameState::Playing)));
    }
}

fn load_heart(mut commands: Commands, asset_server: Res<AssetServer>) {
    let heart_image = asset_server.load("heart.png");
    commands.insert_resource(HeartRes { image: heart_image });
}

pub fn spawn_heart(
    commands: &mut Commands,
    heart_res: &HeartRes,
    position: Vec2,
) {
    commands.spawn((
        Sprite::from_image(heart_res.image.clone()),
        Transform {
            translation: Vec3::new(position.x, position.y, crate::Z_ENTITIES),
            scale: Vec3::splat(1.0),
            ..Default::default()
        },
        Heart,
        GameEntity,
    ));
}

fn collect_heart(
    mut commands: Commands,
    mut player_query: Query<(&Transform, &mut crate::player::Health, &MaxHealth), With<Player>>,
    heart_query: Query<(Entity, &Transform), With<Heart>>,
) {
    let Ok((player_tf, mut health, maxhp)) = player_query.single_mut() else {
        return;
    };
    
    let player_pos = player_tf.translation.truncate();
    let collect_radius = TILE_SIZE * 1.5;
    
    for (heart_entity, heart_tf) in &heart_query {
        let heart_pos = heart_tf.translation.truncate();
        let distance = player_pos.distance(heart_pos);
        
        if distance < collect_radius {
            health.0 = (health.0 + 20.0).min(maxhp.0);
            commands.entity(heart_entity).despawn();
            info!("Heart collected! Health: {}", health.0);
        }
    }
}