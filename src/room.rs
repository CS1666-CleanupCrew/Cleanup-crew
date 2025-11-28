use bevy::prelude::*;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use core::num;
use std::collections::HashSet;

use crate::collidable::{Collidable, Collider};
use crate::{GameState, TILE_SIZE, Z_ENTITIES};
use crate::map::Door;
use crate::map::TileRes;
use crate::player::{NumOfCleared, Player};
use crate::enemy::{EnemyRes, RangedEnemyRes, spawn_enemy_at, spawn_ranged_enemy_at};

#[derive(Resource)]
pub struct EnemyPosition(pub HashSet<(usize, usize)>);

#[derive(Resource)]
pub enum LevelState{
    EnteredRoom(usize),
    InRoom(usize),
    NotRoom
}

#[derive(Resource)]
pub struct RoomVec(pub Vec<Room>);

pub struct Room{
    cleared: bool,
    pub doors:Vec<Entity>,
    pub numofenemies: usize,
    top_left_corner: Vec2,
    bot_right_corner: Vec2,
    pub tile_top_left_corner: Vec2,
    pub tile_bot_right_corner: Vec2,
    layout: Vec<String>,
}

impl Room{
    pub fn new(tlc: Vec2, brc: Vec2, tile_tlc: Vec2, tile_brc: Vec2, room_layout: Vec<String>) -> Self{
        Self{
            cleared: false,
            doors:Vec::new(),
            numofenemies: 0,
            top_left_corner: tlc.clone(),
            bot_right_corner: brc.clone(),
            tile_top_left_corner: tile_tlc.clone(),
            tile_bot_right_corner: tile_brc.clone(),
            layout: room_layout.clone(),
        }
    }

    pub fn bounds_check(&self, pos:Vec2) -> bool{
        self.top_left_corner.x <= pos.x && self.top_left_corner.y >= pos.y && self.bot_right_corner.x >= pos.x && self.bot_right_corner.y <= pos.y
    }

    pub fn within_bounds_check(&self, pos:Vec2) -> bool{
        self.top_left_corner.x+32.0 < pos.x.floor() && self.top_left_corner.y-64.0 > pos.y.floor() && self.bot_right_corner.x-32.0 > pos.x.floor() && self.bot_right_corner.y+32.0 < pos.y.floor()
    }
}

pub struct RoomPlugin;

impl Plugin for RoomPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameState::Loading), setup)
            .add_systems(Update, track_rooms.run_if(in_state(GameState::Playing)))
            .add_systems(Update, entered_room.run_if(in_state(GameState::Playing)))
            .add_systems(Update, playing_room.run_if(in_state(GameState::Playing)))
            ;
    }
}

fn setup(
    mut commands: Commands,
){
    commands.insert_resource(LevelState::NotRoom);
    commands.insert_resource(EnemyPosition(HashSet::new()));
}

//ne = number of enemies
//tlc = top left corner
//brc = bottom right corner
pub fn create_room(
    tlc: Vec2,
    brc: Vec2,
    tile_tlc: Vec2,
    tile_brc: Vec2,
    rooms_vec: &mut RoomVec,
    room_layout: Vec<String>,
){
    rooms_vec.0.push(Room::new(tlc, brc, tile_tlc, tile_brc, room_layout));
}

