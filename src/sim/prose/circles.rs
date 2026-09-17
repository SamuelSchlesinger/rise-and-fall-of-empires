//! Sentences about the ties between people who do not hold a throne.
//!
//! Every one of these is about *two* people, which is the point: a rivalry
//! or a patronage turns each subsequent achievement by either of them into
//! something the reader already has a stake in.

use super::Pick;
use super::{cap, World};

/// Who somebody is, with their trade. One draw is spent by the caller.
fn who(w: &World, i: usize) -> String {
    format!("{} the {}", w.persons[i].name, w.persons[i].role.name())
}

/// Two people of a kind discover they are being compared. One draw.
pub fn rivalry_begun(w: &World, a: usize, b: usize, pick: &Pick) -> String {
    let (na, nb) = (who(w, a), who(w, b));
    match pick.index(3) {
        0 => format!(
            "{} and {} were spoken of in the same breath once too often, and began to mind it.",
            cap(&na),
            nb
        ),
        1 => format!(
            "A rivalry set in between {} and {}. Neither would say when it had begun; \
             both could have told you the day.",
            na, nb
        ),
        _ => format!(
            "{} could not be praised in {}'s hearing, nor {} in {}'s.",
            cap(&na),
            w.persons[b].name,
            w.persons[b].name,
            w.persons[a].name
        ),
    }
}

/// A rivalry that runs across a border rather than within a court.
pub fn rivalry_across_border(w: &World, a: usize, b: usize, pa: usize, pb: usize) -> String {
    format!(
        "{} of {} and {} of {} had never met, and each had spent a career being measured against the other.",
        who(w, a),
        super::realm(w, pa),
        who(w, b),
        super::realm(w, pb)
    )
}

/// Somebody of standing takes up somebody young. One draw.
pub fn patronage(w: &World, patron: usize, protege: usize, pick: &Pick) -> String {
    let p = who(w, patron);
    let name = &w.persons[protege].name;
    match pick.index(3) {
        0 => format!(
            "{} took {} into their household and taught them the trade.",
            cap(&p),
            name
        ),
        1 => format!(
            "{} was found by {}, who saw something in them that nobody else had troubled to look for.",
            name, p
        ),
        _ => format!(
            "{} began as {}'s pupil, and doors that open slowly opened quickly.",
            name,
            w.persons[patron].name
        ),
    }
}

/// A pupil passes the person who made them.
pub fn protege_surpassed(w: &World, protege: usize, patron: usize) -> String {
    format!(
        "{} had outgrown {}, and both of them knew it before either said so.",
        cap(&who(w, protege)),
        w.persons[patron].name
    )
}

/// One rival plainly passes the other.
pub fn rival_eclipsed(w: &World, winner: usize, loser: usize) -> String {
    format!(
        "{} had so far passed {} that the comparison stopped being made, which was the \
         unkindest part of it.",
        cap(&who(w, winner)),
        w.persons[loser].name
    )
}

/// And one outlives the other.
pub fn rival_outlived(w: &World, survivor: usize, gone: usize) -> String {
    format!(
        "{} outlived {}, and found they had less appetite for the work than they expected.",
        cap(&who(w, survivor)),
        w.persons[gone].name
    )
}

/// The shorter road past a rival.
pub fn rival_murdered(w: &World, victim: usize, killer: usize) -> String {
    format!(
        "{} was killed, and everyone knew who by: {} had been second to them for years, \
         and was not second to them now.",
        cap(&who(w, victim)),
        w.persons[killer].name
    )
}

/// How the chronicle records such a death on the person's own page.
pub fn murdered_by_rival(killer: &str) -> String {
    format!("was killed by {}, who had been their rival", killer)
}
