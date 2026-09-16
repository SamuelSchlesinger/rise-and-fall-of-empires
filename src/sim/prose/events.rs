//! Disasters, notable lives, wonders and the naming of ages.

use super::{cap, cap_a, capital_name, count, join_names, realm, realm_full, who, Pick};
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

/// Flood, earthquake or fire in a city. Draws no random number: the
/// variant comes from the year and the city, so the fires of six centuries
/// are not the same sentence six times.
pub fn city_disaster(w: &World, city: usize, kind: &str, wonder_lost: Option<&str>) -> String {
    let cname = &w.cities[city].name;
    let pick = Pick::stable(w.year, city);
    let mut text = match kind {
        "flood" => match pick.index(4) {
            0 => format!(
                "The river burst its banks and drowned the lower town of {}.",
                cname
            ),
            1 => format!(
                "A spring flood carried off the wharves of {} and everyone who slept on them.",
                cname
            ),
            2 => format!(
                "The water rose for nine days at {}, and when it fell the streets were full of river mud.",
                cname
            ),
            _ => format!(
                "The dykes above {} gave way in the night and the town below them went under.",
                cname
            ),
        },
        "earthquake" => match pick.index(4) {
            0 => format!("The earth shook beneath {} and its towers fell.", cname),
            1 => format!(
                "The ground opened across {}; whole streets went into it and did not come back.",
                cname
            ),
            2 => format!(
                "{} was thrown down by an earthquake, and the survivors camped in the fields for a year.",
                cname
            ),
            _ => format!(
                "Three shocks in one morning left the walls of {} standing and everything inside them flat.",
                cname
            ),
        },
        _ => match pick.index(4) {
            0 => format!(
                "{} swept through {}; it burned for three days.",
                cap_a("great fire"),
                cname
            ),
            1 => format!(
                "Fire took the granaries of {} and then the quarter around them, and the wind did the rest.",
                cname
            ),
            2 => format!(
                "A lamp overturned in {}, and by morning half the town was ash and the other half was smoke.",
                cname
            ),
            _ => format!(
                "{} burned. The bucket lines held the fire at the market and lost everything behind it.",
                cname
            ),
        },
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

/// The subject of a song when nothing memorable has happened lately.
pub fn nothing_in_particular() -> &'static str {
    "the Old Days"
}

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
            let house = w.polities[p].dynasty.trim();
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

/// How an age dominated by one empire is remembered. No draw: the caller
/// hands in a stable pick so that consecutive centuries read differently.
pub fn era_of_empire(w: &World, p: usize, pick: &Pick) -> String {
    let realm = realm_full(w, p);
    match pick.index(3) {
        0 => format!(
            "{} ruled a third of the settled world, and the lesser realms lived in its shadow.",
            realm
        ),
        1 => format!(
            "Every road of consequence ran to {}, and every court of consequence sent it envoys.",
            realm
        ),
        _ => format!(
            "The century belongs to {}: its coin was good everywhere, and its peace held as far as its garrisons.",
            realm
        ),
    }
}

/// An age in which the world's people dwindled. No draw.
pub fn era_of_dying(steep: bool, pick: &Pick) -> String {
    let part = if steep { "third" } else { "sixth" };
    match pick.index(3) {
        0 => format!(
            "The peoples of the world dwindled by a {} part; cities emptied and roads grew over.",
            part
        ),
        1 => format!(
            "A {} of all the living were gone by the century's end, and the fields went back to scrub.",
            part
        ),
        _ => format!(
            "Chroniclers counted a {} fewer souls than their grandfathers had, and stopped counting.",
            part
        ),
    }
}

/// An age of constant war. No draw.
pub fn era_of_war(wars: u32, pick: &Pick) -> String {
    let n = count(wars as i64, "war");
    match pick.index(3) {
        0 => format!(
            "{} were fought in a hundred years, and no border stayed where it was drawn.",
            cap(&n)
        ),
        1 => format!(
            "{} in a hundred years. A child born at its start could die of old age without seeing a peace.",
            cap(&n)
        ),
        _ => format!(
            "The century kept {} going at once or one after another, and the mapmakers gave up.",
            n
        ),
    }
}

/// An age of new ideas. No draw.
pub fn era_of_schools(schools: u32, pick: &Pick) -> String {
    let n = count(schools as i64, "new school");
    match pick.index(3) {
        0 => format!(
            "{} of thought arose, and the courts of the world argued over doctrine.",
            cap(&n)
        ),
        1 => format!(
            "{} of thought were founded, and more ink was spilled over them than blood — barely.",
            cap(&n)
        ),
        _ => format!(
            "It was a century for teachers: {} of thought, and every one of them certain.",
            n
        ),
    }
}

/// An age of new crowns. No draw.
pub fn era_of_crowns(born: u32, pick: &Pick) -> String {
    let n = count(born as i64, "new realm");
    match pick.index(3) {
        0 => format!("{} were founded as peoples everywhere took up crowns.", cap(&n)),
        1 => format!(
            "{} rose out of the old ones, and a crown became a thing a determined man could simply take.",
            cap(&n)
        ),
        _ => format!(
            "The century made {}, and half of them would not see the next one.",
            n
        ),
    }
}

/// A quiet age. No draw.
pub fn era_of_peace(pick: &Pick) -> String {
    pick.text(&[
        "Harvests were good, the roads were safe, and the chroniclers complained of having little to write.",
        "Nothing much happened, and the people who lived through it were the luckier for that.",
        "Granaries filled, bridges were built, and the great quarrels of the age were over land boundaries and grazing rights.",
    ])
}

/// An age with nothing to mark it. No draw.
pub fn era_ordinary(pick: &Pick) -> String {
    pick.text(&[
        "The world turned as it always had.",
        "A century of ordinary years, which is to say of weather, taxes and funerals.",
        "Later ages found little in it worth arguing about.",
    ])
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

// ---------------------------------------------------------------------------
// Naming plagues, wonders and ages
// ---------------------------------------------------------------------------

const PLAGUE_ADJ: &[&str] = &[
    "Grey",
    "Red",
    "Weeping",
    "Sweating",
    "Black",
    "Silent",
    "Blistering",
    "Yellow",
    "Coughing",
    "Shivering",
];
const PLAGUE_NOUN: &[&str] = &[
    "Death", "Plague", "Sickness", "Fever", "Rot", "Pox", "Wasting",
];

/// What the physicians call a new sickness. Two draws, as before.
pub fn plague_name(pick: &Pick) -> String {
    let adj = pick.text(PLAGUE_ADJ);
    let noun = pick.text(PLAGUE_NOUN);
    format!("the {} {}", adj, noun)
}

const WONDER_ADJ: &[&str] = &[
    "Great", "Golden", "Black", "Sunken", "Ninefold", "White", "Hanging", "Singing", "Eternal",
    "Iron",
];
const WONDER_KIND: &[&str] = &[
    "Tower",
    "Ziggurat",
    "Library",
    "Colossus",
    "Gardens",
    "Lighthouse",
    "Temple",
    "Walls",
    "Bridge",
    "Aqueduct",
    "Mausoleum",
    "Observatory",
    "Arena",
    "Gate",
];

/// What a great work is called. Two draws, as before.
pub fn wonder_name(city: &str, pick: &Pick) -> String {
    let adj = pick.text(WONDER_ADJ);
    let kind = pick.text(WONDER_KIND);
    format!("the {} {} of {}", adj, kind, city)
}

// The families a century's name is drawn from. A world never repeats a
// name, so each family is a list to be walked rather than a single choice
// (see `events::unique_era_name`), and the lists are deliberately longer
// than they need to be for one world.

/// Names for a century in which the world lost people.
pub const ERA_NAMES_DYING: &[&str] = &[
    "the Silent Years",
    "the Long Winter",
    "the Age of Ash",
    "the Dark Age",
    "the Hungry Years",
    "the Emptying",
    "the Age of Empty Roads",
];

/// Names for a century of war.
pub const ERA_NAMES_WAR: &[&str] = &[
    "the Warring Age",
    "the Age of Blood",
    "the Century of Spears",
    "the Age of Iron",
    "the Years of the Sword",
    "the Riven Age",
    "the Age of Burned Fields",
    "the Century of Broken Treaties",
];

/// Names for a century of new ideas.
pub const ERA_NAMES_SCHOOLS: &[&str] = &[
    "the Age of Wonders",
    "the Age of the Star-Readers",
    "the Age of Prophets",
    "the Enlightenment",
    "the Age of Argument",
    "the Age of Open Schools",
    "the Century of Questions",
];

/// Names for a century of new crowns.
pub const ERA_NAMES_CROWNS: &[&str] = &[
    "the Age of Kings",
    "the Age of Petty Kings",
    "the Age of Banners",
    "the Age of New Crowns",
    "the Century of Upstarts",
    "the Age of Many Thrones",
];

/// Names for a quiet century.
pub const ERA_NAMES_PEACE: &[&str] = &[
    "the Long Peace",
    "the Age of Plenty",
    "the Quiet Age",
    "the Age of Roads",
    "the Age of Full Granaries",
    "the Century of Small Things",
];

/// Names for a century with nothing to mark it.
pub const ERA_NAMES_ORDINARY: &[&str] = &[
    "the Middle Years",
    "the Age of Walls",
    "the Uncertain Age",
    "the Unremarked Age",
    "the Century Between",
    "the Age of Ordinary Days",
];

/// Names for a century ruled by one empire, built around that empire.
pub fn era_names_empire(w: &World, p: usize) -> Vec<String> {
    let adj = &w.polities[p].adj;
    vec![
        format!("the Age of {}", realm(w, p)),
        format!("the {} Peace", adj),
        format!("the {} Ascendancy", adj),
        format!("the {} Century", adj),
        format!("the Age of the {} Crown", adj),
    ]
}
