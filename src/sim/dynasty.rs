//! Houses, families and the making of a great figure.
//!
//! Everything in this module exists to answer one complaint: that the world
//! only ever told you somebody was remarkable in their obituary. A ruler's
//! epithet used to be chosen inside `politics::ruler_dies`, which meant the
//! chronicle could not name a conqueror until the conquering had stopped —
//! so the reader met Alexander as a corpse and Charlemagne as an estate to
//! be divided.
//!
//! Three things fix that, and they are the three halves of this file:
//!
//! * **Kin that exist before they matter.** Rulers marry and have children,
//!   and those children are real [`Person`]s who grow up in the chronicle
//!   with names and traits. Succession then draws on people the reader has
//!   already met rather than conjuring an heir at the graveside.
//! * **Standing, computed every year.** [`greatness`] scores a living
//!   career in points. When somebody crosses the line that separates a ruler
//!   from a figure, the world says so *at the time*, in [`acclaim`], and
//!   they carry the title for the rest of their life.
//! * **Ties between houses.** A marriage is a diplomatic fact
//!   ([`Stance::Married`]), and an heir with a claim in two realms can unite
//!   them without a battle.
//!
//! The scoring weights here are mirrored by `explain::standing`, and the two
//! must change together — see the note on duplication in `explain`.

use super::chronicle::{EventKind, Ref};
use super::prose::{self, Pick};
use super::{Gender, Inheritance, PolityKind, Role, SchoolKind, Stance, World};

/// The floor below which nobody is acclaimed however thin the competition.
pub const ACCLAIM_THRESHOLD: f32 = 175.0;
/// How far clear of the tenth-best living career a life must stand before
/// the world calls it great.
pub const ACCLAIM_MARGIN: f32 = 1.7;
/// The youngest age at which somebody can inherit without a regency.
pub const MAJORITY: i32 = 16;
/// The youngest age at which somebody can marry.
pub const MARRIAGE_AGE: i32 = 16;

// ---------------------------------------------------------------------------
// Standing
// ---------------------------------------------------------------------------

/// One contribution to somebody's standing: what it was, and what it was
/// worth in points.
pub struct Claim {
    pub what: String,
    pub points: f32,
}

/// A kind of deed a standing is made of.
///
/// The enum exists so that the *scoring* and the *wording* can be separated
/// without letting them drift: [`for_each_deed`] decides the points and
/// nothing else, and [`Claim`]s are only built when somebody is going to
/// read them. The simulation scores every living notable every year, and it
/// was previously formatting a sentence for each one to do it — a thousand
/// people times eight `format!` calls times every year of the world.
#[derive(Clone, Copy)]
pub enum Deed {
    Conquest(i32),
    Losses(i32),
    Settlement(i32),
    Speed(f32),
    CitiesTaken(u32),
    CitiesFounded(u32),
    Wars(u32),
    Battles(u32),
    LongReign(i32),
    Hegemon(f32),
    Imperial,
    Renown,
    Anointed,
    House(usize),
}

impl Deed {
    /// How this deed reads on a page.
    fn describe(self) -> String {
        match self {
            Deed::Conquest(n) => prose::claim_conquest(n),
            Deed::Losses(n) => prose::claim_losses(n),
            Deed::Settlement(n) => prose::claim_settlement(n),
            Deed::Speed(r) => prose::claim_speed(r),
            Deed::CitiesTaken(n) => prose::claim_cities_taken(n),
            Deed::CitiesFounded(n) => prose::claim_cities_founded(n),
            Deed::Wars(n) => prose::claim_wars(n),
            Deed::Battles(n) => prose::claim_battles(n),
            Deed::LongReign(n) => prose::claim_long_reign(n),
            Deed::Hegemon(share) => prose::claim_hegemon(share),
            Deed::Imperial => prose::claim_imperial(),
            Deed::Renown => prose::claim_renown(),
            Deed::Anointed => prose::claim_anointed(),
            Deed::House(n) => prose::claim_house(n),
        }
    }
}

