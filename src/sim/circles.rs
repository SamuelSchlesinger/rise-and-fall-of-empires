//! Who a person knows, and what happens when those people collide.
//!
//! A ruler had a rich life: they married, had heirs, conquered, were
//! acclaimed, were murdered. Everybody else had one event and then nothing.
//! A poet composed a single lay and was never heard from again; an explorer
//! named one sea; a general accumulated battles in silence. Worse, none of
//! them had any relationship with each other — `Person::rival` existed in
//! the struct and was never once written to.
//!
//! So the two ties that actually generate story between people who do not
//! hold a throne:
//!
//! * **Rivalry.** Two people of comparable standing, in the same realm and
//!   the same line of work, measuring themselves against each other. A
//!   rivalry is worth having because it turns every subsequent achievement
//!   by either of them into an event about *both* — and because it can go
//!   bad, which is where a knife in a corridor comes from.
//! * **Patronage.** Somebody of standing takes up somebody young. The
//!   protégé rises faster for it, inherits the position when the patron
//!   dies, and sometimes outgrows them — which is its own event, and the
//!   reason a line of generals can run three deep.
//!
//! Both are cheap: they walk the living notables, of whom there are a few
//! dozen, not the person vector, of which there are tens of thousands.

use super::chronicle::{EventKind, Ref};
use super::prose::{self, Pick};
use super::{Role, World};

/// Renown below which nobody is worth a rivalry: the world has to have
/// heard of you before it can care that somebody is your equal.
const NOTABLE_RENOWN: f32 = 2.0;
/// How close two people's standing must be to be rivals, as a ratio.
const MATCHED: f32 = 0.55;

/// One year of rivalries and patronage.
pub fn tick(w: &mut World) {
    pair_rivals(w);
    take_proteges(w);
    rivalries_tell(w);
}

/// Everybody currently worth watching who does not hold a throne.
fn notables(w: &World) -> Vec<usize> {
    w.alive_persons
        .iter()
        .copied()
        .filter(|&i| {
            let per = &w.persons[i];
            // The living index carries this year's dead until `recompute`
            // sweeps it, and a corpse is nobody's rival.
            if !per.alive() || per.renown < NOTABLE_RENOWN {
                return false;
            }
            // A ruler's story is told elsewhere.
            match per.polity {
                Some(p) => w.polities[p].alive() && w.polities[p].ruler != Some(i),
                None => false,
            }
        })
        .collect()
}

/// Two people of a kind, in one realm, find they are being compared.
fn pair_rivals(w: &mut World) {
    let rng = w.rng.clone();
    if !rng.chance(w.tuning.rivalry_chance) {
        return;
    }
    let pool = notables(w);
    if pool.len() < 2 {
        return;
    }
    // Find every pair that could be rivals, then choose among them.
    //
    // This used to draw two people out of the pool at random and try again
    // if they did not suit, twenty-four times. Four conditions have to hold
    // at once --- same trade, comparable fame, neither already matched, and
    // realms that at least share a border --- and with a pool of half a
    // dozen the chance that a random pair satisfies all four is small
    // enough that the whole system fired about once a century. The
    // machinery was right; the search was a rejection sampler with an
    // acceptance rate near zero.
    //
    // Sorting by trade and then by fame puts every candidate next to the
    // people it could plausibly be measured against, so walking neighbours
    // finds the matches instead of hoping to stumble on them.
    let mut free: Vec<usize> = pool
        .into_iter()
        .filter(|&i| w.persons[i].rival.is_none())
        .collect();
    free.sort_by(|&a, &b| {
        (w.persons[a].role as u8)
            .cmp(&(w.persons[b].role as u8))
            .then(w.persons[b].renown.total_cmp(&w.persons[a].renown))
            .then(a.cmp(&b))
    });
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for pair in free.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if w.persons[a].role != w.persons[b].role {
            continue;
        }
        let (Some(pa), Some(pb)) = (w.persons[a].polity, w.persons[b].polity) else {
            continue;
        };
        // In one realm, or in realms that know of each other. A rivalry
        // across a border is the better story anyway.
        if pa != pb && !w.polities[pa].neighbors.iter().any(|&(q, _)| q == pb) {
            continue;
        }
        let (ra, rb) = (w.persons[a].renown, w.persons[b].renown);
        if ra.min(rb) < ra.max(rb) * MATCHED {
            continue;
        }
        pairs.push((a, b));
    }
    if pairs.is_empty() {
        return;
    }
    let (a, b) = pairs[rng.below(pairs.len())];
    let (pa, pb) = (
        w.persons[a].polity.unwrap_or(0),
        w.persons[b].polity.unwrap_or(0),
    );
    w.persons[a].rival = Some(b);
    w.persons[b].rival = Some(a);
    let p = pa;
    let text = if pa == pb {
        prose::rivalry_begun(w, a, b, &Pick::rolled(&rng))
    } else {
        // One draw either way, so the stream does not depend on which arm
        // is taken.
        let _ = Pick::rolled(&rng).index(3);
        prose::rivalry_across_border(w, a, b, pa, pb)
    };
    w.log(
        1,
        EventKind::Person,
        &[Ref::Person(a), Ref::Person(b), Ref::Polity(p)],
        w.capital_cell(p),
        text,
    );
}

