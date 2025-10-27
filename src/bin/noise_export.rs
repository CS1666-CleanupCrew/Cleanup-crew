use image::{ImageBuffer, Luma};
use noise::{NoiseFn, Perlin};

pub struct PerlinField {
    perlin: Perlin,
    pub scale: f64, // base frequency
    pub octaves: u32, // number of layers (>=1)
    pub gain: f32, // amplitude falloff per octave (0..1)
    pub lacunarity: f64, // frequency multiplier per octave (>=1)
}

impl PerlinField {
    pub fn new(_seed: u32) -> Self {
        // The `noise` crate’s Perlin is deterministic; it doesn’t take a seed directly.
        // For different seeds, offset sampling coords or add multiple
        // Perlin instances. For now we just keep one and use the seed to tweak offsets
        Self {
            perlin: Perlin::new(0),
            scale: 0.05,
            octaves: 1,
            gain: 0.5,
            lacunarity: 2.0,
        }
    }

    /// fBM (fractal brownian motion) perlin in [-1,1] → remap to [0,5]
    pub fn density(&self, x: usize, y: usize) -> f32 {
        let mut amp: f32 = 1.0;
        let mut freq: f64 = 1.0;
        let mut sum: f32 = 0.0;
        let mut norm: f32 = 0.0;

        for _ in 0..self.octaves.max(1) {
            let nx = x as f64 * self.scale * freq;
            let ny = y as f64 * self.scale * freq;
            let v = self.perlin.get([nx, ny]) as f32; // [-1,1]
            sum  += v * amp;
            norm += amp;
            amp  *= self.gain.max(0.0).min(1.0);
            freq *= self.lacunarity.max(1.0);
        }

        let val = if norm > 0.0 { sum / norm } else { 0.0 }; // [-1,1]
        ((val + 1.0) * 0.5) * 5.0                           // [0,5]
    }
}

fn main() {

    let width = 512;
    let height = 512;


    let mut noise = PerlinField::new(42);


    noise.scale = 0.02;
    noise.octaves = 4;
    noise.gain = 0.5;
    noise.lacunarity = 2.0;


    let mut imgbuf = ImageBuffer::new(width, height);


    for y in 0..height {
        for x in 0..width {
            let p = noise.density(x as usize, y as usize);

            let pixel_value = (p * 255.0 / 5.0) as u8;

            imgbuf.put_pixel(x, y, Luma([pixel_value]));
        }
    }

    imgbuf.save("heatmap.png").unwrap();
    println!("✅ Saved heatmap.png");
}


