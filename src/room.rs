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
use crate::player::Player;
use crate::enemy::{EnemyRes, spawn_enemy_at};

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
    pub enemies:Vec<Entity>,
    top_left_corner: Vec2,
    bot_right_corner: Vec2,
    tile_top_left_corner: Vec2,
    tile_bot_right_corner: Vec2,
    layout: Vec<String>,
}

impl Room{
    pub fn new(tlc: Vec2, brc: Vec2, tile_tlc: Vec2, tile_brc: Vec2, room_layout: Vec<String>) -> Self{
        Self{
            cleared: false,
            doors:Vec::new(),
            enemies:Vec::new(),
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
        self.top_left_corner.x < pos.x.floor() && self.top_left_corner.y > pos.y.floor() && self.bot_right_corner.x > pos.x.floor() && self.bot_right_corner.y < pos.y.floor()
    }
}

pub struct RoomPlugin;

impl Plugin for RoomPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameState::Loading), setup)
            .add_systems(Update, track_rooms.run_if(in_state(GameState::Playing)))
            .add_systems(Update, entered_room.run_if(in_state(GameState::Playing)))
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
        LevelState::EnteredRoom(index) =>
        {
        }
        _ =>
        {
            let pos = player.into_inner();
        
            for (index, room )in rooms.0.iter_mut().enumerate(){

                if room.within_bounds_check(Vec2::new(pos.translation.x, pos.translation.y)) && !room.cleared{
                    *lvlstate = LevelState::EnteredRoom(index);
                }
            }
        }
    }
    
}

pub fn entered_room(
    rooms:  Res<RoomVec>,
    mut lvlstate: ResMut<LevelState>,
    mut commands: Commands,
    tiles: Res<TileRes>,
    enemy_res: Res<EnemyRes>,
){
    match *lvlstate
    {
        LevelState::EnteredRoom(index) =>
        {
            for door in rooms.0[index].doors.iter(){

                commands.entity(*door).insert(Collidable);
                commands.entity(*door).insert(Collider { half_extents: Vec2::splat(TILE_SIZE * 0.5) },);

                let image = tiles.closed_door.clone();
                commands.entity(*door).entry::<Sprite>().and_modify(|mut sprite|{
                    sprite.image = image;
                });
            }
            generate_enemies_in_room(15, None, & rooms, index, commands, & enemy_res);
            *lvlstate = LevelState::InRoom(index);
        }
        _ => {

        }

    }
}

pub fn playing_room(
    rooms:  ResMut<RoomVec>,
    mut lvlstate: ResMut<LevelState>,
){
    match *lvlstate
    {
        LevelState::InRoom(index) =>
        {
            
        }
        _ => {

        }

    }
}

pub fn generate_enemies_in_room(
    num_of_enemies: usize,
    seed: Option<u64>,
    rooms: & RoomVec,
    index: usize,
    mut commands: Commands,
    enemy_res: & EnemyRes,
){  
    let mut floors: Vec<(f32, f32)> = Vec::new();
    
    let room = & rooms.0[index];

    let top = room.tile_top_left_corner.y as usize - 1;
    let bot = room.tile_bot_right_corner.y as usize + 1;

    for (y, row) in room.layout.iter().enumerate().skip(1)
    {
        let pos_y = room.bot_right_corner.y + (y as f32 * 32.0);

        for (x, ch) in row.chars().enumerate()
        {
            let pos_x = room.bot_right_corner.x + (x as f32 * 32.0);
            if x > room.tile_top_left_corner.x as usize && x < room.tile_bot_right_corner.x as usize
            {
                if ch == '#' 
                {
                        floors.push((pos_x, pos_y));
                }
            }
        }
    }

    // Shuffle and pick up to max_enemies positions
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

    for i in floors.into_iter().take(num_of_enemies){
        spawn_enemy_at(&mut commands, &enemy_res, Vec3::new(i.0 as f32, i.1 as f32, Z_ENTITIES), true); // active now
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
