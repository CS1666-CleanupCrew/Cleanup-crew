use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

const GRID_WIDTH: usize = 128;
const GRID_HEIGHT: usize = 96;
const RELAXATION_TIME: f32 = 0.55;
const OMEGA: f32 = 1.0 / RELAXATION_TIME;

// D2Q9 lattice velocities
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

#[derive(Component)]
pub struct FluidGrid {
    pub width: usize,
    pub height: usize,
    pub distribution: Vec<[f32; 9]>,
    pub obstacles: Vec<bool>,
    pub breaches: Vec<(usize, usize)>, // Positions of window breaches
}

#[derive(Component)]
pub struct PulledByFluid {
    pub mass: f32,  // Heavier objects resist more
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
        let scale = 0.05;
        
        info!("Initializing fluid grid with Perlin noise (seed: {})", seed);

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.get_index(x, y);
                
                let noise_val = perlin.get([x as f64 * scale, y as f64 * scale]);
                let density = 0.9 + (noise_val as f32 + 1.0) * 0.1;
                
                let vx_noise = perlin.get([x as f64 * scale + 100.0, y as f64 * scale]);
                let vy_noise = perlin.get([x as f64 * scale, y as f64 * scale + 100.0]);
                
                let vx = vx_noise as f32 * 0.01;
                let vy = vy_noise as f32 * 0.01;
                
