use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use std::fs;


use crate::room::Room;
use crate::room::RoomVec;

//Division of the room in small grids for the airflow measurement there
pub const GRID_WIDTH: usize = 79;
pub const GRID_HEIGHT: usize = 39;

//responsible for the thickness of the air
const RELAXATION_TIME: f32 = 0.55;
//how long it takes particles to get back to the original state after the serious destrurbance
const OMEGA: f32 = 1.0 / RELAXATION_TIME;

// breach values
// pressure of the vaccume of space
#[allow(dead_code)]
const VACUUM_PRESSURE: f32 = 0.001;
// the fraction of air that gets transfered from a neighbor's cell into the breach cell
#[allow(dead_code)]
const TRANSFER_FRACTION: f32 = 0.02;
// strength of pushing neighbor cell's velocity towards the breach cell
#[allow(dead_code)]
const PUSH_STRENGTH: f32 = 0.15;
// saftey for the density value
#[allow(dead_code)]
const MIN_RHO: f32 = 1e-6;
// how much the neighbor cell's distribution is replaced with equilibrum
#[allow(dead_code)]
const BLEND: f32 = 0.4;

//D2Q9 directions
const C_X: [f32; 9] = [0.0, 1.0, 0.0, -1.0, 0.0, 1.0, -1.0, -1.0, 1.0];
const C_Y: [f32; 9] = [0.0, 0.0, 1.0, 0.0, -1.0, 1.0, 1.0, -1.0, -1.0];

//D2Q9 opposite directions for bounce back
const OPPOSITE_DIR: [usize; 9] = [0, 3, 4, 1, 2, 7, 8, 5, 6];

// D2Q9 weights
const WEIGHTS: [f32; 9] = [
    4.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
];

//2d coordinates are transfered into a 1d array
#[derive(Component)]
pub struct FluidGrid {
    pub width: usize,
    pub height: usize,
    pub distribution: Vec<[f32; 9]>,
    pub scratch: Vec<[f32; 9]>,
    pub obstacles: Vec<bool>,
    pub breaches: Vec<(usize, usize)>, //location of the window, where the air is leaking
}

#[derive(Component)]
pub struct PulledByFluid {
    pub mass: f32, //this is like the mass of the object. coeff by how much the object is being pulled towards the window
}

pub struct FluidSimPlugin;

impl Plugin for FluidSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_fluid_grid)
            .add_systems(
                Update, 
                (
                    collision_step,
                    streaming_step,
                    apply_breach_forces,
                    pull_objects_toward_breaches,
                )
                    .chain()
                    .run_if(in_state(crate::GameState::Playing))
            );
    }
}

impl FluidGrid {
    pub fn new(width: usize, height: usize) -> Self {
        let size = width * height;
        Self {
            width,
            height,
            distribution: vec![[0.0; 9]; size],
            scratch: vec![[0.0; 9]; size],
            obstacles: vec![false; size],
            breaches: Vec::new(),
        }
    }

    pub fn set_obstacles_from_map(&mut self, map_content: &str) {
        self.obstacles = vec![false; self.width * self.height];

        // loop for y coordinate
        for (y, line) in map_content.lines().enumerate() {
            // stops if the map file is differnt from grid height
            if y >= self.height {
                break;
            }
            
            // loop for x coordinate
            for (x, char) in line.chars().enumerate() {
                // stop if map file is wider than grid width
                if x >= self.width {
                    break;
                }

                let idx = self.get_index(x, y);
                if char == 'W' {
                    self.obstacles[idx] = true; // adds wall to obstacles
                }
            }
        }
    }

    pub fn initialize_with_perlin(&mut self, seed: u32) {
        // The `noise` crate’s Perlin may not accept a seed on some versions.
        // If yours doesn't, use Perlin::new(0) and add (seed as f64) to sample coords.
        let perlin = Perlin::new(seed);
        //frequency multiplier for the noise
        let scale = 0.05;
        
        //loop throught the whole grid
        for y in 0..self.height {
            for x in 0..self.width {
                //convertion to the array here
                let idx = self.get_index(x, y);
                //noise value is always in the range of -1 to 1
                //density shifts it roughly from 0 to 1.9-2
                // 1 is regular pressure, 0.9 less, 1.1 more pressure
                let noise_val = perlin.get([x as f64 * scale, y as f64 * scale]);
                let density = 0.9 + (noise_val as f32 + 1.0) * 0.1;
                //noise field of air density, but at a different location
                let vx_noise = perlin.get([x as f64 * scale + 100.0, y as f64 * scale]);
                let vy_noise = perlin.get([x as f64 * scale, y as f64 * scale + 100.0]);
                //these are the initial velocities range. they should be set equal to the noise velocity!
                let vx = vx_noise as f32 * 0.01;
                let vy = vy_noise as f32 * 0.01;
                // for all the directions calculate the optimal density, velocity and direction
                for i in 0..9 {
                    self.distribution[idx][i] = 
                        self.compute_equilibrium(density, vx, vy, i);
                }
            }
        }
    }

