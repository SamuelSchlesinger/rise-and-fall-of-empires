//! Weather that lasts longer than a lifetime.
//!
//! The terrain is immutable, which is right for elevation and rivers and
//! wrong for rainfall: a map generated once and never touched again means
//! that the steppe is exactly as dry in year nine thousand as it was in year
//! one, and no region ever has a bad century. Every famine in the world was
//! a die roll rather than a climate.
//!
//! So a slow field drifts over the map: bands of wet and dry years that move
//! and turn, on a scale of centuries rather than seasons. It does one thing
//! — it moves the carrying capacity of land up and down — and everything
//! else follows from that on its own, because the simulation already knows
//! what to do when land stops feeding people. They leave.
//!
//! That is the mechanism this exists for. A wet century pushes farming out
//! into the margins; a dry one pushes the margins back onto the farmers, and
//! the people who live where the grass fails are already the ones with
//! horses. It is the engine behind most of the steppe's history and it gives
//! [`crate::sim::PolityKind::Horde`] a reason to exist beyond a die roll.
//!
//! Cost: one noise sample per cell, every [`STEP`] years, cached in between.

use super::chronicle::{EventKind, Ref};
use super::prose::{self, Pick};
use super::World;
use crate::noise::Noise;
use crate::rng::Rng;

/// How often the field is worked out again. A climate that changed yearly
/// would be weather; the point is that it changes over lifetimes.
pub const STEP: i32 = 8;

/// How far the field drifts across the map each year. Slow enough that a
/// band takes centuries to cross a continent.
const DRIFT: f32 = 0.0045;

/// The spatial scale of a wet or dry band, in cells.
const BAND: f32 = 26.0;

/// A world's climate: how kind each cell is being, in roughly `[-1, 1]`.
///
/// Rebuilt from noise rather than stored, because it is a pure function of
/// the world's noise field and the year.
pub struct Climate {
    /// The drifting field, one value per cell.
    pub at: Vec<f32>,
    /// The world's average, for the naming of ages: below zero is a cold,
    /// dry stretch of centuries and above it a kind one.
    pub mean: f32,
    /// What it was last time, so a turn can be noticed.
    pub was: f32,
}

impl Default for Climate {
    fn default() -> Climate {
        Climate {
            at: Vec::new(),
            mean: 0.0,
            was: 0.0,
        }
    }
}

/// The noise a world's weather is drawn from. Built once from the world's
/// own seed so that reloading a save gives back the same centuries.
pub fn field(seed: u64) -> Noise {
    Noise::new(&Rng::new(seed ^ 0x005E_A50Fu64))
}

/// The stretch of years the field currently stands for. Everything is
/// worked out for an epoch rather than for a year, which is what lets the
/// whole field be rebuilt from nothing at any moment — including halfway
/// through an epoch, after a save is loaded.
fn epoch_of(year: i32) -> i32 {
    year - year.rem_euclid(STEP)
}

/// The field as it stands in a given epoch, and its average over land.
fn compute(w: &World, epoch: i32) -> (Vec<f32>, f32) {
    let n = w.cells.len();
    let noise = field(w.seed);
    let t = epoch as f32 * DRIFT;
    let mut at = vec![0.0f32; n];
    let mut total = 0.0f32;
    let mut land = 0usize;
    for (i, slot) in at.iter_mut().enumerate() {
        if !w.terrain.is_land(i) {
            continue;
        }
        let (x, y) = w.terrain.xy(i);
        // The field drifts north and east as the centuries pass, so a band
        // that sat over the grasslands moves off them.
        let v = noise.fbm(
            (x as f32 + t * 40.0) / BAND,
            (y as f32 + t * 17.0) / BAND,
            3,
            2.0,
            0.5,
        );
        *slot = v.clamp(-1.0, 1.0);
        total += *slot;
        land += 1;
    }
    (at, if land > 0 { total / land as f32 } else { 0.0 })
}