                for i in 0..9 {
                    self.distribution[idx][i] = 
                        self.compute_equilibrium(density, vx, vy, i);
                }
            }
        }
        
        info!("Fluid grid initialized: {}x{} cells", self.width, self.height);
    }

    /// Add a breach that sucks air out (creates vacuum)
    pub fn add_breach(&mut self, x: usize, y: usize) {
        if x < self.width && y < self.height {
            self.breaches.push((x, y));
            info!("Breach created at ({}, {}) - Air escaping to space!", x, y);
        }
    }

    #[inline]
    fn get_index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    #[inline]
    fn compute_equilibrium(&self, density: f32, vx: f32, vy: f32, i: usize) -> f32 {
        let cu = C_X[i] * vx + C_Y[i] * vy;
        let u_sq = vx * vx + vy * vy;
        WEIGHTS[i] * density * (1.0 + 3.0 * cu + 4.5 * cu * cu - 1.5 * u_sq)
    }

    fn compute_macroscopic(&self, x: usize, y: usize) -> (f32, f32, f32) {
        let idx = self.get_index(x, y);
        
        let mut rho = 0.0;
        let mut ux = 0.0;
        let mut uy = 0.0;
        
        for i in 0..9 {
            let f = self.distribution[idx][i];
            rho += f;
            ux += C_X[i] * f;
            uy += C_Y[i] * f;
        }
        
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

fn setup_fluid_grid(mut commands: Commands) {
    let mut grid = FluidGrid::new(GRID_WIDTH, GRID_HEIGHT);
    grid.initialize_with_perlin(42);
    
    // Example: Add breaches at window positions
    // You would call this from your map generation
    grid.add_breach(10, 48);   // Left wall breach
    grid.add_breach(118, 48);  // Right wall breach
    
    commands.spawn((grid, Name::new("FluidGrid")));
    info!("Fluid simulation with breach physics initialized");
}

fn collision_step(mut query: Query<&mut FluidGrid>) {
    for mut grid in &mut query {
        for y in 0..grid.height {
            for x in 0..grid.width {
                let idx = grid.get_index(x, y);
                
                if grid.obstacles[idx] {
                    continue;
                }
                
                let (rho, ux, uy) = grid.compute_macroscopic(x, y);
                
                for i in 0..9 {
                    let f_old = grid.distribution[idx][i];
                    let f_eq = grid.compute_equilibrium(rho, ux, uy, i);
                    grid.distribution[idx][i] = f_old - OMEGA * (f_old - f_eq);
                }
            }
        }
    }
}

fn streaming_step(mut query: Query<&mut FluidGrid>) {
    for mut grid in &mut query {
        let mut new_dist = grid.distribution.clone();
        
        for y in 0..grid.height {
            for x in 0..grid.width {
                let idx = grid.get_index(x, y);
                
                if grid.obstacles[idx] {
                    continue;
                }
                
                for i in 1..9 {
                    let src_x = x as isize - C_X[i] as isize;
                    let src_y = y as isize - C_Y[i] as isize;
                    
                    if src_x >= 0 && src_x < grid.width as isize 
                        && src_y >= 0 && src_y < grid.height as isize {
                        let src_idx = grid.get_index(src_x as usize, src_y as usize);
                        new_dist[idx][i] = grid.distribution[src_idx][i];
                    }
                }
            }
        }
        
        grid.distribution = new_dist;
    }
}

/// Create strong vacuum at breach locations
fn apply_breach_forces(mut query: Query<&mut FluidGrid>) {
    for mut grid in &mut query {
        for &(bx, by) in &grid.breaches.clone() {
            let breach_radius = 5;
            
            for dy in -(breach_radius as isize)..=(breach_radius as isize) {
                for dx in -(breach_radius as isize)..=(breach_radius as isize) {
                    let x = (bx as isize + dx) as usize;
                    let y = (by as isize + dy) as usize;
                    
                    if x < grid.width && y < grid.height {
                        let idx = grid.get_index(x, y);
                        let dist_sq = (dx * dx + dy * dy) as f32;
                        
                        if dist_sq < (breach_radius * breach_radius) as f32 {
                            // Create vacuum - remove air density
                            let vacuum_strength = 1.0 - (dist_sq / (breach_radius * breach_radius) as f32);
                            
                            for i in 0..9 {
                                grid.distribution[idx][i] *= 1.0 - (vacuum_strength * 0.1);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Pull objects (tables, debris) toward breaches
fn pull_objects_toward_breaches(
    grid_query: Query<&FluidGrid>,
    mut objects: Query<(&Transform, &mut Velocity, &PulledByFluid), Without<crate::player::Player>>,
) {
    let Ok(grid) = grid_query.get_single() else {
        return;
    };
    
    if grid.breaches.is_empty() {
        return;
    }
    
    let cell_size = 8.0; // World units per grid cell
    let grid_origin_x = -(grid.width as f32 * cell_size) / 2.0;
    let grid_origin_y = -(grid.height as f32 * cell_size) / 2.0;
    
    for (transform, mut velocity, pulled) in &mut objects {
        let world_pos = transform.translation.truncate();
        
        // Convert world position to grid coordinates
        let grid_x = ((world_pos.x - grid_origin_x) / cell_size) as usize;
        let grid_y = ((world_pos.y - grid_origin_y) / cell_size) as usize;
        
        if grid_x >= grid.width || grid_y >= grid.height {
            continue;
        }
        
        // Get fluid velocity at object's position
        let (density, fluid_vx, fluid_vy) = grid.compute_macroscopic(grid_x, grid_y);
        
        // Calculate pull force toward nearest breach
        let mut total_force = Vec2::ZERO;
        
        for &(bx, by) in &grid.breaches {
            let breach_world_x = grid_origin_x + (bx as f32 * cell_size);
            let breach_world_y = grid_origin_y + (by as f32 * cell_size);
            let breach_pos = Vec2::new(breach_world_x, breach_world_y);
            
            let to_breach = breach_pos - world_pos;
            let distance = to_breach.length();
            
            if distance > 1.0 {
                // Inverse square law for suction force
                let force_magnitude = 5000.0 / (distance * distance);
                
                // Vacuum strength multiplier (low density = strong pull)
                let vacuum_multiplier = (1.0 - density).max(0.0) * 2.0;
                
                total_force += to_breach.normalize() * force_magnitude * vacuum_multiplier;
            }
        }
        
        // Apply force based on mass (heavier objects resist more)
        let acceleration = total_force / pulled.mass;
        
        // Also add fluid velocity influence
        let fluid_force = Vec2::new(fluid_vx, fluid_vy) * 100.0;
        
        velocity.0 += (acceleration + fluid_force) * 0.016; // Assuming ~60fps
        
        // Cap maximum velocity so objects don't fly too fast
        let max_velocity = 300.0;
        if velocity.0.length() > max_velocity {
            velocity.0 = velocity.0.normalize() * max_velocity;
        }
    }
}

// Helper to convert world position to grid coordinates
pub fn world_to_grid(world_pos: Vec2, grid_width: usize, grid_height: usize) -> (usize, usize) {
    let cell_size = 8.0;
    let grid_origin_x = -(grid_width as f32 * cell_size) / 2.0;
    let grid_origin_y = -(grid_height as f32 * cell_size) / 2.0;
    
    let grid_x = ((world_pos.x - grid_origin_x) / cell_size).max(0.0).min((grid_width - 1) as f32) as usize;
    let grid_y = ((world_pos.y - grid_origin_y) / cell_size).max(0.0).min((grid_height - 1) as f32) as usize;
    
    (grid_x, grid_y)
}

// Add this component to your player.rs
