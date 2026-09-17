//! What the world works out, and does not forget when the realm that
//! learned it falls.
//!
//! Everything else in the simulation cycles. Realms rise and break, houses
//! run out, faiths spread and fade, and the map at year fifteen hundred is
//! arranged differently from the map at year three hundred but is not
//! different *in kind*. That is because nothing accumulated:
//! [`crate::sim::Polity::dev`] was the only thing that went up, it was held
//! by the realm rather than by the ground, and it died with it. Every new
//! realm started from nothing and climbed the same short hill.
//!
//! A ratchet is state that goes one way and **outlives whoever built it**.
//! Knowledge here is held by *cells*, so when a realm falls the knowledge
//! stays in the ground and the conqueror inherits it. That is what gives a
//! history direction instead of weather.
//!
//! # The tree is grown, not written
//!
//! Every world grows its own. A fixed table would mean every world worked
//! out the same things in the same order, which is the sameness this module
//! exists to cure — and a hand-written two dozen were known everywhere by
//! year eight hundred, after which the ratchet went inert again. So the tree
//! is built from the world's seed, the way its languages, peoples and places
//! already are: a hundred-odd innovations, named in their own idiom,
//! arranged in tiers, each needing what came before it.
//!
//! What is *universal* is the vocabulary of consequences — [`Effect`]. The
//! simulation has to read a tree it did not write, so the shapes an effect
//! can take are fixed even though which innovation carries which, what it is
//! called, and when it arrives are not. Every world eventually works out
//! something that makes walls stop mattering; no two worlds call it the same
//! thing or reach it in the same century.
//!
//! Three things gate discovery:
//!
//! * **Place.** An innovation appears where it plausibly would — irrigation
//!   on watered land, smelting where there is ore, ley-craft where the lines
//!   run thick — so geography keeps mattering long after worldgen.
//! * **Prerequisite.** Its parents must already be known in that realm.
//! * **Surplus.** A city must be able to support it. This is what paces a
//!   tree across ten thousand years rather than four hundred: what gates
//!   invention is not time but people fed well enough that some of them can
//!   do something other than farm.
//!
//! Knowledge can also be lost. Ground whose people are gone forgets, which
//! is how a dark age happens — and, centuries later, how something is worked
//! out a second time.

use super::chronicle::{EventKind, Ref};
use super::prose::{self, Pick};
use super::{SchoolKind, World};
use crate::geo::Biome;
use crate::rng::Rng;

/// The most innovations a world can have. Knowledge is a `u128` per cell,
/// which is the whole memory cost: sixteen bytes of ground truth per cell,
/// about 160 KB on a default map.
pub const MAX_TECHS: usize = 128;

/// A branch of knowledge. Universal, because the *kinds* of thing a world
/// can learn are the same everywhere; what it learns within them is not.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Field {
    Husbandry,
    Metalcraft,
    Building,
    Seafaring,
    Statecraft,
    Warcraft,
    Leycraft,
    Physic,
    Letters,
    Reckoning,
}

pub const FIELDS: [Field; 10] = [
    Field::Husbandry,
    Field::Metalcraft,
    Field::Building,
    Field::Seafaring,
    Field::Statecraft,
    Field::Warcraft,
    Field::Leycraft,
    Field::Physic,
    Field::Letters,
    Field::Reckoning,
];

impl Field {
    pub fn name(self) -> &'static str {
        match self {
            Field::Husbandry => "husbandry",
            Field::Metalcraft => "metalcraft",
            Field::Building => "building",
            Field::Seafaring => "seafaring",
            Field::Statecraft => "statecraft",
            Field::Warcraft => "warcraft",
            Field::Leycraft => "ley-craft",
            Field::Physic => "physic",
            Field::Letters => "letters",
            Field::Reckoning => "reckoning",
        }
    }
}

/// What an innovation does to the world.
///
/// Fixed, because the simulation reads it. The magnitude is not: it rises
/// with the tier an innovation sits at, so a late advance in a field is
/// worth more than the first fumbling one.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Effect {
    /// The land feeds more people.
    Yield(f32),
    /// Armies fight better.
    Arms(f32),
    /// Walls can be brought down.
    Siegecraft(f32),
    /// Walls stand longer.
    Ramparts(f32),
    /// More land can be governed.
    Administration(f32),
    /// More of what is produced reaches the treasury.
    Revenue(f32),
    /// The realm reaches further overland.
    Roadcraft(f32),
    /// The realm reaches further over water.
    Seacraft(f32),
    /// Cities are richer.
    Prosperity(f32),
    /// Schools of thought carry further.
    Doctrine(f32),
    /// The realm holds together better.
    Order(f32),
    /// Fewer people die of what kills people.
    Health(f32),
}