/// Put the field where it should be for the world's current year.
///
/// Called on load as well as in the tick, which is why it works from the
/// epoch rather than from whatever happened last: a world reloaded three
/// years into an epoch has to get back exactly the field it had, and a
/// derived thing that can only be built at a boundary is not derived at all.
pub fn rebuild(w: &mut World) {
    let epoch = epoch_of(w.year);
    let (at, mean) = compute(w, epoch);
    let (_, was) = if epoch >= STEP {
        compute(w, epoch - STEP)
    } else {
        (Vec::new(), mean)
    };
    w.climate.at = at;
    w.climate.mean = mean;
    w.climate.was = was;
}

/// Work the field out again, and say so if the age has turned.
pub fn tick(w: &mut World) {
    if w.year % STEP != 0 && w.climate.at.len() == w.cells.len() {
        return;
    }
    rebuild(w);
    notice_a_turn(w);
}

/// Tell the world when the weather over some inhabited place has plainly
/// changed since living memory.
///
/// Compared against the field a century ago rather than against the last
/// epoch, and per *region* rather than for the world.
///
/// Both of those were wrong at first and nothing fired at all. The world's
/// average over a drifting field is very nearly constant — one place dries
/// while another wets — and eight years of drift is a cell and a half, which
/// is nothing. What a chronicler notices is that the country their
/// grandfather farmed is not farmable now.
fn notice_a_turn(w: &mut World) {
    let rng = w.rng.clone();
    let epoch = epoch_of(w.year);
    // Once a lifetime, not once every epoch. The comparison is against a
    // rolling century, so a region that has genuinely turned goes on
    // qualifying for as long as the change stands — and said so every eight
    // years in the same words.
    if epoch < MEMORY || epoch % MEMORY != 0 {
        return;
    }
    let (then, _) = compute(w, epoch - MEMORY);
    // Which inhabited, named region has moved furthest in a lifetime.
    let mut worst: Option<(usize, f32, f32)> = None;
    for (fi, f) in w.terrain.features.iter().enumerate() {
        if f.name.is_none() || f.cells.len() < 12 {
            continue;
        }
        let sample = f.cells.len().min(60);
        let mut now_sum = 0.0;
        let mut then_sum = 0.0;
        let mut peopled = false;
        for &c in f.cells.iter().take(sample) {
            now_sum += w.climate.at.get(c).copied().unwrap_or(0.0);
            then_sum += then.get(c).copied().unwrap_or(0.0);
            if w.cells[c].pop > 0.05 {
                peopled = true;
            }
        }
        if !peopled {
            continue;
        }
        let now = now_sum / sample as f32;
        let change = now - then_sum / sample as f32;
        if worst
            .map(|(_, _, b)| change.abs() > b.abs())
            .unwrap_or(true)
        {
            worst = Some((fi, now, change));
        }
    }
    let Some((fi, level, change)) = worst else {
        return;
    };
    if change.abs() < w.tuning.climate_notice {
        return;
    }
    let name = w.terrain.features[fi].display();
    let text = prose::climate_turned(&name, level, change > 0.0, &Pick::rolled(&rng));
    let loc = {
        let c = w.terrain.features[fi].center;
        Some(w.terrain.idx(c.0, c.1))
    };
    w.log(1, EventKind::Disaster, &[Ref::Feature(fi)], loc, text);
}

/// How far back "living memory" reaches, for deciding that a region's
/// weather has turned. Rounded to whole epochs.
const MEMORY: i32 = 96;

impl World {
    /// What the weather is doing to a cell's harvests, as a multiplier.
    ///
    /// The only thing climate does. Everything else — famine, migration,
    /// nomads coming off a failed steppe — the simulation already knows how
    /// to do once the land stops feeding people.
    pub fn climate_mult(&self, i: usize) -> f32 {
        let v = self.climate.at.get(i).copied().unwrap_or(0.0);
        (1.0 + v * self.tuning.climate_amplitude).clamp(0.35, 1.7)
    }
}