/// Visit every deed that makes up somebody's standing, with its points.
///
/// This is the whole of the greatness model. It allocates nothing and writes
/// no prose, because it runs for every living notable every year.
fn for_each_deed(w: &World, r: usize, f: &mut impl FnMut(Deed, f32)) {
    let per = &w.persons[r];
    let mut add = |d: Deed, points: f32| {
        if points.abs() >= 0.5 {
            f(d, points);
        }
    };
    // Conquest, which is what makes a conqueror. Land taken from another
    // realm counts for far more than land settled, because clearing forest
    // for fifty years is a different life from breaking an empire in ten.
    if per.taken > 0 {
        add(
            Deed::Conquest(per.taken),
            (per.taken as f32 * 1.0).min(110.0),
        );
    } else if per.taken < 0 {
        add(
            Deed::Losses(-per.taken),
            (per.taken as f32 * 0.8).max(-70.0),
        );
    }
    let settled = per.gained - per.taken;
    if settled > 12 {
        add(Deed::Settlement(settled), (settled as f32 * 0.25).min(40.0));
    }
    // Speed counts for more than size, and this is the whole difference
    // between a world that has figures in it and one that only has
    // obituaries. Greatness scored on totals alone is reached at the end of
    // a long reign, so the world named its great rulers a few years before
    // they died and a reader never got to watch one. Scored on rate, the
    // ruler who takes thirty lands in eight years is acclaimed young and
    // has a lifetime left to be followed.
    if per.reign_years >= 3 && per.taken > 12 {
        let rate = per.taken as f32 / per.reign_years as f32;
        if rate > 1.0 {
            add(Deed::Speed(rate), ((rate - 1.0) * 46.0).min(150.0));
        }
    }
    if per.cities_taken > 0 {
        add(
            Deed::CitiesTaken(per.cities_taken),
            (per.cities_taken as f32 * 9.0).min(70.0),
        );
    }
    if per.cities_founded > 0 {
        add(
            Deed::CitiesFounded(per.cities_founded),
            (per.cities_founded as f32 * 7.0).min(55.0),
        );
    }
    if per.wars_won > 0 {
        add(
            Deed::Wars(per.wars_won),
            (per.wars_won as f32 * 8.0).min(55.0),
        );
    }
    if per.battles_won > 0 {
        add(
            Deed::Battles(per.battles_won),
            (per.battles_won as f32 * 3.0).min(45.0),
        );
    }
    // A long reign is long *for that people*: eighty years is a life's work
    // for one race and an apprenticeship for another, and a race that lives
    // for centuries must not collect the points for free.
    let lifespan = w.races[per.race].lifespan.max(1.0);
    let held = per.reign_years as f32 / lifespan;
    if held > 0.3 {
        add(
            Deed::LongReign(per.reign_years),
            ((held - 0.3) * 60.0).min(20.0),
        );
    }
    // Ruling the first realm in the world is a fact about the age, not just
    // about the ruler, and it belongs in the score.
    if let Some(p) = per.polity {
        if w.polities[p].alive() && w.polities[p].ruler == Some(r) {
            let share = world_share(w, p);
            if share > 0.12 {
                add(Deed::Hegemon(share), ((share - 0.1) * 320.0).min(80.0));
            }
            if w.polities[p].kind == PolityKind::Empire {
                add(Deed::Imperial, 18.0);
            }
        }
    }
    if per.renown > 4.0 {
        add(Deed::Renown, (per.renown * 1.6).min(30.0));
    }
    // A crown given by a faith is worth more than a crown taken.
    if per.title.is_some() {
        add(Deed::Anointed, 22.0);
    }
    if !per.children.is_empty() && per.reign_years > 10 {
        add(Deed::House(per.children.len()), 6.0);
    }
}

/// Why somebody stands where they do, ranked, largest first.
///
/// The interface's view of [`for_each_deed`]: the same points, with the
/// wording attached. Because both come from one place, an explanation
/// cannot drift away from the number it explains.
pub fn standing(w: &World, r: usize) -> Vec<Claim> {
    let mut out: Vec<Claim> = Vec::new();
    for_each_deed(w, r, &mut |d, points| {
        out.push(Claim {
            what: d.describe(),
            points,
        });
    });
    out.sort_by(|a, b| b.points.total_cmp(&a.points));
    out
}