impl Effect {
    /// The magnitude, whatever the kind.
    pub fn size(self) -> f32 {
        match self {
            Effect::Yield(v)
            | Effect::Arms(v)
            | Effect::Siegecraft(v)
            | Effect::Ramparts(v)
            | Effect::Administration(v)
            | Effect::Revenue(v)
            | Effect::Roadcraft(v)
            | Effect::Seacraft(v)
            | Effect::Prosperity(v)
            | Effect::Doctrine(v)
            | Effect::Order(v)
            | Effect::Health(v) => v,
        }
    }
    /// A short word for the interface.
    pub fn label(self) -> &'static str {
        match self {
            Effect::Yield(_) => "harvests",
            Effect::Arms(_) => "arms",
            Effect::Siegecraft(_) => "siegecraft",
            Effect::Ramparts(_) => "ramparts",
            Effect::Administration(_) => "administration",
            Effect::Revenue(_) => "revenue",
            Effect::Roadcraft(_) => "roads",
            Effect::Seacraft(_) => "seamanship",
            Effect::Prosperity(_) => "wealth",
            Effect::Doctrine(_) => "doctrine",
            Effect::Order(_) => "order",
            Effect::Health(_) => "health",
        }
    }
}

/// Where an innovation can first be worked out.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Ground {
    Anywhere,
    Watered,
    Open,
    Highland,
    Ore,
    Shore,
    Ley,
    Town,
    GreatCity,
}

pub const GROUNDS: [Ground; 9] = [
    Ground::Anywhere,
    Ground::Watered,
    Ground::Open,
    Ground::Highland,
    Ground::Ore,
    Ground::Shore,
    Ground::Ley,
    Ground::Town,
    Ground::GreatCity,
];

impl Ground {
    /// Whether the cell suits it.
    fn suits(self, w: &World, i: usize) -> bool {
        let t = &w.terrain;
        let city_pop = w.cells[i]
            .city
            .filter(|&c| w.cities[c].destroyed.is_none())
            .map(|c| w.cities[c].pop)
            .unwrap_or(0.0);
        match self {
            Ground::Anywhere => true,
            Ground::Watered => {
                t.river[i] > 0 || matches!(t.biome[i], Biome::Swamp) || t.moist[i] > 0.6
            }
            Ground::Open => matches!(
                t.biome[i],
                Biome::Grassland | Biome::Steppe | Biome::Savanna | Biome::Forest
            ),
            Ground::Highland => matches!(t.biome[i], Biome::Hills | Biome::Mountain | Biome::Peak),
            Ground::Ore => t.minerals[i] > 0.35,
            Ground::Shore => t.coast[i],
            Ground::Ley => t.mana[i] > 0.55,
            Ground::Town => city_pop > 0.0,
            Ground::GreatCity => city_pop >= 15.0,
        }
    }
}

/// One thing a world can learn.
#[derive(Clone, Debug)]
pub struct Innovation {
    pub id: usize,
    pub name: String,
    pub field: Field,
    /// Depth in the tree. Difficulty and magnitude both rise with it.
    pub tier: u8,
    /// What must be known in the realm first.
    pub needs: Vec<usize>,
    pub ground: Ground,
    pub effect: Effect,
    /// How much a place must be able to support to work it out.
    pub difficulty: f32,
    /// How readily it travels once somebody has it.
    pub spread: f32,
}

impl Innovation {
    /// Its place in the knowledge bitset.
    pub fn bit(&self) -> u128 {
        1u128 << self.id
    }
    /// Whether this is one an age might be named for: a big effect of the
    /// kind that changes what everything else is worth.
    pub fn is_landmark(&self) -> bool {
        matches!(
            self.effect,
            Effect::Siegecraft(_) | Effect::Administration(_) | Effect::Seacraft(_)
        ) && self.effect.size() >= 0.22
    }
}

// ---------------------------------------------------------------------------
// Growing a tree
// ---------------------------------------------------------------------------

/// Words an innovation's name is built from, per field.
///
/// The grammar is combinatorial rather than generative: every part is
/// written by hand and only the *combination* is drawn, which is the bargain
/// `lang.rs` already makes and the reason a generated name still reads like
/// something a chronicler would write.
struct Lexicon {
    stuff: &'static [&'static str],
    craft: &'static [&'static str],
    adj: &'static [&'static str],
    thing: &'static [&'static str],
    process: &'static [&'static str],
}

