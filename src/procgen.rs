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
    // Probability (0.0–1.0) that any given wall run gets a window burst
    pub density: f32,
    // Minimum number of consecutive windows in a burst
    pub min_burst: usize,
    // Maximum number of consecutive windows in a burst
    pub max_burst: usize,
    // Max fraction of a wall run that can become windows (prevents full-wall coverage)
    pub max_wall_fraction: f32,
    // Distance around doors where we *won’t* place windows
    pub avoid_doors_radius: usize,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            density: 0.6,
            min_burst: 2,
            max_burst: 4,
            max_wall_fraction: 0.5,
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
        let max_w = self.rect.w - 5;
        let max_h = self.rect.h - 5;

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
                density: 0.6,
                min_burst: 2,
                max_burst: 4,
                max_wall_fraction: 0.5,
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
    // +40 and +20 are edge padding kept clear for wall generation.
    // BSP area is MAP_W-40 × MAP_H-20.  MIN_LEAF_SIZE scaled proportionally
    // to the larger area keeps the expected room count the same as before.
    const MAP_W: usize = 250 + 40;   // was 200+40
    const MAP_H: usize = 250 + 20;   // was 200+20
    const MIN_LEAF_SIZE: usize = 44;  // was 35  (35 * 250/200 ≈ 44)
    const MIN_ROOM_SIZE: usize = 30;  // was 24  (slightly larger rooms)
    let seed: u64 = random_range(0..u64::MAX);

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
    debug!("Finished BSP generation.");

    // Add the player's boarding airlock room before wall generation so
    // generate_walls handles airlock borders automatically.
    add_airlock_room(&mut map, &mut room_vec);
    debug!("Finished airlock placement.");

    generate_walls(&mut map);
    debug!("Finished wall generation.");

    let mut rng = StdRng::seed_from_u64(seed);
    place_windows(&mut map, &room_vec, &window_cfg, &mut rng);
    debug!("Finished placing windows.");

    place_doors(&mut map, &room_vec);
    debug!("Finished placing doors.");

    let window_count = map.iter()
    .flat_map(|row| row.iter())
    .filter(|&&c| c == 'G')
    .count();
    debug!("Placed {} windows in this level.", window_count);


    let f = File::create("assets/rooms/level.txt").expect("Couldn't create output file");
    let mut writer = BufWriter::new(f);

    for row in map {
        let line: String = row.into_iter().collect();
        writeln!(writer, "{line}").expect("Failed to write map row");
    }
    debug!("Finished writing to file.");
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
    let max_split_attempts = 10;

    let mut terminals = Vec::new();
    split_leaf_recursive(
        &root,
        &mut rng,
        min_leaf_size,
        min_room_size,
        max_split_attempts,
        &mut terminals,
    );

    // Place a room inside each terminal leaf.  Preset rooms are used when they
    // fit inside the leaf's reserved area; otherwise a random rectangle fills it.
    for terminal in terminals.iter() {
        let mut leaf = terminal.borrow_mut();
        // Clone to own the rect — avoids borrow conflicts when we later write leaf.room.
        if leaf.room.is_none() { continue; }

        let choice = rng.random_range(1..=8);

        // Try a preset room first; fall back to random if it doesn't fit.
        let placed_preset = if choice <= 6 {
            let preset_room: &RoomLayout = match choice {
                1 => rooms.room(1),
                2 => rooms.room(2),
                3 => rooms.room(3),
                4 => rooms.room(4),
                5 => rooms.room(5),
                6 => rooms.room(6),
                _ => unreachable!(),
            };
            let preset_w = preset_room.layout[0].len();
            let preset_h = preset_room.layout.len();
            if preset_w + 2 <= leaf.rect.w && preset_h + 2 <= leaf.rect.h {
                let top_left_x = leaf.rect.x + (leaf.rect.w - preset_w) / 2;
                let top_left_y = leaf.rect.y + (leaf.rect.h - preset_h) / 2;
                write_room(map, preset_room, top_left_x, top_left_y, room_vec);
                leaf.room = Some(Rect { x: top_left_x, y: top_left_y, w: preset_w, h: preset_h });
                true
            } else {
                false
            }
        } else {
            false
        };

        if !placed_preset {
            // Random rectangle sized to fit inside the full leaf boundary.
            let room_w = rng.random_range(min_room_size / 2..=leaf.rect.w - 5);
            let room_h = rng.random_range(min_room_size / 2..=leaf.rect.h - 5);
            let room_x = rng.random_range(leaf.rect.x..=leaf.rect.x + leaf.rect.w - room_w);
            let room_y = rng.random_range(leaf.rect.y..=leaf.rect.y + leaf.rect.h - room_h);

            let mut random_layout = vec![String::new(); room_h];
            for y in 0..room_h {
                if y == 0 || y == room_h - 1 {
                    random_layout[y] = ".".repeat(room_w);
                } else {
                    random_layout[y] = "#".repeat(room_w);
                    random_layout[y].insert(0, '.');
                    random_layout[y].push_str(".");
                }
            }
            let random_room = RoomLayout {
                layout: random_layout,
                width: room_w as f32 + 2.0,
                height: room_h as f32,
            };

            write_room(map, &random_room, room_x - 1, room_y - 1, room_vec);
            leaf.room = Some(Rect { x: room_x, y: room_y, w: room_w, h: room_h });
        }
    }

    // connect rooms with hallways
    recursive_hallway(&root, map, &mut rng);

    // connect_terminals(&terminals, map);
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
        leaf_mut.create_random_room(rng, min_room_size/2);
        terminals.push(Rc::clone(leaf));
    }
}