/// Somebody of standing takes up somebody young.
fn take_proteges(w: &mut World) {
    let rng = w.rng.clone();
    if !rng.chance(w.tuning.patronage_chance) {
        return;
    }
    let pool = notables(w);
    if pool.is_empty() {
        return;
    }
    let patron = pool[rng.below(pool.len())];
    let Some(p) = w.persons[patron].polity else {
        return;
    };
    if !w.polities[p].alive() {
        return;
    }
    // One at a time. A patron who takes on four pupils in twenty years is
    // running a school, not making a protege, and the chronicle said so
    // four times in the same words.
    if w.alive_persons
        .iter()
        .any(|&x| w.persons[x].alive() && w.persons[x].served == Some(patron))
    {
        return;
    }
    // A protégé is made, not found: somebody young of the same realm, who
    // will carry on the patron's line of work.
    let culture = w.polities[p].culture;
    let role = w.persons[patron].role;
    let age = w.year - rng.int(16, 26);
    let protege = w.new_person(culture, role, Some(p), age, None);
    w.persons[protege].served = Some(patron);
    // They begin with a share of their patron's standing: the point of a
    // patron is that doors are already open.
    w.persons[protege].renown = w.persons[patron].renown * 0.25;
    if role == Role::General {
        super::war::role_general(w, p, protege);
    }
    let text = prose::patronage(w, patron, protege, &Pick::rolled(&rng));
    w.log(
        1,
        EventKind::Person,
        &[Ref::Person(patron), Ref::Person(protege), Ref::Polity(p)],
        w.capital_cell(p),
        text,
    );
}

/// What rivalries and patronage do once they exist.
///
/// Three things can happen, and all of them are events about two people
/// rather than one: a rival is outshone, a protégé outgrows their patron, or
/// somebody decides the simplest way past a rival is through them.
fn rivalries_tell(w: &mut World) {
    let rng = w.rng.clone();
    for i in w.alive_persons.clone() {
        // The dead are somebody else's business.
        if !w.persons[i].alive() {
            continue;
        }
        // A protégé who has outgrown the person who made them.
        if let Some(patron) = w.persons[i].served {
            if w.persons[patron].alive()
                && w.persons[i].renown > w.persons[patron].renown * 1.5
                && w.persons[i].renown > NOTABLE_RENOWN * 2.0
                && rng.chance(w.tuning.protege_surpasses_chance)
            {
                // Recorded by clearing the tie: they are their own figure
                // now, and it only happens once.
                w.persons[i].served = None;
                if let Some(p) = w.persons[i].polity {
                    let text = prose::protege_surpassed(w, i, patron);
                    w.log(
                        1,
                        EventKind::Person,
                        &[Ref::Person(i), Ref::Person(patron), Ref::Polity(p)],
                        w.capital_cell(p),
                        text,
                    );
                }
            }
        }
        let Some(rival) = w.persons[i].rival else {
            continue;
        };
        if !w.persons[rival].alive() {
            // A rivalry ends when one of them does, and the survivor is
            // remembered as having outlived it.
            w.persons[i].rival = None;
            if w.persons[i].renown > NOTABLE_RENOWN * 2.0 {
                if let Some(p) = w.persons[i].polity {
                    let text = prose::rival_outlived(w, i, rival);
                    w.log(
                        1,
                        EventKind::Death,
                        &[Ref::Person(i), Ref::Person(rival), Ref::Polity(p)],
                        w.capital_cell(p),
                        text,
                    );
                }
            }
            continue;
        }
        // One has plainly passed the other.
        let (mine, theirs) = (w.persons[i].renown, w.persons[rival].renown);
        if mine > theirs * 2.0 && mine > NOTABLE_RENOWN * 2.0 && rng.chance(0.06) {
            if let Some(p) = w.persons[i].polity {
                let text = prose::rival_eclipsed(w, i, rival);
                w.log(
                    1,
                    EventKind::Person,
                    &[Ref::Person(i), Ref::Person(rival), Ref::Polity(p)],
                    w.capital_cell(p),
                    text,
                );
            }
            // Being beaten is a spur to some and a poison to others.
            w.persons[rival].renown += 0.5;
            w.persons[i].rival = None;
            w.persons[rival].rival = None;
            continue;
        }
        // Or the loser decides the shorter road is through the winner.
        let spite = w.persons[rival].traits.cruelty * (1.0 - w.persons[rival].traits.wisdom);
        if mine > theirs * 1.4 && rng.chance(w.tuning.rival_murder_chance * spite as f64) {
            let killer = rival;
            w.persons[i].died = Some(w.year);
            w.persons[i].death = prose::murdered_by_rival(&w.persons[killer].name.clone());
            w.persons[killer].rival = None;
            if let Some(p) = w.persons[i].polity {
                w.polities[p].stability = (w.polities[p].stability - 0.04).max(0.0);
                let text = prose::rival_murdered(w, i, killer);
                w.log(
                    2,
                    EventKind::Death,
                    &[Ref::Person(i), Ref::Person(killer), Ref::Polity(p)],
                    w.capital_cell(p),
                    text,
                );
            }
        }
    }
}
