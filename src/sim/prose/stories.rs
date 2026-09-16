//! Relics and prophecies: what a thing is, how it changes hands, and
//! what was foretold about it.

use super::{cap, realm, realm_full, years, Pick};
use crate::sim::{ArtifactKind, ProphecyKind, World};

// ---------------------------------------------------------------------------
// Relics
// ---------------------------------------------------------------------------

/// What a relic is said to do.
pub fn artifact_description(kind: ArtifactKind) -> String {
    match kind {
        ArtifactKind::Crown => "a circlet that is said to weigh heavier on an unjust brow",
        ArtifactKind::Blade => "a sword that has never been sharpened and has never needed to be",
        ArtifactKind::Tome => "a book whose last pages are always blank until they are needed",
        ArtifactKind::Gem => "a stone in which, by candlelight, a city can be seen burning",
        ArtifactKind::Banner => {
            "a standard that has never been taken in battle, whatever the outcome"
        }
        ArtifactKind::Staff => "a staff of wood from a tree no one alive has seen",
        ArtifactKind::Chalice => "a cup that turns wine bitter in the mouth of a liar",
        ArtifactKind::Horn => "a horn whose note is heard in every valley of the realm",
        ArtifactKind::Mirror => "a mirror that shows the room as it will be in a hundred years",
        ArtifactKind::Shard => "a splinter of glass from an unmade city, warm to the touch",
    }
    .to_string()
}

/// The making of a relic. `occasion` is a clause ending in a comma or
/// "and", supplied by the caller, that says why it was made.
pub fn artifact_made(occasion: &str, name: &str, description: &str) -> String {
    let occasion = occasion.trim();
    if occasion.is_empty() {
        format!("{} was made: {}.", cap(name), description)
    } else {
        format!("{} {} was made: {}.", cap(occasion), name, description)
    }
}

/// A relic changes hands. `how` is a verb phrase ending in a full stop.
pub fn artifact_passed(name: &str, how: &str) -> String {
    format!("{} {}", cap(name), how)
}

/// Made to remember a great ruler.
pub fn occasion_ruler_memorial(ruler: &str, capital: &str) -> String {
    format!(
        "In memory of {}, the smiths of {} laboured a year, and",
        ruler, capital
    )
}

/// Made to crown a new wonder.
pub fn occasion_wonder(city: &str) -> String {
    format!("To crown the new wonder of {},", city)
}

/// Made for a new school.
pub fn occasion_school(followers: &str, school: &str) -> String {
    format!("For the {} of {},", followers, school)
}

/// Found among the glass of an unmade city.
pub fn occasion_catastrophe() -> &'static str {
    "Among the glass of the unmade city,"
}

/// A relic lost in the sack of a city.
pub fn relic_vanished_in_sack(city: &str) -> String {
    format!("vanished in the sack of {}.", city)
}

/// A relic carried off as spoils.
pub fn relic_taken_as_spoils(w: &World, winner: usize) -> String {
    format!("was carried off to {} as spoils.", realm(w, winner))
}

/// A note in a battle report that a relic was lost.
pub fn relic_lost_aside(name: &str) -> String {
    format!(" {} vanished in the confusion.", cap(name))
}

/// A note in a battle report that a relic was taken.
pub fn relic_taken_aside(name: &str) -> String {
    format!(" {} was carried off by the victors.", cap(name))
}

/// A relic inherited by a conqueror.
pub fn relic_passed_with_fall(w: &World, to: usize, from: usize) -> String {
    format!(
        "passed to {} with the fall of {}.",
        realm(w, to),
        realm_full(w, from)
    )
}

/// A relic lost when its realm ended.
pub fn relic_lost_in_ruin(w: &World, p: usize) -> String {
    format!("was lost in the ruin of {}.", realm_full(w, p))
}

/// A relic dug out of the ground centuries later.
pub fn relic_found(w: &World, p: usize, finders: &str, place: &str, buried: i32) -> String {
    format!(
        "was found: {} of {} dug it out of the earth {}, {} after it was last seen.",
        finders,
        realm_full(w, p),
        place,
        years(buried.max(1) as i64)
    )
}

