//! What the world was, decade by decade.
//!
//! The simulation keeps a great many numbers about the present and almost
//! nothing about the past. `Stats` is recomputed every year and overwritten;
//! the peaks remember a high-water mark and not when the tide was anywhere
//! else; the chronicle remembers *events* and cannot say what the population
//! was in the eighth century. So the one question a reader of a world most
//! obviously wants to ask — what happened to it over time — had no answer at
//! all, and the only series ever collected, `Stats::pop_history`, was never
//! drawn anywhere.
//!
//! # Why this is stored when `flows` is not
//!
//! [`crate::sim::flows`] argues at length *against* keeping history, and it
//! is right about what it is talking about: a window over the last N years
//! of a forty-thousand-cell map is megabytes to carry and to save, so it
//! keeps a decaying total instead and lets it heal after a load.
//!
//! The calculus is four orders of magnitude different here. A sample is one
//! world, not one cell: eleven numbers every ten years, which is under a
//! kilobyte per millennium and about eight for the longest game anybody will
//! play. And unlike a flow field it cannot heal — a world loaded without its
//! history has lost that history for good, because no amount of running
//! forward recovers what the population was before the save. So it is
//! stored, in a chunk of its own.

use super::{explain, World};

/// How often the world is measured. Ten years is the cadence `pop_history`
/// already used, and it is short enough to catch a plague and long enough
/// that ten thousand years is a thousand samples.
pub const SAMPLE_EVERY: i32 = 10;

/// One reading of the whole world.
#[derive(Clone, Copy, Default)]
pub struct Sample {
    pub year: i32,
    pub pop: f32,
    pub realms: f32,
    pub cities: f32,
    pub wars: f32,
    pub schools: f32,
    pub trade: f32,
    pub dev: f32,
    pub stability: f32,
    pub treasury: f32,
    pub income: f32,
    pub decadence: f32,
}

/// A series: what it is called, and how to read it out of a sample.
pub type Series = (&'static str, fn(&Sample) -> f32);

/// Every series, named, with how to read it out of a sample.
///
/// One table, so the chart page and the CSV export cannot disagree about
/// what the world records — the same bargain `query::fields_for` makes for
/// the list pages. The order is the order they are drawn in.
pub const SERIES: [Series; 11] = [
    ("people", |s| s.pop),
    ("realms", |s| s.realms),
    ("cities", |s| s.cities),
    ("wars", |s| s.wars),
    ("schools", |s| s.schools),
    ("trade", |s| s.trade),
    ("development", |s| s.dev),
    ("stability", |s| s.stability),
    ("treasury", |s| s.treasury),
    ("income", |s| s.income),
    ("decadence", |s| s.decadence),
];

/// A world's own record of itself.
#[derive(Default)]
pub struct History {
    pub samples: Vec<Sample>,
    /// How much of the chronicle has been forgotten to compaction.
    ///
    /// It lives here rather than on the chronicle because the chronicle is
    /// rebuilt from its events on load, through `Chronicle::default`, which
    /// resets the count to nothing — so a world that had forgotten half a
    /// million events came back claiming to have forgotten none. Carried
    /// here, it survives.
    pub forgotten: usize,
}

impl History {
    /// The year of the first sample and of the last, if there are any.
    pub fn span(&self) -> Option<(i32, i32)> {
        Some((self.samples.first()?.year, self.samples.last()?.year))
    }
}

/// Measure the world, if this is a year for it.
///
/// Called at the end of the tick, after `recompute` has brought every
/// aggregate up to date. Everything read here is either already computed or
/// a single pass over the living, of which there are a few hundred — so a
/// sample costs about what one ordinary year of one realm costs, once a
/// decade.
///
/// Deliberately *not* reading the `century_*` counters: `events::eras` runs
/// between `recompute` and here and zeroes them, so on every hundredth year
/// — one sample in ten — they would read as nothing at all.
pub fn tick(w: &mut World) {
    if w.year % SAMPLE_EVERY != 0 {
        return;
    }
    let alive: Vec<usize> = w.alive_polities.clone();
    let n = alive.len().max(1) as f32;
    let mean = |f: &dyn Fn(usize) -> f32| -> f32 { alive.iter().map(|&p| f(p)).sum::<f32>() / n };

    let mut purses: Vec<f32> = alive.iter().map(|&p| w.polities[p].treasury).collect();
    purses.sort_by(f32::total_cmp);
    let median = purses.get(purses.len() / 2).copied().unwrap_or(0.0);

    let sample = Sample {
        year: w.year,
        pop: w.stats.pop as f32,
        realms: alive.len() as f32,
        cities: w.stats.cities_alive as f32,
        wars: w.alive_wars.len() as f32,
        schools: w.stats.schools_alive as f32,
        // The cross-border total, not the sum of what each city takes:
        // `trade::reckon` credits a route to both of its ends, so adding up
        // the cities counts every road twice.
        trade: w.pair_trade.values().sum(),
        dev: mean(&|p| w.polities[p].dev),
        stability: mean(&|p| w.polities[p].stability),
        treasury: median,
        income: mean(&|p| explain::income_total(w, p)),
        decadence: mean(&|p| w.polities[p].decadence),
    };
    w.history.samples.push(sample);
    w.history.forgotten = w.chronicle.dropped;
}