fn lexicon(f: Field) -> Lexicon {
    match f {
        Field::Husbandry => Lexicon {
            stuff: &["seed", "root", "graft", "beast", "orchard", "fallow"],
            craft: &["tending", "breeding", "grafting", "raising", "husbanding"],
            adj: &["heavy", "iron-shod", "wheeled", "deep", "broad", "banked"],
            thing: &[
                "plough", "harrow", "sickle", "granary", "terrace", "sluice", "dyke", "millrace",
            ],
            process: &[
                "crop rotation",
                "the three-field year",
                "irrigation",
                "terracing",
                "selective breeding",
                "the drained field",
                "the seed drill",
                "silage",
            ],
        },
        Field::Metalcraft => Lexicon {
            stuff: &["bronze", "iron", "steel", "brass", "tin", "quench"],
            craft: &[
                "working",
                "smelting",
                "casting",
                "drawing",
                "forging",
                "tempering",
            ],
            adj: &["banked", "blast", "twin", "cold", "folded", "crucible"],
            thing: &[
                "furnace",
                "bellows",
                "forge",
                "crucible",
                "anvil",
                "die",
                "draw-plate",
            ],
            process: &[
                "the lost-wax cast",
                "wire-drawing",
                "case-hardening",
                "pattern-welding",
                "the drawn nail",
            ],
        },
        Field::Building => Lexicon {
            stuff: &["stone", "brick", "lime", "timber", "slate", "mortar"],
            craft: &["dressing", "vaulting", "coursing", "raising", "spanning"],
            adj: &[
                "dressed",
                "true",
                "pointed",
                "ribbed",
                "buttressed",
                "banked",
            ],
            thing: &[
                "arch", "vault", "aqueduct", "cistern", "span", "pier", "dome", "culvert",
            ],
            process: &[
                "the load-bearing arch",
                "the surveyed road",
                "the drained foundation",
                "the paved way",
                "the tiled roof",
            ],
        },
        Field::Seafaring => Lexicon {
            stuff: &["keel", "hull", "sail", "rope", "pitch", "oar"],
            craft: &["laying", "rigging", "caulking", "shaping", "setting"],
            adj: &["deep", "carvel", "lateen", "double", "steering", "weather"],
            thing: &[
                "hull",
                "keel",
                "rudder",
                "sail",
                "harbour",
                "mole",
                "lead-line",
                "compass",
            ],
            process: &[
                "the sea-chart",
                "dead reckoning",
                "the tidal table",
                "the sounded channel",
                "the fixed rudder",
            ],
        },
        Field::Statecraft => Lexicon {
            stuff: &["tax", "census", "seal", "charter", "writ", "coin"],
            craft: &["keeping", "reckoning", "sealing", "granting", "minting"],
            adj: &["standing", "sworn", "royal", "common", "written", "assized"],
            thing: &[
                "roll", "register", "assize", "court", "treasury", "chancery", "mint",
            ],
            process: &[
                "the tax roll",
                "coined money",
                "written law",
                "the standing census",
                "the sworn assize",
                "the chartered town",
                "double-entry accounts",
            ],
        },
        Field::Warcraft => Lexicon {
            stuff: &["shield", "pike", "siege", "saddle", "bow", "ram"],
            craft: &["drill", "work", "training", "order"],
            adj: &[
                "locked",
                "massed",
                "counterweighted",
                "torsion",
                "mounted",
                "tiered",
            ],
            thing: &[
                "ram",
                "engine",
                "tower",
                "pike-wall",
                "stirrup",
                "bastion",
                "trebuchet",
                "sap",
            ],
            process: &[
                "the sapper's tunnel",
                "the counterweighted engine",
                "the locked shield-wall",
                "the mounted charge",
                "the ordered siege",
            ],
        },
        Field::Leycraft => Lexicon {
            stuff: &["ley", "ward", "sigil", "ether", "node", "thread"],
            craft: &["binding", "warding", "tracing", "knotting", "drawing"],
            adj: &[
                "bound",
                "silent",
                "nine-fold",
                "anchored",
                "hollow",
                "still",
            ],
            thing: &[
                "ward", "sigil", "lattice", "anchor", "conduit", "lens", "knot",
            ],
            process: &[
                "ley-binding",
                "the anchored ward",
                "the traced lattice",
                "node-reading",
                "the quieted line",
            ],
        },
        Field::Physic => Lexicon {
            stuff: &["herb", "bone", "wound", "fever", "blood"],
            craft: &["setting", "dressing", "drawing", "purging", "tending"],
            adj: &["boiled", "clean", "separated", "quarantined", "running"],
            thing: &["lazaret", "drain", "conduit", "bath", "ward", "poultice"],
            process: &[
                "the boiled dressing",
                "quarantine",
                "the separated well",
                "the drained street",
                "bone-setting",
                "the physic garden",
            ],
        },
        Field::Letters => Lexicon {
            stuff: &["script", "paper", "ink", "letter", "block"],
            craft: &["cutting", "setting", "copying", "binding", "ruling"],
            adj: &["running", "cut", "bound", "ruled", "common", "moveable"],
            thing: &["script", "codex", "press", "library", "archive", "primer"],
            process: &[
                "writing",
                "the bound codex",
                "paper",
                "the copied archive",
                "moveable type",
                "the common primer",
            ],
        },
        Field::Reckoning => Lexicon {
            stuff: &["star", "angle", "number", "lens", "shadow"],
            craft: &["reckoning", "sighting", "measuring", "grinding", "tabling"],
            adj: &[
                "fixed",
                "graduated",
                "ground",
                "true",
                "sighted",
                "tabulated",
            ],
            thing: &[
                "table",
                "astrolabe",
                "gnomon",
                "lens",
                "quadrant",
                "abacus",
                "clock",
            ],
            process: &[
                "the star-tables",
                "the graduated circle",
                "the ground lens",
                "positional numbers",
                "the water-clock",
                "the surveyed baseline",
            ],
        },
    }
}

