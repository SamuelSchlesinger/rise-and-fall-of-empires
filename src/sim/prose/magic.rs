//! Schools of thought: how an order, a faith or a philosophy is founded,
//! taken up by a crown, driven out, split, forgotten, or allowed to go
//! too far.

use super::{cap, realm, realm_full, school_full, school_the, who, years, Pick};
use crate::sim::{SchoolKind, World};

/// A new school is founded in a city. No draw.
pub fn school_founded(w: &World, s: usize, city: usize, founder: usize, place: &str) -> String {
    let kind = w.schools[s].kind;
    let cname = &w.cities[city].name;
    let fname = &w.persons[founder].name;
    match kind {
        SchoolKind::Arcane => format!(
            "In {}, {}, the mage {} gathered students and founded {}, which {}.",
            cname,
            place,
            fname,
            school_full(w, s),
            w.schools[s].doctrine
        ),
        SchoolKind::Divine => format!(
            "{} began to preach in the streets of {} that the old rites were not enough. {} {}, and its first tenet is: \"{}\"",
            fname,
            cname,
            cap(&school_full(w, s)),
            w.schools[s].doctrine,
            w.schools[s].tenets.first().cloned().unwrap_or_default()
        ),
        SchoolKind::Philosophical => format!(
            "{} taught in the markets of {} that \"{}\" From those lessons grew {}, which {}.",
            fname,
            cname,
            w.schools[s].tenets.first().cloned().unwrap_or_default(),
            school_full(w, s),
            w.schools[s].doctrine
        ),
    }
}

/// A school founded after a vision rather than in a city's schools.
pub fn school_from_vision(w: &World, s: usize, founder: usize, city: usize) -> String {
    format!(
        "{} of {} had a vision in the wilderness and returned to found {}, which {}.",
        w.persons[founder].name,
        w.cities[city].name,
        school_full(w, s),
        w.schools[s].doctrine
    )
}

/// A crown takes a school as its own. No draw.
pub fn school_adopted(w: &World, p: usize, s: usize, influence: f32) -> String {
    let kind = w.schools[s].kind;
    let because = if influence > 0.7 {
        "most of the realm already followed it"
    } else {
        "it had won the towns and the roads"
    };
    match kind {
        SchoolKind::Arcane => format!(
            "{} took the adepts of {} into royal service, since {}. Henceforth it would be the magic of {}.",
            w.ruler_title(p),
            school_full(w, s),
            because,
            realm(w, p)
        ),
        SchoolKind::Divine => format!(
            "{} was baptised into {}, since {}, and it became the faith of {}.",
            w.ruler_title(p),
            school_full(w, s),
            because,
            realm_full(w, p)
        ),
        SchoolKind::Philosophical => format!(
            "The court of {} adopted the teachings of {}, since {}, and its laws were rewritten by the {}.",
            realm_full(w, p),
            school_full(w, s),
            because,
            w.schools[s].kind.follower()
        ),
    }
}

/// A crown changes its school for a rival. No draw.
pub fn school_converted(w: &World, p: usize, from: usize, to: usize) -> String {
    format!(
        "{} forsook {} for {}, the newer teaching having grown the stronger in {}.",
        w.ruler_title(p),
        school_full(w, from),
        school_full(w, to),
        realm(w, p)
    )
}

/// A school is outlawed. No draw.
pub fn school_persecuted(w: &World, p: usize, s: usize, state: usize, capital: &str) -> String {
    format!(
        "{} outlawed {} throughout {}, which was sworn to {}. The {} of {} were driven from {}.",
        w.ruler_title(p),
        school_full(w, s),
        realm_full(w, p),
        school_full(w, state),
        w.schools[s].kind.follower(),
        school_the(w, s),
        capital
    )
}

/// A follower burned for the outlawed teaching. No draw.
pub fn martyr_made(w: &World, s: usize, martyr: usize, capital: &str) -> String {
    let g = who(w, martyr);
    format!(
        " {} was burned in the square of {}; the {} of {} call {} a martyr.",
        w.persons[martyr].name,
        capital,
        w.schools[s].kind.follower(),
        school_the(w, s),
        g.object
    )
}

/// The record of a martyr's death.
pub fn martyr_death(w: &World, s: usize, capital: &str) -> String {
    format!(
        "was burned in the square of {} for the teachings of {}.",
        capital,
        school_the(w, s)
    )
}

/// What a schism was about. One draw for faiths, none otherwise.
pub fn schism_dispute(w: &World, parent: usize, child: usize, pick: &Pick) -> String {
    match w.schools[parent].kind {
        SchoolKind::Arcane => format!(
            "over whether {} may be practised on the living",
            crate::sim::magic::practice_word(w.schools[child].practice).to_lowercase()
        ),
        SchoolKind::Divine => match pick.index(3) {
            0 => "over the true name of the god".to_string(),
            1 => format!(
                "over the tenet \"{}\"",
                w.schools[parent]
                    .tenets
                    .first()
                    .cloned()
                    .unwrap_or_default()
            ),
            _ => "over who might sit on the high seat".to_string(),
        },
        SchoolKind::Philosophical => {
            "over the meaning of a single sentence of the founder".to_string()
        }
    }
}

/// A school splits in two. No draw.
pub fn schism(
    w: &World,
    parent: usize,
    child: usize,
    dispute: &str,
    founder: usize,
    city: usize,
) -> String {
    format!(
        "{} split {}. {} of {} led the dissenters out, and their teaching became known as {}.",
        cap(&school_full(w, parent)),
        dispute,
        w.persons[founder].name,
        w.cities[city].name,
        school_full(w, child)
    )
}

