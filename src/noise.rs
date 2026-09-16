//! 2D gradient noise with fractal Brownian motion, used for terrain,
//! climate and mana fields.

use crate::rng::Rng;

/// A permutation table: one gradient field, sampled by [`Noise::at`],
/// [`Noise::fbm`] and [`Noise::ridged`].
pub struct Noise {
    perm: [u8; 512],
}

/// The eight unit gradients a lattice corner may take.
///
/// The diagonals are the truncated `0.7071`, not `std::f32::consts::FRAC_1_SQRT_2`:
/// their exact bits decide every elevation, coastline and mana field, so a
/// seed would no longer name the same world if they changed.
// The truncation is deliberate and load-bearing, so the near-constant check is off here.
#[allow(clippy::approx_constant)]
const GRAD: [(f32, f32); 8] = [
    (1.0, 0.0),
    (-1.0, 0.0),
    (0.0, 1.0),
    (0.0, -1.0),
    (0.7071, 0.7071),
    (-0.7071, 0.7071),
    (0.7071, -0.7071),
    (-0.7071, -0.7071),
];

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

impl Noise {
    /// A fresh field, its permutation table shuffled by `rng`.
    pub fn new(rng: &Rng) -> Noise {
        let mut p: Vec<u8> = (0..=255).collect();
        rng.shuffle(&mut p);
        let mut perm = [0u8; 512];
        for i in 0..512 {
            perm[i] = p[i & 255];
        }
        Noise { perm }
    }

    fn grad(&self, ix: i32, iy: i32, dx: f32, dy: f32) -> f32 {
        let h = self.perm[(self.perm[(ix & 255) as usize] as usize + (iy & 255) as usize) & 511];
        let g = GRAD[(h & 7) as usize];
        g.0 * dx + g.1 * dy
    }

    /// Single octave, roughly in [-1, 1].
    pub fn at(&self, x: f32, y: f32) -> f32 {
        let x0 = x.floor();
        let y0 = y.floor();
        let ix = x0 as i32;
        let iy = y0 as i32;
        let fx = x - x0;
        let fy = y - y0;
        let u = fade(fx);
        let v = fade(fy);
        let n00 = self.grad(ix, iy, fx, fy);
        let n10 = self.grad(ix + 1, iy, fx - 1.0, fy);
        let n01 = self.grad(ix, iy + 1, fx, fy - 1.0);
        let n11 = self.grad(ix + 1, iy + 1, fx - 1.0, fy - 1.0);
        lerp(lerp(n00, n10, u), lerp(n01, n11, u), v) * 1.414
    }

    /// Fractal Brownian motion, normalised to roughly [-1, 1].
    pub fn fbm(&self, x: f32, y: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
        let mut sum = 0.0;
        let mut amp = 1.0;
        let mut norm = 0.0;
        let mut fx = x;
        let mut fy = y;
        for _ in 0..octaves {
            sum += self.at(fx, fy) * amp;
            norm += amp;
            amp *= gain;
            fx *= lacunarity;
            fy *= lacunarity;
        }
        sum / norm
    }

    /// Ridged multifractal, in [0, 1]; good for mountain ranges.
    pub fn ridged(&self, x: f32, y: f32, octaves: u32) -> f32 {
        let mut sum = 0.0;
        let mut amp = 0.5;
        let mut fx = x;
        let mut fy = y;
        let mut weight = 1.0f32;
        for _ in 0..octaves {
            let mut n = 1.0 - self.at(fx, fy).abs();
            n *= n;
            n *= weight;
            weight = (n * 2.0).clamp(0.0, 1.0);
            sum += n * amp;
            amp *= 0.5;
            fx *= 2.0;
            fy *= 2.0;
        }
        sum.clamp(0.0, 1.0)
    }
}