/// Which grounds a field's innovations can come from, and which effects they
/// can carry.
///
/// This is the whole of what makes a *generated* tree sensible: husbandry
/// improves harvests on watered or open land, and never improves siegecraft.
fn field_shape(f: Field) -> (&'static [Ground], &'static [u8]) {
    match f {
        Field::Husbandry => (
            &[Ground::Watered, Ground::Open, Ground::Anywhere],
            &[0, 0, 0, 11],
        ),
        Field::Metalcraft => (&[Ground::Ore, Ground::Town], &[1, 1, 2, 8]),
        Field::Building => (
            &[Ground::Highland, Ground::Town, Ground::GreatCity],
            &[3, 6, 8, 0],
        ),
        Field::Seafaring => (&[Ground::Shore], &[7, 7, 8, 5]),
        Field::Statecraft => (&[Ground::Town, Ground::GreatCity], &[4, 5, 10, 4]),
        Field::Warcraft => (
            &[Ground::Town, Ground::Open, Ground::Anywhere],
            &[1, 2, 3, 1],
        ),
        Field::Leycraft => (&[Ground::Ley], &[9, 9, 1, 10]),
        Field::Physic => (&[Ground::Town, Ground::Anywhere], &[11, 11, 8, 10]),
        Field::Letters => (&[Ground::Town, Ground::GreatCity], &[4, 9, 10, 4]),
        Field::Reckoning => (&[Ground::GreatCity, Ground::Town], &[7, 5, 4, 9]),
    }
}

/// Build an effect of the given code at the given magnitude.
pub fn effect_of(code: u8, v: f32) -> Effect {
    match code {
        0 => Effect::Yield(v),
        1 => Effect::Arms(v),
        2 => Effect::Siegecraft(v),
        3 => Effect::Ramparts(v),
        4 => Effect::Administration(v),
        5 => Effect::Revenue(v),
        6 => Effect::Roadcraft(v),
        7 => Effect::Seacraft(v),
        8 => Effect::Prosperity(v),
        9 => Effect::Doctrine(v),
        10 => Effect::Order(v),
        _ => Effect::Health(v),
    }
}

/// The code for an effect, for the save file.
pub fn effect_code(e: Effect) -> u8 {
    match e {
        Effect::Yield(_) => 0,
        Effect::Arms(_) => 1,
        Effect::Siegecraft(_) => 2,
        Effect::Ramparts(_) => 3,
        Effect::Administration(_) => 4,
        Effect::Revenue(_) => 5,
        Effect::Roadcraft(_) => 6,
        Effect::Seacraft(_) => 7,
        Effect::Prosperity(_) => 8,
        Effect::Doctrine(_) => 9,
        Effect::Order(_) => 10,
        Effect::Health(_) => 11,
    }
}

/// Grow a world's tree of innovations.
///
/// Deterministic from the RNG it is handed. Stored in the save rather than
/// regenerated, because it is a world's own history of ideas and not a
/// function of its terrain.
pub fn grow_tree(rng: &Rng, count: usize) -> Vec<Innovation> {
    let count = count.min(MAX_TECHS);
    let mut out: Vec<Innovation> = Vec::with_capacity(count);
    let mut used: Vec<String> = Vec::with_capacity(count);
    // How many sit at each depth. The tree is deliberately narrow at the
    // root — a world has only a few first ideas — and widens as it goes.
    let per_tier = 7usize;
    for id in 0..count {
        let tier = (id / per_tier) as u8;
        let field = *rng.pick(&FIELDS);
        let (grounds, effects) = field_shape(field);
        let ground = *rng.pick(grounds);
        // Magnitude grows with depth, with diminishing returns: the tenth
        // advance in husbandry is worth more than the first, and not ten
        // times more.
        let depth = tier as f32;
        let mag = (0.025 + depth.sqrt() * 0.016) * rng.range32(0.75, 1.3);
        let effect = effect_of(*rng.pick(effects), mag);
        // Parents: prefer the same field, from any earlier tier.
        let mut needs: Vec<usize> = Vec::new();
        if tier > 0 {
            let earlier: Vec<usize> = out.iter().filter(|x| x.tier < tier).map(|x| x.id).collect();
            if !earlier.is_empty() {
                let same: Vec<usize> = earlier
                    .iter()
                    .copied()
                    .filter(|&x| out[x].field == field)
                    .collect();
                let pool = if same.is_empty() { &earlier } else { &same };
                needs.push(pool[rng.below(pool.len())]);
                // A second parent, sometimes, out of any field: where two
                // bodies of knowledge meet is where the interesting things
                // happen.
                if rng.chance(0.3) {
                    let other = earlier[rng.below(earlier.len())];
                    if !needs.contains(&other) {
                        needs.push(other);
                    }
                }
            }
        }
        let name = coin_name(rng, field, effect, &mut used);
        out.push(Innovation {
            id,
            name,
            field,
            tier,
            needs,
            ground,
            effect,
            difficulty: 1.0 + depth.powf(1.55) * 0.8,
            spread: rng.range32(0.7, 1.4),
        });
    }
    out
}