// outdated way of doing hallways

// fn connect_terminals(
//     terminals: &[LeafRef],
//     map: &mut Vec<Vec<char>>,
// ) {
//     let mut rooms: Vec<Rect> = Vec::new();

//     for leaf in terminals {
//         if let Some(room) = leaf.borrow().room.clone() {
//             rooms.push(room);
//         }
//     }

//     rooms.sort_by_key(|r| r.center());

//     for i in 0..rooms.len().saturating_sub(1) {
//         draw_hallway(&rooms[i], &rooms[i + 1], map);
//     }
// }


// Finds the next room recursively
fn find_next_room(stay_right: bool, leaf_rc: &Rc<RefCell<Leaf>>) -> Option<Rect> {
    let leaf = leaf_rc.borrow();
    if let Some(room) = &leaf.room {
        return Some(room.clone());
    }

    if stay_right {
        if let Some(r) = &leaf.right {
            if let Some(room) = find_next_room(true, r) {
                return Some(room);
            }
        }
        if let Some(l) = &leaf.left {
            return find_next_room(true, l);
        }
    } else {
        if let Some(l) = &leaf.left {
            if let Some(room) = find_next_room(false, l) {
                return Some(room);
            }
        }
        if let Some(r) = &leaf.right {
            return find_next_room(false, r);
        }
    }

    None
}

