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

#[derive(Event)]
pub struct LevelWritten;


pub type TablePositions = HashSet<(usize, usize)>;

//Layout of each room
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
}

impl RoomRes{
    fn room(&mut self, n: i8) -> &mut RoomLayout{
        match n {
            1 => &mut self.room1,
            2 => &mut self.room2,
            _ => panic!("Room doesn't exist"),
        }
    }
}

pub struct ProcGen;

impl Plugin for ProcGen{
    fn build(&self, app: &mut App) {
        app
            // .add_systems(OnEnter(GameState::Loading), load_rooms)
            // .add_systems(OnEnter(GameState::Loading), write_room.after(load_rooms))
            ;
    }
}

pub fn load_rooms(
    mut commands: Commands,
){
    //Update numroom here to increase or decrease the number of rooms
    let mut rooms:RoomRes = RoomRes{
        numroom:2,
        room1:RoomLayout::new(),
        room2:RoomLayout::new(),
    };
   
    for n in 1..=rooms.numroom{
        let room = rooms.room(n);

        //Create the filename for each room
        let mut filename: String = "assets/rooms/room".to_owned();
        filename.push_str(&n.to_string());
        filename.push_str(".txt");

        //Read the file for that room
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

pub fn write_room(
    mut commands: Commands,
    rooms: Res<RoomRes>,
){
    
    let f = File::create("assets/rooms/level.txt").expect("file don't exist");
    let mut writer = BufWriter::new(f);

    for line in &rooms.room1.layout{
        writer.write(line.as_bytes()).expect("Smth went wrong");
        writer.write("\n".as_bytes()).expect("Smth went wrong");
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