    /// Add a breach that sucks air out (creates vacuum)
    pub fn add_breach(&mut self, x: usize, y: usize) {
        if x < self.width && y < self.height {
            self.breaches.push((x, y));
            debug!("Breach created at ({}, {}) ", x, y);
        }
    }
    
    pub fn remove_breach(&mut self, x: usize, y: usize) {
        self.breaches.retain(|&(bx, by)| !(bx == x && by == y));
        debug!("Breach removed at ({}, {})", x, y);
    }

    // convert from vector to 2d
    #[inline]
    fn get_index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    // mystirious formula that was passed down from wise men(or women)
    #[inline]
    fn compute_equilibrium(&self, density: f32, vx: f32, vy: f32, i: usize) -> f32 {
        // according to website this is like a dot product of lattice velocity 
        let cu = C_X[i] * vx + C_Y[i] * vy;
        // kinetic enegry of the flow
        let u_sq = vx * vx + vy * vy;
        //Maxwell-Boltzmann equilibrium formula 
        // 1 is there even if the velocity is 0, 3 is a coeff for the lattice speed of sound 4.5 * cu * cu is particles gathering together with speed - kinetic enegry
        WEIGHTS[i] * density * (1.0 + 3.0 * cu + 4.5 * cu * cu - 1.5 * u_sq)
    }

    pub fn compute_macroscopic(&self, x: usize, y: usize) -> (f32, f32, f32) {
        let idx = self.get_index(x, y);
        
        //these are the accumulators for the velocity. they sum up all the 9 directions
        let mut rho = 0.0;
        let mut ux = 0.0;
        let mut uy = 0.0;
        
        for i in 0..9 {
            //total density is the sum of all distribution functions. Each f[i] tells us how many particles move in direction i, so summing gives total particles in the cell
            let f = self.distribution[idx][i];
            rho += f;
            //momentums in x and y directions
            ux += C_X[i] * f;
            uy += C_Y[i] * f;
        }
        //check that if the velocity is very small because of the breach, we would rather set it to be a very small number. no division
        if rho > 0.001 {
            ux /= rho;
            uy /= rho;
        } else {
            ux = 0.0;
            uy = 0.0;
        }
        (rho, ux, uy)
    }
}

//this method will be called from the map generation
pub fn setup_fluid_grid(mut commands: Commands) {
    let mut grid = FluidGrid::new(GRID_WIDTH, GRID_HEIGHT);
    grid.initialize_with_perlin(42);
    
    //this part for loading the walls from map file
    let map_path = "assets/rooms/level.txt";
    match fs::read_to_string(map_path) {
        Ok(map_content) => {
            grid.set_obstacles_from_map(&map_content);
        }
        Err(e) => {
            // avoid panic; just warn and proceed with empty obstacles
            warn!("Failed to read map file '{}': {}", map_path, e);
        }
    }
  
    // a simple default breach so you see suction effects
    // grid.add_breach(GRID_WIDTH / 2, GRID_HEIGHT / 2);
     
    commands.spawn((grid, Name::new("FluidGrid")));
    info!("Fluid simulation initialized");
}


//step 1 of LBM: Particles are supposed to collide in each cell and then, using other methods they should come back to the optimal stage
fn collision_step(mut query: Query<&mut FluidGrid>) {
    for mut grid in &mut query {
        for y in 0..grid.height {
            for x in 0..grid.width {
                let idx = grid.get_index(x, y);

                //if there is no collission in the cell, then it is fine. nothing needs to be changed
                if grid.obstacles[idx] {
                    continue;
                }
                let (rho, ux, uy) = grid.compute_macroscopic(x, y); // get the classic density and velocity of particles in the given cell
                
                for i in 0..9 {
                    // current distribution of particles in the cell
                    let f_old = grid.distribution[idx][i];
                    //calculating the optimal one
                    let f_eq = grid.compute_equilibrium(rho, ux, uy, i);
                    //BGK formula, omega controls the speed(Remember not too fast, and not too slow for density) multiplied by the difference in states
                    grid.distribution[idx][i] = f_old - OMEGA * (f_old - f_eq);
                }
            }
        }
    }
}

