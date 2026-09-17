//! Sentences about arrangements between realms: alliances, tribute, war
//! aims and the peaces that settle them.
//!
//! The war-aim sentences are the reason this file exists. A war used to be
//! announced without saying what it was for and settled without saying what
//! had been decided, so the chronicle filled with declarations that promised
//! nothing and peaces that reported nothing — the commonest line in a
//! two-thousand-year history was "neither side the master". Here every
//! declaration states a purpose and every peace returns a verdict on it.

use super::{realm, realm_adj, realm_full, realm_full_cap, years};
use super::{Pick, World};
use crate::sim::WarAim;

// ---------------------------------------------------------------------------
// What a war is for
// ---------------------------------------------------------------------------

/// The aim as a purpose clause: "to take Aldar", "to break the grip of Velen".
pub fn aim_purpose(w: &World, attacker: usize, defender: usize, aim: WarAim) -> String {
    match aim {
        WarAim::Border => format!("to take the marches of {}", realm(w, defender)),
        WarAim::City(c) => format!("to take {}", w.cities[c].name),
        WarAim::Vassalage => format!("to make {} pay tribute", realm(w, defender)),
        WarAim::Independence => format!("to throw off the yoke of {}", realm(w, defender)),
        WarAim::Claimant(r) => format!(
            "to set {} upon the throne of {}",
            w.persons[r].name,
            realm(w, defender)
        ),
        WarAim::Faith => match w.polities[attacker].school {
            Some(s) => format!("to bring {} to {}", realm(w, defender), w.schools[s].short),
            None => format!("to bring {} to the true teaching", realm(w, defender)),
        },
        WarAim::Relic(x) => format!("to seize {}", w.artifacts[x].name),
        WarAim::Plunder => format!("to strip {} of everything it had", realm(w, defender)),
        WarAim::Containment => format!(
            "to break the grip of {} before it swallowed them all",
            realm(w, defender)
        ),
    }
}

/// The aim as a noun phrase, for a peace line: "the city", "the tribute".
fn aim_object(w: &World, aim: WarAim) -> String {
    match aim {
        WarAim::Border => "the lands it had claimed".to_string(),
        WarAim::City(c) => w.cities[c].name.clone(),
        WarAim::Vassalage => "the submission it had demanded".to_string(),
        WarAim::Independence => "its independence".to_string(),
        WarAim::Claimant(r) => format!("the throne for {}", w.persons[r].name),
        WarAim::Faith => "the conversion it had demanded".to_string(),
        WarAim::Relic(x) => w.artifacts[x].name.clone(),
        WarAim::Plunder => "the plunder it had come for".to_string(),
        WarAim::Containment => "the humbling of its enemy".to_string(),
    }
}

// ---------------------------------------------------------------------------
// The peaces
// ---------------------------------------------------------------------------

/// The attacker prevailed. `met` says whether the stated aim was actually
/// achieved, because a realm can win a war and still not get what it came
/// for — which is a better sentence than either outcome alone.
pub fn peace_attacker_won_aim(
    w: &World,
    attacker: usize,
    defender: usize,
    years_fought: i32,
    aim: WarAim,
    met: bool,
) -> String {
    let t = years(years_fought.max(1) as i64);
    if !met {
        return format!(
            "ended in victory for {} after {}, though {} was never taken.",
            realm(w, attacker),
            t,
            aim_object(w, aim)
        );
    }
    match aim {
        WarAim::Vassalage => format!(
            "ended after {} with {} bending the knee: it kept its crown and its customs, and sent tribute to {} every year thereafter.",
            t,
            realm(w, defender),
            realm(w, attacker)
        ),
        WarAim::Independence => format!(
            "ended after {} with {} free of its overlord, and the tribute rolls burned in the square.",
            t,
            realm(w, attacker)
        ),
        WarAim::Claimant(r) => format!(
            "ended after {} with {} crowned in the enemy's own capital, a ruler set on a throne by a foreign army.",
            t, w.persons[r].name
        ),
        WarAim::Faith => format!(
            "ended after {} with {} swearing to the victor's faith, whatever its people believed in private.",
            t,
            realm(w, defender)
        ),
        WarAim::Relic(x) => format!(
            "ended after {} with {} carried back in triumph to {}.",
            t,
            w.artifacts[x].name,
            realm(w, attacker)
        ),
        WarAim::City(c) => format!(
            "ended after {} with {} in the victor's hands and its walls in the victor's keeping.",
            t, w.cities[c].name
        ),
        WarAim::Plunder => format!(
            "ended after {} when the raiders went home heavy, leaving {} nothing worth taking twice.",
            t,
            realm(w, defender)
        ),
        WarAim::Containment => format!(
            "ended after {} with {} stripped of its clients and its reputation. The coalition had not taken its land; it had taken its future.",
            t,
            realm(w, defender)
        ),
        WarAim::Border => format!(
            "ended in victory for {} after {}; {} ceded the lands it had lost and paid an indemnity.",
            realm(w, attacker),
            t,
            realm(w, defender)
        ),
    }
}

