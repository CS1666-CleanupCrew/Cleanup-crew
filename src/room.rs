use bevy::prelude::*;

#[derive(Component)]
pub struct Room{
    pub cleared: bool,
    pub num_of_enemies: i32,
    pub doors: Vec<Door>,
    pub top_left_corner: Vec2,
    pub bot_right_corner: Vec2,
}

impl Room{
    fn new(ne: i32, tlc: Vec2, brc: Vec2) -> Self{
        Self{
            cleared: false,
            num_of_enemies: ne,
            doors: Vec::new(),
            top_left_corner: tlc.clone(),
            bot_right_corner: brc.clone(),
        }
    }
}

#[derive(Component)]
pub struct Door {
    pub is_open: bool,
}

impl Plugin for Room{
    fn build(&self, app: &mut App){
        
    }
}

//ne = number of enemies
//tlc = top left corner
//brc = bottome right corner
pub fn create_room(
    ne: i32,
    tlc: Vec2,
    brc: Vec2,
) -> Room{
    Room::new(ne, tlc, brc)
}