/// Nouns that suit a particular consequence, whatever field they turn up in.
///
/// Without this a warcraft innovation called "the engine" could come out
/// making walls *stand longer*, which reads as a mistake even though the
/// numbers are fine. Where an effect has an obvious vocabulary, the name is
/// drawn from it instead of the field's general pool.
fn effect_words(e: Effect) -> Option<&'static [&'static str]> {
    match e {
        Effect::Siegecraft(_) => {
            Some(&["ram", "engine", "trebuchet", "sap", "siege-tower", "mine"])
        }
        Effect::Ramparts(_) => Some(&["bastion", "curtain wall", "rampart", "redoubt", "barbican"]),
        Effect::Seacraft(_) => Some(&["rudder", "keel", "chart", "lead-line", "compass", "sail"]),
        Effect::Roadcraft(_) => Some(&["causeway", "post-road", "bridge", "milestone", "ford"]),
        Effect::Health(_) => Some(&["lazaret", "drain", "conduit", "physic garden", "clean well"]),
        Effect::Yield(_) => Some(&["plough", "harrow", "sluice", "terrace", "granary", "dyke"]),
        Effect::Administration(_) => Some(&["register", "roll", "chancery", "assize", "survey"]),
        Effect::Revenue(_) => Some(&["mint", "treasury", "toll-house", "customs roll", "ledger"]),
        _ => None,
    }
}

/// Coin a name nobody in this world has used.
fn coin_name(rng: &Rng, field: Field, effect: Effect, used: &mut Vec<String>) -> String {
    let lex = lexicon(field);
    let things: &[&str] = effect_words(effect).unwrap_or(lex.thing);
    for _ in 0..24 {
        let name = match rng.below(4) {
            0 => format!("{}-{}", rng.pick(lex.stuff), rng.pick(lex.craft)),
            1 => format!("the {} {}", rng.pick(lex.adj), rng.pick(things)),
            2 => (*rng.pick(lex.process)).to_string(),
            _ => format!("the {}", rng.pick(things)),
        };
        if !used.contains(&name) {
            used.push(name.clone());
            return name;
        }
    }
    // Everything plausible is taken: a numbered refinement, so a very deep
    // tree still names its later advances rather than running out.
    let base = *rng.pick(lex.process);
    for n in 2..60 {
        let name = format!("{}, {} refinement", base, prose::ordinal(n));
        if !used.contains(&name) {
            used.push(name.clone());
            return name;
        }
    }
    format!("an advance in {}", field.name())
}

// ---------------------------------------------------------------------------
// Reading what is known
// ---------------------------------------------------------------------------

impl World {
    /// Whether the ground at `i` knows innovation `t`.
    pub fn cell_knows(&self, i: usize, t: usize) -> bool {
        match (self.known.get(i), self.techs.get(t)) {
            (Some(k), Some(inn)) => k & inn.bit() != 0,
            _ => false,
        }
    }

    /// Whether a realm knows it anywhere in its lands.
    pub fn knows(&self, p: usize, t: usize) -> bool {
        match (self.polity_known.get(p), self.techs.get(t)) {
            (Some(k), Some(inn)) => k & inn.bit() != 0,
            _ => false,
        }
    }

    /// How many innovations a realm holds.
    pub fn tech_count(&self, p: usize) -> u32 {
        self.polity_known
            .get(p)
            .map(|k| k.count_ones())
            .unwrap_or(0)
    }

    /// Everything a realm knows, in the order it was invented.
    pub fn tech_list(&self, p: usize) -> Vec<usize> {
        let Some(&k) = self.polity_known.get(p) else {
            return Vec::new();
        };
        self.techs
            .iter()
            .filter(|inn| k & inn.bit() != 0)
            .map(|inn| inn.id)
            .collect()
    }

