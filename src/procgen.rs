use crate::room::*;
use crate::{GameState, TILE_SIZE};
use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng, random_range};
use std::cell::RefCell;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::io::{BufWriter, Write};
use std::rc::Rc;

#[derive(Event)]
pub struct LevelWritten;

#[derive(Clone, Copy)]
struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}
impl Rect {
    fn new(x: usize, y: usize, w: usize, h: usize) -> Self {
        Self { x, y, w, h }
    }
    fn center(&self) -> (usize, usize) {
        (self.x + (self.w / 2), self.y + (self.h / 2))
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum ProcgenSet {
    LoadRooms,
    BuildFullLevel,
}

type LeafRef = Rc<RefCell<Leaf>>;

#[derive(Resource)]
pub struct WindowConfig {
    // Fraction of eligible wall tiles to convert to windows (0.0–1.0)
    pub density: f32,
    // Hard minimum number of windows per room (if enough spots exist)
    pub min_per_room: usize,
    // Hard maximum number of windows per room
    pub max_per_room: usize,
    // distance around doors where we *won’t* place windows
    pub avoid_doors_radius: usize,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            density: 0.80,// 80% of eligible wall segments so we can tune how many windows appear
            min_per_room: 10,
            max_per_room: 5000,
            avoid_doors_radius: 2,
        }
    }
}

struct Leaf {
    rect: Rect,
    left: Option<LeafRef>,
    right: Option<LeafRef>,
    room: Option<Rect>,
}

impl Leaf {
    fn new(rect: Rect) -> LeafRef {
        Rc::new(RefCell::new(Self {
            rect,
            left: None,
            right: None,
            room: None,
        }))
    }

    // returns true if split occured
    fn split<R: Rng>(
        &mut self,
        rng: &mut R,
        min_leaf_size: usize,
        max_split_attempt: usize,
    ) -> bool {
        // return if it's already been split
        if self.left.is_some() || self.right.is_some() {
            return false;
        }

        // return if it's too small to split
        let w = self.rect.w;
        let h = self.rect.h;
        if w <= min_leaf_size * 2 && h <= min_leaf_size * 2 {
            return false;
        }

        // try to split it 'max_split_attemt' times
        for _ in 0..max_split_attempt {
            let split_dir = rng.random_range(0..=1);
            if split_dir == 0 && h > min_leaf_size * 2 {
                let split = rng.random_range(min_leaf_size..=(h - min_leaf_size));
                self.left = Some(Leaf::new(Rect {
                    x: self.rect.x,
                    y: self.rect.y,
                    w,
                    h: split,
                }));
                self.right = Some(Leaf::new(Rect {
                    x: self.rect.x,
                    y: self.rect.y + split,
                    w,
                    h: h - split,
                }));
                return true;
            } else if split_dir == 1 && w > min_leaf_size * 2 {
                let split = rng.random_range(min_leaf_size..=(w - min_leaf_size));
                self.left = Some(Leaf::new(Rect {
                    x: self.rect.x,
                    y: self.rect.y,
                    w: split,
                    h,
                }));
                self.right = Some(Leaf::new(Rect {
                    x: self.rect.x + split,
                    y: self.rect.y,
                    w: w - split,
                    h,
                }));
                return true;
            }
        }
        false
    }

    fn create_random_room<R: Rng>(&mut self, rng: &mut R, min_room_size: usize) {
        // rooms dont take up full rectangle of space in leaf
        let max_w = self.rect.w - 2;
        let max_h = self.rect.h - 2;

        // this should never occur due to splitting logic
        if max_w < min_room_size || max_h < min_room_size {
            self.room = None;
            return;
        }

        let room_w = rng.random_range(min_room_size..=max_w);
        let room_h = rng.random_range(min_room_size..=max_h);
        let room_x = rng.random_range(self.rect.x..=self.rect.x + self.rect.w - room_w);
        let room_y = rng.random_range(self.rect.y..=self.rect.y + self.rect.h - room_h);
        self.room = Some(Rect {
            x: room_x,
            y: room_y,
            w: room_w,
            h: room_h,
        });
    }
}

pub type TablePositions = HashSet<(usize, usize)>;

// layout of each room
pub struct RoomLayout {
    pub layout: Vec<String>,
    pub width: f32,
    pub height: f32,
}

