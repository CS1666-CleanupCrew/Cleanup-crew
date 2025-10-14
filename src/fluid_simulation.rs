use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

//Division of the room in small grids for the airflow measurement there
const GRID_WIDTH: usize = 128;
const GRID_HEIGHT: usize = 96;

//responsible for the thickness of the air
const RELAXATION_TIME: f32 = 0.55;
//how long it takes particles to get back to the original state after the serious destrurbance
const OMEGA: f32 = 1.0 / RELAXATION_TIME;

// breach values
// pressure of the vaccume of space
const VACUUM_PREASSURE: f32 = 0.001;
// the fraction of air that gets transfered from a neighbor's cell into the breach cell
const TRANSFER_FRACTION: f32 = 0.02;
// strength of pushing neighbor cell's velocity towards the breach cell
const PUSH_STRENGTH: f32 = 0.15;
// saftey for the density value
const MIN_RHO: f32 = 1e-6;
// how much the neighbor cell's distribution is replaced with equilibrum
const BLEND: f32 = 0.4;

//D2Q9 directions
const C_X: [f32; 9] = [0.0, 1.0, 0.0, -1.0, 0.0, 1.0, -1.0, -1.0, 1.0];
const C_Y: [f32; 9] = [0.0, 0.0, 1.0, 0.0, -1.0, 1.0, 1.0, -1.0, -1.0];

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
                ).chain()
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
            obstacles: vec![false; size],
            breaches: Vec::new(),
        }
    }

    pub fn initialize_with_perlin(&mut self, seed: u32) {
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
    pub fn add_breach(&mut self, x: usize, y: usize) 
    {
        if x < self.width && y < self.height {
            self.breaches.push((x, y));
            info!("Breach created at ({}, {}) ", x, y);
        }
    }

    // convert from vector to 2d
    #[inline]
    fn get_index(&self, x: usize, y: usize) -> usize 
    {
        y * self.width + x
    }

    // mystirious formula that was passed down from wise men(or women)
    #[inline]
    fn compute_equilibrium(&self, density: f32, vx: f32, vy: f32, i: usize) -> f32 
    {
        // according to website this is like a dot product of lattice velocity 
        let cu = C_X[i] * vx + C_Y[i] * vy;
        // kinetic enegry of the flow
        let u_sq = vx * vx + vy * vy;
        //Maxwell-Boltzmann equilibrium formula 
        // 1 is there even if the velocity is 0, 3 is a coeff for the lattice speed of sound 4.5 * cu * cu is particles gathering together with speed - kinetic enegry
        WEIGHTS[i] * density * (1.0 + 3.0 * cu + 4.5 * cu * cu - 1.5 * u_sq)
    }

    fn compute_macroscopic(&self, x: usize, y: usize) -> (f32, f32, f32) {
        let idx = self.get_index(x, y);
        

        //these are the accumulators for the velocity. they sum up all the 9 directions
        let mut rho = 0.0;
        let mut ux = 0.0;
        let mut uy = 0.0;
        
        for i in 0..9 
        {
            //total density is the sum of all distribution functions. Each f[i] tells us how many particles move in direction i, so summing gives total particles in the cell
            let f = self.distribution[idx][i];
            rho += f;
            //momentums in x and y directions
            ux += C_X[i] * f;
            uy += C_Y[i] * f;
        }
        //check that if the velocity is very small because of the breach, we would rather set it to be a very small number. no division
        if rho > 0.001 
        {
            ux /= rho;
            uy /= rho;
        } else 
        {
            ux = 0.0;
            uy = 0.0;
        }
        (rho, ux, uy)
    }
}

//this method will be called from the map generation
fn setup_fluid_grid(mut commands: Commands) {
    let mut grid = FluidGrid::new(GRID_WIDTH, GRID_HEIGHT);
    grid.initialize_with_perlin(42);
    
  
   
    //grid.add_breach(10, 48); 
    //grid.add_breach(118, 48); 
    
    commands.spawn((grid, Name::new("FluidGrid")));
    info!("Fluid simulation with breach aaaaaaaahhhhhh");
}


