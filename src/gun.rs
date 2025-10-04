use bevy::{prelude::*};

#[derive(Resource)]
pub struct BulletRes(Handle<Image>);


pub struct GunPlugin;
impl Plugin for GunPlugin{
    fn build(&self, app: &mut App){
        app
            //.systems
    }
}

fn load_bullet(mut commands: Commands, asset_server: Res<AssetServer>){
    fn load_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    let bullet: Handle<Image> = asset_server.load("bullet.png");

    commands.insert_resource(BulletRes(bullet.clone()));
}
}

fn spawn_bullet(){

}