//moving particles into the neighboring cells based on the direction
fn streaming_step(mut query: Query<&mut FluidGrid>) {
    for mut grid in &mut query {
        let width = grid.width;
        let height = grid.height;

        // Clone the distribution to avoid borrow conflicts
        let old_dist = grid.distribution.clone();

        for y in 0..height {
            for x in 0..width {
                let idx = grid.get_index(x, y);

                if grid.obstacles[idx] {
                    continue;
                }
                //this loop goes through all the directions
                for i in 0..9 {
                    // see where did the particles came from, not where they are going. this is backstreaming
                    let src_x = x as isize - C_X[i] as isize;
                    let src_y = y as isize - C_Y[i] as isize;

                    let bounced_back = if src_x < 0
                        || src_x >= width as isize
                        || src_y < 0
                        || src_y >= height as isize
                    {
                        true
                    } else {
                        //check if it came from an obstacle
                        let src_idx = grid.get_index(src_x as usize, src_y as usize);
                        grid.obstacles[src_idx]
                    };

                    //if bounced back, reverse direction
                    if bounced_back {
                        let opposite_i = OPPOSITE_DIR[i];
                        grid.scratch[idx][i] = old_dist[idx][opposite_i];
                    } else {
                        let src_idx = grid.get_index(src_x as usize, src_y as usize);
                        grid.scratch[idx][i] = old_dist[src_idx][i];
                    }
                }
            }
        }

        // swap buffers after all modifications
        let temp = grid.distribution.clone();
        grid.distribution = grid.scratch.clone();
        grid.scratch = temp;
    }
}

fn apply_breach_forces(mut query: Query<&mut FluidGrid>) {
    for mut grid in &mut query {
        //collect breach positions first
        let breach_positions: Vec<(usize, usize)> = grid.breaches.clone();
        //loop through each breach position
        for &(bx, by) in &breach_positions {
            let breach_radius = 5;
            //loop through all cells in a square around the breach
            for dy in -(breach_radius as isize)..=(breach_radius as isize) {
                for dx in -(breach_radius as isize)..=(breach_radius as isize) {
                    let x = (bx as isize + dx) as isize;
                    let y = (by as isize + dy) as isize;
                    //check if this cell is within grid bounds
                    if x < 0 || y < 0 || x >= grid.width as isize || y >= grid.height as isize {
                        continue;
                    }
                    let idx = grid.get_index(x as usize, y as usize);
                    //calculate distance squared from breach center
                    let dist_sq = (dx * dx + dy * dy) as f32;
                    //only affect cells within circular radius
                    if dist_sq < (breach_radius * breach_radius) as f32 {
                        //vacuum strength decreases with distance from breach
                        let vacuum_strength = 1.0 - (dist_sq / (breach_radius * breach_radius) as f32);
                        //reduce air density in all directions
                        for i in 0..9 {
                            grid.distribution[idx][i] *= 1.0 - (vacuum_strength * 0.1);
                        }
                    }
                }
            }
        }
    }
    
}

