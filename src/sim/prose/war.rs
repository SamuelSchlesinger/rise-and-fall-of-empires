//! War: why it was declared, what happened on the field, what became of
//! the cities, and how it ended.

use super::{cap, count, ordinal_word, realm, realm_adj, realm_full, who, years, Pick};
use crate::sim::{WarKind, World};

// ---------------------------------------------------------------------------
// Naming a war
// ---------------------------------------------------------------------------

/// "the Second " for a second war between the same two realms, and
/// nothing at all for the first.
pub fn war_ordinal(previous: usize) -> String {
    match previous {
        0 => String::new(),
        n if n < 10 => format!("{} ", cap(&ordinal_word(n as i64 + 1))),
        _ => "Latest ".to_string(),
    }
}

/// The name of a rising, civil war, war of succession or holy war.
pub fn war_name_internal(w: &World, attacker: usize, defender: usize, kind: WarKind) -> String {
    match kind {
        WarKind::Rebellion => format!(
            "the {} Rising",
            w.cultures[w.polities[attacker].culture].adj
        ),
        WarKind::CivilWar => format!("the {} Civil War", realm_adj(w, defender)),
        WarKind::Succession => format!("the War of the {} Succession", realm_adj(w, defender)),
        WarKind::Holy => match w.polities[attacker].school {
            Some(s) => format!("the {} Crusade", w.schools[s].short),
            None => format!("the {} Holy War", realm_adj(w, attacker)),
        },
        _ => format!("the {} War", realm_adj(w, defender)),
    }
}

/// A war named after the ground it is fought over.
pub fn war_name_feature(ordinal: &str, feature: &str) -> String {
    format!(
        "the {}War of {}",
        ordinal,
        feature.trim_start_matches("the ")
    )
}

/// A war named after the realms fighting it. One draw.
pub fn war_name_realms(
    w: &World,
    attacker: usize,
    defender: usize,
    ordinal: &str,
    pick: &Pick,
) -> String {
    match pick.index(3) {
        0 => format!(
            "the {}{}-{} War",
            ordinal,
            realm_adj(w, attacker),
            realm_adj(w, defender)
        ),
        1 => format!("the {}{} War", ordinal, realm_adj(w, defender)),
        _ => format!("the {}War of {}", ordinal, realm(w, attacker)),
    }
}

// ---------------------------------------------------------------------------
// Declaring a war
// ---------------------------------------------------------------------------

/// Why one realm attacked another. One draw for the ordinary causes.
pub fn war_cause(w: &World, attacker: usize, defender: usize, holy: bool, pick: &Pick) -> String {
    if holy {
        return format!("the heresies of {}", realm(w, defender));
    }
    match pick.index(6) {
        0 => format!("a dispute over the borderlands of {}", realm(w, defender)),
        1 => "the insult of a refused marriage".to_string(),
        2 => format!("the ambition of {}", w.ruler_short(attacker)),
        3 => format!(
            "the {} settlers living under {} rule",
            w.cultures[w.polities[attacker].culture].adj,
            realm_adj(w, defender)
        ),
        4 => "a murdered envoy".to_string(),
        _ => "old grievances no one could quite recall".to_string(),
    }
}

/// A declaration of war, naming who, whom, why and what the war is
/// called. No draw.
pub fn war_declared(
    w: &World,
    attacker: usize,
    defender: usize,
    kind: WarKind,
    cause: &str,
    war_name: &str,
) -> String {
    match kind {
        WarKind::Holy => format!(
            "{} declared holy war upon {}, citing {}. So began {}.",
            w.ruler_title(attacker),
            realm_full(w, defender),
            cause,
            war_name
        ),
        WarKind::Raid => format!(
            "The riders of {} fell upon {} for {}. So began {}.",
            realm_full(w, attacker),
            realm_full(w, defender),
            cause,
            war_name
        ),
        _ => format!(
            "{} declared war upon {}, citing {}. So began {}.",
            w.ruler_title(attacker),
            realm_full(w, defender),
            cause,
            war_name
        ),
    }
}

// ---------------------------------------------------------------------------
// Battles
// ---------------------------------------------------------------------------

/// The opening sentence of a battle report. Draws no random number: the
/// variant comes from the year and the two realms, so a long war does not
/// read as the same sentence over and over.
pub fn battle_opening(
    w: &World,
    offence: usize,
    defence: usize,
    place: &str,
    attacker_won: bool,
) -> String {
    let pick = Pick::stable(w.year, offence * 131 + defence);
    if attacker_won {
        match pick.index(3) {
            0 => format!(
                "The armies of {} beat {} at the Battle of {}.",
                realm_full(w, offence),
                realm_full(w, defence),
                place
            ),
            1 => format!(
                "{} met {} at {} and broke its line.",
                realm_full(w, offence),
                realm_full(w, defence),
                place
            ),
            _ => format!(
                "At the Battle of {}, the {} host drove the {} from the field.",
                place,
                realm_adj(w, offence),
                realm_adj(w, defence)
            ),
        }
    } else {
        match pick.index(2) {
            0 => format!(
                "The {} attack on {} was thrown back at the Battle of {}.",
                realm_adj(w, offence),
                realm_full(w, defence),
                place
            ),
            _ => format!(
                "{} held the field at the Battle of {}, and the {} withdrew.",
                realm_full(w, defence),
                place,
                realm_adj(w, offence)
            ),
        }
    }
}

/// A city that held out behind its walls.
pub fn siege_withstood(w: &World, city: usize) -> String {
    format!(
        " {} shut its gates and withstood the siege.",
        w.cities[city].name
    )
}

/// The defenders turn a defence into an advance.
pub fn pressed_advantage(w: &World, winner: usize) -> String {
    format!(
        " The {} pressed their advantage and took ground.",
        realm_adj(w, winner)
    )
}

