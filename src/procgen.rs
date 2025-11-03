use bevy::prelude::*;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::io::{BufWriter, Write};
use std::io::prelude::*;
use crate::map::TileRes;
use crate::{GameState, TILE_SIZE};
use crate::room::*;

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
        (self.x + (self.w / 2), self.y + (self.h / 2))
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
                    continue;
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
                    continue;
                }
                let split = rng.random_range(min_leaf_size..=(w - min_leaf_size));
                let left_rect = Rect { x: self.rect.x, y: self.rect.y, w: split, h };
                let right_rect = Rect { x: self.rect.x + split, y: self.rect.y, w: w - split, h };
                self.left = Some(Box::new(Leaf::new(left_rect)));
                self.right = Some(Box::new(Leaf::new(right_rect)));
                return true;
            }
        }

        false
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

// layout of each room
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

// contains all the different rooms
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

impl RoomRes {
    // immutable read
    fn room(&self, n: i8) -> &RoomLayout {
        match n {
            1 => &self.room1,
            2 => &self.room2,
            3 => &self.room3,
            4 => &self.room4,
            5 => &self.room5,
            6 => &self.room6,
            _ => panic!("Room doesn't exist"),
        }
    }

    // mutable access
    fn room_mut(&mut self, n: i8) -> &mut RoomLayout {
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



pub fn load_rooms(mut commands: Commands) {
    // update numroom here to increase or decrease the number of rooms
    let mut rooms: RoomRes = RoomRes {
        numroom: 6,
        room1: RoomLayout::new(),
        room2: RoomLayout::new(),
        room3: RoomLayout::new(),
        room4: RoomLayout::new(),
        room5: RoomLayout::new(),
        room6: RoomLayout::new(),
    };

    commands.insert_resource(RoomVec(Vec::new()));

    for n in 1..=rooms.numroom {
        // create the filename for each room
        let filename = format!("assets/rooms/room{}.txt", n);

        // read the file for that room
        let f = File::open(filename).expect("file doesn't exist");
        let reader = BufReader::new(f);

        // collect lines into a Vec<String>
        let lines: Vec<String> = reader
            .lines()
            .map(|line_result| line_result.expect("Failed to read line"))
            .collect();

        // now borrow the room mutably and set its layout
        let room = rooms.room_mut(n);
        room.layout = lines;
        
        room.height = room.layout.len() as f32;
        room.width = room.layout[0].len() as f32;
    }

    // insert the rooms resource
    commands.insert_resource(rooms);
}

pub fn build_full_level(
    rooms: Res<RoomRes>,
    mut room_vec: ResMut<RoomVec>,
) {
    const MAP_W: usize = 500;
    const MAP_H: usize = 500;
    const MIN_LEAF_SIZE: usize = 100;
    const MIN_ROOM_SIZE: usize = 40;
    const SEED: u64 = 122;

    // full map of '.'
    let mut map: Vec<Vec<char>> = vec![vec!['.'; MAP_W]; MAP_H];

    // empty map now created add rooms
    bsp_generate_level(&mut map, &rooms, MIN_LEAF_SIZE, MIN_ROOM_SIZE, SEED, &mut room_vec);
    
    generate_walls(&mut map);

    let f = File::create("assets/rooms/level.txt").expect("Couldn't create output file");
    let mut writer = BufWriter::new(f);

    for row in map {
        let line: String = row.into_iter().collect();
        writeln!(writer, "{line}").expect("Failed to write map row");
    }    
}

// map: mutable 2D vector representing the map tiles.
// min_leaf_size: smallest width or height a leaf can be before it stops splitting.
// min_room_size: smallest allowed room dimension.
// rng_seed: seed for reproducibility.


fn bsp_generate_level(
    map: &mut Vec<Vec<char>>,
    rooms: &RoomRes,
    min_leaf_size: usize,
    min_room_size: usize,
    seed: u64,
    room_vec: &mut RoomVec,
) {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let map_w = map[0].len();
    let map_h = map.len();
    let mut root = Leaf::new(Rect::new(0,0, map_w, map_h));

    let max_split_attempts = 10;
    
    split_leaf_recursive(&mut root, &mut rng, min_leaf_size, min_room_size, max_split_attempts);

    let mut terminals = Vec::new();
    root.collect_leaves(&mut terminals);

    // create rooms inside each terminal
    for terminal in terminals.iter_mut() {
        terminal.create_room(&mut rng, min_room_size);

        if let Some(room_rect) = &terminal.room {
            let choice = rng.random_range(1..=12);
            

            if choice <= 6 {
                // Use a preset room
                let preset_room: &RoomLayout = match choice {
                    1 => rooms.room(1),
                    2 => rooms.room(2),
                    3 => rooms.room(3),
                    4 => rooms.room(4),
                    5 => rooms.room(5),
                    6 => rooms.room(6),
                    _ => unreachable!(),
                };
                // let preset_room: &mut RoomLayout = rooms.room(choice as i8);

                // center the preset room inside the leaf
                let top_left_x = room_rect.x + (room_rect.w.saturating_sub(preset_room.layout[0].len())) / 2;
                let top_left_y = room_rect.y + (room_rect.h.saturating_sub(preset_room.layout.len())) / 2;

                write_room(map, preset_room, top_left_x, top_left_y, room_vec);
            } else {
                // random rectangle
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
    }

    // now its hallway time booyah

    let terminal_refs: Vec<&Leaf> = terminals.iter().map(|t| &**t).collect();
    connect_terminals(&terminal_refs, map);
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

fn connect_terminals(terminals: &[&Leaf], map: &mut Vec<Vec<char>>) {
    let mut rooms: Vec<&Rect> = terminals
        .iter()
        .filter_map(|leaf| leaf.room.as_ref())
        .collect();

    // Sort rooms by their center x, then y for consistent ordering
    rooms.sort_by_key(|r| r.center());

    // Connect each room to the next one
    for i in 0..rooms.len().saturating_sub(1) {
        let start = rooms[i];
        let end = rooms[i + 1];
        draw_hallway(start, end, map);
    }
}


// just might have to come back to these ones

// fn recursive_hallway<R: Rng>(
//     leaf: &mut Leaf,
//     map: &mut Vec<Vec<char>>,
//     rng: &mut R
// ) {
//     let mut start: Option<&Rect> = None;
//     let mut end: Option<&Rect> = None;
//     let stay_right: bool;
//     if let (Some(left), Some(right)) = (leaf.left.as_mut(), leaf.right.as_mut()) {
//         recursive_hallway(left, map, rng);
//         recursive_hallway(right, map, rng);
//     }
//     if let Some(room) = &(leaf.left.as_ref().unwrap()).room {
//         start = Some(room);
//         if let Some(room) = &(leaf.right.as_ref().unwrap()).room {
//             end = Some(room);
//         } else {
//             stay_right = false;
//             if let Some(room) = find_next_room(stay_right, leaf) {
//                 end = Some(room);
//             }
//         }
//     } else {
//         if let Some(room) = &(leaf.right.as_ref().unwrap()).room {
//             end = Some(room);
//             stay_right = true;
//             if let Some(room) = find_next_room(stay_right, leaf) {
//                 start = Some(room);
//             }
//         }
//     }
//     if let (Some(s), Some(e)) = (start, end) {
//         draw_hallway(s, e, map);
//     }
// }

// fn find_next_room<'a>(stay_right: bool, leaf: &'a Leaf) -> Option<&'a Rect> {
//     if let Some(room) = &leaf.room {
//         return Some(room);
//     }
//     if stay_right {
//         if let Some(r) = &leaf.right {
//             if let Some(room) = find_next_room(true, r) {
//                 return Some(room);
//             }
//         }
//         if let Some(l) = &leaf.left {
//             return find_next_room(true, l);
//         }
//     } else {
//         if let Some(l) = &leaf.left {
//             if let Some(room) = find_next_room(false, l) {
//                 return Some(room);
//             }
//         }
//         if let Some(r) = &leaf.right {
//             return find_next_room(false, r);
//         }
//     }
//     None
// }

fn draw_hallway(start: &Rect, end: &Rect, map: &mut Vec<Vec<char>>) {
    let (x1, y1) = start.center();
    let (x2, y2) = end.center();
    let thickness = 5; // 5 tiles thick

    if rand::random() {
        // horizontal then vertical
        for x in x1.min(x2)..=x1.max(x2) {
            for dy in 0..thickness {
                let y = (y1 + dy).min(map.len() - 1);
                map[y][x] = '#';
            }
        }
        for y in y1.min(y2)..=y1.max(y2) {
            for dx in 0..thickness {
                let x = (x2 + dx).min(map[0].len() - 1);
                map[y][x] = '#';
            }
        }
    } else {
        // vertical then horizontal
        for y in y1.min(y2)..=y1.max(y2) {
            for dx in 0..thickness {
                let x = (x1 + dx).min(map[0].len() - 1);
                map[y][x] = '#';
            }
        }
        for x in x1.min(x2)..=x1.max(x2) {
            for dy in 0..thickness {
                let y = (y2 + dy).min(map.len() - 1);
                map[y][x] = '#';
            }
        }
    }
}



// writes a room into an existing map at a given top-left coordinate
pub fn write_room(
    map: &mut Vec<Vec<char>>,
    room: &RoomLayout,
    top_left_x: usize,
    top_left_y: usize,
    room_vec: &mut RoomVec,
) {
    let map_height = map.len();
    let map_width = if map_height > 0 { map[0].len() } else { 0 };

    let actual_top_left_x = (top_left_x as f32 - 250.0) * TILE_SIZE;
    let actual_top_left_y = -(top_left_y as f32 - 250.0) * TILE_SIZE;

    let actual_bot_right_x = actual_top_left_x + (room.width * TILE_SIZE);
    let actual_bot_right_y = actual_top_left_y - (room.height * TILE_SIZE);

    let bot_right_xy = Vec2::new(actual_bot_right_x, actual_bot_right_y);
    let top_left_xy = Vec2::new(actual_top_left_x, actual_top_left_y);

    create_room(0, top_left_xy, bot_right_xy, room_vec);


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

// generates table positions from a grid representation of the room.
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