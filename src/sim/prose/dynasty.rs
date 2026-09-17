//! Sentences about houses, marriages, children and greatness.
//!
//! The acclaim line is the most important sentence in this file, because it
//! is the one that puts a figure in front of the reader while they are still
//! alive to be watched. It names the deed that earned the title, so the
//! chronicle says *why* as well as *who*.

use super::{cap, join_names, ordinal, realm, realm_full, years};
use super::{Pick, World};
use crate::sim::{Gender, SchoolKind};

// ---------------------------------------------------------------------------
// Standing: what a life is made of
// ---------------------------------------------------------------------------

// Every claim is a past participle, because the acclaim sentence puts it
// after "had" and the detail page puts it after a bullet. "taken 50 lands"
// works in both places; "took 50 lands" works in neither.

pub fn claim_conquest(lands: i32) -> String {
    format!("taken {} lands in war", lands)
}

pub fn claim_losses(lands: i32) -> String {
    format!("lost {} lands", lands)
}

pub fn claim_settlement(lands: i32) -> String {
    format!("settled {} lands", lands)
}

pub fn claim_speed(rate: f32) -> String {
    format!("conquered at {:.1} lands a year", rate)
}

pub fn claim_cities_taken(n: u32) -> String {
    if n == 1 {
        "stormed a city".to_string()
    } else {
        format!("stormed {} cities", n)
    }
}

pub fn claim_cities_founded(n: u32) -> String {
    if n == 1 {
        "founded a city".to_string()
    } else {
        format!("founded {} cities", n)
    }
}

pub fn claim_wars(n: u32) -> String {
    if n == 1 {
        "won a war".to_string()
    } else {
        format!("won {} wars", n)
    }
}

pub fn claim_battles(n: u32) -> String {
    if n == 1 {
        "won a battle".to_string()
    } else {
        format!("won {} battles", n)
    }
}

pub fn claim_long_reign(years_held: i32) -> String {
    format!("reigned {} years", years_held)
}

pub fn claim_hegemon(share: f32) -> String {
    format!("ruled {:.0}% of the settled world", share * 100.0)
}

pub fn claim_imperial() -> String {
    "worn an emperor's crown".to_string()
}

pub fn claim_renown() -> String {
    "been spoken of in every land".to_string()
}

pub fn claim_anointed() -> String {
    "been crowned by a faith".to_string()
}

pub fn claim_house(children: usize) -> String {
    format!("left a house of {} children", children)
}

// ---------------------------------------------------------------------------
// Acclaim
// ---------------------------------------------------------------------------

/// The world names somebody great while they live. One draw, in `pick`.
///
/// The sentence has three jobs: say who, say what they are now called, and
/// say what they did to earn it. It carries the reign length when there is
/// one worth carrying, because a title won in four years reads differently
/// from one won in forty.
pub fn acclaimed(
    w: &World,
    r: usize,
    p: usize,
    epithet: &str,
    deed: &str,
    is_ruler: bool,
) -> String {
    let per = &w.persons[r];
    let who = if is_ruler {
        format!("{} {}", w.honorific(p, per.gender), per.name)
    } else {
        format!("{} {}", cap(per.role.name()), per.name)
    };
    let age = per.age(w.year).max(1);
    let lifespan = w.races[per.race].lifespan.max(1.0);
    let young = (age as f32 / lifespan) < 0.45;
    // A young figure is remarkable for their age; an old one is remarkable
    // for how long they have been at it. Saying "but ninety-eight years" of
    // either is how the sentence used to embarrass itself.
    let standing = if young {
        format!("and {} was but {} old", per.they(), years(age as i64))
    } else if per.reign_years >= 2 {
        format!(
            "in the {} year of {} reign",
            ordinal(per.reign_years as i64),
            per.their()
        )
    } else {
        format!("and {} was {} old", per.they(), years(age as i64))
    };
    format!(
        "In the halls and the market squares they began to call {} {}: {} had {}, {}.",
        who,
        epithet,
        per.they(),
        deed,
        standing
    )
}

// ---------------------------------------------------------------------------
// Marriage and children
// ---------------------------------------------------------------------------