/// A ruler who led from the front and did not come back.
pub fn ruler_fell_in_battle(name: &str) -> String {
    format!(" {} fell in the fighting.", name)
}

/// The death of a general, named with their epithet if they had one.
pub fn general_slain(w: &World, general: usize, side: usize) -> String {
    format!(
        " {}, a general of {}, was slain.",
        w.persons[general].full_name(),
        realm(w, side)
    )
}

/// How a ruler or general died, for their own record.
pub fn fell_at(place: &str) -> String {
    format!("fell at the Battle of {}.", place)
}

/// A closing line of colour after a battle, at high detail. One draw.
pub fn battle_flourish(w: &World, winner: usize, loser: usize, pick: &Pick) -> String {
    match pick.index(5) {
        0 => " The river ran red for a day.".to_string(),
        1 => format!(" The {} held the high ground.", realm_adj(w, winner)),
        2 => " Rain turned the field to mud, and the wounded drowned in it.".to_string(),
        3 => format!(
            " The {} fled at dusk, leaving their baggage.",
            realm_adj(w, loser)
        ),
        _ => " Both sides claimed the victory; the ravens did not care.".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Cities taken
// ---------------------------------------------------------------------------

/// A city stormed and plundered.
pub fn city_sacked(w: &World, city: &str, winner: usize, wonder_lost: Option<&str>) -> String {
    let lost = match wonder_lost {
        Some(wn) => format!(" {} was cast down.", wn),
        None => String::new(),
    };
    format!(
        " {} was taken and sacked by the {}.{}",
        city,
        realm_adj(w, winner),
        lost
    )
}

/// A city that surrendered rather than be stormed.
pub fn city_surrendered(w: &World, city: &str, winner: usize) -> String {
    format!(" {} opened its gates to the {}.", city, realm_adj(w, winner))
}

/// A defeated realm moves its court.
pub fn court_flees(w: &World, loser: usize, to: &str) -> String {
    format!(" The court of {} fled to {}.", realm(w, loser), to)
}

/// A realm that has lost its last city.
pub fn capital_lost(w: &World, loser: usize) -> String {
    format!(
        " With its capital lost and no city left to rule from, {} was no more.",
        realm_full(w, loser)
    )
}

/// Why a realm ended: it was conquered.
pub fn conquered_by(w: &World, winner: usize, city: &str) -> String {
    format!(
        "was conquered by {} with the fall of {}.",
        realm_full(w, winner),
        city
    )
}

// ---------------------------------------------------------------------------
// Peace
// ---------------------------------------------------------------------------

/// The sentence that closes a war: the war's name and how it ended.
pub fn war_ended(war_name: &str, result: &str) -> String {
    format!("{} {}", cap(war_name), result)
}

/// A rising put down. `cause` phrasing for the rebel realm's end.
pub fn rebellion_crushed(w: &World, defender: usize, years_fought: i32) -> String {
    format!(
        "was crushed by {} after {} of fighting.",
        realm_full(w, defender),
        years(years_fought.max(1) as i64)
    )
}

/// The rebels won and took the old realm's seat.
pub fn rebellion_triumphant(w: &World, attacker: usize) -> String {
    format!(
        "was overthrown; {} took the old capital and ruled in its place.",
        realm_full(w, attacker)
    )
}

/// The rebels won their independence.
pub fn independence_recognised(w: &World, attacker: usize, defender: usize) -> String {
    format!(
        "ended with {} recognising the independence of {}.",
        realm(w, defender),
        realm_full(w, attacker)
    )
}

/// The attacker got what it came for.
pub fn peace_attacker_won(w: &World, attacker: usize, defender: usize, years_fought: i32) -> String {
    format!(
        "ended in victory for {} after {}; {} ceded the lands it had lost and paid tribute.",
        realm(w, attacker),
        years(years_fought.max(1) as i64),
        realm(w, defender)
    )
}

/// The attacker was thrown back.
pub fn peace_defender_won(w: &World, attacker: usize, years_fought: i32) -> String {
    format!(
        "ended after {} with {} thrown back and humbled.",
        years(years_fought.max(1) as i64),
        realm(w, attacker)
    )
}

/// Neither side could finish the other.
pub fn peace_stalemate(years_fought: i32) -> String {
    format!(
        "ended after {} with neither side the master; the exhausted realms made peace.",
        years(years_fought.max(1) as i64)
    )
}

/// A war that stopped because one of its parties no longer existed.
pub fn war_lapsed(w: &World, p: usize) -> String {
    format!("ended when {} ceased to exist.", realm_full(w, p))
}

/// A rebel leader's end.
pub fn rebel_executed() -> &'static str {
    "was executed after the failure of the rising."
}

/// A general is given command. One draw.
pub fn general_appointed(w: &World, p: usize, general: usize, pick: &Pick) -> String {
    let g = who(w, general);
    match pick.index(3) {
        0 => format!(
            "{}, {} of low birth, rose to command the armies of {} on merit alone.",
            w.persons[general].name,
            super::a(&format!(
                "{} captain",
                w.cultures[w.polities[p].culture].adj
            )),
            realm_full(w, p)
        ),
        1 => format!(
            "{} was given command of the host of {}; the soldiers loved {}.",
            w.persons[general].name,
            realm_full(w, p),
            g.object
        ),
        _ => format!(
            "The armies of {} marched now under {}, who had never lost a skirmish.",
            realm_full(w, p),
            w.persons[general].name
        ),
    }
}

/// How many battles a legend is remembered for.
pub fn battles_remembered(n: u32) -> String {
    count(n.max(3) as i64, "battle")
}
