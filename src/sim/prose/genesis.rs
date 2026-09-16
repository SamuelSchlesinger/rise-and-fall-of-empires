//! The opening lines of the world: how a race is described, and what the
//! chronicle says before anything has happened.

use super::{article, list_with_and};
use crate::geo::BiomeGroup;
use crate::sim::Race;

/// How tall and how built a people is. Each reads as "a {stature} people".
pub const STATURE: &[&str] = &[
    "tall",
    "small",
    "lithe",
    "broad-shouldered",
    "gaunt",
    "stout",
    "long-limbed",
    "heavy-browed",
];

/// What marks a people out. Each is a phrase that follows "people"
/// directly, so the result reads "a stout people with feathers at the
/// brow" rather than "a stout, feathered at the brow people".
pub const FEATURE: &[&str] = &[
    "with grey skin",
    "with amber eyes",
    "with horns",
    "with feathers at the brow",
    "with scaled hides",
    "who go cloaked in fur",
    "with long ears",
    "whose skin is rough as bark",
    "as pale as chalk",
    "as dark as river clay",
    "with luminous eyes",
    "with silver hair",
    "with tusks",
    "with moss for hair",
    "with copper skin",
    "with black eyes",
    "with antlers",
    "with webbed fingers",
];

/// A people in one noun phrase, used both in the chronicle and on the
/// race's page: "a stout people with feathers at the brow, warlike and
/// quick to multiply, long-lived, and most at home in the deep forests".
pub fn race_description(r: &Race, stature: &str, feature: &str) -> String {
    let mut temper: Vec<&str> = Vec::new();
    if r.martial > 0.65 {
        temper.push("warlike");
    }
    if r.mystic > 0.65 {
        temper.push("given to visions and sorcery");
    }
    if r.mercantile > 0.65 {
        temper.push("shrewd in trade");
    }
    if r.fecund > 0.65 {
        temper.push("quick to multiply");
    }
    if r.seafaring > 0.65 {
        temper.push("born to the sea");
    }
    if r.martial < 0.35 {
        temper.push("slow to anger");
    }
    if r.mystic < 0.35 {
        temper.push("distrustful of magic");
    }
    if temper.is_empty() {
        temper.push("patient and enduring");
    }
    let life = if r.lifespan < 55.0 {
        "short-lived"
    } else if r.lifespan > 140.0 {
        "long-lived beyond the memory of other peoples"
    } else if r.lifespan > 95.0 {
        "long-lived"
    } else {
        "mortal as the seasons"
    };
    let mut best = BiomeGroup::Temperate;
    let mut best_v = 0.0;
    for g in BiomeGroup::all() {
        if r.affinity[g as usize] > best_v {
            best_v = r.affinity[g as usize];
            best = g;
        }
    }
    format!(
        "{} {} people {}, {}, {}, and most at home in {}",
        article(stature),
        stature,
        feature,
        list_with_and(&temper),
        life,
        best.phrase()
    )
}

/// The first line of every chronicle.
pub fn first_words(world_name: &str) -> String {
    format!(
        "In the beginning there was only the sea and the stone. Then the world of {} was peopled, and its history began.",
        world_name
    )
}

/// The description of the era the world starts in.
pub fn dawn_age(world_name: &str) -> String {
    format!(
        "The first peoples of {} wake and look about them.",
        world_name
    )
}

/// A race's entry in the chronicle: who they are and where they live.
pub fn race_awakes(plural: &str, description: &str, place: &str) -> String {
    format!(
        "The {} are {}. They dwell {}.",
        plural, description, place
    )
}
