//! The Covenant: what the watcher is, what it costs to act, and where the
//! power to act comes from.
//!
//! Everything else in this tree simulates a world that does not know anyone
//! is watching. This module is the one place the watcher exists in it.
//!
//! The problem it solves is that the Hand of Fate was a cheat menu. Twenty
//! four interventions, every one of them free, every one of them unlimited,
//! with no reason to choose between them and no reason not to use all of
//! them at once. A thing you can do as often as you like is not a decision,
//! and a world you can rewrite at no cost is not a world you can care
//! about. The cure is the oldest one there is: make it cost something, and
//! make what it costs come from somewhere.
//!
//! So: you bind yourself to a people. Their lives are your strength —
//! [`Fate::devotion`] is drawn from how many of them there are and how well
//! they are doing — and that strength is what you spend when you reach into
//! the world. Which gives every act three properties it did not have:
//!
//! * **A price**, so that saving a city means not raising a wonder.
//! * **A source**, so that helping your own people is how you stay able to
//!   help anybody, and spending on a stranger is a real sacrifice.
//! * **A stake**, because a people can be conquered, scattered and
//!   forgotten — and when the last of them is gone the altars go cold and
//!   what you had drains away. That is the losing condition, and it is one
//!   the simulation was already capable of dealing you.
//!
//! Binding is optional. An unbound watcher still gathers a thin trickle from
//! the world at large, which keeps the original sandbox playable for anyone
//! who wants to watch and occasionally nudge; it is simply much poorer than
//! a watcher with a people.

use super::chronicle::{EventKind, Ref};
use super::World;

/// What the watcher is and has.
#[derive(Clone, Debug, Default)]
pub struct Fate {
    /// The people who keep the altars, if any.
    pub patron: Option<usize>,
    /// The year the covenant was made.
    pub bound: i32,
    /// Power in hand, spendable now.
    pub power: f32,
    /// What the last year added, kept so the sidebar can show a rate
    /// without recomputing the whole world to do it.
    pub income: f32,
    /// Everything ever spent, and how many times.
    pub spent: f32,
    pub acts: u32,
    /// The most ever held at once, for the epitaph.
    pub high: f32,
    /// The year the last of the bound people died, if they have.
    pub forsaken: Option<i32>,
}

/// The most that can be held at once.
///
/// A ceiling rather than a purse without a bottom, because a watcher who
/// can bank a thousand years of devotion is back to having no decision to
/// make: they simply wait, and then do everything. It is generous enough to
/// afford the heaviest act twice over and mean enough that a century of
/// hoarding is wasted.
pub const CEILING: f32 = 120.0;

// The two rates live in `Tuning` so that anybody who wants the old sandbox
// back — twenty-four free acts, as often as you like — can have it with one
// line in the config: `tune.fate_trickle = 60`. That was the whole of this
// game before there was a covenant, and some people came for it.

impl Fate {
    /// Whether there is power enough for something that costs `cost`.
    pub fn can(&self, cost: f32) -> bool {
        self.power + 1e-3 >= cost
    }

    /// Take the price of an act. Returns false and takes nothing if there
    /// is not enough, so a caller can use it as the guard as well as the
    /// payment.
    pub fn pay(&mut self, cost: f32) -> bool {
        if !self.can(cost) {
            return false;
        }
        self.power -= cost;
        self.spent += cost;
        self.acts += 1;
        true
    }
}

/// What kind of thing an act is, and so what kind of god it suits.
///
/// Four, because the twenty-four acts really do fall into four piles: things
/// that make more of something, things that end something, things that are
/// built, and things that set people against each other.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Domain {
    /// Harvests, booms, births, good earth: more of what is already there.
    Growth,
    /// Fire, plague, blight, the knife: less of it.
    Ruin,
    /// Walls, wonders, roads, ore, treasuries, brilliance.
    Craft,
    /// War-whispers, ambition, disgrace, a throne emptied.
    Strife,
}

