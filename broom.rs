use bevy::{prelude::*, window::PrimaryWindow};
use crate::collidable::{Collidable, Collider};
use crate::{WIN_W,WIN_H};

fn load_broom(
    mut commands: Commands, 
    asset_server: Res<AssetServer>,
){  
    let broom_image: Handle<Image> = asset_server.load("Broom.png");
    commands.insert_resource(broom_image);
}

