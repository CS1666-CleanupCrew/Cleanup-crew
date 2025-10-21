
use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

//Division of the room in small grids for the airflow measurement there
pub const GRID_WIDTH: usize = 128;
pub const GRID_HEIGHT: usize = 96;

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
pub fn setup_fluid_grid(mut commands: Commands) {
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
fn apply_breach_forces(mut query: Query<&mut FluidGrid>) 
{
    for mut grid in &mut query 
    {
        //loop through each breach position
        for &(bx, by) in &grid.breaches.clone() 
        {
            let breach_radius = 5;
            //loop through all cells in a square around the breach
            for dy in -(breach_radius as isize)..=(breach_radius as isize) 
            {
                for dx in -(breach_radius as isize)..=(breach_radius as isize) 
                {
                    let x = (bx as isize + dx) as usize;
                    let y = (by as isize + dy) as usize;
                    //check if this cell is within grid bounds
                    if x < grid.width && y < grid.height 
                    {
                        let idx = grid.get_index(x, y);
                        //calculate distance squared from breach center
                        let dist_sq = (dx * dx + dy * dy) as f32;
                        //only affect cells within circular radius
                        if dist_sq < (breach_radius * breach_radius) as f32 
                        {
                            //vacuum strength decreases with distance from breach
                            let vacuum_strength = 1.0 - (dist_sq / (breach_radius * breach_radius) as f32);
                            //reduce air density in all directions
                            for i in 0..9 
                            {
                                grid.distribution[idx][i] *= 1.0 - (vacuum_strength * 0.1);
                            }
                        }
                    }
                }
            }
        }
    }
}

//apply suction forces to objects, pulling them toward breaches
fn pull_objects_toward_breaches(
    grid_query: Query<&FluidGrid>,
    mut objects: Query<(&Transform, &mut crate::enemy::Velocity, &PulledByFluid), Without<crate::player::Player>>,
)
{
    //get the fluid grid, exit if it doesn't exist
    let Ok(grid) = grid_query.get_single() else 
    {
        return;
    };
    //no breaches means no pulling force
    if grid.breaches.is_empty() 
    {
        return;
    }
    
    //conversion between world coordinates and grid coordinates
    let cell_size = 8.0;
    let grid_origin_x = -(grid.width as f32 * cell_size) / 2.0;
    let grid_origin_y = -(grid.height as f32 * cell_size) / 2.0;
    //loop through all objects that can be pulled by fluid
    for (transform, mut velocity, pulled) in &mut objects 
    {
        let world_pos = transform.translation.truncate();
        //convert world position to grid coordinates
        let grid_x = ((world_pos.x - grid_origin_x) / cell_size) as usize;
        let grid_y = ((world_pos.y - grid_origin_y) / cell_size) as usize;
        
        //skip objects outside the grid
        if grid_x >= grid.width || grid_y >= grid.height 
        {
            continue;
        }
        
        //get fluid state at object's position
        let (density, fluid_vx, fluid_vy) = grid.compute_macroscopic(grid_x, grid_y);
        //accumulate forces from all breaches
        let mut total_force = Vec2::ZERO;
        //calculate pull force from each breach
        for &(bx, by) in &grid.breaches 
        {
            //convert breach grid position to world position
            let breach_world_x = grid_origin_x + (bx as f32 * cell_size);
            let breach_world_y = grid_origin_y + (by as f32 * cell_size);
            let breach_pos = Vec2::new(breach_world_x, breach_world_y);
            //vector from object to breach
            let to_breach = breach_pos - world_pos;
            let distance = to_breach.length();
            
            //only apply force if not too close to breach
            if distance > 1.0 
            {
                //inverse square law for suction force (like gravity)
                let force_magnitude = 5000.0 / (distance * distance);
                //low air density means stronger vacuum pull
                let vacuum_multiplier = (1.0 - density).max(0.0) * 2.0;
                total_force += to_breach.normalize() * force_magnitude * vacuum_multiplier;
            }
        }
        
        //heavier objects resist more (F = ma, so a = F/m)
        let acceleration = total_force / pulled.mass;
        //add influence from fluid flow itself
        let fluid_force = Vec2::new(fluid_vx, fluid_vy) * 100.0;
        //apply acceleration assuming 60fps
        velocity.velocity += (acceleration + fluid_force) * 0.016;
        //prevent objects from flying too fast
        let max_velocity = 300.0;
        if velocity.velocity.length() > max_velocity 
        {
           velocity.velocity = velocity.velocity.normalize() * max_velocity;
        }
    }
}

pub fn world_to_grid(world_pos: Vec2, grid_width: usize, grid_height: usize) -> (usize, usize) 
{
    let cell_size = 8.0;
    //calculate grid origin (center of grid is at world origin 0,0)
    let grid_origin_x = -(grid_width as f32 * cell_size) / 2.0;
    let grid_origin_y = -(grid_height as f32 * cell_size) / 2.0;
    
    //convert world coordinates to grid coordinates and clamp to valid range
    let grid_x = ((world_pos.x - grid_origin_x) / cell_size).max(0.0).min((grid_width - 1) as f32) as usize;
    let grid_y = ((world_pos.y - grid_origin_y) / cell_size).max(0.0).min((grid_height - 1) as f32) as usize;
    
    (grid_x, grid_y)
}

