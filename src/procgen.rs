use bevy::prelude::*;
use rand::seq::SliceRandom;
use rand::{rng, SeedableRng};
use rand::rngs::StdRng;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::io::{BufWriter, Write};
use std::io::prelude::*;
use crate::GameState;

pub type TablePositions = HashSet<(usize, usize)>;

pub struct RoomLayout{
    layout: Vec<String>,
    width: f32,
    height: f32,
}

impl RoomLayout{
    fn new() -> Self{
        Self{
            layout: Vec::new(),
            width: 0.0,
            height: 0.0,
        }
    }
}

//Contains all the different rooms
#[derive(Resource)]
pub struct RoomRes {
    numroom: i8,
    room1: RoomLayout,
    room2: RoomLayout,
    room3: RoomLayout,
    room4: RoomLayout,
    room5: RoomLayout,
    room6: RoomLayout,
}

impl RoomRes{
    fn room(&mut self, n: i8) -> &mut RoomLayout{
        match n {
            1 => &mut self.room1,
            2 => &mut self.room2,
            3 => &mut self.room3,
            4 => &mut self.room4,
            5 => &mut self.room5,
            6 => &mut self.room6,
            _ => panic!("Room doesn't exist"),
        }
    }
}

pub struct ProcGen;

impl Plugin for ProcGen {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameState::Loading), load_rooms)
            .add_systems(OnEnter(GameState::Loading), build_full_level.after(load_rooms));
    }
}



pub fn load_rooms(
    mut commands: Commands,
){
    //Update numroom here to increase or decrease the number of rooms
    let mut rooms:RoomRes = RoomRes{
        numroom:6,
        room1:RoomLayout::new(),
        room2:RoomLayout::new(),
        room3:RoomLayout::new(),
        room4:RoomLayout::new(),
        room5:RoomLayout::new(),
        room6:RoomLayout::new(),
    };
   
    for n in 1..=rooms.numroom{
        let room = rooms.room(n);

        let mut filename: String = "assets/rooms/room".to_owned();
        filename.push_str(&n.to_string());
        filename.push_str(".txt");

        let f = File::open(filename).expect("file don't exist");
        let reader = BufReader::new(f);
        
        //Push each line into the Vec<String> in room.layout
        for line_result in reader.lines() {
            let line = line_result.unwrap();
            room.layout.push(line);
        }
        
    }

    commands.insert_resource(rooms);
}

pub fn build_full_level(
    rooms: Res<RoomRes>,
) {
    const MAP_W: usize = 300;
    const MAP_H: usize = 300;

    // full map of '.'
    let mut map: Vec<Vec<char>> = vec![vec!['.'; MAP_W]; MAP_H];

    // Empty map now created add starting room

    write_room(&mut map, &rooms.room1, 120, 120);



    let f = File::create("assets/rooms/level.txt").expect("Couldn't create output file");
    let mut writer = BufWriter::new(f);

    for row in map {
        let line: String = row.into_iter().collect();
        writeln!(writer, "{line}").expect("Failed to write map row");
    }    
}

// Writes a room into an existing map at a given top-left coordinate.
// `top_left_x` and `top_left_y` are the coordinates of the room's top-left corner in the map.
pub fn write_room(
    map: &mut Vec<Vec<char>>,
    room: &RoomLayout,
    top_left_x: usize,
    top_left_y: usize,
) {
    let map_height = map.len();
    let map_width = if map_height > 0 { map[0].len() } else { 0 };

    for (row_idx, row_str) in room.layout.iter().enumerate() {
        let y = top_left_y + row_idx;
        if y >= map_height {
            continue;
        }

        for (col_idx, ch) in row_str.chars().enumerate() {
            let x = top_left_x + col_idx;
            if x >= map_width {
                continue;
            }

            map[y][x] = ch;
        }
    }
}


/// Generates table positions from a grid representation of the room.
/// `grid` is a slice of strings where each string represents a row in the room.
/// `#` characters represent floor cells where tables can be placed.
/// `max_tables` is the maximum number of tables to generate.
/// `seed` is an optional seed for random number generation to allow reproducible layouts.
/// Returns a set of (x, y) positions for the tables.
pub fn generate_tables_from_grid(
    grid: &[String],
    max_tables: usize,
    seed: Option<u64>,
) -> TablePositions {
    let rows = grid.len();
    if rows == 0 { return TablePositions::new(); }
    let _cols = grid[0].len();

    // Collect all floor cells ('#')
    let mut floors: Vec<(usize, usize)> = Vec::new();
    for (y, row) in grid.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if ch == '#' {
                floors.push((x, y));
            }
        }
    }
    // Shuffle and pick up to max_tables positions
    if let Some(s) = seed {
        let mut seeded = StdRng::seed_from_u64(s);
        floors.shuffle(&mut seeded);
    } else {
        let mut trng = rng();
        floors.shuffle(&mut trng);
    }

    floors.into_iter().take(max_tables).collect()
}
