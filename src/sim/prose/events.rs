//! Disasters, notable lives, wonders and the naming of ages.

use super::{cap, cap_a, capital_name, count, join_names, realm_full, who, Pick};
use crate::sim::World;

// ---------------------------------------------------------------------------
// Plague, famine and fire
// ---------------------------------------------------------------------------

/// A plague crosses a border. No draw.
pub fn plague_spread(plague: &str, realms: &[String]) -> String {
    format!(
        "{} spread into {}, carried along the trade roads.",
        cap(plague),
        join_names(realms)
    )
}

/// A plague burns out. No draw.
pub fn plague_ended(plague: &str, dead_thousands: i64) -> String {
    format!(
        "{} burned itself out at last, having carried off some {} thousand souls.",
        cap(plague),
        dead_thousands.max(1)
    )
}

/// A plague begins. One draw.
pub fn plague_began(w: &World, plague: &str, city: usize, p: usize, pick: &Pick) -> String {
    let cname = &w.cities[city].name;
    match pick.index(3) {
        0 => format!(
            "{} came to {} with the ships and the caravans. Within a season the streets were empty.",
            cap(plague),
            cname
        ),
        1 => format!(
            "A sickness the physicians called {} broke out in {}. The dead were buried in pits outside the walls.",
            plague, cname
        ),
        _ => format!(
            "{} began in the poor quarters of {} and spread through {}.",
            cap(plague),
            cname,
            realm_full(w, p)
        ),
    }
}

/// What ruined the harvest. One draw.
pub fn famine_cause(pick: &Pick) -> String {
    pick.text(&[
        "The rains failed",
        "Locusts came out of the east",
        "A killing frost struck at midsummer",
        "The rivers ran low",
    ])
}

/// A realm goes hungry. No draw.
pub fn famine(w: &World, p: usize, cause: &str) -> String {
    format!(
        "{} and famine gripped {}. The granaries of {} were emptied, and the people ate bark.",
        cause,
        realm_full(w, p),
        capital_name(w, p)
    )
}

/// Flood, earthquake or fire in a city. No draw.
pub fn city_disaster(w: &World, city: usize, kind: &str, wonder_lost: Option<&str>) -> String {
    let cname = &w.cities[city].name;
    let mut text = match kind {
        "flood" => format!(
            "The river burst its banks and drowned the lower town of {}.",
            cname
        ),
        "earthquake" => format!("The earth shook beneath {} and its towers fell.", cname),
        _ => format!(
            "{} swept through {}; it burned for three days.",
            cap_a("great fire"),
            cname
        ),
    };
    if let Some(wn) = wonder_lost {
        text.push_str(&format!(" {} was destroyed.", wn));
    }
    text
}

/// A portent the whole world saw. One draw.
pub fn omen(pick: &Pick) -> String {
    pick.text(&[
        "A comet with a tail like a sword hung in the sky for forty nights. Priests everywhere read it as they pleased.",
        "The sun went dark at noon and the birds fell silent.",
        "Two moons were seen in the sky, and the second wept.",
        "It rained fish upon the coasts, and the fish were alive.",
    ])
}

// ---------------------------------------------------------------------------
// Notable lives
// ---------------------------------------------------------------------------

/// A poet writes about something that happened. No draw beyond `form`.
pub fn poet_sings(w: &World, p: usize, poet: usize, form: &str, subject: &str) -> String {
    format!(
        "{} of {} composed the {} of {}, which is still sung in {}.",
        w.persons[poet].name,
        realm_full(w, p),
        form,
        subject,
        capital_name(w, p)
    )
}

/// A poet with nothing in particular to sing about. No draw.
pub fn poet_idle(w: &World, p: usize, poet: usize) -> String {
    format!(
        "{} of {} wrote verses on the rivers and the seasons that were long remembered.",
        w.persons[poet].name,
        realm_full(w, p)
    )
}

/// An explorer finds a place and names it. No draw.
pub fn explorer_found(w: &World, p: usize, explorer: usize, what: &str, name: &str) -> String {
    let who_ = &w.persons[explorer].name;
    let from = realm_full(w, p);
    match what {
        "island" => format!(
            "{} of {} sailed beyond the known coasts and came upon {}.",
            who_, from, name
        ),
        "continent" => format!(
            "{} of {} sailed beyond the known coasts and returned with tales of a land called {}.",
            who_, from, name
        ),
        _ => format!("{} of {} charted the waters of {}.", who_, from, name),
    }
}