/// Who dug a relic up. One draw.
pub fn finders(pick: &Pick) -> String {
    pick.text(&[
        "shepherds",
        "grave-robbers",
        "a ploughman",
        "children",
        "soldiers digging a well",
        "a hermit",
    ])
}

/// A relic stolen or lost from a treasury. One draw.
pub fn relic_stolen(w: &World, p: usize, pick: &Pick) -> String {
    match pick.index(3) {
        0 => format!(
            "was stolen from the vaults of {} by a thief who was never caught.",
            realm_full(w, p)
        ),
        1 => format!(
            "was lost when the ship carrying it from {} foundered.",
            realm_full(w, p)
        ),
        _ => format!(
            "vanished from {}; the guards swore the doors had never opened.",
            realm_full(w, p)
        ),
    }
}

/// A relic buried with its owner.
pub fn relic_buried_with(name: &str) -> String {
    format!("was buried with {} and its resting place forgotten.", name)
}

// ---------------------------------------------------------------------------
// Prophecy
// ---------------------------------------------------------------------------

/// What a prophecy says will happen, as a clause after "that".
pub fn prophecy_what(w: &World, kind: ProphecyKind) -> String {
    match kind {
        ProphecyKind::RealmFalls(q) => format!("{} would fall", realm_full(w, q)),
        ProphecyKind::CrownOfEmpire(q) => format!(
            "a ruler of {} would wear an emperor's crown",
            realm(w, q)
        ),
        ProphecyKind::CityBurns(c, _) => format!("{} would burn", w.cities[c].name),
        ProphecyKind::RulerMurdered(q) => format!(
            "a ruler of {} would die by a hand they trusted",
            realm(w, q)
        ),
        ProphecyKind::FaithSpreads(s, n) => format!(
            "{} would be honoured in {}",
            w.schools[s].name,
            super::count(n as i64, "realm")
        ),
        ProphecyKind::RelicReturns(a, q) => format!(
            "{} would return to {}",
            w.artifacts[a].name,
            realm(w, q)
        ),
    }
}

/// How a seer delivered a prophecy. One draw.
pub fn prophecy_opening(pick: &Pick) -> String {
    pick.text(&[
        "spoke in a trance before the court",
        "was found on the temple steps at dawn and said",
        "cried out in the market",
        "read the entrails of a white bull and declared",
        "woke from a fever of nine days and said",
        "wrote on the wall of the granary in charcoal",
    ])
}

/// A prophecy is uttered. No draw.
pub fn prophecy_uttered(seer: &str, where_: &str, opening: &str, what: &str, span: i32) -> String {
    format!(
        "{} of {} {} that {} before {} had passed.",
        seer,
        where_,
        opening,
        what,
        years(span.max(1) as i64)
    )
}

/// A prophecy comes true. No draw.
pub fn prophecy_fulfilled(seer: &str, what: &str, after: i32) -> String {
    format!(
        "The words of {} came true {} after they were spoken: it had been foretold that {}, and so it was.",
        seer,
        years(after.max(1) as i64),
        what
    )
}

/// A prophecy runs out of time. One draw.
pub fn prophecy_failed(seer: &str, what: &str, pick: &Pick) -> String {
    match pick.index(3) {
        0 => format!(
            "The years allotted to the prophecy of {} ran out. It had been foretold that {}; it had not come to pass, and the seer's name became a byword for foolishness.",
            seer, what
        ),
        1 => format!(
            "The prophecy of {} that {} was quietly forgotten.",
            seer, what
        ),
        _ => format!(
            "Nothing came of the words of {}. It had been said that {}; children laughed at the old story.",
            seer, what
        ),
    }
}

/// A person's lineage, as shown on their page.
pub fn lineage(w: &World, person: usize) -> String {
    let mut parts = Vec::new();
    let mut cur = w.persons[person].parent;
    let words = ["child", "grandchild", "great-grandchild"];
    let mut k = 0;
    while let Some(p) = cur {
        if k >= words.len() {
            break;
        }
        parts.push(format!("{} of {}", words[k], w.persons[p].full_name()));
        cur = w.persons[p].parent;
        k += 1;
    }
    parts.join(", ")
}
