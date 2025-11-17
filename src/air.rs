use crate::map::LevelRes;
use crate::noise::PerlinField;
use crate::{TILE_SIZE, Z_ENTITIES};
use bevy::prelude::*;
use rand::Rng;

#[derive(Resource, Clone)]
pub struct AirParams {
    pub seed: u32,
    pub scale: f64,
    pub octaves: u32,
    pub gain: f32,
    pub lacunarity: f64,
}
impl Default for AirParams {
    fn default() -> Self {
        Self {
            seed: 42, // controls random starting state of the noise
            scale: 0.05, // controls how zoomed into the noise image we are
            octaves: 1, // controls how many layers will be stacked
            gain: 0.5, // controls the intensity falloff of each octave
            lacunarity: 2.0, // controls the frequency increase of each octave
        }
    }
}

#[derive(Resource, Component)]
pub struct AirGrid {
    pub w: usize,
    pub h: usize,
    pub pressure: Vec<f32>,   // length = w*h, per tile in [0..5]
    pub obstacles: Vec<bool>, // true = solid
}
impl AirGrid {
    #[inline]
    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.w + x
    }
    #[inline]
    pub fn get(&self, x: usize, y: usize) -> f32 {
        self.pressure[self.idx(x, y)]
    }
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, v: f32) {
        let i = self.idx(x, y);
        self.pressure[i] = v;
    }
    #[inline]
    pub fn is_obstacle(&self, x: usize, y: usize) -> bool {
        self.obstacles[self.idx(x, y)]
    }
}

/// Build the AirGrid from the loaded text level
pub fn init_air_grid(
    mut commands: Commands,
    level: Res<LevelRes>,
    air_cfg: Option<Res<AirParams>>,
) {
    let h = level.level.len();
    let w = level.level.first().map(|s| s.len()).unwrap_or(0);
    assert!(w > 0 && h > 0, "Level has no rows");

    let cfg = air_cfg.map(|r| r.clone()).unwrap_or_default();

    let mut noise = PerlinField::new(cfg.seed);
    let mut rng = rand::rng();
    noise.scale = rng.random_range(0.03..0.08);
    noise.octaves = rng.random_range(1..=4);
    noise.gain = rng.random_range(0.3..0.7);
    noise.lacunarity = rng.random_range(1.8..2.8);

    let mut grid = AirGrid {
        w,
        h,
        pressure: vec![0.0; w * h],
        obstacles: vec![false; w * h],
    };

    for y in 0..h {
        let row = &level.level[y];
        for x in 0..w {
            let ch = row.as_bytes()[x] as char;
            let i = grid.idx(x, y);

            match ch {
                // Tiles that dont get air
                '.' | 'W' | 'D' | 'G' => {
                    grid.obstacles[i] = true;
                    grid.set(x, y, 0.0);
                }

                // Floor + air-allowed tiles
                // Tables and enemy spawn spots sit on floor, so allow air there
                '#' | 'T' | 'E' => {
                    grid.obstacles[i] = false;
                    let p = noise.density(x, y); // 0 → 5
                    grid.set(x, y, p);
                }

                _ => {
                    // Unknown tile → treat as floor
                    grid.set(x, y, noise.density(x, y));
                }
            }
        }
    }

    commands.spawn(grid);
    info!("AirGrid initialized: {}x{} tiles", w, h);
}


#[derive(Component)]
struct PressureLabel;

#[derive(Component, Copy, Clone)]
struct GridPos {
    x: usize,
    y: usize,
}

/// Spawn a tiny text label on every non-wall/space tile with its current pressure.
pub fn spawn_pressure_labels(
    mut commands: Commands,
    assets: Res<AssetServer>,
    air: Query<&AirGrid>,
    level: Res<LevelRes>,
) {
    let Ok(air) = air.single() else {
        return;
    };
    let map_cols = level.level.first().map(|r| r.len()).unwrap_or(0) as f32;
    let map_rows = level.level.len() as f32;
    let map_px_w = map_cols * TILE_SIZE;
    let map_px_h = map_rows * TILE_SIZE;
    let x0 = -map_px_w * 0.5 + TILE_SIZE * 0.5;
    let y0 = -map_px_h * 0.5 + TILE_SIZE * 0.5;

    let font: Handle<Font> = assets.load(
        "fonts/BitcountSingleInk-VariableFont_CRSV,ELSH,ELXP,SZP1,SZP2,XPN1,XPN2,YPN1,YPN2,slnt,wght.ttf"
    );

    for y in 0..air.h {
        for x in 0..air.w {
            // Only place labels on authored floor cells
            let ch = level.level[y].as_bytes()[x] as char;
            if ch != '#' {
                continue;
            } // only floor tiles

            // keep the rest the same
            let world_x = x0 + x as f32 * TILE_SIZE;
            let world_y = y0 + (map_rows - 1.0 - y as f32) * TILE_SIZE;

            commands.spawn((
                Text2d::new(format!("{:.1}", air.get(x, y))),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(pressure_to_color(air.get(x, y))),
                Transform::from_xyz(world_x, world_y, Z_ENTITIES + 10.0),
                PressureLabel,
                GridPos { x, y },
            ));
        }
    }
}

/// Update labels if AirGrid changes (if reseeded or tweak params)
pub fn update_pressure_labels(
    air: Query<&AirGrid, Changed<AirGrid>>,
    mut q: Query<(&GridPos, &mut Text, &mut TextColor), With<PressureLabel>>,
) {
    let Ok(air) = air.single() else {
        return;
    };
    for (pos, mut text, mut color) in &mut q {
        let p = air.get(pos.x, pos.y);
        *text = Text::new(format!("{:.1}", p));
        color.0 = pressure_to_color(p);
    }
}

fn pressure_to_color(p: f32) -> Color {
    let t = (p / 5.0).clamp(0.0, 1.0);
    // blue->red
    Color::srgb(t, 0.0, 1.0 - t)
}
