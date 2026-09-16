//! States: how a realm is announced, how crowns pass, why realms rise in
//! rank, and how they break apart.

use super::{
    cap, capital_name, count, join_names, realm, realm_full, realm_full_cap, realm_it, realm_its,
    realm_was, who, years, Pick,
};
use crate::sim::{PolityKind, World};

// ---------------------------------------------------------------------------
// Founding
// ---------------------------------------------------------------------------

/// A stateless people has raised a leader. Draws no random number: the
/// variant is chosen from the year and the realm's id.
pub fn polity_founded(
    w: &World,
    p: usize,
    kind: PolityKind,
    capital: usize,
    culture: usize,
    place: &str,
) -> String {
    let folk = &w.cultures[culture].plural;
    let seat = &w.cities[capital].name;
    let pick = Pick::stable(w.year, p);
    match kind {
        PolityKind::Horde => format!(
            "{} gathered the {} riders into a single host and pitched their camps at {}, {}. So began {}.",
            w.ruler_short(p),
            folk,
            seat,
            place,
            realm_full(w, p)
        ),
        _ if pick.index(2) == 0 => format!(
            "The {} living {} raised {} as their chief and built the settlement of {}. So began {}.",
            folk,
            place,
            w.ruler_short(p),
            seat,
            realm_full(w, p)
        ),
        _ => format!(
            "Living {}, the {} grew numerous enough to need a leader and took {}. They walled the settlement of {}, and so began {}.",
            place,
            folk,
            w.ruler_short(p),
            seat,
            realm_full(w, p)
        ),
    }
}

/// A realm founds a town. One draw, as before.
pub fn city_founded(w: &World, p: usize, city: usize, place: &str, pick: &Pick) -> String {
    if pick.index(2) == 0 {
        format!(
            "{} founded the town of {} {}.",
            w.ruler_title(p),
            w.cities[city].name,
            place
        )
    } else {
        format!(
            "Settlers out of {} raised the town of {} {}, where the land would feed them.",
            realm_full(w, p),
            w.cities[city].name,
            place
        )
    }
}

/// A realm learns to build seagoing ships.
pub fn learned_seafaring(w: &World, p: usize) -> String {
    format!(
        "The shipwrights of {} learned to build vessels fit for the open sea, and {} looked beyond {} own coasts.",
        realm_full(w, p),
        realm(w, p),
        realm_its(w, p)
    )
}

// ---------------------------------------------------------------------------
// Rulers
// ---------------------------------------------------------------------------

/// How a ruler died, as a complete sentence beginning with their title.
/// `cause` is a verb phrase such as `"died of a fever."`.
pub fn ruler_died(w: &World, p: usize, ruler: usize, cause: &str, reign: i32) -> String {
    let mut text = format!("{} {}", w.ruler_title(p), cause);
    if reign >= 20 {
        let g = who(w, ruler);
        text.push_str(&format!(
            " {} had held the throne of {} for {}.",
            g.subject_cap(),
            realm(w, p),
            years(reign as i64)
        ));
    }
    text
}

/// An extra line about a ruler's passing, at high detail. One draw.
pub fn ruler_death_flourish(w: &World, p: usize, ruler: usize, reign: i32, pick: &Pick) -> String {
    let g = who(w, ruler);
    match pick.index(6) {
        0 if reign < 20 => format!(
            " {} had reigned only {}.",
            g.subject_cap(),
            years(reign.max(0) as i64)
        ),
        0 => String::new(),
        1 => format!(" The people of {} mourned for many days.", realm(w, p)),
        2 => format!(
            " {} {} laid in the tombs of {}.",
            g.subject_cap(),
            g.was(),
            capital_name(w, p)
        ),
        3 => " Few wept.".to_string(),
        4 => format!(
            " Songs were sung of {} reign for a generation.",
            g.possessive
        ),
        _ => String::new(),
    }
}

/// The cause of a natural death, as a verb phrase. One draw.
pub fn natural_death(age: i32, pick: &Pick) -> String {
    match pick.index(7) {
        0 => "died of a fever.".to_string(),
        1 => format!("died in bed at the age of {}.", age),
        2 => "fell from a horse and did not rise.".to_string(),
        3 => "died of a wasting sickness.".to_string(),
        4 => "died in the night, and no one could say why.".to_string(),
        5 => format!("died, full of years, at the age of {}.", age),
        _ => "choked at a feast.".to_string(),
    }
}

