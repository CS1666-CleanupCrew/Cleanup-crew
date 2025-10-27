use bevy::prelude::*;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::io::{BufWriter, Write};
use std::io::prelude::*;
use crate::GameState;
#[derive(Event)]
pub struct LevelWritten;
struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}
impl Rect {
    fn new(x: usize, y: usize, w: usize, h: usize,) -> Self {
         Self { x, y, w, h }
    }

    fn center(&self) -> (usize, usize) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }
}

struct Leaf {
    rect: Rect,
    left: Option<Box<Leaf>>,
    right: Option<Box<Leaf>>,
    room: Option<Rect>,
}
impl Leaf {
    fn new(rect: Rect) -> Self {
        Self { rect, left: None, right: None, room: None }
    }

    // try to split leaf
    // function returns true if it worked
    fn split<R: Rng>(&mut self, rng: &mut R, min_leaf_size: usize, max_split_attempt: usize) -> bool {
        // Already split
        if self.left.is_some() || self.right.is_some() {
            return false;
        }

        let w = self.rect.w;
        let h = self.rect.h;

        // Too small to split
        if w <= min_leaf_size * 2 && h <= min_leaf_size * 2 {
            return false;
        }

        for _ in 0..max_split_attempt {

            let split_dir = rng.random_range(1..=2);
            if split_dir == 1 {
                // horizontal split
                if h <= min_leaf_size * 2 {
                    continue; // can't split, try again
                }
                let split = rng.random_range(min_leaf_size..=(h - min_leaf_size));
                let left_rect = Rect { x: self.rect.x, y: self.rect.y, w, h: split };
                let right_rect = Rect { x: self.rect.x, y: self.rect.y + split, w, h: h - split };
                self.left = Some(Box::new(Leaf::new(left_rect)));
                self.right = Some(Box::new(Leaf::new(right_rect)));
                return true;
            } else {
                // vertical split
                if w <= min_leaf_size * 2 {
                    continue; // can't split, try again
                }
                let split = rng.random_range(min_leaf_size..=(w - min_leaf_size));
                let left_rect = Rect { x: self.rect.x, y: self.rect.y, w: split, h };
                let right_rect = Rect { x: self.rect.x + split, y: self.rect.y, w: w - split, h };
                self.left = Some(Box::new(Leaf::new(left_rect)));
                self.right = Some(Box::new(Leaf::new(right_rect)));
                return true;
            }
        }

        false // failed to split after max attempts
    }

    // collect leaves with no children
    fn collect_leaves(&mut self, out: &mut Vec<&mut Leaf>) {
        let mut stack: Vec<*mut Leaf> = vec![self as *mut Leaf];
        
        while let Some(ptr) = stack.pop() {
            let leaf: &mut Leaf = unsafe { &mut *ptr };

            if let Some(left) = leaf.left.as_mut() {
                stack.push(&mut **left as *mut Leaf);
            }
            if let Some(right) = leaf.right.as_mut() {
                stack.push(&mut **right as *mut Leaf);
            }

            if leaf.left.is_none() && leaf.right.is_none() {
                out.push(leaf);
            }
        }
    }



    // create a random room inside this leaf's rect
    fn create_room<R: Rng>(&mut self, rng: &mut R, min_room_size: usize) {
        let max_w = self.rect.w.saturating_sub(2);
        let max_h = self.rect.h.saturating_sub(2);
        if max_w < min_room_size || max_h < min_room_size {
            return;
        }

        let room_w = rng.random_range(min_room_size..=max_w);
        let room_h = rng.random_range(min_room_size..=max_h);
        let room_x = rng.random_range(self.rect.x..=(self.rect.x + self.rect.w - room_w));
        let room_y = rng.random_range(self.rect.y..=(self.rect.y + self.rect.h - room_h));

        self.room = Some(Rect { x: room_x, y: room_y, w: room_w, h: room_h });
    }
}



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

pub fn build_full_level(
    rooms: Res<RoomRes>,
) {
    const MAP_W: usize = 300;
    const MAP_H: usize = 300;
    const MIN_LEAF_SIZE: usize = 50;
    const MIN_ROOM_SIZE: usize = 20;
    const SEED: u64 = 0;

    // full map of '.'
    let mut map: Vec<Vec<char>> = vec![vec!['.'; MAP_W]; MAP_H];

    // Empty map now created add starting room
    bsp_generate_level(&mut map, &rooms, MIN_LEAF_SIZE, MIN_ROOM_SIZE, SEED);

    generate_walls(&mut map);


    let f = File::create("assets/rooms/level.txt").expect("Couldn't create output file");
    let mut writer = BufWriter::new(f);

    for row in map {
        let line: String = row.into_iter().collect();
        writeln!(writer, "{line}").expect("Failed to write map row");
    }    
}

