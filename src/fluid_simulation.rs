use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

//Division of the room in small grids for the airflow measurement there
const GRID_WIDTH: usize = 128;
const GRID_HEIGHT: usize = 96;

//responsible for the thickness of the air
const RELAXATION_TIME: f32 = 0.55;
//how long it takes particles to get back to the original state after the serious destrurbance
const OMEGA: f32 = 1.0 / RELAXATION_TIME;

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

}