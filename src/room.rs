// use bevy::prelude::*;

// pub struct Room{
//     cleared: bool,
//     num_of_enemies: i32,
//     doors: Vec<Door>,
//     top_left_corner: Vec2,
//     bot_right_corner: Vec2,
// }

// impl Room{
//     fn new(ne: i32, tlc: Vec2, brc: Vec2) -> Self{
//         Self{
//             cleared: false,
//             num_of_enemies: ne,
//             doors: Vec::new(),
//             top_left_corner: tlc.clone(),
//             bot_right_corner: brc.clone(),
//         }
//     }

//     pub fn doors(&self, mut commands: Commands) -> bool{
//         if self.num_of_enemies == 0{
//             return false
//         }

//         for door in self.doors.iter(){
//             if !door.is_open {
//                 sprite.image = tiles.open_door.clone();
//                 door.is_open = true;

//                 // Remove collision
//                 commands.entity(entity).remove::<Collidable>();
//                 commands.entity(entity).remove::<Collider>();
//                 return true
//             }
//             return true
//         }
//     }
// }

// #[derive(Component)]
// pub struct Door {
//     pub is_open: bool,
// }

// impl Plugin for Room{
//     fn build(&self, app: &mut App){
        
//     }
// }

// //ne = number of enemies
// //tlc = top left corner
// //brc = bottome right corner
// pub fn create_room(
//     ne: i32,
//     tlc: Vec2,
//     brc: Vec2,
// ) -> Room{
//     Room::new(ne, tlc, brc)
// }