/// Who killed a ruler. One draw.
pub fn assassin(w: &World, p: usize, pick: &Pick) -> String {
    match pick.index(5) {
        0 => "a cupbearer".to_string(),
        1 => "the palace guard".to_string(),
        2 => "a discontented noble".to_string(),
        3 => match w.polities[p].neighbors.first() {
            Some(&(q, _)) => format!("agents of {}", realm(w, q)),
            None => "an unknown hand".to_string(),
        },
        _ => "a masked assassin".to_string(),
    }
}

/// A murdered ruler, as a verb phrase for [`ruler_died`].
pub fn murdered_by(by: &str) -> String {
    format!("was murdered by {}.", by)
}

// ---------------------------------------------------------------------------
// Succession
// ---------------------------------------------------------------------------

/// A realm without a dynasty chooses its next leader, naming the one who
/// died. No draw.
pub fn elected(w: &World, p: usize, kind: PolityKind, heir: usize, old: usize) -> String {
    let name = &w.persons[heir].name;
    let gone = &w.persons[old].name;
    match kind {
        PolityKind::Republic => format!(
            "The assemblies of {} elected {} consul after the death of {}.",
            realm_full(w, p),
            name,
            gone
        ),
        PolityKind::Magocracy => format!(
            "The towers of {} chose {} as archmage after the death of {}.",
            realm_full(w, p),
            name,
            gone
        ),
        PolityKind::Theocracy => format!(
            "The priests of {} raised {} to the hierarchy in place of {}.",
            realm_full(w, p),
            name,
            gone
        ),
        _ => format!(
            "The {} chose {} to lead them after the death of {}.",
            w.cultures[w.polities[p].culture].plural, name, gone
        ),
    }
}

/// A quiet, expected succession. One draw.
pub fn succession_smooth(w: &World, p: usize, heir: usize, dynasty: &str, pick: &Pick) -> String {
    let hon = w.honorific(p, w.persons[heir].gender);
    let name = &w.persons[heir].name;
    let house = if dynasty.trim().is_empty() {
        String::new()
    } else {
        format!(" {} is of {}.", name, dynasty)
    };
    if pick.index(2) == 0 {
        format!(
            "{} {} succeeded to the throne of {} without dispute.{}",
            hon,
            name,
            realm_full(w, p),
            house
        )
    } else {
        format!(
            "By right of blood the throne passed to {} {} of {}.{}",
            hon,
            name,
            realm_full(w, p),
            house
        )
    }
}

/// No heir, so the strongest hand takes the throne. No draw.
pub fn succession_usurped(
    w: &World,
    p: usize,
    usurper: usize,
    new_dynasty: &str,
    old_dynasty: &str,
) -> String {
    let ended = if old_dynasty.trim().is_empty() {
        String::new()
    } else {
        format!(" {} was ended.", cap(old_dynasty))
    };
    format!(
        "The old ruler left no clear heir, and {} seized the throne of {} and founded {}.{}",
        w.persons[usurper].name,
        realm_full(w, p),
        new_dynasty,
        ended
    )
}

/// Two heirs, and the realm goes to war with itself. No draw.
pub fn succession_war(
    w: &World,
    p: usize,
    rebel: usize,
    name_a: &str,
    name_b: &str,
    seat_a: &str,
) -> String {
    let seat_b = w.polities[rebel]
        .cities
        .first()
        .map(|&c| w.cities[c].name.clone())
        .unwrap_or_else(|| "the provinces".to_string());
    format!(
        "Two children of the dead ruler claimed the throne of {}. {} held {} and the treasury; {} raised banners in {} and would not bend. {} went to war with itself.",
        realm_full(w, p),
        name_a,
        seat_a,
        name_b,
        seat_b,
        realm(w, p)
    )
}

/// A disputed succession settled without a war. No draw.
pub fn succession_disputed(w: &World, p: usize, name_a: &str) -> String {
    format!(
        "{} took the throne of {} after a bitter dispute among the heirs.",
        name_a,
        realm_full(w, p)
    )
}

/// A child inherits and the great houses rule in their name. No draw.
pub fn succession_regency(w: &World, p: usize, regent: usize) -> String {
    let g = who(w, regent);
    format!(
        "The child {} inherited {}, being the only heir of age to claim it. The great houses ruled in {} name and quarrelled over the spoils.",
        w.persons[regent].name,
        realm_full(w, p),
        g.possessive
    )
}