    /// Diminishing returns on an accumulated advantage.
    ///
    /// Every one of these effects is a sum over a tenth of a hundred-node
    /// tree, so an unbounded sum means a realm that is ahead is ahead
    /// without limit — and with cultures having genuine blind spots, a
    /// people bent on warcraft out-armed its neighbours for ever and took
    /// the world by year two thousand. The tenth advance in a field has to
    /// be worth much less than the first.
    ///
    /// `half` is the raw sum at which half of `cap` is granted.
    fn diminishing(raw: f32, cap: f32, half: f32) -> f32 {
        if raw <= 0.0 {
            return 0.0;
        }
        cap * raw / (raw + half)
    }

    /// The sum of one kind of effect over everything a realm knows.
    ///
    /// Walks the tree rather than the map, so it costs the number of
    /// innovations that exist and not the number of cells.
    fn effect_sum(&self, p: usize, pick: impl Fn(Effect) -> Option<f32>) -> f32 {
        let Some(&k) = self.polity_known.get(p) else {
            return 0.0;
        };
        if k == 0 {
            return 0.0;
        }
        let mut total = 0.0;
        for inn in &self.techs {
            if k & inn.bit() != 0 {
                if let Some(v) = pick(inn.effect) {
                    total += v;
                }
            }
        }
        total
    }

    /// Bring one cell's cached yield multiplier up to date. Called wherever
    /// `known` changes, which is rare.
    pub fn refresh_yield(&mut self, i: usize) {
        let v = self.tech_capacity_mult(i);
        if let Some(slot) = self.cell_yield.get_mut(i) {
            *slot = v;
        }
    }

    /// What the land at `i` is worth in food for what is known there.
    pub fn tech_capacity_mult(&self, i: usize) -> f32 {
        let Some(&k) = self.known.get(i) else {
            return 1.0;
        };
        if k == 0 {
            return 1.0;
        }
        let mut raw = 0.0;
        for inn in &self.techs {
            if k & inn.bit() != 0 {
                if let Effect::Yield(v) = inn.effect {
                    raw += v;
                }
            }
        }
        1.0 + Self::diminishing(raw, 0.85, 0.5)
    }

    pub fn tech_army_mult(&self, p: usize) -> f32 {
        let raw = self.effect_sum(p, |e| match e {
            Effect::Arms(v) => Some(v),
            _ => None,
        });
        1.0 + Self::diminishing(raw, 0.40, 0.45)
    }

    pub fn tech_admin_bonus(&self, p: usize) -> f32 {
        let raw = self.effect_sum(p, |e| match e {
            Effect::Administration(v) => Some(v),
            _ => None,
        });
        Self::diminishing(raw, 34.0, 0.5)
    }

    pub fn tech_reach_bonus(&self, p: usize) -> f32 {
        let raw = self.effect_sum(p, |e| match e {
            Effect::Roadcraft(v) => Some(v),
            _ => None,
        });
        Self::diminishing(raw, 11.0, 0.4)
    }

    pub fn tech_sea_bonus(&self, p: usize) -> f32 {
        let raw = self.effect_sum(p, |e| match e {
            Effect::Seacraft(v) => Some(v),
            _ => None,
        });
        Self::diminishing(raw, 13.0, 0.4)
    }

    pub fn tech_income_mult(&self, p: usize) -> f32 {
        let raw = self.effect_sum(p, |e| match e {
            Effect::Revenue(v) => Some(v),
            _ => None,
        });
        1.0 + Self::diminishing(raw, 0.55, 0.5)
    }

    pub fn tech_prosperity_bonus(&self, p: usize) -> f32 {
        let raw = self.effect_sum(p, |e| match e {
            Effect::Prosperity(v) => Some(v),
            _ => None,
        });
        Self::diminishing(raw, 0.5, 0.5)
    }

    /// What knowledge adds to the stability a realm tends toward.
    ///
    /// Deliberately small and tightly capped. Stability is what the sprawl
    /// brake spends to hold an over-large realm together, so anything that
    /// hands it back for free undoes that brake — and a realm with the whole
    /// tree behind it was conquering the world by year nineteen hundred
    /// because knowledge had quietly paid off its own overextension.
    pub fn tech_order_bonus(&self, p: usize) -> f32 {
        (self.effect_sum(p, |e| match e {
            Effect::Order(v) => Some(v),
            _ => None,
        }) * 0.25)
            .min(0.07)
    }

    pub fn tech_doctrine_bonus(&self, p: usize) -> f32 {
        self.effect_sum(p, |e| match e {
            Effect::Doctrine(v) => Some(v),
            _ => None,
        })
    }