/// Two houses joined across a border.
pub fn married_abroad(w: &World, r: usize, h: usize, p: usize, q: usize) -> String {
    let a = &w.persons[r];
    let b = &w.persons[h];
    format!(
        "{} {} of {} was married to {} of {}, and the two houses were joined. \
         The border between them grew quiet.",
        w.honorific(p, a.gender),
        a.name,
        realm(w, p),
        b.name,
        realm_full(w, q)
    )
}

/// A ruler married at home.
pub fn married_at_home(w: &World, r: usize, consort: usize, p: usize) -> String {
    let a = &w.persons[r];
    format!(
        "{} {} of {} took {} as consort.",
        w.honorific(p, a.gender),
        a.name,
        realm(w, p),
        w.persons[consort].name
    )
}

/// An heir was born.
pub fn child_born(w: &World, child: usize, parent: usize, p: usize) -> String {
    let c = &w.persons[child];
    let rank = w.persons[parent]
        .children
        .iter()
        .filter(|&&x| x != child)
        .count();
    let word = match (rank, c.gender) {
        (0, Gender::F) => "a first daughter",
        (0, _) => "a first son",
        (_, Gender::F) => "a daughter",
        (_, Gender::M) => "a son",
        (_, Gender::N) => "a child",
    };
    format!(
        "{} was born to {} {} of {}: {}.",
        c.name,
        w.honorific(p, w.persons[parent].gender),
        w.persons[parent].name,
        realm(w, p),
        word
    )
}

// ---------------------------------------------------------------------------
// Coronation
// ---------------------------------------------------------------------------

/// The style a faith confers. One draw.
pub fn crown_title(w: &World, p: usize, r: usize, pick: &Pick) -> String {
    let adj = &w.polities[p].adj;
    let g = w.persons[r].gender;
    let sovereign = match g {
        Gender::F => "Empress",
        _ => "Emperor",
    };
    match pick.index(4) {
        0 => format!("{} of the {}", sovereign, adj),
        1 => format!("the Anointed of {}", realm(w, p)),
        2 => "Defender of the Faith".to_string(),
        _ => format!("{} by Grace", sovereign),
    }
}

/// A faith crowns a ruler: the single most legitimating thing that can
/// happen to a throne without an army.
pub fn crowned_by_faith(w: &World, r: usize, p: usize, s: usize, title: &str) -> String {
    let per = &w.persons[r];
    let school = &w.schools[s];
    let clergy = match school.kind {
        SchoolKind::Divine => "the high priests",
        SchoolKind::Arcane => "the archmages",
        SchoolKind::Philosophical => "the assembled masters",
    };
    let place = w.polities[p]
        .capital
        .map(|c| w.cities[c].name.clone())
        .unwrap_or_else(|| realm(w, p));
    format!(
        "In {}, {} of {} set a crown upon the head of {} {} and named {} {}. \
         {} rule was no longer merely held but granted, and every lord who \
         doubted it now doubted the faith as well.",
        place,
        clergy,
        school.short,
        w.honorific(p, per.gender),
        per.name,
        per.them(),
        title,
        cap(per.their())
    )
}

// ---------------------------------------------------------------------------
// Succession among real kin
// ---------------------------------------------------------------------------

/// The eldest child takes the whole realm.
pub fn heir_succeeds(w: &World, p: usize, heir: usize, old: usize, house: &str) -> String {
    let h = &w.persons[heir];
    let age = h.age(w.year);
    let relation = if w.persons[heir].parent == Some(old) {
        "child"
    } else {
        "kinsman"
    };
    let youth = if age < 20 {
        format!(" {} is {}.", h.they(), years(age.max(1) as i64))
    } else {
        String::new()
    };
    format!(
        "The throne of {} passed to {} {}, {} of {}, of {}.{}",
        realm(w, p),
        w.honorific(p, h.gender),
        h.name,
        relation,
        w.persons[old].name,
        house,
        youth
    )
}