impl Domain {
    /// What a god of this kind is called, for the card and the sidebar.
    pub fn title(self) -> &'static str {
        match self {
            Domain::Growth => "a god of the harvest",
            Domain::Ruin => "a god of endings",
            Domain::Craft => "a god of the work of hands",
            Domain::Strife => "a god of strife",
        }
    }

    /// What comes cheap to one, in the fewest words that say it.
    pub fn cheap(self) -> &'static str {
        match self {
            Domain::Growth => "harvests and births come cheap",
            Domain::Ruin => "fire and plague come cheap",
            Domain::Craft => "walls, wonders and roads come cheap",
            Domain::Strife => "war and ambition come cheap",
        }
    }
}

/// Which pile each of the six acts on each kind of thing falls into.
///
/// Read against the menus in `ui::detail`, in the same order.
pub fn domain_of(target: Ref, choice: u8) -> Domain {
    use Domain::{Craft, Growth, Ruin, Strife};
    let six = match target {
        // omen, dark omen, vision, gift, war-whisper, quiet death
        Ref::Polity(_) => [Growth, Ruin, Craft, Craft, Strife, Strife],
        // boom, fire, walls, wonder, sickness, road
        Ref::City(_) => [Growth, Ruin, Craft, Craft, Ruin, Craft],
        // renown, ruin, brilliance, ambition, a knife, an heir
        Ref::Person(_) => [Growth, Strife, Craft, Strife, Ruin, Growth],
        // good earth, exhaustion, blight, a lode, a quickening, emptying
        Ref::Feature(_) => [Growth, Ruin, Ruin, Craft, Craft, Ruin],
        _ => [Growth; 6],
    };
    six[(choice as usize).min(5)]
}

/// What a people makes of the thing that watches them.
///
/// Derived rather than chosen, so that picking a people is one decision with
/// two consequences instead of two questions on the way in. It also makes
/// the opening choice a real one: without this, the only sane answer to
/// "whose god will you be?" is "whichever people is largest".
pub fn aspect_of(w: &World, culture: usize) -> Domain {
    if culture >= w.cultures.len() {
        return Domain::Growth;
    }
    let v = w.cultures[culture].values;
    // Strife and ruin are not the same thing: a martial people makes a god
    // of war, and one that keeps to itself and fears the world outside
    // makes a god of endings.
    let scores = [
        (
            Domain::Growth,
            0.35 + v.openness * 0.5 + (1.0 - v.militarism) * 0.3,
        ),
        (Domain::Ruin, 0.2 + v.mysticism * 0.7 + v.tradition * 0.3),
        (
            Domain::Craft,
            0.25 + v.mercantilism * 0.8 + v.openness * 0.2,
        ),
        (Domain::Strife, 0.1 + v.militarism * 1.1),
    ];
    scores
        .iter()
        .copied()
        .fold((Domain::Growth, f32::MIN), |best, x| {
            if x.1 > best.1 {
                x
            } else {
                best
            }
        })
        .0
}

/// How much dearer an act outside your nature is, and how much cheaper one
/// within it.
const IN_DOMAIN: f32 = 0.6;
const OUT_OF_DOMAIN: f32 = 1.25;

/// What an act costs this watcher, which is not the same as what it costs
/// in the abstract.
pub fn price(w: &World, target: Ref, choice: u8) -> f32 {
    let base = cost_of(target, choice);
    let Some(c) = w.fate.patron else {
        return base;
    };
    let scale = if domain_of(target, choice) == aspect_of(w, c) {
        IN_DOMAIN
    } else {
        OUT_OF_DOMAIN
    };
    (base * scale).round()
}

