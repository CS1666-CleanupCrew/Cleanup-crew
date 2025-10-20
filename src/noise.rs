use noise::{NoiseFn, Perlin};


pub struct PerlinField {
    perlin: Perlin,
    pub scale: f64,      // controls the frequency of the noise pattern
    pub amplitude: f32,  // controls the intensity of variation
}

impl PerlinField {
    // Create a new Perlin noise field with a given seed.
    pub fn new(seed: u32) -> Self {
        Self {
            perlin: Perlin::new(seed),
            scale: 0.05,     // higher = more variation
            amplitude: 0.10, // tweak for intensity
        }
    }

    // Returns a density value between roughly
    pub fn density(&self, x: usize, y: usize) -> f32 {
        let nx = x as f64 * self.scale;
        let ny = y as f64 * self.scale;
        let val = self.perlin.get([nx, ny]) as f32;
        ((val + 1.0) * 0.5) * 5.0;
    }
}
