use rand::seq::SliceRandom;
use rand::{rng, SeedableRng};
use rand::rngs::StdRng;
use std::collections::HashSet;

pub type TablePositions = HashSet<(usize, usize)>;

/// Generates table positions from a grid representation of the room.
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
        let mut trng = rng();
        floors.shuffle(&mut trng);
    }

    floors.into_iter().take(max_tables).collect()
}