/// Somebody's standing in points. Allocates nothing.
pub fn greatness(w: &World, r: usize) -> f32 {
    let mut total = 0.0;
    for_each_deed(w, r, &mut |_, points| total += points);
    total.max(0.0)
}

/// A realm's share of all the settled land in the world.
pub fn world_share(w: &World, p: usize) -> f32 {
    // Clamped: the owned-cell count is a tick behind the claims, so early in
    // a world a realm can briefly appear to hold more land than exists.
    let owned = w.stats.owned_cells.max(1);
    (w.polities[p].cells as f32 / owned as f32).clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// The yearly pass
// ---------------------------------------------------------------------------

/// Marriages, births, standing and acclaim, for one year.
///
/// Ordered deliberately: standing is scored before acclaim reads it, and
/// marriages are made before births can come of them.
pub fn tick(w: &mut World) {
    reigning(w);
    mortality(w);
    child_deaths(w);
    marriages(w);
    births(w);
    growing_up(w);
    score(w);
    coronations(w);
}

/// A year on the throne, for everyone who is on one.
fn reigning(w: &mut World) {
    for p in w.living_polities() {
        if let Some(r) = w.polities[p].ruler {
            w.persons[r].reign_years += 1;
        }
    }
}

/// Everybody who is not on a throne grows old and dies.
///
/// Rulers have always had a death roll of their own, in `politics::rulers`,
/// because their deaths start successions. Nobody else did — which meant a
/// king's younger children, once they were past the age of childhood
/// illness, simply never died. A house accumulated immortal claimants for a
/// thousand years, `adult_heirs` returned people born in the third century
/// to inherit in the twelfth, and the person list grew without bound.
fn mortality(w: &mut World) {
    let rng = w.rng.clone();
    let tn = w.tuning;
    w.forget_the_dead();
    for i in w.alive_persons.clone() {
        let per = &w.persons[i];
        if !per.alive() {
            continue;
        }
        // Whoever holds a throne is `politics::rulers`' business, so that a
        // death there still runs a succession.
        if let Some(p) = per.polity {
            if w.polities[p].alive() && w.polities[p].ruler == Some(i) {
                continue;
            }
        }
        let lifespan = w.races[per.race].lifespan.max(1.0);
        let rel = per.age(w.year) as f32 / lifespan;
        let p_die = tn.ruler_death_base + tn.ruler_death_age_weight * rel.powi(6);
        if !rng.chance(p_die as f64) {
            continue;
        }
        let cause = prose::natural_death(w.persons[i].age(w.year), &Pick::rolled(&rng));
        w.persons[i].died = Some(w.year);
        w.persons[i].death = cause;
        // Only somebody the world had heard of is worth a line.
        if w.persons[i].is_acclaimed() || w.persons[i].renown > 6.0 {
            let text = prose::notable_died(
                &w.persons[i].full_name(),
                w.persons[i].role.name(),
                w.persons[i].age(w.year),
            );
            let loc = w.persons[i].polity.and_then(|p| w.capital_cell(p));
            w.log(1, EventKind::Death, &[Ref::Person(i)], loc, text);
        }
        if let Some(h) = w.persons[i].house {
            w.close_house_if_spent(h);
        }
    }
}

/// Not every child grows up. Without this the world fills with idle
/// claimants and a succession is never in doubt.
fn child_deaths(w: &mut World) {
    let rng = w.rng.clone();
    for p in w.living_polities() {
        let heirs = w.polities[p].heirs.clone();
        for h in heirs {
            if !w.persons[h].alive() || w.persons[h].age(w.year) >= MAJORITY {
                continue;
            }
            if !rng.chance(w.tuning.child_death_chance) {
                continue;
            }
            w.persons[h].died = Some(w.year);
            w.persons[h].death = prose::child_death(&Pick::rolled(&rng));
            let text = prose::child_died(w, h, p);
            w.log(
                0,
                EventKind::Death,
                &[Ref::Person(h), Ref::Polity(p)],
                w.capital_cell(p),
                text,
            );
        }
    }
}

/// Recompute every living notable's standing, and acclaim whoever has
/// crossed the line this year.
///
/// The bar is deliberately *relative*: greatness is being conspicuously
/// above your contemporaries, not passing a fixed number. On a crowded map
/// a hundred points is ordinary and on an empty one it is extraordinary, and
/// a title the chronicle hands out twice a century is worth reading where
/// one it hands out twice a decade is not.
fn score(w: &mut World) {
    let living: Vec<usize> = w.alive_persons.clone();
    let mut scores: Vec<f32> = Vec::with_capacity(living.len());
    for &r in &living {
        let g = greatness(w, r);
        w.persons[r].greatness = g;
        if g > 0.0 {
            scores.push(g);
        }
    }
    // The bar has three parts, and a life must clear all of them:
    //
    //  * the fixed floor, so a thin generation cannot promote a mediocrity;
    //  * a clear margin over the *third*-best living career, so the title
    //    means outstanding rather than merely high;
    //  * a margin over whoever is already acclaimed and still living, so the
    //    world does not carry six Greats at once.
    //
    // Together these make acclaim something the reader meets a couple of
    // times a century, which is the rate at which it stays worth reading.
    scores.sort_by(|a, b| b.total_cmp(a));
    let third = scores.get(2).copied().unwrap_or(0.0);
    let living_acclaimed = living
        .iter()
        .filter(|&&r| w.persons[r].is_acclaimed())
        .map(|&r| w.persons[r].greatness)
        .fold(0.0f32, f32::max);
    let bar = ACCLAIM_THRESHOLD
        .max(third * ACCLAIM_MARGIN)
        .max(living_acclaimed * 1.05);
    // One a year at most, and the best of them.
    let best = living
        .iter()
        .copied()
        .filter(|&r| w.persons[r].acclaimed.is_none() && w.persons[r].greatness >= bar)
        .max_by(|&a, &b| {
            w.persons[a]
                .greatness
                .total_cmp(&w.persons[b].greatness)
                .then(b.cmp(&a))
        });
    if let Some(r) = best {
        acclaim(w, r);
    }
}

/// Name somebody great while they can still hear it.
///
/// This is the event the world used to miss. It takes the epithet the
/// obituary would have used and awards it now, in a line of its own, at an
/// importance that puts it in front of any reader.
fn acclaim(w: &mut World, r: usize) {
    let p = match w.persons[r].polity {
        Some(p) if w.polities[p].alive() => p,
        _ => return,
    };
    let rng = w.rng.clone();
    let epithet = acclaim_epithet(w, r, &Pick::rolled(&rng));
    w.persons[r].epithet = Some(epithet.clone());
    w.persons[r].acclaimed = Some(w.year);
    let is_ruler = w.polities[p].ruler == Some(r);
    let deeds = standing(w, r);
    let first = deeds.first().map(|c| c.what.clone()).unwrap_or_default();
    let text = prose::acclaimed(w, r, p, &epithet, &first, is_ruler);
    w.log(
        3,
        EventKind::Person,
        &[Ref::Person(r), Ref::Polity(p)],
        w.capital_cell(p),
        text,
    );
}

/// The style the world settles on, chosen from what they actually did.
/// Exactly one draw.
fn acclaim_epithet(w: &World, r: usize, pick: &Pick) -> String {
    let per = &w.persons[r];
    let t = per.traits;
    let mut opts: Vec<&str> = Vec::new();
    let fast = per.reign_years >= 3 && per.taken as f32 / per.reign_years as f32 > 3.0;
    if fast && per.taken > 50 {
        opts.extend(["the Great", "the Conqueror", "the Unstoppable"]);
    }
    if per.taken > 80 {
        opts.extend(["the Great", "Widereach", "the World-Taker"]);
    }
    if per.taken < 15 && per.gained > 40 {
        opts.extend(["the Settler", "the Provider", "Openhand"]);
    }
    if per.cities_taken >= 4 {
        opts.extend(["the Citybreaker", "the Stormer"]);
    }
    if per.cities_founded >= 4 {
        opts.extend(["the Builder", "the Founder", "the Lawgiver"]);
    }
    if per.battles_won >= 8 {
        opts.extend(["the Invincible", "the Lion", "Ironhand"]);
    }
    if per.reign_years >= 45 {
        opts.extend(["the Long-Reigning", "the Old", "the Enduring"]);
    }
    if t.cruelty > 0.72 {
        opts.extend(["the Terrible", "the Scourge", "Bloodhand"]);
    }
    if t.wisdom > 0.72 {
        opts.extend(["the Wise", "the Just", "the Lawgiver"]);
    }
    if t.piety > 0.74 {
        opts.extend(["the Blessed", "the Devout"]);
    }
    if t.charisma > 0.74 {
        opts.extend(["the Beloved", "Goldentongue"]);
    }
    if opts.is_empty() {
        opts.extend(["the Great", "the Mighty", "the Renowned"]);
    }
    pick.of(&opts).to_string()
}

// ---------------------------------------------------------------------------
// Marriage
// ---------------------------------------------------------------------------

/// Whether two people may marry: living adults, unwed, and not close kin.
fn may_marry(w: &World, a: usize, b: usize) -> bool {
    if a == b {
        return false;
    }
    let (pa, pb) = (&w.persons[a], &w.persons[b]);
    if !pa.alive() || !pb.alive() || pa.spouse.is_some() || pb.spouse.is_some() {
        return false;
    }
    if pa.age(w.year) < MARRIAGE_AGE || pb.age(w.year) < MARRIAGE_AGE {
        return false;
    }
    // No siblings, and no marrying your own parent or child.
    if pa.parent.is_some() && pa.parent == pb.parent {
        return false;
    }
    if pa.parent == Some(b) || pb.parent == Some(a) {
        return false;
    }
    if w.races[pa.race].lifespan != w.races[pb.race].lifespan && pa.race != pb.race {
        // Peoples who do not share a lifespan rarely share a marriage bed;
        // it is allowed, but it is not the ordinary case.
        return w.rng.chance(0.15);
    }
    true
}

/// Join two people, and their houses with them.
fn wed(w: &mut World, a: usize, b: usize) {
    w.persons[a].spouse = Some(b);
    w.persons[b].spouse = Some(a);
}

/// Unwed rulers look for a match: abroad if there is a realm worth tying to,
/// at home otherwise.
fn marriages(w: &mut World) {
    let rng = w.rng.clone();
    for p in w.living_polities() {
        let r = match w.polities[p].ruler {
            Some(r) => r,
            None => continue,
        };
        if w.persons[r].spouse.is_some() || w.persons[r].age(w.year) < MARRIAGE_AGE {
            continue;
        }
        if !rng.chance(w.tuning.marriage_chance) {
            continue;
        }
        // A foreign match, if a neighbour has an unwed heir and no war on.
        let mut foreign: Option<(usize, usize)> = None;
        let neighbors: Vec<usize> = w.polities[p].neighbors.iter().map(|&(q, _)| q).collect();
        for q in neighbors {
            if !w.polities[q].alive() || w.war_between(p, q).is_some() {
                continue;
            }
            if w.polities[q].culture != w.polities[p].culture && rng.chance(0.4) {
                continue;
            }
            let candidates: Vec<usize> = w.polities[q]
                .heirs
                .iter()
                .copied()
                .filter(|&h| may_marry(w, r, h))
                .collect();
            if let Some(&h) = candidates.first() {
                foreign = Some((q, h));
                break;
            }
        }
        match foreign {
            Some((q, h)) => {
                wed(w, r, h);
                set_stance(w, p, q, Stance::Married);
                // A marriage cools a border for a generation.
                w.polities[p].tension.insert(q, 0.0);
                w.polities[q].tension.insert(p, 0.0);
                let text = prose::married_abroad(w, r, h, p, q);
                w.log(
                    2,
                    EventKind::Politics,
                    &[
                        Ref::Person(r),
                        Ref::Person(h),
                        Ref::Polity(p),
                        Ref::Polity(q),
                    ],
                    w.capital_cell(p),
                    text,
                );
            }
            None => {
                // A match at home. The consort is a real person, so their
                // children have two parents the chronicle can name.
                let culture = w.polities[p].culture;
                let gender = match w.persons[r].gender {
                    Gender::F => Gender::M,
                    Gender::M => Gender::F,
                    Gender::N => Gender::N,
                };
                let age = w.year - rng.int(MARRIAGE_AGE, 34);
                let consort = w.new_person(culture, Role::Noble, Some(p), age, None);
                w.persons[consort].gender = gender;
                if let Some(h) = w.polities[p].house {
                    w.join_house(consort, h);
                }
                wed(w, r, consort);
                let text = prose::married_at_home(w, r, consort, p);
                w.log(
                    0,
                    EventKind::Politics,
                    &[Ref::Person(r), Ref::Person(consort), Ref::Polity(p)],
                    w.capital_cell(p),
                    text,
                );
            }
        }
    }
}

/// Record a stance both ways round, with the year it was taken.
pub fn set_stance(w: &mut World, p: usize, q: usize, st: Stance) {
    for (a, b) in [(p, q), (q, p)] {
        if st == Stance::Neutral {
            w.polities[a].stance.remove(&b);
            w.polities[a].stance_since.remove(&b);
        } else {
            w.polities[a].stance.insert(b, st);
            w.polities[a].stance_since.insert(b, w.year);
        }
    }
}

/// How two realms stand. An absent arrangement is [`Stance::Neutral`].
pub fn stance_of(w: &World, p: usize, q: usize) -> Stance {
    w.polities[p]
        .stance
        .get(&q)
        .copied()
        .unwrap_or(Stance::Neutral)
}

// ---------------------------------------------------------------------------
// Children
// ---------------------------------------------------------------------------

/// Married rulers have children, who become the realm's heirs.
fn births(w: &mut World) {
    let rng = w.rng.clone();
    for p in w.living_polities() {
        let r = match w.polities[p].ruler {
            Some(r) => r,
            None => continue,
        };
        let spouse = match w.persons[r].spouse {
            Some(s) if w.persons[s].alive() => s,
            _ => continue,
        };
        // Fertility falls away with age, measured against the people's own
        // span rather than a fixed number of years.
        let lifespan = w.races[w.persons[r].race].lifespan;
        let rel = w.persons[r].age(w.year) as f32 / lifespan;
        if rel > 0.6 {
            continue;
        }
        let fecund = w.races[w.persons[r].race].fecund;
        let chance = w.tuning.birth_chance * (0.6 + fecund as f64) * (1.0 - rel as f64).max(0.1);
        if !rng.chance(chance) {
            continue;
        }
        if w.persons[r].children.len() >= 7 {
            continue;
        }
        let culture = w.polities[p].culture;
        let traits = w.persons[r].traits.inherit(&rng);
        let child = w.new_person(culture, Role::Noble, Some(p), w.year, Some(traits));
        w.persons[child].parent = Some(r);
        if let Some(h) = w.polities[p].house {
            w.join_house(child, h);
        }
        w.persons[r].children.push(child);
        w.persons[spouse].children.push(child);
        let text = prose::child_born(w, child, r, p);
        w.log(
            0,
            EventKind::Person,
            &[Ref::Person(child), Ref::Person(r), Ref::Polity(p)],
            w.capital_cell(p),
            text,
        );
    }
}

/// Keep each realm's list of claimants current: living children of the
/// ruling line, oldest first, with the heir apparent at the front.
fn growing_up(w: &mut World) {
    for p in w.living_polities() {
        let r = match w.polities[p].ruler {
            Some(r) => r,
            None => {
                w.polities[p].heirs.clear();
                continue;
            }
        };
        let mut heirs: Vec<usize> = w.persons[r]
            .children
            .iter()
            .copied()
            .filter(|&c| w.persons[c].alive())
            .collect();
        heirs.sort_by_key(|&c| (w.persons[c].born, c));
        w.polities[p].heirs = heirs;
    }
}

/// Everyone with a claim on `p` who is old enough to press it.
pub fn adult_heirs(w: &World, p: usize) -> Vec<usize> {
    w.polities[p]
        .heirs
        .iter()
        .copied()
        .filter(|&h| w.persons[h].alive() && w.persons[h].age(w.year) >= MAJORITY)
        .collect()
}

// ---------------------------------------------------------------------------
// Crowns given rather than taken
// ---------------------------------------------------------------------------

/// A faith crowns a ruler it approves of.
///
/// This is the moment that turns a successful king into an emperor without
/// a battle: a realm large enough to matter, a state faith with reach, and a
/// ruler pious enough for the clergy to bless. It grants a title the person
/// keeps, and it is the one route to [`PolityKind::Empire`] that does not
/// run through conquest.
fn coronations(w: &mut World) {
    let rng = w.rng.clone();
    for p in w.living_polities() {
        let r = match w.polities[p].ruler {
            Some(r) => r,
            None => continue,
        };
        if w.persons[r].title.is_some() {
            continue;
        }
        let s = match w.polities[p].school {
            Some(s) if w.schools[s].alive() => s,
            _ => continue,
        };
        if w.schools[s].kind != SchoolKind::Divine {
            continue;
        }
        let pol = &w.polities[p];
        if pol.cells < w.tuning.coronation_min_cells || pol.prestige < 12.0 {
            continue;
        }
        if w.persons[r].traits.piety < 0.5 || w.persons[r].greatness < 55.0 {
            continue;
        }
        // The faith has to be more than a household chapel: it must be the
        // state faith of somebody else too, or hold this realm outright.
        let reach = w.schools[s].influence.get(&p).copied().unwrap_or(0.0);
        if reach < 0.6 && w.schools[s].state_of.len() < 2 {
            continue;
        }
        if !rng.chance(w.tuning.coronation_chance) {
            continue;
        }
        let title = prose::crown_title(w, p, r, &Pick::rolled(&rng));
        w.persons[r].title = Some(title.clone());
        w.polities[p].prestige += 18.0;
        w.polities[p].stability = (w.polities[p].stability + 0.12).min(1.0);
        // A crowned king is an emperor, whatever the map says.
        if w.polities[p].kind == PolityKind::Kingdom
            && w.polities[p].cells >= (w.tuning.empire_min_cells / 2) as usize
        {
            super::politics::promote_to_empire(w, p);
        }
        let text = prose::crowned_by_faith(w, r, p, s, &title);
        w.log(
            3,
            EventKind::Politics,
            &[Ref::Person(r), Ref::Polity(p), Ref::School(s)],
            w.capital_cell(p),
            text,
        );
    }
}

// ---------------------------------------------------------------------------
// Inheritance customs
// ---------------------------------------------------------------------------

/// How a realm of this kind and culture passes its throne on.
///
/// Drawn once, when the realm is founded, from what the culture values:
/// traditional peoples keep the whole together, open ones divide it, and a
/// republic or a magocracy does not inherit at all.
pub fn inheritance_for(w: &World, culture: usize, kind: PolityKind) -> Inheritance {
    if !kind.has_dynasty() {
        return Inheritance::Elective;
    }
    let v = w.cultures[culture].values;
    let rng = &w.rng;
    // One draw, whatever the branch, so the stream does not depend on
    // which arm a culture happens to fall in.
    let roll = rng.f32();
    if v.tradition > 0.62 {
        if roll < 0.75 {
            Inheritance::Primogeniture
        } else {
            Inheritance::Tanistry
        }
    } else if v.openness > 0.6 {
        if roll < 0.55 {
            Inheritance::Partition
        } else {
            Inheritance::Primogeniture
        }
    } else if roll < 0.5 {
        Inheritance::Primogeniture
    } else if roll < 0.8 {
        Inheritance::Partition
    } else {
        Inheritance::Tanistry
    }
}