// apply suction forces ONLY to objects inside the same room as the breach
fn pull_objects_toward_breaches(
    rooms: Res<RoomVec>,
    grid_query: Query<&FluidGrid>,
    mut objects: Query<(&Transform, &mut crate::enemy::Velocity, &PulledByFluid), Without<crate::player::Player>>,
) {
    let Ok(grid) = grid_query.single() else {
        return;
    };

    if grid.breaches.is_empty() {
        return;
    }

    // convert breach grid coords → world coords
    let cell_size = crate::TILE_SIZE;
    let grid_origin_x = -(grid.width as f32 * cell_size) / 2.0;
    let grid_origin_y = -(grid.height as f32 * cell_size) / 2.0;

    for (transform, mut velocity, pulled) in &mut objects {
        let world_pos = transform.translation.truncate();

        //find the room object is in
        let mut current_room: Option<&Room> = None;
        for room in rooms.0.iter() {
            if room.bounds_check(world_pos) {

                current_room = Some(room);
                break;
            }
        }

        //dont pull if not in any room
        let Some(room) = current_room else {
            continue;
        };

        // dont apply suction if room has no breaches
        if room.breaches.is_empty() {
            continue;
        }

        //apply suction forces from all breaches in the room
        let mut total_force = Vec2::ZERO;

        for &breach_world_pos in &room.breaches {
            let to_breach = breach_world_pos - world_pos;
            let distance = to_breach.length();

            if distance > 1.0 {
                let force_magnitude = 25000.0;
                total_force += to_breach.normalize() * force_magnitude;
            }
        }

        // apply physics
        let acceleration = total_force / pulled.mass;
        velocity.velocity += acceleration * 0.016;

        // clamp excessive speeds
        let max_velocity = 200.0;
        if velocity.velocity.length() > max_velocity {
            velocity.velocity = velocity.velocity.normalize() * max_velocity;
        }
    }
}

// Quiet helper: World → grid; returns None if outside (no logs)
fn world_to_grid_opt(
    world_pos: Vec2,
    grid_width: usize,
    grid_height: usize,
    cell_size: f32,
) -> Option<(usize, usize)> {
    //calculate grid origin (center of grid is at world origin 0,0)
    let grid_origin_x = -(grid_width as f32 * cell_size) / 2.0;
    let grid_origin_y = -(grid_height as f32 * cell_size) / 2.0;
    
    let gx = (world_pos.x - grid_origin_x) / cell_size;
    let gy = (world_pos.y - grid_origin_y) / cell_size;

    if gx < 0.0 || gy < 0.0 {
        return None;
    }
    let (ux, uy) = (gx as usize, gy as usize);
    if ux >= grid_width || uy >= grid_height {
        return None;
    }
    Some((ux, uy))
}

// Kept for compatibility with any existing callers; uses TILE_SIZE
pub fn world_to_grid(world_pos: Vec2, grid_width: usize, grid_height: usize) -> (usize, usize) {
    let cell_size = crate::TILE_SIZE;
    //calculate grid origin (center of grid is at world origin 0,0)
    let grid_origin_x = -(grid_width as f32 * cell_size) / 2.0;
    let grid_origin_y = -(grid_height as f32 * cell_size) / 2.0;
    
    //convert world coordinates to grid coordinates and clamp to valid range
    let grid_x = ((world_pos.x - grid_origin_x) / cell_size).max(0.0).min((grid_width - 1) as f32) as usize;
    let grid_y = ((world_pos.y - grid_origin_y) / cell_size).max(0.0).min((grid_height - 1) as f32) as usize;
    
    (grid_x, grid_y)
}


pub fn sync_air_to_fluid(
    air_grid_q: Query<&crate::air::AirGrid>,
    mut fluid_grid_q: Query<&mut FluidGrid>,
) {
    let Ok(air_grid) = air_grid_q.single() else {
        return;
    };
    
    let Ok(mut fluid_grid) = fluid_grid_q.single_mut() else {
        return;
    };

    if air_grid.w != fluid_grid.width || air_grid.h != fluid_grid.height {
        warn!("Grid size mismatch!");
        return;
    }

    info!("Syncing Perlin air pressure to LBM fluid distribution...");

    for y in 0..fluid_grid.height {
        for x in 0..fluid_grid.width {
            let idx = fluid_grid.get_index(x, y);
            
            if fluid_grid.obstacles[idx] {
                continue;
            }

            let air_pressure = air_grid.get(x, y);
            let density = air_pressure * 0.4;
            
            
            let mut vx = 0.0;
            let mut vy = 0.0;
            
          
            if x > 0 && x < fluid_grid.width - 1 {
                let p_left = air_grid.get(x - 1, y);
                let p_right = air_grid.get(x + 1, y);
                vx = (p_right - p_left) * 0.01; 
            }
            
            if y > 0 && y < fluid_grid.height - 1 {
                let p_down = air_grid.get(x, y - 1);
                let p_up = air_grid.get(x, y + 1);
                vy = (p_up - p_down) * 0.01; 
            }

            for i in 0..9 {
                fluid_grid.distribution[idx][i] = 
                    fluid_grid.compute_equilibrium(density, vx, vy, i);
            }
        }
    }

    
}