/// The attacker was thrown back, and the aim says what they failed at.
pub fn peace_defender_won_aim(
    w: &World,
    attacker: usize,
    defender: usize,
    years_fought: i32,
    aim: WarAim,
) -> String {
    let t = years(years_fought.max(1) as i64);
    match aim {
        WarAim::Containment => format!(
            "ended after {} with the coalition broken and {} stronger than when it began. Nobody would try that again for a generation.",
            t,
            realm(w, defender)
        ),
        WarAim::Independence => format!(
            "ended after {} with the rising crushed and the tribute doubled.",
            t
        ),
        WarAim::Claimant(r) => format!(
            "ended after {} with {} a pretender still, and an exile now.",
            t, w.persons[r].name
        ),
        _ => format!(
            "ended after {} with {} thrown back and humbled, {} never within reach.",
            t,
            realm(w, attacker),
            aim_object(w, aim)
        ),
    }
}

/// Neither side could finish the other — but the chronicle still says what
/// the war had been about, so the line carries a fact and not just a mood.
pub fn peace_stalemate_aim(
    w: &World,
    attacker: usize,
    defender: usize,
    years_fought: i32,
    aim: WarAim,
) -> String {
    let t = years(years_fought.max(1) as i64);
    match aim {
        WarAim::Containment => format!(
            "ended after {} with {} checked but unbroken, which both sides called a victory.",
            t,
            realm(w, defender)
        ),
        WarAim::City(c) => format!(
            "ended after {} with {} still standing and still its own; the siege works were left to rot.",
            t, w.cities[c].name
        ),
        WarAim::Independence => format!(
            "ended after {} in an exhausted settlement: the tribute was halved and nobody spoke of loyalty.",
            t
        ),
        _ => format!(
            "ended after {} with neither side the master. {} gave up {} and both realms counted their dead.",
            t,
            realm_full_cap(w, attacker),
            aim_object(w, aim)
        ),
    }
}

/// A ruler removed by a foreign army.
pub fn deposed() -> &'static str {
    "was deposed and put to death by a foreign army."
}

// ---------------------------------------------------------------------------
// Declarations that say what they want
// ---------------------------------------------------------------------------

/// A declaration, with the aim stated. Falls back to the plain form for the
/// kinds of war that already read well without one.
pub fn war_declared_aim(
    w: &World,
    attacker: usize,
    defender: usize,
    kind: crate::sim::WarKind,
    cause: &str,
    war_name: &str,
    aim: WarAim,
) -> String {
    let base = super::war_declared(w, attacker, defender, kind, cause, war_name);
    match aim {
        WarAim::Border => base,
        _ => format!(
            "{} {} marched {}.",
            base,
            realm_full_cap(w, attacker),
            aim_purpose(w, attacker, defender, aim)
        ),
    }
}

// ---------------------------------------------------------------------------
// Alliances
// ---------------------------------------------------------------------------