/// An explorer who found nothing worth naming. No draw.
pub fn explorer_empty_handed(w: &World, p: usize, explorer: usize) -> String {
    format!(
        "{} of {} sailed west for a year and returned with strange fruit and stranger stories.",
        w.persons[explorer].name,
        realm_full(w, p)
    )
}

/// A scholar improves the running of a realm. One draw.
pub fn reformer(w: &World, p: usize, person: usize, pick: &Pick) -> String {
    let deed = pick.text(&[
        "reformed the calendar and the weights of the market",
        "wrote a treatise on the governance of cities that was copied in every court",
        "taught the use of the arch and the aqueduct",
        "compiled the laws of the realm into a single book",
        "mapped the stars and the roads alike",
    ]);
    format!(
        "{} of {} {}, and the realm was the better run for it.",
        w.persons[person].name,
        realm_full(w, p),
        deed
    )
}

/// A notable dies of old age. No draw.
pub fn notable_died(name: &str, role: &str, age: i32) -> String {
    format!("{}, the {}, died at the age of {}.", name, role, age)
}

// ---------------------------------------------------------------------------
// Wonders
// ---------------------------------------------------------------------------

/// A great work is finished. One draw.
pub fn wonder_built(w: &World, p: usize, name: &str, ruler: &str, pick: &Pick) -> String {
    match pick.index(3) {
        0 => format!(
            "{} raised {}. It took a generation to build and beggared the treasury, but travellers came from every land to see it.",
            ruler, name
        ),
        1 => format!(
            "{} was completed in {}, in the reign of {}.",
            cap(name),
            realm_full(w, p),
            ruler
        ),
        _ => {
            let house = w.polities[p].dynasty.trim_start_matches("the ").trim();
            if house.is_empty() {
                format!(
                    "The masons of {} finished {}, greatest of the works of the age.",
                    realm_full(w, p),
                    name
                )
            } else {
                format!(
                    "The masons of {} finished {}, greatest of the works of {}.",
                    realm_full(w, p),
                    name,
                    house
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Ages
// ---------------------------------------------------------------------------

/// How an age dominated by one empire is remembered. No draw beyond the
/// name, which the caller picks.
pub fn era_of_empire(w: &World, p: usize) -> String {
    format!(
        "{} ruled a third of the settled world, and the lesser realms lived in its shadow.",
        realm_full(w, p)
    )
}

/// An age in which the world's people dwindled.
pub fn era_of_dying(steep: bool) -> String {
    format!(
        "The peoples of the world dwindled by a {} part; cities emptied and roads grew over.",
        if steep { "third" } else { "sixth" }
    )
}

/// An age of constant war.
pub fn era_of_war(wars: u32) -> String {
    format!(
        "{} were fought in a hundred years, and no border stayed where it was drawn.",
        cap(&count(wars as i64, "war"))
    )
}

/// An age of new ideas.
pub fn era_of_schools(schools: u32) -> String {
    format!(
        "{} arose, and the courts of the world argued over doctrine.",
        cap(&count(schools as i64, "new school of thought"))
    )
}

/// An age of new crowns.
pub fn era_of_crowns(born: u32) -> String {
    format!(
        "{} were founded as peoples everywhere took up crowns.",
        cap(&count(born as i64, "new realm"))
    )
}

/// A quiet age.
pub fn era_of_peace() -> &'static str {
    "Harvests were good, the roads were safe, and the chroniclers complained of having little to write."
}

/// An age with nothing to mark it.
pub fn era_ordinary() -> &'static str {
    "The world turned as it always had."
}

/// The chronicle's own summary of a century.
pub fn era_named(name: &str, description: &str) -> String {
    format!(
        "The chroniclers call the century now ending {}. {}",
        name, description
    )
}

// ---------------------------------------------------------------------------
// Legends
// ---------------------------------------------------------------------------

/// What is said of a great life at its end. No draw.
pub fn legend_remembered(w: &World, person: usize, role: &str, folk: &str) -> String {
    let g = who(w, person);
    let name = w.persons[person].full_name();
    match role {
        "general" => format!(
            "{} was laid on a pyre with {} sword. The {} still tell of {}, and the number grows with each telling.",
            name,
            g.possessive,
            folk,
            super::battles_remembered(w.persons[person].battles_won)
        ),
        "poet" => format!(
            "{} died; the {} sing {} verses still.",
            name, folk, g.possessive
        ),
        _ => format!(
            "{} died. The {} count {} among the great of their people, and {} deeds are told to children.",
            name, folk, g.object, g.possessive
        ),
    }
}