/// The claim a rebel claimant fights under.
pub fn succession_claim(w: &World, p: usize, claimant: &str) -> String {
    format!("the claim of {} to the throne of {}", claimant, realm(w, p))
}

// ---------------------------------------------------------------------------
// Rank
// ---------------------------------------------------------------------------

/// What a realm was and what it has become, for [`rank_changed`].
pub struct RankChange<'a> {
    /// The kind of thing it was.
    pub old: PolityKind,
    /// The kind of thing it now is.
    pub new: PolityKind,
    /// What it was called before.
    pub old_name: &'a str,
    /// What it is called now.
    pub new_name: &'a str,
    /// The ruler presiding over the change.
    pub ruler: &'a str,
    /// The capital it happened in.
    pub capital: &'a str,
}

/// A realm changes what kind of thing it is. Returns importance and text.
/// No draws.
pub fn rank_changed(w: &World, p: usize, c: &RankChange) -> (u8, String) {
    let RankChange {
        old,
        new,
        old_name,
        new_name,
        ruler,
        capital,
    } = *c;
    let short = realm(w, p);
    // The old name may be plural ("the Velenic Clans are now ...").
    let was_named = if super::name_is_plural(old_name, old) {
        "are"
    } else {
        "is"
    };
    match new {
        PolityKind::Chiefdom => (
            0,
            format!(
                "The clans of {} bent the knee to a single chieftain, having tired of raiding one another. {} {} now {}.",
                short,
                cap(old_name),
                was_named,
                new_name
            ),
        ),
        PolityKind::Kingdom if old == PolityKind::Empire => (
            2,
            format!(
                "Shrunken and humbled, {} {} an empire no longer. Its rulers styled it {}.",
                cap(old_name),
                if was_named == "are" { "were" } else { "was" },
                new_name
            ),
        ),
        PolityKind::Kingdom => (
            1,
            format!(
                "{} was crowned in {}, and the chiefdom of {} became {}.",
                ruler, capital, short, new_name
            ),
        ),
        PolityKind::Republic => (
            1,
            format!(
                "The merchant houses of {} cast down their chieftain, being rich enough to do without one, and proclaimed {}.",
                capital, new_name
            ),
        ),
        PolityKind::Empire => (
            3,
            format!(
                "{} {} proclaimed {}. {} took the imperial diadem before the assembled peoples of {}.",
                cap(old_name),
                if was_named == "are" { "were" } else { "was" },
                new_name,
                ruler,
                count(w.polities[p].cells as i64, "land")
            ),
        ),
        PolityKind::Theocracy => (
            2,
            format!(
                "The priests of {} took the crown for their own, the ruler being devout and the faith strong. {} {} now {}.",
                short,
                cap(old_name),
                was_named,
                new_name
            ),
        ),
        PolityKind::Magocracy => (
            2,
            format!(
                "The mages who counselled the throne of {} dispensed with the throne. {} {} now {}.",
                short,
                cap(old_name),
                was_named,
                new_name
            ),
        ),
        _ => (
            1,
            format!("{} {} now {}.", cap(old_name), was_named, new_name),
        ),
    }
}

// ---------------------------------------------------------------------------
// Unrest and endings
// ---------------------------------------------------------------------------

/// Why a province rose against its ruler.
pub fn revolt_cause(w: &World, p: usize, rebel_culture: usize, civil: bool, cruel: bool) -> String {
    if civil {
        if cruel {
            format!("the cruelty of {}", w.ruler_short(p))
        } else {
            format!("the misrule of {}", w.ruler_short(p))
        }
    } else {
        format!(
            "the {} wish to be free of {}",
            w.cultures[rebel_culture].adj,
            realm(w, p)
        )
    }
}

/// A province has raised the banner of revolt. No draw.
pub fn revolt(w: &World, p: usize, rebel: usize, leader: usize, seat: &str, cause: &str) -> String {
    format!(
        "{} raised the banner of revolt at {} against {}, blaming {}. The rebels named their realm {}.",
        w.persons[leader].name,
        seat,
        realm_full(w, p),
        cause,
        realm_full(w, rebel)
    )
}