/// A waning school is taken into a larger one of its own kind. Draws no
/// random number: the variant comes from the year and the school.
pub fn school_absorbed(w: &World, s: usize, into: usize, waned: i32) -> String {
    let gone = school_full(w, s);
    let big = school_full(w, into);
    let follower = w.schools[s].kind.follower();
    match Pick::stable(w.year, s).index(4) {
        0 => format!(
            "{} had kept no congregation of its own for {}. Its last {} were counted among {}, and the two were one teaching thereafter.",
            cap(&gone),
            years(waned.max(1) as i64),
            follower,
            big
        ),
        1 => format!(
            "The remaining {} of {} gave up the distinction and went over to {}, whose houses had long outnumbered theirs.",
            follower,
            school_the(w, s),
            big
        ),
        2 => format!(
            "After {} of decline, {} was absorbed into {}; its books were copied into the greater library and its name kept only as a chapter heading.",
            years(waned.max(1) as i64),
            gone,
            big
        ),
        _ => format!(
            "Nobody disputed the merger of {} into {}: there was nothing left to dispute it with.",
            gone, big
        ),
    }
}

/// A school with no followers left.
pub fn school_forgotten(w: &World, s: usize, age: i32) -> String {
    format!(
        "The last {} of {} died after {}, and its teachings were forgotten.",
        w.schools[s].kind.follower(),
        school_full(w, s),
        years(age.max(1) as i64)
    )
}

/// An arcane order unmakes the city that raised it. Draws no random
/// number: the variant comes from the year and the school, so the loudest
/// thing that can happen to a city does not read the same way twice.
pub fn catastrophe(
    w: &World,
    s: usize,
    city_name: &str,
    blight_name: &str,
    dead_thousands: i32,
) -> String {
    let order = school_the(w, s);
    let dead = dead_thousands.max(1);
    match Pick::stable(w.year, s).index(4) {
        0 => format!(
            "The adepts of {} in {} reached too far. In a single night the city was unmade: a great working went wrong, and where {} had stood there was only glass, ash and silence. The land for miles about was blighted and is called {}. Some {} thousand souls perished.",
            order, city_name, city_name, blight_name, dead
        ),
        1 => format!(
            "Something the adepts of {} were holding in {} got away from them. The light over the city was seen from the coast; by morning there was no city, only a plain of glass that is called {}. Some {} thousand souls perished.",
            order, city_name, blight_name, dead
        ),
        2 => format!(
            "{} kept its deepest working under {}, and the working ended. The ground took the towers, the towers took the streets, and what was left is named {}. Some {} thousand souls perished, and no body was ever recovered.",
            cap(&order),
            city_name,
            blight_name,
            dead
        ),
        _ => format!(
            "For three days the adepts of {} would not say what they had begun beneath {}. On the fourth the question stopped mattering: the city went out like a lamp, and the blighted country about it is called {}. Some {} thousand souls perished.",
            order, city_name, blight_name, dead
        ),
    }
}

/// The ruler who died with the unmade city.
pub fn catastrophe_ruler(name: &str) -> String {
    format!(" {} died with the city.", name)
}

/// How a ruler died in an unmaking.
pub fn perished_in_unmaking(city: &str) -> String {
    format!("perished in the unmaking of {}.", city)
}

/// A realm that could not survive its capital being unmade.
pub fn catastrophe_realm_ends(w: &World, p: usize) -> String {
    format!(" {} did not survive the loss.", cap(&realm_full(w, p)))
}

/// Why that realm ended.
pub fn perished_with(city: &str) -> String {
    format!("perished with {}.", city)
}

// ---------------------------------------------------------------------------
// Adepts, prophets and philosophers abroad
// ---------------------------------------------------------------------------

/// A named follower advances their school in a realm. No draw.
pub fn school_champion(w: &World, p: usize, s: usize, person: usize) -> String {
    let g = who(w, person);
    match w.schools[s].kind {
        SchoolKind::Arcane => format!(
            "{}, an adept of {}, performed wonders before the court of {} and won many of its nobles to that teaching.",
            w.persons[person].name,
            school_full(w, s),
            realm_full(w, p)
        ),
        SchoolKind::Divine => format!(
            "{} walked the roads of {} preaching {}, and the villages followed {}.",
            w.persons[person].name,
            realm_full(w, p),
            school_full(w, s),
            g.object
        ),
        SchoolKind::Philosophical => format!(
            "{} taught {} in the schools of {} to a generation of clerks and princes.",
            w.persons[person].name,
            school_full(w, s),
            realm_full(w, p)
        ),
    }
}

// ---------------------------------------------------------------------------
// Naming and doctrine
// ---------------------------------------------------------------------------

/// A school of thought named after the person who started it:
/// `ism("Kelano") == "Kelanism"`, `ism("Vard") == "Vardism"`. Trailing
/// vowels are dropped only while a pronounceable stem remains, so a short
/// name is not filed down to a single letter.
pub fn ism(founder: &str) -> String {
    let trimmed = founder.trim_end_matches(|c: char| "aeiouAEIOU".contains(c));
    let stem = if trimmed.chars().count() >= 3 {
        trimmed
    } else {
        founder
    };
    format!("{}ism", stem)
}

/// What an arcane order teaches: "teaches that all magic is the binding
/// of fire".
pub fn doctrine_arcane(practice: &str, aspect: &str) -> String {
    format!(
        "teaches that all magic is the {} of {}",
        practice.to_lowercase(),
        aspect
    )
}

/// What a faith is: "is an ascetic faith devoted to the Drowned God".
pub fn doctrine_divine(practice: &str, deity: &str) -> String {
    format!("is {} faith devoted to {}", super::a(practice), deity)
}

/// What a philosophy holds: "holds that the good life is found in doubt".
pub fn doctrine_philosophy(tenet: &str) -> String {
    format!(
        "holds that the good life is found in {}",
        tenet.to_lowercase()
    )
}
