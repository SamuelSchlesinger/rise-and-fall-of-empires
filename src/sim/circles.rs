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
    // Look for an unmatched pair in the same line of work and of comparable
    // standing, in the same realm or in realms that share a border.
    //
    // Requiring the same realm made rivalries so rare they never once fired:
    // two generals of equal fame in one kingdom at one time is a narrow
    // thing to ask for. Across a border it is common — and a rivalry between
    // two men on opposite sides of a war is the better story anyway.
    for _ in 0..24 {
        let a = pool[rng.below(pool.len())];
        let b = pool[rng.below(pool.len())];
        if a == b || w.persons[a].rival.is_some() || w.persons[b].rival.is_some() {
            continue;
        }
        if w.persons[a].role != w.persons[b].role {
            continue;
        }
        let (Some(pa), Some(pb)) = (w.persons[a].polity, w.persons[b].polity) else {
            continue;
        };
        let known_to_each_other =
            pa == pb || w.polities[pa].neighbors.iter().any(|&(q, _)| q == pb);
        if !known_to_each_other {
            continue;
        }
        let (ra, rb) = (w.persons[a].renown, w.persons[b].renown);
        if ra.min(rb) < ra.max(rb) * MATCHED {
            continue;
        }
        w.persons[a].rival = Some(b);
        w.persons[b].rival = Some(a);
        let p = pa;
        let text = if pa == pb {
            prose::rivalry_begun(w, a, b, &Pick::rolled(&rng))
        } else {
            // One draw either way, so the stream does not depend on which
            // arm is taken.
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
        return;
    }
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