    /// How much less of a plague's toll a realm suffers for what it knows.
    pub fn tech_health(&self, p: usize) -> f32 {
        self.effect_sum(p, |e| match e {
            Effect::Health(v) => Some(v),
            _ => None,
        })
        .min(0.7)
    }

    /// What a siege is worth, for what both sides know.
    ///
    /// The one place knowledge takes away rather than adds: until somebody
    /// works out how to bring a wall down, a walled city is very nearly
    /// untakeable, and the century somebody does is the one in which every
    /// realm that trusted its walls finds out it was wrong.
    pub fn siege_advantage(&self, attacker: usize, defender: usize) -> f32 {
        let attack = self.effect_sum(attacker, |e| match e {
            Effect::Siegecraft(v) => Some(v),
            _ => None,
        });
        let hold = self.effect_sum(defender, |e| match e {
            Effect::Ramparts(v) => Some(v),
            _ => None,
        });
        // Bounded, because both sides of a war advance: what matters is the
        // gap between them, and an unbounded gap means the first realm to
        // work out how to break a wall takes every city in the world.
        (1.0 + attack * 1.1 - hold * 1.0).clamp(0.3, 2.0)
    }
}

// ---------------------------------------------------------------------------
// The yearly pass
// ---------------------------------------------------------------------------

/// How much feeling for a field a people needs before it will work
/// anything out in it, or take anything up. Below this a whole branch of
/// knowledge simply passes a people by — which is what makes one region's
/// strengths another's blind spots.
const MIN_BENT: f32 = 0.22;

/// How often knowledge spreads. The one full sweep this module does, so it
/// runs on a cycle rather than every year.
const DIFFUSION_EVERY: i32 = 3;

/// Discovery, diffusion and forgetting, for one year.
pub fn tick(w: &mut World) {
    discover(w);
    if w.year % DIFFUSION_EVERY == 0 {
        diffuse(w);
    }
}

/// Somebody works something out.
///
/// At cities, because that is where the people who work things out are and
/// because it keeps the cost proportional to the handful of cities rather
/// than to the ten thousand cells.
fn discover(w: &mut World) {
    let rng = w.rng.clone();
    // How far the world's frontier has already been pushed. The drag is
    // global rather than per-realm because discovery is rolled at every
    // city: two hundred cities each rolling rarely still works a tree
    // through in a few centuries, so what has to slow down is the *world's*
    // rate, not one realm's. Each thing already known makes the next one
    // dearer, which is what spreads a hundred and twenty innovations across
    // ten thousand years instead of one thousand.
    let frontier = w.tech_seen.iter().filter(|&&b| b).count() as f64;
    let drag = 1.0 + frontier * w.tuning.tech_frontier_drag;
    let cities: Vec<usize> = (0..w.cities.len())
        .filter(|&c| w.cities[c].destroyed.is_none() && w.cities[c].polity.is_some())
        .collect();
    for c in cities {
        let cell = w.cities[c].cell;
        let Some(p) = w.cities[c].polity else {
            continue;
        };
        if !w.polities[p].alive() {
            continue;
        }
        // What this place can support: people, wealth and the state's own
        // sophistication multiplied together. A hamlet works out the plough;
        // it does not work out a tax roll.
        let capability =
            w.cities[c].pop * (0.5 + w.cities[c].prosperity) * (1.0 + w.polities[p].dev);
        let culture = w.polities[p].culture;
        let vals = w.cultures[culture].values;
        let school_bonus = match w.polities[p].school.map(|s| w.schools[s].kind) {
            Some(SchoolKind::Philosophical) => 1.5,
            Some(SchoolKind::Arcane) => 1.25,
            _ => 1.0,
        };
        // Each further advance is harder than the last. Without this the
        // tree is worked through as fast as a world can grow cities, and a
        // hundred and twenty innovations are exhausted by year one thousand
        // exactly as two dozen were exhausted by year four hundred. The
        // frontier has to recede as it is approached.
        let chance = w.tuning.tech_discover_chance
            * (capability as f64 / 20.0).clamp(0.15, 4.0)
            * (0.5 + vals.openness as f64)
            * school_bonus
            / drag;
        if !rng.chance(chance) {
            continue;
        }
        // What a people can work out is bounded by what it is good at. A
        // martial people finds the pike wall; it does not stumble on double
        // entry accounts. This is the whole of the link between culture and
        // knowledge, and it is what keeps a world's regions distinct — the
        // frontier is not one frontier but ten, and no people pushes all of
        // them.
        let mut options: Vec<(usize, f64)> = Vec::new();
        for inn in &w.techs {
            if w.cell_knows(cell, inn.id) {
                continue;
            }
            let bent = w.cultures[culture].bent(inn.field);
            // A people needs some feeling for a field before it will work
            // anything out in it at all.
            if bent < MIN_BENT {
                continue;
            }
            // What it can support, eased by how well it takes to the field.
            if capability < inn.difficulty * w.tuning.tech_effort / (0.5 + bent) {
                continue;
            }
            if !inn.ground.suits(w, cell) {
                continue;
            }
            if !inn.needs.iter().all(|&need| w.knows(p, need)) {
                continue;
            }
            options.push((inn.id, (bent * bent) as f64));
        }
        if options.is_empty() {
            continue;
        }
        let weights: Vec<f64> = options.iter().map(|&(_, wt)| wt).collect();
        let t = options[rng.weighted(&weights)].0;
        let first = !w.tech_seen[t];
        learn(w, cell, t);
        for nb in w.terrain.neighbors8(cell).collect::<Vec<_>>() {
            if w.terrain.is_land(nb) {
                learn(w, nb, t);
            }
        }
        if first {
            w.tech_seen[t] = true;
            w.tech_first_year[t] = w.year;
            let importance = if w.techs[t].is_landmark() { 3 } else { 2 };
            let text = prose::tech_discovered(w, p, c, t, &Pick::rolled(&rng));
            w.log(
                importance,
                EventKind::Discovery,
                &[Ref::Polity(p), Ref::City(c)],
                Some(cell),
                text,
            );
        } else {
            let text = prose::tech_reached(w, p, c, t);
            w.log(
                0,
                EventKind::Discovery,
                &[Ref::Polity(p), Ref::City(c)],
                Some(cell),
                text,
            );
        }
    }
}

