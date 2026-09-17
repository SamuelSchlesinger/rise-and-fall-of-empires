//! What a person inherits from both of their parents.
//!
//! Traits used to pass down by blending a *single* parent's value with a
//! hard pull toward the middle: `child = Normal(parent * 0.6 + 0.2, 0.2)`.
//! Three things were wrong with that, and the third only became wrong when
//! houses and marriages were built:
//!
//! * **Variation collapsed.** Every generation dragged toward 0.5, so four
//!   generations after a ruler of 0.9 ambition the line was at 0.55.
//!   Remarkable families could not stay remarkable.
//! * **Nothing could skip a generation.** A blend has no memory: what is not
//!   expressed is gone. There was no way for a strain to lie hidden and
//!   surface two centuries later, which is most of what makes a dynasty
//!   worth following.
//! * **The other parent did not exist.** Consorts and foreign brides are
//!   real people with their own traits, and contributed nothing at all.
//!
//! So: two alleles per trait, one drawn from each parent, kept whole rather
//! than averaged. Variation persists because alleles persist. A recessive
//! allele can sit unexpressed in a house for generations and then meet
//! another copy of itself, which is exactly the event a chronicle wants to
//! report.
//!
//! It also gives the game the Habsburg story for free. A house that marries
//! its own to keep a claim concentrates its recessives and loses vigour;
//! one that marries out gets it back.

use super::chronicle::{EventKind, Ref};
use super::prose;
use super::{Traits, World};
use crate::rng::Rng;

/// How many heritable traits there are: the six of [`Traits`].
pub const TRAITS: usize = 6;
/// Two alleles each.
pub const GENES: usize = TRAITS * 2;

/// An allele below this is a rare recessive: it shows only when both copies
/// carry one, which is what lets a strain hide in a line for generations.
const RECESSIVE_LOW: u8 = 26;
/// And above this, at the other end.
const RECESSIVE_HIGH: u8 = 229;

/// Chance that a copied allele is misread, which is where new variation
/// comes from and why a line does not simply run down.
const MUTATION: f64 = 0.04;

/// A person's inheritance: two alleles for each trait, laid out as
/// `[trait0_a, trait0_b, trait1_a, ...]`.
pub type Genome = [u8; GENES];

/// A genome drawn from nothing, for somebody with no recorded parents.
pub fn fresh(rng: &Rng) -> Genome {
    let mut g = [128u8; GENES];
    for slot in g.iter_mut() {
        // Most alleles sit near the middle; a few are the rare extremes
        // that make a line interesting a century later.
        // Drawn about the middle with a real tail, so that the average of
        // two alleles has about the spread a single trait used to: an
        // allele range of [70, 186] made every expressed trait fall between
        // 0.27 and 0.73 and nobody was ever remarkable at all.
        *slot = if rng.chance(0.012) {
            if rng.chance(0.5) {
                rng.int(0, RECESSIVE_LOW as i32) as u8
            } else {
                rng.int(RECESSIVE_HIGH as i32, 255) as u8
            }
        } else {
            (rng.trait_value(0.5, 0.28) * 255.0).clamp(0.0, 255.0) as u8
        };
    }
    g
}

/// A child's genome: one allele of each pair from each parent.
pub fn cross(a: &Genome, b: &Genome, rng: &Rng) -> Genome {
    let mut g = [128u8; GENES];
    for t in 0..TRAITS {
        let from_a = a[t * 2 + rng.below(2)];
        let from_b = b[t * 2 + rng.below(2)];
        g[t * 2] = mutate(from_a, rng);
        g[t * 2 + 1] = mutate(from_b, rng);
    }
    g
}

/// Copying is imperfect.
fn mutate(v: u8, rng: &Rng) -> u8 {
    if rng.chance(MUTATION) {
        let delta = rng.int(-40, 40);
        (v as i32 + delta).clamp(0, 255) as u8
    } else {
        v
    }
}