// - `map`: mutable 2D vector representing the map tiles.
// - `min_leaf_size`: smallest width or height a leaf can be before it stops splitting.
// - `min_room_size`: smallest allowed room dimension.
// - `rng_seed`: optional seed for reproducibility.
// 
// # Returns
// - A vector of all generated room rectangles.

// struct Leaf {
//     rect: Rect,
//     left: Option<Box<Leaf>>,
//     right: Option<Box<Leaf>>,
//     room: Option<Rect>,
// }

fn bsp_generate_level(
    map: &mut Vec<Vec<char>>,
    rooms: &RoomRes,
    min_leaf_size: usize,
    min_room_size: usize,
    seed: u64,
) -> Vec<Rect> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    // initialize root leaf
    let map_w = map[0].len();
    let map_h = map.len();
    let mut root = Leaf::new(Rect::new(0,0, map_w, map_h));

    let max_split_attempts = 100;
    
    split_leaf_recursive(&mut root, &mut rng, min_leaf_size, min_room_size, max_split_attempts);

    let mut terminals = Vec::new();
    root.collect_leaves(&mut terminals);

    // create rooms inside each leaf
    for terminal in terminals{
        terminal.create_room(&mut rng, min_room_size);
    // if let Some(room_rect) = &terminal.room {
    //     let room_num = rng.random_range(1..=6);
    //     let room_layout = match room_num {
    //         1 => &rooms.room1,
    //         2 => &rooms.room2,
    //         3 => &rooms.room3,
    //         4 => &rooms.room4,
    //         5 => &rooms.room5,
    //         6 => &rooms.room6,
    //         _ => &rooms.room1,
    //     };

    //     write_room(map, room_layout, room_rect.x, room_rect.y);
    // }
        if let Some(room_rect) = &terminal.room {
            let temp_w = rng.random_range(min_room_size..=room_rect.w);
            let temp_h = rng.random_range(min_room_size..=room_rect.h);

            let temp_x = rng.random_range(room_rect.x..=(room_rect.x + room_rect.w - temp_w));
            let temp_y = rng.random_range(room_rect.y..=(room_rect.y + room_rect.h - temp_h));

            for y in temp_y..(temp_y + temp_h) {
                for x in temp_x..(temp_x + temp_w) {
                    map[y][x] = '#';
                }
            }
        }

    }

    let errorfix = Vec::new();
    return errorfix;
}

fn split_leaf_recursive<R: Rng>(
    leaf: &mut Leaf,
    rng: &mut R,
    min_leaf_size: usize,
    min_room_size: usize,
    max_split_attempts: usize,
) {
    if leaf.split(rng, min_leaf_size, max_split_attempts) {
        if let Some(left) = leaf.left.as_mut() {
            split_leaf_recursive(left, rng, min_leaf_size, min_room_size, max_split_attempts);
        }
        if let Some(right) = leaf.right.as_mut() {
            split_leaf_recursive(right, rng, min_leaf_size, min_room_size, max_split_attempts);
        }
    } else {
        leaf.create_room(rng, min_room_size);
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
        let mut trng = rand::rng();
        floors.shuffle(&mut trng);
    }

    floors.into_iter().take(max_tables).collect()
}

// turns empty space . into wall W if it touches floor #
pub fn generate_walls(map: &mut Vec<Vec<char>>) {
    let rows = map.len();
    if rows == 0 {
        return;
    }
    let cols = map[0].len();

    let mut updated_map = map.clone();

    let neighbor_offsets: [(isize, isize); 8] = [
        (-1, -1), (0, -1), (1, -1),
        (-1,  0),          (1,  0),
        (-1,  1), (0,  1), (1,  1),
    ];

    for y in 0..rows {
        for x in 0..cols {
            if map[y][x] != '.' {
                continue;
            }

            let mut has_floor_neighbor = false;
            for (dx, dy) in neighbor_offsets.iter() {
                let nx = x as isize + dx;
                let ny = y as isize + dy;

                if nx < 0 || ny < 0 || nx >= cols as isize || ny >= rows as isize {
                    continue;
                }

                if map[ny as usize][nx as usize] == '#' {
                    has_floor_neighbor = true;
                    break;
                }
            }

            if has_floor_neighbor {
                updated_map[y][x] = 'W';
            }
        }
    }

    *map = updated_map;
}