//step 1 of LBM: Particles are supposed to collide in each cell and then, using other methods they should come back to the optimal stage
fn collision_step(mut query: Query<&mut FluidGrid>) 
{
    for mut grid in &mut query 
    {
        for y in 0..grid.height 
        {
            for x in 0..grid.width 
            {
                let idx = grid.get_index(x, y);
                

                //if there is no collission in the cell, then it is fine. nothing needs to be changed
                if grid.obstacles[idx] 
                {
                    continue;
                }
                let (rho, ux, uy) = grid.compute_macroscopic(x, y); // get the classic density and velocity of particles in the given cell
                
                for i in 0..9 
                {
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
    for mut grid in &mut query 
    {
        //copy of the entire grid
        let mut new_dist = grid.distribution.clone();
        //loop through all the cells
        for y in 0..grid.height 
        {
            for x in 0..grid.width 
            {
                let idx = grid.get_index(x, y);
                
                if grid.obstacles[idx] 
                {
                    continue;
                }
                //this loop goes through all the directions aside from the rest state
                for i in 1..9 
                {
                    // see where did the particles came from, not where they are going. this is backstreaming
                    let src_x = x as isize - C_X[i] as isize;
                    let src_y = y as isize - C_Y[i] as isize;
                    //check for bounds
                    if src_x >= 0 && src_x < grid.width as isize 
                        && src_y >= 0 && src_y < grid.height as isize {
                        let src_idx = grid.get_index(src_x as usize, src_y as usize);
                         //particles that were moving in direction i at the source have now arrived at the current cell.
                        new_dist[idx][i] = grid.distribution[src_idx][i];
                    }
                }
            }
        }
        //just replace the old one with the newly calculated one 
        grid.distribution = new_dist;
    }
}

// simulates the breach forces
fn apply_breach_forces(mut query: Query<&mut FluidGrid>) {
    // see coordinates of where the breach has occured
    for mut grid in &mut query {
        // if it doesn't find any breaches we don't have to simulate anything so continue
        if grid.breaches.is_empty() {
            continue;
        }
        // checks if each breach coordinate is on the grid
        for &(bx, by) in &grid.breaches {
            if bx >= grid.width || by >= grid.height {
                continue;
            }


            let breach_index = grid.get_index(bx, by);
            // make the density of the breach cell to be close to 0
            // not exactly 0 because of float division I think so it's (0.001)
            // also sets the velocity to 0
            // simulates a vacuum
            for i in 0..9 {
                grid.distribution[breach_index][i] = grid.compute_equilibrium(VACUUM_PREASSURE, 0.0, 0.0, i)
            }

            // transfer some energy to neighbors on the N, S, E, W coordinates (not the 9 directions)
            let neighbor_offsets: &[(isize, isize)] = &[(1,0), (-1,0), (0,1), (0,-1)];

            // interates over the cell's neighbors, calculates neighbor's coordinates
            for &(ox, oy) in neighbor_offsets {
                let nx_isize = bx as isize + ox;
                let ny_isize = by as isize + oy;

                if (nx_isize < 0 || nx_isize >= grid.width as isize || ny_isize < 0 || ny_isize >= grid.height as isize) {
                    continue;
                }

                let nx = nx_isize as usize;
                let ny = ny_isize as usize;
                let neighbor_index =  grid.get_index(nx, ny);

                // ignores obsticles
                if grid.obstacles[neighbor_index] {
                    continue;
                }
                
                // transfer mass/ air flow from neighbor's cell to breach cell
                for i in 0..9 {
                    let transfer_mass = grid.distribution[neighbor_index][i] * TRANSFER_FRACTION;
                    grid.distribution[neighbor_index][i] -= transfer_mass;
                    grid.distribution[breach_index][i] += transfer_mass;
                }

                // recalculate the neighbor cell's macroscopic state
                let (mut rho, mut ux, mut uy) = grid.compute_macroscopic(nx, ny) ;
                if rho < MIN_RHO {
                    rho = MIN_RHO;
                }

                // slightly push neighbor's velocity towards the breach location
                let dir_x = bx as f32 - nx as f32;
                let dir_y = bx as f32 - ny as f32;
                // formula to compute the squared distance of a 2d vector
                let distance = (dir_x * dir_x + dir_y * dir_y).sqrt().max(1.0);
                let push_x = (dir_x /distance) * PUSH_STRENGTH;
                let push_y = (dir_y /distance) * PUSH_STRENGTH;
                ux += push_x;
                uy += push_y;

                // remakes the neighbor's distributions by blend toward the new equilibrium
                for i in 0..9 {
                    let f_eq = grid.compute_equilibrium(rho, ux, uy, i);
                    grid.distribution[neighbor_index][i] = (1.0 - BLEND) * grid.distribution[neighbor_index][i] + BLEND * f_eq;
                }
            }
        }


    }


}
