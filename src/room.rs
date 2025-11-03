use bevy::prelude::*;

use crate::player::Player;
use crate::map::Door;

#[derive(Resource)]
pub struct RoomVec(pub Vec<Room>);
pub struct Room{
    entered: bool,
    cleared: bool,
    pub doors:Vec<Entity>,
    num_of_enemies: i32,
    top_left_corner: Vec2,
    bot_right_corner: Vec2,
}

impl Room{
    pub fn new(ne: i32, tlc: Vec2, brc: Vec2) -> Self{
        Self{
            entered: false,
            cleared: false,
            doors:Vec::new(),
            num_of_enemies: ne,
            top_left_corner: tlc.clone(),
            bot_right_corner: brc.clone(),
        }
    }

    pub fn bounds_check(&self, pos:Vec2) -> bool{
        self.top_left_corner.x <= pos.x && self.top_left_corner.y >= pos.y && self.bot_right_corner.x >= pos.x && self.bot_right_corner.y <= pos.y
    }
}

//ne = number of enemies
//tlc = top left corner
//brc = bottom right corner
pub fn create_room(
    ne: i32,
    tlc: Vec2,
    brc: Vec2,
    room_vec: &mut RoomVec,
){
    room_vec.0.push(Room::new(ne, tlc, brc));
}

pub fn assign_doors(
    doors: Query<(Entity, &Transform, &mut Door)>,
    mut rooms: ResMut<RoomVec>,
){
    for (entity, pos, door) in doors.iter(){

        for room in rooms.0.iter_mut(){

            if room.bounds_check(Vec2::new(pos.translation.x, pos.translation.y)) {
                room.doors.push(entity);
                break;
            }

        }
    }

}



pub fn track_rooms(
    player: Single<&Transform, With<Player>>,
    mut rooms: ResMut<RoomVec>,
){
    let pos = player.into_inner();
    
    for room in rooms.0.iter_mut(){

        if room.bounds_check(Vec2::new(pos.translation.x, pos.translation.y)){
            room.entered = true;
            println!("Entered Room! {}", room.top_left_corner.x);
        }
        else{
            room.entered = false;
        }

    }
    
}