// Recursive hallway creation
fn recursive_hallway<R: Rng>(
    leaf_rc: &Rc<RefCell<Leaf>>,
    map: &mut Vec<Vec<char>>,
    rng: &mut R,
) {
    // Recurse first
    {
        let leaf = leaf_rc.borrow();
        if let (Some(left_rc), Some(right_rc)) = (&leaf.left, &leaf.right) {
            recursive_hallway(left_rc, map, rng);
            recursive_hallway(right_rc, map, rng);
        }
    }

    // After recursion, find start/end rooms
    let leaf = leaf_rc.borrow();
    let start = leaf.left.as_ref().and_then(|l| find_next_room(false, l));
    let end = leaf.right.as_ref().and_then(|r| find_next_room(true, r));

    if let (Some(s), Some(e)) = (start, end) {
        draw_hallway(&s, &e, map);
    }
}


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
    // Keep 1-tile margin at each map edge so generate_walls can always place boundary walls
    let rows = map.len() as isize;
    let cols = map[0].len() as isize;
    let mut draw_rect = |x_min: isize, y_min: isize, x_max: isize, y_max: isize| {
        for y in y_min..=y_max {
            for x in x_min..=x_max {
                if y > 0 && x > 0 && y < rows - 1 && x < cols - 1 {
                    let tile = &mut map[y as usize][x as usize];
                    if *tile == '.' {
                        *tile = '#';
                    }
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
    let map_center_x = (map[0].len() / 2) as f32;
    let map_center_y = (map.len() / 2) as f32;

    let map_height = map.len();
    let map_width = if map_height > 0 { map[0].len() } else { 0 };

    let actual_top_left_x = (top_left_x as f32 - map_center_x) * TILE_SIZE;
    let actual_top_left_y = -(top_left_y as f32 - map_center_y) * TILE_SIZE;

    let actual_bot_right_x = actual_top_left_x + (room.width * TILE_SIZE);
    let actual_bot_right_y = actual_top_left_y - (room.height * TILE_SIZE);

    let bot_right_xy = Vec2::new(actual_bot_right_x, actual_bot_right_y);
    let top_left_xy = Vec2::new(actual_top_left_x, actual_top_left_y);

    let tile_top_xy = Vec2::new(top_left_x as f32, top_left_y as f32);
    let tile_bot_xy = Vec2::new(top_left_x as f32+room.width-1.0, top_left_y as f32+room.height-1.0);

    create_room(top_left_xy, bot_right_xy, tile_top_xy, tile_bot_xy, room_vec, room.layout.clone());

    for (row_idx, row_str) in room.layout.iter().enumerate() {
        let y = top_left_y + row_idx;
        // Keep the last row empty so generate_walls can always place a wall there
        if y == 0 || y >= map_height.saturating_sub(1) {
            continue;
        }

        for (col_idx, ch) in row_str.chars().enumerate() {
            let x = top_left_x + col_idx;
            // Keep the last column empty for the same reason
            if x == 0 || x >= map_width.saturating_sub(1) {
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

/// Generate table positions per room using geometric patterns (rows, clusters,
/// paired rows) instead of fully random placement.  Each non-airlock room gets
/// 1–2 independent table groups so the result looks intentionally furnished.
pub fn generate_shaped_tables(rooms: &RoomVec, grid: &[String], seed: Option<u64>) -> TablePositions {
    let mut out = TablePositions::new();
    let seed_val = seed.unwrap_or_else(|| random_range(0..u64::MAX));
    let mut rng = StdRng::seed_from_u64(seed_val);

    let grows = grid.len();
    let gcols = if grows > 0 { grid[0].len() } else { return out; };

    // Returns true if (x,y) is a floor tile in the full level grid.
    let is_floor = |x: usize, y: usize| -> bool {
        y < grows && x < gcols && grid[y].as_bytes().get(x).copied() == Some(b'#')
    };

    for room in &rooms.0 {
        if room.is_airlock { continue; }

        let rx1 = room.tile_top_left_corner.x as usize;
        let ry1 = room.tile_top_left_corner.y as usize;
        let rx2 = room.tile_bot_right_corner.x as usize;
        let ry2 = room.tile_bot_right_corner.y as usize;

        // Inset 2 tiles from each edge so tables don't sit against walls.
        let ix1 = rx1 + 2;
        let iy1 = ry1 + 2;
        let ix2 = rx2.saturating_sub(2);
        let iy2 = ry2.saturating_sub(2);

        // Room interior must be large enough to fit at least one pattern.
        if ix2 <= ix1 + 2 || iy2 <= iy1 + 2 { continue; }

        let num_groups = rng.random_range(1u32..=2);

        for _ in 0..num_groups {
            match rng.random_range(0u32..3) {
                // ── Pattern 0: single horizontal row of 3–5 tables ──────────────
                0 => {
                    let y = rng.random_range(iy1..=iy2);
                    let row_xs: Vec<usize> = (ix1..=ix2).filter(|&x| is_floor(x, y)).collect();
                    if row_xs.len() < 3 { continue; }
                    let count = rng.random_range(3usize..=row_xs.len().min(5));
                    let start = rng.random_range(0..=row_xs.len() - count);
                    for &x in &row_xs[start..start + count] {
                        out.insert((x, y));
                    }
                }

                // ── Pattern 1: 2×N or N×2 compact cluster (desk island) ─────────
                1 => {
                    let max_ax = if ix2 > ix1 + 1 { ix2 - 1 } else { continue };
                    let max_ay = if iy2 > iy1 { iy2 - 1 } else { continue };
                    let ax = rng.random_range(ix1..=max_ax);
                    let ay = rng.random_range(iy1..=max_ay);
                    let cols = rng.random_range(2usize..=3).min(ix2 - ax + 1);
                    let rows = rng.random_range(2usize..=3).min(iy2 - ay + 1);
                    let group: Vec<(usize, usize)> = (0..rows)
                        .flat_map(|dy| (0..cols).map(move |dx| (ax + dx, ay + dy)))
                        .filter(|&(x, y)| is_floor(x, y))
                        .collect();
                    if group.len() >= 3 {
                        for pos in group { out.insert(pos); }
                    }
                }

                // ── Pattern 2: two parallel rows (cafeteria / lab bench) ─────────
                _ => {
                    if iy2 < iy1 + 3 { continue; }
                    let y_a = rng.random_range(iy1..=iy2 - 2);
                    let y_b = y_a + 2; // one-tile aisle between the rows
                    let count = rng.random_range(2usize..=4);
                    for &y_row in &[y_a, y_b] {
                        let row_xs: Vec<usize> = (ix1..=ix2).filter(|&x| is_floor(x, y_row)).collect();
                        if row_xs.len() < 2 { continue; }
                        let actual = count.min(row_xs.len());
                        let start = rng.random_range(0..=row_xs.len() - actual);
                        for &x in &row_xs[start..start + actual] {
                            out.insert((x, y_row));
                        }
                    }
                }
            }
        }
    }

    out
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
            if map[y][x] != '.' && map[y][x] != ',' {
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

fn extract_consecutive_runs(sorted: &[usize]) -> Vec<Vec<usize>> {
    let mut runs: Vec<Vec<usize>> = Vec::new();
    if sorted.is_empty() {
        return runs;
    }
    let mut current = vec![sorted[0]];
    for &v in &sorted[1..] {
        if v == *current.last().unwrap() + 1 {
            current.push(v);
        } else {
            runs.push(current.clone());
            current = vec![v];
        }
    }
    runs.push(current);
    runs
}

/// Attach a small airlock room to the top of the map, connected via a corridor
/// to the nearest floor tile below it.  The player spawns here (via the 'S'
/// marker) and returns here at the end of the level to choose Leave / Continue.
/// The room is pre-marked `cleared = true` and `is_airlock = true` so the combat
/// system never triggers inside it.
pub fn add_airlock_room(map: &mut Vec<Vec<char>>, room_vec: &mut RoomVec) {
    const AIRLOCK_W: usize = 14;
    const AIRLOCK_H: usize = 7;

    let map_rows = map.len();
    let map_cols = map[0].len();
    let center_col = map_cols / 2;

    // Find the topmost floor tile near the horizontal center so the corridor
    // has a guaranteed connection to the main dungeon.
    let mut connect_col = center_col;
    let mut connect_row = map_rows / 2; // safe fallback
    'search: for row in 0..map_rows {
        for radius in 0..60usize {
            for &col in &[center_col.saturating_add(radius), center_col.saturating_sub(radius)] {
                if col < map_cols && map[row][col] == '#' {
                    connect_col = col;
                    connect_row = row;
                    break 'search;
                }
            }
        }
    }

    // Place the airlock so its bottom border row is just above the corridor start.
    // Leave at least 1 row gap so generate_walls can form a clean outer wall.
    let airlock_y = if connect_row >= AIRLOCK_H + 2 {
        connect_row - AIRLOCK_H - 2
    } else {
        1 // clamp to top of map (row 0 is always border)
    };
    let airlock_y = airlock_y.max(1);

    // Center the airlock horizontally over the connection column.
    let airlock_x = if connect_col >= AIRLOCK_W / 2 {
        (connect_col - AIRLOCK_W / 2).min(map_cols - AIRLOCK_W - 1)
    } else {
        1
    };

    let airlock_end_y = airlock_y + AIRLOCK_H; // exclusive bottom row

    // Spawn point: centre of the interior (will land the player inside on load).
    let spawn_col = airlock_x + AIRLOCK_W / 2;
    let spawn_row = airlock_y + AIRLOCK_H / 2;

    // Write the airlock interior and '.' borders (borders become 'W' in generate_walls).
    for row in airlock_y..airlock_end_y {
        for col in airlock_x..airlock_x + AIRLOCK_W {
            if row >= map_rows || col >= map_cols { continue; }
            let is_border = row == airlock_y || row == airlock_end_y - 1
                || col == airlock_x || col == airlock_x + AIRLOCK_W - 1;
            if !is_border {
                if row == spawn_row && col == spawn_col {
                    map[row][col] = 'S';
                } else {
                    map[row][col] = '#';
                }
            }
            // border tiles stay '.' so generate_walls turns them into 'W'
        }
    }

    // Open a 3-tile-wide passage through the bottom border so the corridor
    // can connect flush with the rest of the dungeon.
    let corridor_center = airlock_x + AIRLOCK_W / 2;
    for dc in 0..3usize {
        let col = corridor_center - 1 + dc;
        if col < map_cols {
            map[airlock_end_y - 1][col] = '#'; // open bottom border
        }
    }

    // Carve a 3-wide corridor from just below the airlock down to the first
    // floor tile we found earlier.
    for row in airlock_end_y..=connect_row {
        for dc in 0..3usize {
            let col = corridor_center - 1 + dc;
            if col < map_cols && row < map_rows && map[row][col] == '.' {
                map[row][col] = '#';
            }
        }
    }

    // Compute world and tile bounds (same formula used by write_room).
    let map_center_x = map_cols as f32 / 2.0;
    let map_center_y = map_rows as f32 / 2.0;

    let world_tlx = (airlock_x as f32 - map_center_x) * TILE_SIZE;
    let world_tly = -(airlock_y as f32 - map_center_y) * TILE_SIZE;
    let world_brx = world_tlx + AIRLOCK_W as f32 * TILE_SIZE;
    let world_bry = world_tly - AIRLOCK_H as f32 * TILE_SIZE;

    // Build layout snapshot (used by air-pressure system).
    let mut layout = Vec::with_capacity(AIRLOCK_H);
    for row in airlock_y..airlock_end_y {
        let mut line = String::with_capacity(AIRLOCK_W);
        for col in airlock_x..airlock_x + AIRLOCK_W {
            line.push(if row < map_rows && col < map_cols { map[row][col] } else { '.' });
        }
        layout.push(line);
    }

    use crate::room::create_room;
    create_room(
        Vec2::new(world_tlx, world_tly),
        Vec2::new(world_brx, world_bry),
        Vec2::new(airlock_x as f32, airlock_y as f32),
        Vec2::new((airlock_x + AIRLOCK_W - 1) as f32, (airlock_y + AIRLOCK_H - 1) as f32),
        room_vec,
        layout,
    );

    // Mark the airlock as pre-cleared so combat never triggers there.
    if let Some(room) = room_vec.0.last_mut() {
        room.cleared = true;
        room.is_airlock = true;
    }
}

pub fn place_windows<R: Rng>(
    map: &mut Vec<Vec<char>>,
    room_vec: &RoomVec,
    cfg: &WindowConfig,
    rng: &mut R,
) {
    let rows = map.len();
    if rows == 0 {
        return;
    }
    let cols = map[0].len();

    let mut candidates: Vec<(usize, usize)> = Vec::new();

    // Hull walls: 'W' with at least one '#' neighbor and at least one '.' neighbor
    for room in &room_vec.0 {
        let x1 = room.tile_top_left_corner.x as usize;
        let y1 = room.tile_top_left_corner.y as usize;
        let x2 = room.tile_bot_right_corner.x as usize;
        let y2 = room.tile_bot_right_corner.y as usize;

        for y in y1..=y2 {
            for x in x1..=x2 {
                if y >= rows {
                    return;
                }
                if map[y][x] != 'W' {
                    continue;
                }

                let mut has_floor = false;
                let mut has_empty = false;

                for (dx, dy) in [
                    (-1,-1), (-1, 0), (-1, 1),
                    ( 0,-1),          ( 0, 1),
                    ( 1,-1), ( 1, 0), ( 1, 1),
                ] {
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
    }

    // Filter out candidates too close to doors
    if cfg.avoid_doors_radius > 0 {
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

    use std::collections::HashMap;
    let candidate_set: HashSet<(usize, usize)> = candidates.iter().cloned().collect();

    // Group by row (horizontal runs) and by column (vertical runs)
    let mut by_row: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut by_col: HashMap<usize, Vec<usize>> = HashMap::new();
    for &(x, y) in &candidates {
        by_row.entry(y).or_default().push(x);
        by_col.entry(x).or_default().push(y);
    }
    for xs in by_row.values_mut() { xs.sort_unstable(); }
    for ys in by_col.values_mut() { ys.sort_unstable(); }

    let mut placed: HashSet<(usize, usize)> = HashSet::new();

    // Place bursts along horizontal runs
    for (&y, xs) in &by_row {
        for run in extract_consecutive_runs(xs) {
            if run.len() < cfg.min_burst { continue; }
            if rng.random::<f32>() > cfg.density { continue; }

            let max_in_run = ((run.len() as f32 * cfg.max_wall_fraction) as usize)
                .max(cfg.min_burst)
                .min(run.len());
            let burst_size = rng.random_range(cfg.min_burst..=cfg.max_burst.min(max_in_run));
            let max_start = run.len() - burst_size;
            let start = rng.random_range(0..=max_start);

            for i in start..start + burst_size {
                placed.insert((run[i], y));
            }
        }
    }

    // Place bursts along vertical runs (handles walls not covered horizontally)
    for (&x, ys) in &by_col {
        for run in extract_consecutive_runs(ys) {
            if run.len() < cfg.min_burst { continue; }
            if rng.random::<f32>() > cfg.density { continue; }

            let max_in_run = ((run.len() as f32 * cfg.max_wall_fraction) as usize)
                .max(cfg.min_burst)
                .min(run.len());
            let burst_size = rng.random_range(cfg.min_burst..=cfg.max_burst.min(max_in_run));
            let max_start = run.len() - burst_size;
            let start = rng.random_range(0..=max_start);

            for i in start..start + burst_size {
                placed.insert((x, run[i]));
            }
        }
    }

    let window_targets: Vec<(usize, usize)> = placed
        .into_iter()
        .filter(|p| candidate_set.contains(p))
        .collect();

    debug!(
        "Global windows: {} candidate hull walls, placing {} windows in bursts",
        candidates.len(),
        window_targets.len()
    );

    for (x, y) in window_targets {
        if map[y][x] == 'W' {
            map[y][x] = 'G';
        }
    }
}