/// A discontented notable gathers the aggrieved. One draw for the
/// rebel's station in life.
pub fn rebellion_of_the_wronged(
    w: &World,
    p: usize,
    rebel: usize,
    leader: usize,
    pick: &Pick,
) -> String {
    let station = pick.text(&[
        "a soldier",
        "a farmer's child",
        "a minor noble",
        "a priest",
        "a bandit",
        "a tax-collector",
    ]);
    format!(
        "{}, {} of {} who had been wronged by the crown, gathered the discontented and proclaimed {}.",
        w.persons[leader].name,
        station,
        realm_full(w, p),
        realm_full(w, rebel)
    )
}

/// The obituary of a realm. `cause` is a verb phrase such as
/// `"was overrun and destroyed."`.
pub fn realm_fell(w: &World, p: usize, cause: &str, cities_built: usize) -> String {
    let mut text = format!("{} {}", realm_full_cap(w, p), cause);
    let peak = w.polities[p].peak_cells;
    if peak > 40 {
        text.push_str(&format!(
            " At {} height in year {}, {} had ruled {} and {}.",
            realm_its(w, p),
            w.polities[p].peak_year,
            realm_it(w, p),
            count(peak as i64, "land"),
            count(cities_built as i64, "city")
        ));
    }
    text
}

/// A great realm breaks into pieces. Draws no random number: the variant
/// comes from the year and the realm, so the loudest sentence in the
/// chronicle is not the same one every time.
pub fn shattered(
    w: &World,
    p: usize,
    old_name: &str,
    successor_names: &[String],
    capital: &str,
) -> String {
    let now = realm_full(w, p);
    let remnant = if now == old_name {
        format!("What remained held only the lands about {}.", capital)
    } else {
        format!(
            "What remained of the old realm, now {}, held only the lands about {}.",
            now, capital
        )
    };
    let risen = join_names(successor_names);
    let opening = match Pick::stable(w.year, p).index(4) {
        0 => format!(
            "{} shattered, its throne too weak to hold its provinces. Its governors and generals each crowned themselves, and from its ruin rose {}.",
            cap(old_name),
            risen
        ),
        1 => format!(
            "The provinces of {} stopped waiting for orders that never came. Within a year the tax rolls were being read out in the names of {}.",
            old_name, risen
        ),
        2 => format!(
            "{} came apart along every seam it had ever been sewn along. Where one realm had been there were now {}.",
            cap(old_name),
            risen
        ),
        _ => format!(
            "No single blow broke {}: the garrisons simply began to obey the nearest lord instead of the furthest. Out of that habit came {}.",
            old_name, risen
        ),
    };
    format!("{} {}", opening, remnant)
}

/// A realm with nothing left to divide. No draw.
pub fn collapsed_into_lawlessness(w: &World, p: usize) -> String {
    format!(
        "collapsed into lawlessness, {} lords each seizing what they could.",
        realm_its(w, p)
    )
}

/// A realm that simply ran out of land. No draw.
pub fn faded_away(w: &World, p: usize, at_war: bool) -> String {
    if at_war {
        format!("{} overrun and destroyed.", realm_was(w, p))
    } else {
        format!("faded away, {} last lands abandoned.", realm_its(w, p))
    }
}

// ---------------------------------------------------------------------------
// Tyrants and usurping generals
// ---------------------------------------------------------------------------

/// A ruler earns a reputation for cruelty.
pub fn tyrant(w: &World, p: usize, ruler: usize, nobles: usize) -> String {
    let g = who(w, ruler);
    format!(
        "The people of {} spoke of {} only in whispers. {} had {} put to death in a single winter for plotting that no one could prove, and the roads were lined with what remained.",
        realm(w, p),
        w.ruler_title(p),
        g.subject_cap(),
        count(nobles as i64, "noble")
    )
}

/// A realm's army, so that an adjective is never used as a noun:
/// "the Velenic host".
pub fn host(w: &World, p: usize) -> String {
    format!("the {} host", w.polities[p].adj)
}

/// A general takes the throne by force. No draw.
pub fn general_usurps(w: &World, p: usize, general: usize, capital: &str) -> String {
    let g = who(w, general);
    format!(
        "The general {} marched on {} and took the throne of {} for {}, the realm being too weak to stop {}.",
        w.persons[general].name,
        capital,
        realm_full(w, p),
        g.reflexive,
        g.object
    )
}

/// The epithet and death notice of a ruler deposed by their own general.
pub fn deposed_by(general: &str) -> String {
    format!("was deposed and killed by the general {}.", general)
}