/// Two realms come to an arrangement, and the sentence names the reason.
pub fn alliance_made(
    w: &World,
    p: usize,
    q: usize,
    threat: usize,
    against_hegemon: bool,
) -> String {
    if against_hegemon {
        format!(
            "{} and {} set their own quarrel aside and swore to stand together, \
             for {} had grown too great for either to face alone.",
            realm_full_cap(w, p),
            realm_full(w, q),
            realm_full(w, threat)
        )
    } else {
        format!(
            "{} and {} exchanged envoys and oaths against {}.",
            realm_full_cap(w, p),
            realm_full(w, q),
            realm_full(w, threat)
        )
    }
}

/// An alliance that has outlived its reason.
pub fn alliance_lapsed(w: &World, p: usize, q: usize) -> String {
    format!(
        "The old oath between {} and {} was allowed to lapse, unrenewed and unmourned.",
        realm_full(w, p),
        realm_full(w, q)
    )
}

/// An ally answers the call.
pub fn ally_joined(w: &World, q: usize, principal: usize, foe: usize, war_name: &str) -> String {
    format!(
        "{} honoured its oath to {} and declared against {}, and {} was no longer a quarrel between two realms.",
        realm_full_cap(w, q),
        realm(w, principal),
        realm_full(w, foe),
        war_name
    )
}

// ---------------------------------------------------------------------------
// Tribute
// ---------------------------------------------------------------------------

/// The casus belli of a tributary that has had enough.
pub fn tribute_refused(w: &World, p: usize, over: usize) -> String {
    format!(
        "the tribute owed by {} to {}, which was not sent",
        realm(w, p),
        realm(w, over)
    )
}

/// A tributary tests its overlord's grip.
pub fn tribute_revolt(w: &World, p: usize, over: usize, war_name: &str) -> String {
    format!(
        "The tribute wagons of {} did not come. {} had judged that {} was too \
         stretched to send anyone to ask why, and so began {}.",
        realm_full(w, p),
        realm_full_cap(w, p),
        realm_full(w, over),
        war_name
    )
}

/// A realm reduced to a tributary, for the sidebar and the detail page.
pub fn tributary_line(w: &World, over: usize) -> String {
    format!("pays tribute to {}", realm(w, over))
}

// ---------------------------------------------------------------------------
// Hegemony
// ---------------------------------------------------------------------------

/// The world notices that one realm has become the question of the age.
/// One draw.
pub fn hegemon_recognised(w: &World, p: usize, share: f32, pick: &Pick) -> String {
    let adj = realm_adj(w, p);
    let pct = format!("{:.0}%", share * 100.0);
    match pick.index(3) {
        0 => format!(
            "By this year {} held {} of all the settled land in the world. \
             Mapmakers had begun to draw the {} border first and fit everyone \
             else around it.",
            realm_full(w, p),
            pct,
            adj
        ),
        1 => format!(
            "{} now ruled {} of the settled world. In every court that did not \
             yet answer to it, the first question of any council had become what \
             {} would do next.",
            realm_full_cap(w, p),
            pct,
            realm(w, p)
        ),
        _ => format!(
            "{} of the world's settled land now lay under {}. There was no \
             longer a balance of powers, only a power and the rest.",
            pct,
            realm_full(w, p)
        ),
    }
}

/// Who else was on the field. Appended to a battle line when the fighting
/// was not a private matter between two realms.
pub fn battle_allies(w: &World, winner_side: &[usize], loser_side: &[usize]) -> String {
    let names =
        |side: &[usize]| -> Vec<String> { side.iter().skip(1).map(|&p| realm(w, p)).collect() };
    let (wa, la) = (names(winner_side), names(loser_side));
    match (wa.is_empty(), la.is_empty()) {
        (true, true) => String::new(),
        (false, true) => format!(
            " {} had come in alongside the victors.",
            super::join_names(&wa)
        ),
        (true, false) => format!(
            " {} had stood with the beaten host, and shared in the rout.",
            super::join_names(&la)
        ),
        (false, false) => format!(
            " It was no longer a quarrel between two realms: {} stood with the victors and {} with the beaten.",
            super::join_names(&wa),
            super::join_names(&la)
        ),
    }
}