impl RoomLayout {
    fn new() -> Self {
        Self {
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
    pub room1: RoomLayout,
    pub room2: RoomLayout,
    pub room3: RoomLayout,
    pub room4: RoomLayout,
    pub room5: RoomLayout,
    pub room6: RoomLayout,
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
            // label the room-loading system
            .add_systems(
                OnEnter(GameState::Loading),
                load_rooms.in_set(ProcgenSet::LoadRooms),
            )
            // label the BSP/full-level build and order it after load-rooms
            .add_systems(
                OnEnter(GameState::Loading),
                build_full_level
                    .in_set(ProcgenSet::BuildFullLevel)
                    .after(ProcgenSet::LoadRooms),
            );
            app.insert_resource(WindowConfig {
                density: 0.25,
                min_per_room: 10,
                max_per_room: 5000,
                avoid_doors_radius: 0,
        });
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
    window_cfg: Res<WindowConfig>,
) {
    // +40 and +20 are padding
    const MAP_W: usize = 400 + 40;
    const MAP_H: usize = 400 + 20;
    const MIN_LEAF_SIZE: usize = 100;
    const MIN_ROOM_SIZE: usize = 40;
    let seed: u64 = 140;//random_range(0..=10000000);

    // full map of '.'
    let mut map: Vec<Vec<char>> = vec![vec!['.'; MAP_W]; MAP_H];

    // empty map now created add rooms
    bsp_generate_level(
        &mut map,
        &rooms,
        MIN_LEAF_SIZE,
        MIN_ROOM_SIZE,
        seed,
        &mut room_vec,
    );
    info!("Finished BSP generation.");

    generate_walls(&mut map);
    info!("Finished wall generation.");

    let mut rng = StdRng::seed_from_u64(seed);
    place_windows(&mut map, &room_vec, &window_cfg, &mut rng);
    info!("Finished placing windows.");

    place_doors(&mut map, &room_vec);
    info!("Finished placing doors.");

    let window_count = map.iter()
    .flat_map(|row| row.iter())
    .filter(|&&c| c == 'G')
    .count();
    info!("Placed {} windows in this level.", window_count);


    let f = File::create("assets/rooms/level.txt").expect("Couldn't create output file");
    let mut writer = BufWriter::new(f);

    for row in map {
        let line: String = row.into_iter().collect();
        writeln!(writer, "{line}").expect("Failed to write map row");
    }
    info!("Finished writing to file.");
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
    let mut rng = StdRng::seed_from_u64(seed);
    let map_w = map[0].len() - 40;
    let map_h = map.len() - 20;
    let root = Leaf::new(Rect::new(20, 10, map_w, map_h));
    let max_split_attempts = 5;

    let mut terminals = Vec::new();
    split_leaf_recursive(
        &root,
        &mut rng,
        min_leaf_size,
        min_room_size,
        max_split_attempts,
        &mut terminals,
    );

    // place rooms inside each terminal leaf
    for terminal in terminals.iter() {
        let mut leaf = terminal.borrow_mut();
        if let Some(room_rect) = &leaf.room {
            let choice = rng.random_range(1..=12);
            if choice <= 6 {
                // preset room from set of 6
                let preset_room: &RoomLayout = match choice {
                    1 => rooms.room(1),
                    2 => rooms.room(2),
                    3 => rooms.room(3),
                    4 => rooms.room(4),
                    5 => rooms.room(5),
                    6 => rooms.room(6),
                    _ => unreachable!(),
                };
                let top_left_x = room_rect.x + (room_rect.w.saturating_sub(preset_room.layout[0].len())) / 2;
                let top_left_y = room_rect.y + (room_rect.h.saturating_sub(preset_room.layout.len())) / 2;
                write_room(map, preset_room, top_left_x, top_left_y, room_vec);
                leaf.room = Some(Rect { x: top_left_x, y: top_left_y, w: preset_room.layout[0].len(), h: preset_room.layout.len() });
            } else {
                // create a random rectangle inside leaf
                let room_w = rng.random_range(min_room_size..=leaf.rect.w - 2);
                let room_h = rng.random_range(min_room_size..=leaf.rect.h - 2);
                let room_x = rng.random_range(leaf.rect.x..=leaf.rect.x + leaf.rect.w - room_w);
                let room_y = rng.random_range(leaf.rect.y..=leaf.rect.y + leaf.rect.h - room_h);

                // create a "RoomLayout" for this random room
                let mut random_layout = vec![String::new(); room_h];
                for y in 0..room_h {
                    random_layout[y] = "#".repeat(room_w);
                }
                let random_room = RoomLayout {
                    layout: random_layout,
                    width: room_w as f32,
                    height: room_h as f32,
                };

                // write the random room into the map using the same function as presets
                write_room(map, &random_room, room_x, room_y, room_vec);

                // update the leaf's room rect (needed for hallway connections)
                leaf.room = Some(Rect { x: room_x, y: room_y, w: room_w, h: room_h });

            }
        }
    }

    // connect rooms with hallways
    connect_terminals(&terminals, map);
}

fn split_leaf_recursive<R: Rng>(
    leaf: &LeafRef,
    rng: &mut R,
    min_leaf_size: usize,
    min_room_size: usize,
    max_split_attempts: usize,
    terminals: &mut Vec<LeafRef>,
) {
    let mut leaf_mut = leaf.borrow_mut();
    if leaf_mut.split(rng, min_leaf_size, max_split_attempts) {
        // release borrow before recursing
        drop(leaf_mut);
        if let Some(left) = &leaf.borrow().left {
            // split left leaf
            split_leaf_recursive(
                left,
                rng,
                min_leaf_size,
                min_room_size,
                max_split_attempts,
                terminals,
            );
        }
        if let Some(right) = &leaf.borrow().right {
            // split right leaf
            split_leaf_recursive(
                right,
                rng,
                min_leaf_size,
                min_room_size,
                max_split_attempts,
                terminals,
            );
        }
    } else {
        leaf_mut.create_random_room(rng, min_room_size);
        terminals.push(Rc::clone(leaf));
    }
}

fn connect_terminals(
    terminals: &[LeafRef],
    map: &mut Vec<Vec<char>>,
) {
    let mut rooms: Vec<Rect> = Vec::new();

    for leaf in terminals {
        if let Some(room) = leaf.borrow().room.clone() {
            rooms.push(room);
        }
    }

    rooms.sort_by_key(|r| r.center());

    for i in 0..rooms.len().saturating_sub(1) {
        draw_hallway(&rooms[i], &rooms[i + 1], map);
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

fn draw_hallway(
    start: &Rect,
    end: &Rect,
    map: &mut Vec<Vec<char>>
) {
    let (x1, y1) = start.center();
    let (x2, y2) = end.center();
    let thickness = 5;
    let half = thickness as isize / 2;

    let (x1, y1, x2, y2) = (x1 as isize, y1 as isize, x2 as isize, y2 as isize);

    // draw a filled rectangle from (x_min,y_min) to (x_max,y_max)
    let mut draw_rect = |x_min: isize, y_min: isize, x_max: isize, y_max: isize| {
        for y in y_min..=y_max {
            for x in x_min..=x_max {
                if y >= 0 && x >= 0 &&
                   y < map.len() as isize &&
                   x < map[0].len() as isize {
                    map[y as usize][x as usize] = '#';
                }
            }
        }
    };

    if rand::random() {
        // horizontal first
        draw_rect(x1.min(x2), y1 - half, x1.max(x2), y1 + half);
        draw_rect(x2 - half, y1.min(y2), x2 + half, y1.max(y2));

        // fill corner
        draw_rect(x2 - half, y1 - half, x2 + half, y1 + half);

    } else {
        // vertical first
        draw_rect(x1 - half, y1.min(y2), x1 + half, y1.max(y2));
        draw_rect(x1.min(x2), y2 - half, x1.max(x2), y2 + half);

        // fill corner
        draw_rect(x1 - half, y2 - half, x1 + half, y2 + half);
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

    let tile_top_xy = Vec2::new(top_left_x as f32, top_left_y as f32);
    let tile_bot_xy = Vec2::new((top_left_x as f32+room.width), (top_left_y as f32+room.height));

    create_room(top_left_xy, bot_right_xy, tile_top_xy, tile_bot_xy, room_vec, room.layout.clone());

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
    if rows == 0 {
        return TablePositions::new();
    }
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
    let cols = map[0].len();
    let neighbor_offsets: [(isize, isize); 8] = [
        (-1, -1),   (0, -1),    (1, -1),
        (-1, 0),                (1, 0),
        (-1, 1),    (0, 1),     (1, 1),
    ];
    let mut walls_to_add = Vec::new();

    for y in 0..rows {
        for x in 0..cols {
            if map[y][x] != '.' {
                continue;
            }
            for (dx, dy) in neighbor_offsets.iter() {
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if nx < 0 || ny < 0 || nx >= cols as isize || ny >= rows as isize {
                    continue;
                }

                if map[ny as usize][nx as usize] == '#' {
                    walls_to_add.push((x, y));
                    break;
                }
            }
        }
    }

    // apply all walls at once
    for (x, y) in walls_to_add {
        map[y][x] = 'W';
    }
}

pub fn place_doors(map: &mut Vec<Vec<char>>, room_vec: &RoomVec) {
    let height = map.len();
    let width = map[0].len();

    for room in &room_vec.0 {
        let x1 = room.tile_top_left_corner.x as usize;
        let y1 = room.tile_top_left_corner.y as usize;
        let x2 = room.tile_bot_right_corner.x as usize;
        let y2 = room.tile_bot_right_corner.y as usize;

        // Top & bottom edges
        for x in x1..=x2 {
            if y1 < height && x < width && map[y1][x] == '#' {
                map[y1][x] = 'D';
            }
            if y2 < height && x < width && map[y2][x] == '#' {
                map[y2][x] = 'D';
            }
        }

        // Left & right edges
        for y in y1+1..y2 { // skip corners
            if y < height && x1 < width && map[y][x1] == '#' {
                map[y][x1] = 'D';
            }
            if y < height && x2 < width && map[y][x2] == '#' {
                map[y][x2] = 'D';
            }
        }
    }
}

pub fn place_windows<R: Rng>(
    map: &mut Vec<Vec<char>>,
    _room_vec: &RoomVec, // kept for signature compatibility, but unused now
    cfg: &WindowConfig,
    rng: &mut R,
) {
    let rows = map.len();
    if rows == 0 {
        return;
    }
    let cols = map[0].len();

    let mut candidates: Vec<(usize, usize)> = Vec::new();

    // Any hull wall: 'W' with at least one '#' neighbor and at least one '.' neighbor
    for y in 0..rows {
        for x in 0..cols {
            if map[y][x] != 'W' {
                continue;
            }

            let mut has_floor = false;
            let mut has_empty = false;

            // 4 connected neighbors (up, down, left, right)
            for (dx, dy) in [(1isize, 0isize), (-1, 0), (0, 1), (0, -1)] {
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if nx < 0 || ny < 0 || nx >= cols as isize || ny >= rows as isize {
                    continue;
                }

                match map[ny as usize][nx as usize] {
                    '#' => has_floor = true,
                    '.' => has_empty = true,
                    _ => {}
                }
            }

            if has_floor && has_empty {
                candidates.push((x, y));
            }
        }
    }

        // Filter out candidates too close to doors
    if cfg.avoid_doors_radius > 0 {
        // collect all doors
        let mut doors: Vec<(isize, isize)> = Vec::new();
        for y in 0..rows {
            for x in 0..cols {
                if map[y][x] == 'D' {
                    doors.push((x as isize, y as isize));
                }
            }
        }

        candidates.retain(|&(cx, cy)| {
            doors.iter().all(|&(dx, dy)| {
                let dist = (cx as isize - dx).abs() + (cy as isize - dy).abs();
                (dist as usize) > cfg.avoid_doors_radius
            })
        });
    }

    if candidates.is_empty() {
        info!("No candidate wall tiles for windows");
        return;
    }

    candidates.shuffle(rng);

    // Decide how many windows globally (we reuse the same fields)
    let total = candidates.len();
    let mut desired = ((total as f32) * cfg.density).round() as usize;
    if desired < cfg.min_per_room {
        desired = cfg.min_per_room;
    }
    if desired > cfg.max_per_room {
        desired = cfg.max_per_room;
    }
    desired = desired.min(total);

    info!(
        "Global windows: {} candidate hull walls, placing {} windows",
        total, desired
    );

    for &(x, y) in candidates.iter().take(desired) {
        if map[y][x] == 'W' {
            map[y][x] = 'G';
        }
    }
}