/// A realm divided among the heirs, which is how a great reign is undone by
/// its own custom.
pub fn partitioned(
    w: &World,
    p: usize,
    old: usize,
    eldest: usize,
    shares: &[(usize, String)],
) -> String {
    let others: Vec<String> = shares
        .iter()
        .map(|(h, name)| format!("{} took {}", w.persons[*h].name, name))
        .collect();
    let aside = if w.persons[old].is_acclaimed() {
        ", who had held it whole,"
    } else {
        ""
    };
    format!(
        "By the custom of {}, the realm of {}{} was divided among {} children. \
         {} kept {} and the old seat; {}. What one hand had gathered, {} hands held.",
        w.cultures[w.polities[p].culture].plural,
        w.persons[old].full_name(),
        aside,
        shares.len() + 1,
        w.persons[eldest].name,
        realm(w, p),
        join_names(&others),
        shares.len() + 1
    )
}

/// The generals of a dead conqueror carve up what he took. The Diadochi.
pub fn generals_divide(
    w: &World,
    p: usize,
    old: usize,
    names: &[String],
    successors: &[usize],
) -> String {
    let with_leaders: Vec<String> = successors
        .iter()
        .zip(names.iter())
        .map(|(&s, name)| match w.polities[s].ruler {
            Some(r) => format!("{} took {}", w.persons[r].name, name),
            None => name.clone(),
        })
        .collect();
    format!(
        "{} died and left no heir who could hold what {} had taken. \
         The generals who had marched with {} did not wait for a council: {}. \
         Within a year there was no {} at all, only the men who had served it.",
        w.persons[old].full_name(),
        w.persons[old].they(),
        w.persons[old].them(),
        join_names(&with_leaders),
        realm(w, p)
    )
}

/// Two realms come under one crown because an heir had a claim in both.
pub fn personal_union(w: &World, heir: usize, keeps: usize, gains: usize) -> String {
    let h = &w.persons[heir];
    format!(
        "{} {} of {} was also, by the marriage of {} parents, the nearest heir of {}. \
         No army marched and no treaty was signed: two crowns simply came to rest on one head.",
        w.honorific(keeps, h.gender),
        h.name,
        realm(w, keeps),
        h.their(),
        realm_full(w, gains)
    )
}

/// A sibling refuses the settlement.
pub fn sibling_war(w: &World, p: usize, rebel_realm: usize, winner: usize, loser: usize) -> String {
    let kin = if w.persons[winner].parent == w.persons[loser].parent {
        "sibling"
    } else {
        "kin"
    };
    format!(
        "{} would not kneel to {}, {} own {}. Taking the lords of the provinces along, \
         {} raised the standard of {}, and the house turned upon itself while {} held {}.",
        w.persons[loser].name,
        w.persons[winner].name,
        w.persons[loser].their(),
        kin,
        w.persons[loser].they(),
        realm_full(w, rebel_realm),
        w.persons[winner].name,
        realm(w, p)
    )
}

// ---------------------------------------------------------------------------
// Children who do not grow up
// ---------------------------------------------------------------------------

/// What took a child. One draw.
pub fn child_death(pick: &Pick) -> String {
    pick.text(&[
        "died of a fever",
        "died in infancy",
        "was carried off by a summer sickness",
        "died of a fall",
        "wasted and died",
    ])
}

/// A child of the ruling house dies before reaching majority.
///
/// Not every realm has a dynasty to name — a chiefdom often has none — so
/// the house is only mentioned when there is one.
pub fn child_died(w: &World, child: usize, p: usize) -> String {
    let c = &w.persons[child];
    let age = c.age(w.year);
    let house = match c.house {
        Some(h) => w.houses[h].name.clone(),
        None => w.polities[p].dynasty.clone(),
    };
    let whose = if house.is_empty() {
        match w.polities[p].ruler {
            Some(r) => format!("child of {}", w.persons[r].name),
            None => "a child of the ruling house".to_string(),
        }
    } else {
        format!("child of the house of {}", house)
    };
    format!(
        "{}, {}, {} at {}.",
        c.name,
        whose,
        c.death,
        years(age.max(1) as i64)
    )
}

/// The epitaph for a realm that ended by inheritance rather than by war.
pub fn united_by_marriage(w: &World, p: usize, _q: usize) -> String {
    format!("inherited by the crown of {}", realm(w, p))
}
