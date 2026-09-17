//! War: why it was declared, what happened on the field, what became of
//! the cities, and how it ended.

use super::{
    cap, count, host, ordinal_word, realm, realm_adj, realm_full, realm_full_cap, realm_its,
    realm_was, who, years, Pick,
};
use crate::sim::{WarKind, World};

// ---------------------------------------------------------------------------
// Naming a war
// ---------------------------------------------------------------------------

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
pub fn war_name_feature(feature: &str) -> String {
    format!("the War of {}", feature)
}

/// A war named after the realms fighting it. One draw.
pub fn war_name_realms(w: &World, attacker: usize, defender: usize, pick: &Pick) -> String {
    match pick.index(3) {
        0 => format!(
            "the {}-{} War",
            realm_adj(w, attacker),
            realm_adj(w, defender)
        ),
        1 => format!("the {} War", realm_adj(w, defender)),
        _ => format!("the War of {}", realm(w, attacker)),
    }
}

/// A war fought to take one city.
pub fn war_name_city(city: &str) -> String {
    format!("the War for {}", city)
}

/// A war fought to throw off an overlord.
pub fn war_name_independence(adj: &str) -> String {
    format!("the {} War of Independence", adj)
}

/// A war fought to make a realm pay tribute.
pub fn war_name_vassalage(short: &str) -> String {
    format!("the Subjugation of {}", short)
}

/// A war fought to put somebody on a throne.
pub fn war_name_claim(claimant: &str) -> String {
    format!("the War of {}'s Claim", claimant)
}

/// A war fought over a relic.
pub fn war_name_relic(relic: &str) -> String {
    format!("the War of {}", relic)
}

/// A war fought to pull down the strongest realm in the world.
pub fn war_name_containment(adj: &str) -> String {
    format!("the Great {} War", adj)
}

/// Number a war after the ones before it of the same name: the first keeps
/// the bare name, and the rest are counted.
pub fn war_name_numbered(base: &str, previous: usize) -> String {
    if previous == 0 {
        return base.to_string();
    }
    let stem = war_name_stem(base);
    match previous {
        n if n < 10 => format!("the {} {}", cap(&ordinal_word(n as i64 + 1)), stem),
        _ => format!("the Latest {}", stem),
    }
}

/// The part of a war's name that its successors will share: the article and
/// any ordinal stripped back off.
///
/// This is the inverse of [`war_name_numbered`], and it is what the tally of
/// used names is keyed on — both when a name is coined and when the tally is
/// rebuilt from the wars in a loaded save. The two have to agree, so they
/// are written next to each other.
pub fn war_name_stem(name: &str) -> &str {
    let s = name.strip_prefix("the ").unwrap_or(name);
    match s.split_once(' ') {
        Some((first, rest)) if ORDINALS_USED.contains(&first) => rest,
        _ => s,
    }
}

/// The words [`war_name_numbered`] can put in front of a stem.
const ORDINALS_USED: [&str; 10] = [
    "Second", "Third", "Fourth", "Fifth", "Sixth", "Seventh", "Eighth", "Ninth", "Tenth", "Latest",
];

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
        2 => format!("the ambition of the crown of {}", realm(w, attacker)),
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
                "{} met {} at {} and broke {} line.",
                realm_full_cap(w, offence),
                realm_full(w, defence),
                place,
                realm_its(w, defence)
            ),
            _ => format!(
                "At the Battle of {}, {} drove {} from the field.",
                place,
                host(w, offence),
                host(w, defence)
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
                "{} held the field at the Battle of {}, and {} withdrew.",
                realm_full_cap(w, defence),
                place,
                host(w, offence)
            ),
        }
    }
}

/// A city that held out behind its walls. Draws no random number: the
/// variant comes from the year and the city, so two sieges in one report
/// do not read alike.
pub fn siege_withstood(w: &World, city: usize) -> String {
    let name = &w.cities[city].name;
    match Pick::stable(w.year, city).index(3) {
        0 => format!(" {} shut its gates and withstood the siege.", name),
        1 => format!(" The walls of {} held.", name),
        _ => format!(" {} was besieged and did not fall.", name),
    }
}