/// Teach one cell one thing.
fn learn(w: &mut World, i: usize, t: usize) {
    let bit = match w.techs.get(t) {
        Some(inn) => inn.bit(),
        None => return,
    };
    if let Some(k) = w.known.get_mut(i) {
        let before = *k;
        *k |= bit;
        if *k != before {
            w.refresh_yield(i);
        }
    }
}

/// Knowledge travels, and dies where nobody is left to carry it.
fn diffuse(w: &mut World) {
    let rng = w.rng.clone();
    let n = w.cells.len();
    let base = w.tuning.tech_spread_chance * DIFFUSION_EVERY as f64;
    let mut gained: Vec<(usize, u128)> = Vec::new();
    for i in 0..n {
        if !w.terrain.is_land(i) {
            continue;
        }
        // Knowledge needs somebody to carry it. Empty ground forgets.
        if w.cells[i].pop < 0.02 {
            if w.known[i] != 0 && rng.chance(w.tuning.tech_forget_chance) {
                w.known[i] = 0;
                w.refresh_yield(i);
            }
            continue;
        }
        let mine = w.known[i];
        let mut offered = 0u128;
        let mut foreign = 0u128;
        for nb in w.terrain.neighbors8(i) {
            let theirs = w.known[nb];
            if theirs & !mine == 0 {
                continue;
            }
            if w.cells[nb].culture == w.cells[i].culture {
                offered |= theirs & !mine;
            } else {
                foreign |= theirs & !mine;
            }
        }
        if offered == 0 && foreign == 0 {
            continue;
        }
        // Rivers and coasts are the roads of a world without roads.
        let along = if w.terrain.river[i] > 0 || w.terrain.coast[i] {
            1.5
        } else {
            1.0
        };
        // Who lives here decides what they will take up, and from whom.
        let (bent_of, keeps_own_ways) = match w.cells[i].culture {
            Some(c) => (w.cultures[c].learning, w.cultures[c].values.tradition),
            None => ([0.5; FIELDS.len()], 0.5),
        };
        let mut take = 0u128;
        for inn in &w.techs {
            let b = inn.bit();
            let from_own = offered & b != 0;
            let from_other = foreign & b != 0;
            if !from_own && !from_other {
                continue;
            }
            // A prerequisite you do not have is a thing you cannot copy: you
            // can see a pike wall without knowing how to make the iron.
            if !inn
                .needs
                .iter()
                .all(|&need| mine & w.techs[need].bit() != 0)
            {
                continue;
            }
            let bent = bent_of[inn.field as usize];
            if bent < MIN_BENT {
                continue;
            }
            // A technique out of a stranger's country has to get past what
            // a people thinks of strangers, which is why a border can hold
            // an idea out for centuries while a trade good crosses it in a
            // season.
            let across: f64 = if from_own {
                1.0
            } else {
                (0.55 - keeps_own_ways as f64 * 0.45).max(0.03)
            };
            if rng.chance(base * inn.spread as f64 * along * across * bent as f64) {
                take |= b;
            }
        }
        if take != 0 {
            gained.push((i, take));
        }
    }
    for (i, take) in gained {
        w.known[i] |= take;
        w.refresh_yield(i);
    }
}