pub fn assign_doors(
    doors: Query<(Entity, &Transform), With<Door>>,
    mut rooms: ResMut<RoomVec>,
){
    for (entity, pos) in doors.iter(){

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
    mut lvlstate: ResMut<LevelState>,
){
    match *lvlstate
    {
        LevelState::EnteredRoom(_) =>
        {
        }
        LevelState::InRoom(_)=>
        {
        }
        _ =>
        {
            let pos = player.into_inner();
        
            for (index, room )in rooms.0.iter_mut().enumerate(){

                if !room.cleared && room.within_bounds_check(Vec2::new(pos.translation.x, pos.translation.y)){
                    println!("Entered Room");
                    *lvlstate = LevelState::EnteredRoom(index);
                }
            }
        }
    }
    
}

pub fn entered_room(
    mut rooms:  ResMut<RoomVec>,
    mut lvlstate: ResMut<LevelState>,
    mut commands: Commands,
    tiles: Res<TileRes>,
    enemy_res: Res<EnemyRes>,
    ranged_res: Res<RangedEnemyRes>,
    play_query: Single<&NumOfCleared, With<Player>>,
){
    match *lvlstate
    {
        LevelState::EnteredRoom(index) =>
        {
            for door in rooms.0[index].doors.iter(){

                commands.entity(*door).insert(Collidable);
                commands.entity(*door).insert(Collider { half_extents: Vec2::splat(TILE_SIZE * 0.5) },);

                commands.entity(*door).insert(Sprite::from_image(tiles.closed_door.clone()));

                // let image = tiles.closed_door.clone();
                // commands.entity(*door).entry::<Sprite>().and_modify(|mut sprite|{
                //     sprite.image = image;
                // });
            }
            generate_enemies_in_room(1, None, &mut rooms, index, commands, & enemy_res, & ranged_res, & play_query);
            //println!("Generated Enemies. Moving to InRoom State");
            *lvlstate = LevelState::InRoom(index);
        }
        _ => {

        }

    }
}

pub fn playing_room(
    mut rooms:  ResMut<RoomVec>,
    mut lvlstate: ResMut<LevelState>,
    mut commands: Commands,
    tiles: Res<TileRes>,
    mut player: Single<&mut NumOfCleared, With<Player>>,
    heart_res: Res<crate::heart::HeartRes>
){
    match *lvlstate
    {
        LevelState::InRoom(index) =>
        {
            //println!("Num of Enemies: {}", rooms.0[index].numofenemies);
            if rooms.0[index].numofenemies == 0{
                println!("All enemies defeated");

                // Calculate room center and spawn heart
                let center_x = (rooms.0[index].top_left_corner.x + rooms.0[index].bot_right_corner.x) / 2.0;
                let center_y = (rooms.0[index].top_left_corner.y + rooms.0[index].bot_right_corner.y) / 2.0;
                let room_center = Vec2::new(center_x, center_y);
                crate::heart::spawn_heart(&mut commands, &heart_res, room_center);

                for door in rooms.0[index].doors.iter(){

                    commands.entity(*door).remove::<Collidable>();
                    commands.entity(*door).remove::<Collider>();

                    commands.entity(*door).insert(Sprite::from_image(tiles.open_door.clone()));
                }
                rooms.0[index].cleared = true;

                rooms.0.remove(index); //Not sure if we'll need a room after its cleared

                player.0 += 1;
                *lvlstate = LevelState::NotRoom;
            }

        }
        _ => {

        }

    }
}

pub fn generate_enemies_in_room(
    num_of_enemies: usize,
    seed: Option<u64>,
    rooms: &mut RoomVec,
    index: usize,
    mut commands: Commands,
    enemy_res: &EnemyRes,
    ranged_res: &RangedEnemyRes,
    play_query: &NumOfCleared,
) {
    //println!("Room is {}", index);
    let rooms_cleared = play_query.0;

    let mut floors: Vec<(f32, f32)> = Vec::new();

    let room = &mut rooms.0[index];

    let scaled_num_enemies = 1*rooms_cleared + num_of_enemies;

    room.numofenemies = scaled_num_enemies;

    let top =  (room.tile_bot_right_corner.y - room.tile_top_left_corner.y) as usize - 2;

    for (y, row) in room.layout[2..top].iter().enumerate()
    {
        let pos_y = room.top_left_corner.y - ((y+2) as f32 * 32.0);
        for (x, ch) in row.chars().enumerate()
        {
            if x != 1 && x < row.len()-2{
                let pos_x = room.top_left_corner.x + (x as f32 * 32.0);
                if ch == '#' 
                {
                        floors.push((pos_x, pos_y));
                }
            }
        }
    }

    // println!("All tiles located");
    // println!("# of Floors: {}", floors.len());
    if let Some(s) = seed 
    {
        let mut seeded = StdRng::seed_from_u64(s);
        floors.shuffle(&mut seeded);
    } 
    else 
    {
        let mut trng = rand::rng();
        floors.shuffle(&mut trng);
    }

    for (idx, (x, y)) in floors.into_iter().take(scaled_num_enemies).enumerate() {
        let pos = Vec3::new(x as f32, y as f32, Z_ENTITIES);

        if idx % 4 == 0 {
            // 1 in 4 are rangers
            spawn_ranged_enemy_at(&mut commands, ranged_res, pos, true);
        } else {
            spawn_enemy_at(&mut commands, enemy_res, pos, true);
        }
    }
}

pub fn generate_enemies_from_grid(
    grid: &[String],
    num_of_enemies: usize,
    seed: Option<u64>,
    enemy_hash: &mut EnemyPosition,
    rooms: & RoomVec,
){  
    for (i, room) in rooms.0.iter().enumerate()
    {
        let mut floors: Vec<(usize, usize)> = Vec::new();

        let top = room.tile_top_left_corner.y as usize;
        let bot = room.tile_bot_right_corner.y as usize;

        for y in bot..top
        { 
            let row = &grid[y];

            for (x, ch) in row.chars().enumerate()
            {
                if x > room.tile_top_left_corner.x as usize && x < room.tile_bot_right_corner.x as usize
                {
                    if ch == '#' 
                    {
                        floors.push((x, y));
                    }
                }
            }
        }

        // Shuffle and pick up to max_tables positions
        if let Some(s) = seed 
        {
            let mut seeded = StdRng::seed_from_u64(s);
            floors.shuffle(&mut seeded);
        } 
        else 
        {
            let mut trng = rand::rng();
            floors.shuffle(&mut trng);
        }

        enemy_hash.0.extend(floors.into_iter().take(num_of_enemies));
    }
}