/// The defenders turn a defence into an advance.
pub fn pressed_advantage(w: &World, winner: usize) -> String {
    format!(
        " {} pressed its advantage and took ground.",
        cap(&host(w, winner))
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
        1 => format!(" {} held the high ground.", cap(&host(w, winner))),
        2 => " Rain turned the field to mud, and the wounded drowned in it.".to_string(),
        3 => format!(
            " {} fled at dusk, leaving its baggage.",
            cap(&host(w, loser))
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
        " {} was taken and sacked by {}.{}",
        city,
        host(w, winner),
        lost
    )
}

/// A city that surrendered rather than be stormed.
pub fn city_surrendered(w: &World, city: &str, winner: usize) -> String {
    format!(" {} opened its gates to {}.", city, host(w, winner))
}

/// A defeated realm moves its court.
pub fn court_flees(w: &World, loser: usize, to: &str) -> String {
    format!(" The court of {} fled to {}.", realm(w, loser), to)
}

/// A realm that has lost its last city.
pub fn capital_lost(w: &World, loser: usize) -> String {
    format!(
        " With {} capital lost and no city left to rule from, {} {} no more.",
        realm_its(w, loser),
        realm_full(w, loser),
        realm_was(w, loser)
    )
}

/// Why a realm ended: it was conquered. Draws no random number; the
/// variant comes from the year and the realm that fell, so the commonest
/// epitaph in the chronicle is not the same sentence every century.
pub fn conquered_by(w: &World, loser: usize, winner: usize, city: &str) -> String {
    let was = realm_was(w, loser);
    let by = realm_full(w, winner);
    match Pick::stable(w.year, loser).index(4) {
        0 => format!("{} conquered by {} with the fall of {}.", was, by, city),
        1 => format!(
            "{} swallowed by {} when {} fell and there was nothing behind it.",
            was, by, city
        ),
        2 => format!(
            "{} ended at {}: {} took the city, and the realm went with it.",
            was, city, by
        ),
        _ => format!(
            "{} extinguished by {}; {} was the last of {} cities to fall.",
            was,
            by,
            city,
            realm_its(w, loser)
        ),
    }
}

// ---------------------------------------------------------------------------
// Peace
// ---------------------------------------------------------------------------

/// The sentence that closes a war: the war's name and how it ended.
pub fn war_ended(war_name: &str, result: &str) -> String {
    format!("{} {}", cap(war_name), result)
}

/// What an internal war's attacker is called, so that a civil war is not
/// described as a rising and a war of succession is not described as
/// either. Only these three kinds have an insurgent side at all; every
/// other war is between two realms that each have a right to be there.
pub fn insurgent(kind: WarKind) -> &'static str {
    match kind {
        WarKind::Rebellion => "the rising",
        WarKind::CivilWar => "the rebels",
        WarKind::Succession => "the claimant",
        _ => "the attacker",
    }
}

/// A rising, a civil war or a claim put down. `cause` phrasing for the
/// defeated realm's end.
pub fn rebellion_crushed(
    w: &World,
    rebel: usize,
    defender: usize,
    kind: WarKind,
    years_fought: i32,
) -> String {
    let span = years(years_fought.max(1) as i64);
    let by = realm_full(w, defender);
    match kind {
        WarKind::Rebellion => format!(
            "{} crushed by {} after {} of fighting.",
            realm_was(w, rebel),
            by,
            span
        ),
        WarKind::CivilWar => format!(
            "{} put down by {} after {} of fighting between one half of the realm and the other.",
            realm_was(w, rebel),
            by,
            span
        ),
        _ => format!(
            "{} beaten by {} after {} of war, and the claim died with it.",
            realm_was(w, rebel),
            by,
            span
        ),
    }
}

/// The rebels won and took the old realm's seat.
pub fn rebellion_triumphant(w: &World, defender: usize, attacker: usize) -> String {
    format!(
        "{} overthrown; {} took the old capital and ruled in its place.",
        realm_was(w, defender),
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

/// The cause a wronged notable raises an army under.
pub fn grievances_of(name: &str) -> String {
    format!("the grievances of {}", name)
}