/// Whether an allele is one of the rare extremes.
fn is_recessive(v: u8) -> bool {
    v <= RECESSIVE_LOW || v >= RECESSIVE_HIGH
}

/// What a genome shows.
///
/// Mostly the average of the two alleles, which is how most inherited things
/// actually behave — but when *both* copies are a rare extreme, the extreme
/// shows through undiluted. That is the whole of the recessive machinery,
/// and it is what produces a ruler whose like the line has not seen in four
/// generations.
pub fn express(g: &Genome) -> Traits {
    let mut out = [0.0f32; TRAITS];
    for (t, slot) in out.iter_mut().enumerate() {
        let (a, b) = (g[t * 2], g[t * 2 + 1]);
        let both_low = a <= RECESSIVE_LOW && b <= RECESSIVE_LOW;
        let both_high = a >= RECESSIVE_HIGH && b >= RECESSIVE_HIGH;
        let v = if both_low {
            // Doubled and undiluted.
            (a.min(b) as f32 / 255.0) * 0.5
        } else if both_high {
            1.0 - ((255 - a.max(b)) as f32 / 255.0) * 0.5
        } else {
            (a as f32 + b as f32) / 510.0
        };
        *slot = v.clamp(0.0, 1.0);
    }
    Traits {
        ambition: out[0],
        valor: out[1],
        wisdom: out[2],
        piety: out[3],
        cruelty: out[4],
        charisma: out[5],
    }
}

/// Whether a genome shows a doubled recessive, and which trait.
///
/// Used to decide whether a birth is worth a line of its own: a child who
/// shows something neither parent did is the moment a hidden strain
/// surfaces.
pub fn surfaced(g: &Genome) -> Option<usize> {
    (0..TRAITS).find(|&t| {
        let (a, b) = (g[t * 2], g[t * 2 + 1]);
        is_recessive(a) && is_recessive(b) && (a <= RECESSIVE_LOW) == (b <= RECESSIVE_LOW)
    })
}

/// Traits somebody carries one copy of without showing it.
///
/// The thing a reader following a house actually wants to know: what is in
/// the blood that has not come out yet. A single rare allele is invisible in
/// its holder and can meet its match three generations later.
pub fn carried(g: &Genome) -> Vec<usize> {
    (0..TRAITS)
        .filter(|&t| {
            let (a, b) = (g[t * 2], g[t * 2 + 1]);
            // Exactly one copy is rare: carried, not shown.
            is_recessive(a) != is_recessive(b)
        })
        .collect()
}

/// The name of a heritable trait.
pub fn trait_name(t: usize) -> &'static str {
    match t {
        0 => "ambition",
        1 => "valour",
        2 => "wisdom",
        3 => "piety",
        4 => "cruelty",
        _ => "presence",
    }
}

// ---------------------------------------------------------------------------
// Kinship
// ---------------------------------------------------------------------------

/// How many generations back kinship is traced. Four is enough to catch
/// cousins marrying cousins, which is what a house does when it is trying to
/// keep a claim in the family, and cheap: at most fifteen ancestors a side.
const KIN_DEPTH: usize = 4;

/// Everyone in somebody's ancestry, up to [`KIN_DEPTH`] generations.
fn ancestors(w: &World, p: usize) -> Vec<usize> {
    let mut out = Vec::with_capacity(16);
    let mut frontier = vec![p];
    for _ in 0..KIN_DEPTH {
        let mut next = Vec::new();
        for x in frontier {
            if let Some(f) = w.persons[x].parent {
                if !out.contains(&f) {
                    out.push(f);
                    next.push(f);
                }
            }
            if let Some(s) = w.persons[x].spouse {
                // A step-parent is not blood, but the *other* blood parent
                // is reached through them when we only record one.
                if let Some(f) = w.persons[s].parent {
                    if !out.contains(&f) {
                        out.push(f);
                        next.push(f);
                    }
                }
            }
        }
        frontier = next;
        if frontier.is_empty() {
            break;
        }
    }
    out
}