/// The price of each intervention, by what it is worked on and which of the
/// six it is.
///
/// Priced by how far the world is moved rather than by how dramatic the
/// sentence is: raising walls on one town is a small thing however loudly
/// the chronicle says it, and emptying a region is not. The numbers are
/// against [`CEILING`], so a heavy act is roughly half of everything a
/// full watcher has.
pub fn cost_of(target: Ref, choice: u8) -> f32 {
    match target {
        Ref::Polity(_) => [46.0, 52.0, 30.0, 34.0, 26.0, 40.0][(choice as usize).min(5)],
        Ref::City(_) => [24.0, 30.0, 14.0, 34.0, 36.0, 20.0][(choice as usize).min(5)],
        Ref::Person(_) => [10.0, 16.0, 18.0, 14.0, 28.0, 22.0][(choice as usize).min(5)],
        Ref::Feature(_) => [26.0, 30.0, 44.0, 24.0, 20.0, 38.0][(choice as usize).min(5)],
        _ => 20.0,
    }
}

/// How the bound people are faring, as a share of the world, in `[0, 1]`.
///
/// Population rather than land, because a covenant is with people and not
/// with dirt, and because a people driven into the hills still keeps its
/// altars while an empty province does not.
pub fn devotion(w: &World) -> f32 {
    let Some(c) = w.fate.patron else {
        return 0.0;
    };
    if c >= w.cultures.len() || w.cultures[c].extinct.is_some() {
        return 0.0;
    }
    let total = w.stats.pop.max(1.0);
    let theirs = w.cultures[c].pop;
    // Clamped, because `stats.pop` is a derived figure refreshed by
    // `recompute` and can lag a culture's own count in the first year of a
    // world — which made a share of eleven, and an income to match.
    ((theirs / total) as f32).clamp(0.0, 1.0)
}

/// A year's gathering, and the fate of a covenant whose people are gone.
pub fn tick(w: &mut World) {
    let share = devotion(w);
    let bound = w.fate.patron.is_some();
    // A people's first century is thin and a people that holds a fifth of
    // the world is not five times as generous as one that holds a
    // twentieth: the root flattens it, so that a small nation is worth
    // playing for and a large one is not a licence to do as you please.
    let income = if bound {
        w.tuning.fate_trickle * 0.4 + w.tuning.fate_covenant * share.sqrt()
    } else {
        w.tuning.fate_trickle
    };
    w.fate.income = income;
    w.fate.power = (w.fate.power + income).min(CEILING);
    if w.fate.power > w.fate.high {
        w.fate.high = w.fate.power;
    }
    // The altars go cold. A covenant does not survive the people who made
    // it, and what is left drains rather than vanishing, so that a watcher
    // who has just lost everything still has one last thing they can do
    // with it.
    if bound && share <= 0.0 {
        if w.fate.forsaken.is_none() {
            w.fate.forsaken = Some(w.year);
            let name = w
                .fate
                .patron
                .map(|c| w.cultures[c].plural.clone())
                .unwrap_or_default();
            let text = format!(
                "The last of the {} were gone, and there was no one left who \
                 remembered what the altars had been for. What was promised \
                 there began to drain away.",
                name
            );
            let refs: Vec<Ref> = Vec::new();
            w.log(3, EventKind::Death, &refs, None, text);
        }
        w.fate.power = (w.fate.power - 1.2).max(0.0);
    }
}

/// Make the covenant: bind the watcher to a people and say so in the
/// chronicle, which is the one line in the whole history that is about the
/// reader.
pub fn bind(w: &mut World, culture: usize) {
    if culture >= w.cultures.len() {
        return;
    }
    w.fate.patron = Some(culture);
    w.fate.bound = w.year;
    w.fate.forsaken = None;
    let text = format!(
        "Something without a name took an interest in the {}, and they began \
         to leave out offerings for it. Neither party wrote down the terms.",
        w.cultures[culture].plural
    );
    let at = w.cultures[culture].home;
    w.log(
        3,
        EventKind::Founding,
        &[Ref::Culture(culture)],
        Some(at),
        text,
    );
}
