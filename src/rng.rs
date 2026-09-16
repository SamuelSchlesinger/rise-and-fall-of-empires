//! Deterministic xoshiro256** PRNG with interior mutability so it can be
//! shared freely across split borrows of the world state. Cloning an `Rng`
//! shares the same stream rather than forking an identical one, so a handle
//! taken with `w.rng.clone()` and the world's own generator never draw the
//! same numbers.

use std::cell::Cell;
use std::rc::Rc;

/// A shared handle on one xoshiro256** stream.
#[derive(Clone)]
pub struct Rng {
    s: Rc<Cell<[u64; 4]>>,
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Rng {
    /// A fresh stream, its four words spread out of `seed` by `SplitMix64`.
    pub fn new(seed: u64) -> Rng {
        let mut st = seed;
        let s = [
            splitmix64(&mut st),
            splitmix64(&mut st),
            splitmix64(&mut st),
            splitmix64(&mut st),
        ];
        Rng {
            s: Rc::new(Cell::new(s)),
        }
    }

    /// The generator's whole state, for saving a world mid-history.
    pub fn state(&self) -> [u64; 4] {
        self.s.get()
    }

    /// Put a saved state back, so the stream carries on where it left off.
    pub fn set_state(&self, s: [u64; 4]) {
        self.s.set(s);
    }

    /// The next 64 raw bits. Every other method is built on this one.
    pub fn next_u64(&self) -> u64 {
        let mut s = self.s.get();
        let result = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        self.s.set(s);
        result
    }

    /// Uniform in `[0, 1)`.
    pub fn f64(&self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform in `[0, 1)`, narrowed to `f32`.
    pub fn f32(&self) -> f32 {
        self.f64() as f32
    }

    /// Uniform in `[lo, hi)`.
    pub fn range(&self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.f64()
    }

    /// Uniform in `[lo, hi)`, narrowed to `f32`.
    pub fn range32(&self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }

    /// Uniform integer in `[0, n)`.
    ///
    /// # Panics
    ///
    /// If `n` is 0: there is no integer to return.
    pub fn below(&self, n: usize) -> usize {
        assert!(n > 0, "Rng::below(0): no integer below zero");
        (self.next_u64() % n as u64) as usize
    }

    /// Uniform integer in `[lo, hi]` inclusive; `lo` if the range is empty.
    pub fn int(&self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + self.below((hi - lo + 1) as usize) as i32
    }

    /// True with probability `p`.
    pub fn chance(&self, p: f64) -> bool {
        self.f64() < p
    }

    /// One element of `xs`, uniformly.
    ///
    /// # Panics
    ///
    /// If `xs` is empty. Callers that build a list of candidates must check
    /// it first; every one in this tree does.
    pub fn pick<'a, T>(&self, xs: &'a [T]) -> &'a T {
        assert!(!xs.is_empty(), "Rng::pick on an empty slice");
        &xs[self.below(xs.len())]
    }

    /// Pick an index weighted by `w` (weights must be non-negative).
    pub fn weighted(&self, w: &[f64]) -> usize {
        let total: f64 = w.iter().sum();
        if total <= 0.0 {
            return self.below(w.len().max(1));
        }
        let mut r = self.f64() * total;
        for (i, &x) in w.iter().enumerate() {
            if r < x {
                return i;
            }
            r -= x;
        }
        w.len() - 1
    }

    /// Fisher-Yates, in place.
    pub fn shuffle<T>(&self, xs: &mut [T]) {
        for i in (1..xs.len()).rev() {
            let j = self.below(i + 1);
            xs.swap(i, j);
        }
    }

    /// Approximately standard normal (Box-Muller).
    pub fn normal(&self) -> f64 {
        let u1 = self.f64().max(1e-12);
        let u2 = self.f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }

    /// Value in `[0, 1]` clamped from a normal around `mean` with `sd`.
    pub fn trait_value(&self, mean: f64, sd: f64) -> f32 {
        (mean + self.normal() * sd).clamp(0.0, 1.0) as f32
    }
}