/// How closely two people are related, roughly, in `[0, 1]`.
///
/// The share of one's recent ancestry that also appears in the other's. Not
/// a true coefficient of relatedness — it does not weight by generation —
/// but it separates cousins from strangers, which is all that is wanted.
pub fn kinship(w: &World, a: usize, b: usize) -> f32 {
    if a == b {
        return 1.0;
    }
    let (ga, gb) = (ancestors(w, a), ancestors(w, b));
    if ga.is_empty() || gb.is_empty() {
        return 0.0;
    }
    let shared = ga.iter().filter(|x| gb.contains(x)).count();
    shared as f32 / ga.len().min(gb.len()) as f32
}

// ---------------------------------------------------------------------------
// Making a child
// ---------------------------------------------------------------------------

/// Everything a birth settles: the child's blood, what it shows, and what
/// its parents' closeness cost it.
pub struct Birth {
    pub genes: Genome,
    pub traits: Traits,
    /// Below 1.0 for a child of close kin, above it for one whose parents
    /// were strangers.
    pub vigour: f32,
    /// How close the parents were.
    pub inbred: f32,
    /// A trait that showed itself doubled, if one did.
    pub surfaced: Option<usize>,
}

/// Conceive a child of two people.
pub fn conceive(w: &World, mother: usize, father: usize, rng: &Rng) -> Birth {
    let ga = w.persons[mother].genes;
    let gb = w.persons[father].genes;
    let inbred = kinship(w, mother, father);
    let mut genes = cross(&ga, &gb, rng);
    // Close kin do not introduce new alleles, they double the ones already
    // there. Nudging the two copies of a pair together is a cheap stand-in
    // for the real thing and gives the right consequence: a house that
    // marries its own surfaces its own recessives.
    if inbred > 0.15 {
        for t in 0..TRAITS {
            if rng.chance(inbred as f64 * 0.7) {
                genes[t * 2 + 1] = genes[t * 2];
            }
        }
    }
    let traits = express(&genes);
    // Vigour: what the marriage cost or gained. Cousins pay; strangers
    // gain a little.
    let vigour = (1.0 + 0.12 - inbred * 0.75).clamp(0.35, 1.15);
    Birth {
        surfaced: surfaced(&genes),
        genes,
        traits,
        vigour,
        inbred,
    }
}

/// Tell the world when a hidden strain has surfaced.
///
/// The interest is entirely in the *contrast*: a child who shows something
/// neither parent had. Without checking the parents this fired on every
/// doubled recessive — twenty-two times a century, which made a rare event
/// into wallpaper — and half of those were children of parents who plainly
/// had the same trait themselves.
pub fn note_if_remarkable(
    w: &mut World,
    child: usize,
    p: usize,
    parents: (usize, usize),
    surfaced: Option<usize>,
) {
    let Some(t) = surfaced else { return };
    let value = trait_value(&w.persons[child].traits, t);
    // Neither parent showed it: that is what makes it worth saying.
    let (ma, pa) = parents;
    let shown_before = trait_value(&w.persons[ma].traits, t)
        .max(trait_value(&w.persons[pa].traits, t))
        > 0.72
        || trait_value(&w.persons[ma].traits, t).min(trait_value(&w.persons[pa].traits, t)) < 0.28;
    if shown_before {
        return;
    }
    if !(0.08..=0.92).contains(&value) {
        let house = w.persons[child].house;
        let text = prose::strain_surfaced(w, child, t, value, house);
        let mut refs = vec![Ref::Person(child), Ref::Polity(p)];
        if let Some(h) = house {
            refs.push(Ref::House(h));
        }
        w.log(1, EventKind::Person, &refs, w.capital_cell(p), text);
    }
}

/// Read one trait out of a set by index.
pub fn trait_value(t: &Traits, i: usize) -> f32 {
    match i {
        0 => t.ambition,
        1 => t.valor,
        2 => t.wisdom,
        3 => t.piety,
        4 => t.cruelty,
        _ => t.charisma,
    }
}
