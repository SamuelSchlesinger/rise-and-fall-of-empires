//! What is moving, as distinct from what is there.
//!
//! Every layer of the map was a photograph. Where people live, who holds
//! what, what the ground yields — all of it true of one year and silent
//! about the year before, so the most interesting facts about a world were
//! the ones a reader had to infer by watching the screen and remembering.
//! A steppe emptying out and a steppe that has always been empty look the
//! same in a snapshot.
//!
//! So the world keeps a short memory of its own motion: four signed fields,
//! one value per cell, each a decaying running total of something that
//! happened rather than something that is. They answer questions a static
//! map cannot:
//!
//! * **Settling** — where people are arriving and where they are leaving.
//! * **Fighting** — where the war actually is, as opposed to who is
//!   nominally at war with whom.
//! * **Changing hands** — where the map is being redrawn.
//! * **Drift** — where the weather is turning, which is not stored at all
//!   but recomputed, because the climate is a pure function of the world's
//!   seed and the year.
//!
//! # Why a decaying total rather than a history
//!
//! A window over the last N years would need N snapshots of the map, which
//! for a world of forty thousand cells is megabytes of history to carry and
//! to save. A running total with a decay carries one number per cell and
//! answers the same question: recent motion counts for more than old
//! motion, and a quiet cell falls back to zero on its own.
//!
//! It also means these fields need no place in a save file. They are not
//! derived — you cannot recompute where people moved last century from
//! where they are now — but they *heal*: a world loaded from a file starts
//! with them empty and has them back within a lifetime of play. That is a
//! better bargain than a hundred and sixty kilobytes a field, and it is why
//! [`HALF_LIFE`] is a few decades rather than a few centuries.

use super::World;

/// How long a year's motion goes on counting, in years. After this much
/// time an event contributes half of what it did when it happened.
pub const HALF_LIFE: f32 = 24.0;

/// The per-year decay that gives [`HALF_LIFE`].
///
/// `0.5^(1/half_life)`, worked out once rather than per cell per year.
fn keep() -> f32 {
    0.5f32.powf(1.0 / HALF_LIFE)
}

/// A world's memory of its own motion.
#[derive(Default)]
pub struct Flows {
    /// Net people settled, per cell: positive where they arrive, negative
    /// where they leave.
    pub settled: Vec<f32>,
    /// Where the fighting has been.
    pub fought: Vec<f32>,
    /// Where ground has changed hands.
    pub changed: Vec<f32>,
}

impl Flows {
    /// Make room for a map of `n` cells, keeping whatever is already there.
    pub fn fit(&mut self, n: usize) {
        for v in [&mut self.settled, &mut self.fought, &mut self.changed] {
            if v.len() != n {
                v.resize(n, 0.0);
            }
        }
    }
}

impl World {
    /// Note that `amount` of people left `from` for `to`.
    ///
    /// Called from the migration step, which already knows both ends; the
    /// sign is what makes the layer readable, because a cell that is losing
    /// people and a cell that never had any are the same colour otherwise.
    pub fn note_settled(&mut self, from: usize, to: usize, amount: f32) {
        if self.flows.settled.len() != self.cells.len() {
            self.flows.fit(self.cells.len());
        }
        if let Some(v) = self.flows.settled.get_mut(from) {
            *v -= amount;
        }
        if let Some(v) = self.flows.settled.get_mut(to) {
            *v += amount;
        }
    }

    /// Note that something was fought over `cell`.
    pub fn note_fought(&mut self, cell: usize, weight: f32) {
        if self.flows.fought.len() != self.cells.len() {
            self.flows.fit(self.cells.len());
        }
        if let Some(v) = self.flows.fought.get_mut(cell) {
            *v += weight;
        }
    }

    /// Note that `cell` changed hands.
    pub fn note_changed(&mut self, cell: usize) {
        if self.flows.changed.len() != self.cells.len() {
            self.flows.fit(self.cells.len());
        }
        if let Some(v) = self.flows.changed.get_mut(cell) {
            *v += 1.0;
        }
    }

    /// How far a cell's weather has moved since living memory, in the same
    /// units as [`World::climate_mult`] minus one.
    ///
    /// Recomputed rather than remembered: the climate field is a pure
    /// function of the world's seed and the year, so the century-old field
    /// costs one noise sample per cell and nothing at all to carry. Cached
    /// for the epoch, because a whole map of samples is too much to redo
    /// sixty times a second.
    pub fn climate_drift(&self, i: usize) -> f32 {
        let now = self.climate.at.get(i).copied().unwrap_or(0.0);
        let then = self.climate.was_long_ago.get(i).copied().unwrap_or(now);
        (now - then) * self.tuning.climate_amplitude
    }
}

/// Let a year's motion fade. Run once a year, after the phases that add to
/// it, so that what a layer draws is the year just finished included.
pub fn tick(w: &mut World) {
    let n = w.cells.len();
    w.flows.fit(n);
    let k = keep();
    for v in [
        &mut w.flows.settled,
        &mut w.flows.fought,
        &mut w.flows.changed,
    ] {
        for x in v.iter_mut() {
            *x *= k;
            // Denormals cost more than they are worth and a cell that has
            // been quiet for two centuries is quiet.
            if x.abs() < 1e-4 {
                *x = 0.0;
            }
        }
    }
}
