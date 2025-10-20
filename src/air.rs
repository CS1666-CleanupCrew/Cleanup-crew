use bevy::prelude::*;
use crate::noise::PerlinField;

use crate::map::LevelRes;

#[derive(Resource)]
pub struct AirGrid {
    pub w: usize,
    pub h: usize,
    pub pressure: Vec<f32>,   // length = w*h, 1 value per tile
    pub obstacles: Vec<bool>, // true = solid (e.g., 'W' walls)
}

impl AirGrid {
    #[inline]
    fn idx(&self, x: usize, y: usize) -> usize { y * self.w + x }
    #[inline]
    pub fn get(&self, x: usize, y: usize) -> f32 { self.pressure[self.idx(x, y)] }
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, v: f32) { let i = self.idx(x,y); self.pressure[i] = v; }
    #[inline]
    pub fn is_obstacle(&self, x: usize, y: usize) -> bool { self.obstacles[self.idx(x,y)] }
}

pub fn init_air_grid(
    mut commands: Commands,
    level: Res<LevelRes>,       // provides level.level: Vec<String>
) {
    let h = level.level.len();
    let w = level.level.first().map(|s| s.len()).unwrap_or(0);
    assert!(w > 0 && h > 0, "Level has no rows");

    let mut grid = AirGrid {
        w, h,
        pressure: vec![1.0; w*h],    // baseline 1.0 everywhere
        obstacles: vec![false; w*h], // mark walls later
    };

    // Seed can be constant for reproducibility or come from settings/save.
    let noise = PerlinField::new(42);

for y in 0..h {
    let row = &level.level[y];
    for x in 0..w {
        let ch = row.as_bytes()[x] as char;

        if ch == 'W' {
            let i = y * grid.w + x;
            grid.obstacles[i] = true;
            grid.set(x, y, 1.0);          
            continue;
        }

        let p = noise.density(x, y);
        grid.set(x, y, p);
    }
}

    commands.insert_resource(grid);
    info!("AirGrid initialized: {}x{} tiles", w, h);
}