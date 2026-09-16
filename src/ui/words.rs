//! Numbers in plain words. The simulation thinks in percentages; a viewer
//! reads "on the brink" faster than "12%". Every mapping here is a pure
//! function so it can be tested and reused by the sidebar, the lists and
//! the detail pages alike.

use super::Layer;
use crate::sim::{PolityKind, SchoolKind, World};

/// steady · restless · troubled · on the brink
pub fn stability(v: f32) -> &'static str {
    if v >= 0.65 {
        "steady"
    } else if v >= 0.45 {
        "restless"
    } else if v >= 0.25 {
        "troubled"
    } else {
        "on the brink"
    }
}

/// bankrupt · poor · solvent · rich
pub fn treasury(v: f32) -> &'static str {
    if v < 0.0 {
        "bankrupt"
    } else if v < 40.0 {
        "poor"
    } else if v < 200.0 {
        "solvent"
    } else {
        "rich"
    }
}

/// A realm's army measured against its strongest neighbour.
pub fn army_standing(w: &World, p: usize) -> &'static str {
    let pol = &w.polities[p];
    let best = pol
        .neighbors
        .iter()
        .filter(|&&(q, _)| w.polities[q].alive())
        .map(|&(q, _)| w.polities[q].army)
        .fold(0.0f32, f32::max);
    if best <= 0.0 {
        return "unchallenged";
    }
    let ratio = pol.army / best;
    if ratio >= 1.5 {
        "formidable"
    } else if ratio >= 0.7 {
        "matched"
    } else {
        "outmatched"
    }
}

/// "a small realm", "a great power".
pub fn realm_size(cells: usize) -> &'static str {
    match cells {
        0 => "a realm of no land at all",
        1..=14 => "a tiny realm",
        15..=39 => "a small realm",
        40..=99 => "a middling realm",
        100..=249 => "a large realm",
        _ => "a great power",
    }
}

/// A one-word standing for a realm, for lists and the map legend.
pub fn realm_rank(cells: usize) -> &'static str {
    match cells {
        0..=14 => "tiny",
        15..=39 => "small",
        40..=99 => "middling",
        100..=249 => "large",
        _ => "great",
    }
}

/// "a village", "a great city".
pub fn city_size(pop: f32) -> &'static str {
    if pop < 3.0 {
        "a village"
    } else if pop < 10.0 {
        "a town"
    } else if pop < 30.0 {
        "a city"
    } else if pop < 80.0 {
        "a great city"
    } else {
        "one of the wonders of the world"
    }
}

/// How far a school of thought has spread, in its own idiom.
pub fn school_reach(w: &World, s: usize) -> &'static str {
    let sc = &w.schools[s];
    if sc.extinct.is_some() {
        return "remembered only in books";
    }
    let realms = sc
        .influence
        .iter()
        .filter(|(&p, &v)| v > 0.1 && w.polities[p].alive())
        .count();
    match (sc.kind, realms) {
        (SchoolKind::Arcane, 0..=1) => "a circle of one city",
        (SchoolKind::Arcane, 2..=3) => "an order that is spreading",
        (SchoolKind::Arcane, 4..=8) => "a great order",
        (SchoolKind::Arcane, _) => "an order that spans the world",
        (SchoolKind::Divine, 0..=1) => "a local cult",
        (SchoolKind::Divine, 2..=3) => "a faith that is spreading",
        (SchoolKind::Divine, 4..=8) => "a great faith",
        (SchoolKind::Divine, _) => "a faith that spans the world",
        (SchoolKind::Philosophical, 0..=1) => "a local school",
        (SchoolKind::Philosophical, 2..=3) => "a teaching that is spreading",
        (SchoolKind::Philosophical, 4..=8) => "a broad tradition",
        (SchoolKind::Philosophical, _) => "a tradition that spans the world",
    }
}

/// "rising" / "falling" for a realm, from what its ruler has won or lost.
pub fn trend(w: &World, p: usize) -> Option<&'static str> {
    let pol = &w.polities[p];
    if !pol.alive() {
        return None;
    }
    let years = (w.year - pol.reign_start).max(1);
    let per_year = pol.reign_gained as f32 / years as f32;
    if pol.reign_gained >= 8 && per_year > 0.15 {
        Some("rising")
    } else if pol.reign_gained <= -8 && per_year < -0.15 {
        Some("falling")
    } else {
        None
    }
}

/// A word for how a realm is governed, for the "Selected" block.
pub fn polity_kind(kind: PolityKind) -> &'static str {
    match kind {
        PolityKind::Tribe => "a tribe",
        PolityKind::Chiefdom => "a chiefdom",
        PolityKind::Kingdom => "a kingdom",
        PolityKind::Empire => "an empire",
        PolityKind::Republic => "a republic",
        PolityKind::Theocracy => "a theocracy",
        PolityKind::Magocracy => "a magocracy",
        PolityKind::Horde => "a horde",
    }
}

/// 10.2 (thousands) becomes "10,200".
pub fn folk(thousands: f32) -> String {
    let n = (thousands * 1000.0).round().max(0.0) as i64;
    let n = if n >= 10_000 {
        (n / 100) * 100
    } else if n >= 1000 {
        (n / 10) * 10
    } else {
        n
    };
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// One plain sentence for whatever the cursor is sitting on.
pub fn here_sentence(w: &World, cell: usize) -> String {
    let t = &w.terrain;
    let cs = &w.cells[cell];
    let mut s = capitalize(t.biome[cell].name());
    match t.river_at(cell).and_then(|f| t.features[f].name.clone()) {
        Some(n) => s.push_str(&format!(" by the river {}", n)),
        None if t.river[cell] >= 2 => s.push_str(" on a nameless river"),
        None if t.river[cell] == 1 => s.push_str(" by a stream"),
        None if t.coast[cell] && !t.biome[cell].is_water() => s.push_str(" on the coast"),
        None => {
            if let Some(n) = t.feature_at(cell).and_then(|f| t.features[f].name.clone()) {
                s.push_str(&format!(" in {}", n));
            }
        }
    }
    if let Some(c) = cs.city {
        let city = &w.cities[c];
        if city.destroyed.is_none() {
            s.push_str(&format!(
                ", where {} stands ({}, {} people)",
                city.name,
                city_size(city.pop),
                folk(city.pop)
            ));
        } else {
            s.push_str(&format!(", where {} stood until {}", city.name, city.destroyed.unwrap()));
        }
    }
    match cs.owner {
        Some(p) => s.push_str(&format!(", ruled by {}", w.polities[p].name)),
        None => {
            if !t.biome[cell].is_water() {
                s.push_str(", claimed by no one");
            }
        }
    }
    match cs.culture {
        Some(c) if cs.pop >= 0.05 => s.push_str(&format!(
            ", home to {} {} folk",
            folk(cs.pop),
            w.cultures[c].adj
        )),
        _ if !t.biome[cell].is_water() => s.push_str(", empty of people"),
        _ => {}
    }
    s.push('.');
    s
}

/// The one-line key to whatever the map is currently showing.
pub fn legend(layer: Layer, ascii: bool) -> String {
    let s = match layer {
        Layer::Political => {
            "colours are realms · brighter edges are borders · @ capital · # city · × ruins · ! this year's news"
        }
        Layer::Terrain => {
            "the bare land · ▲ mountains · ♣ forest · : desert · ≈ rivers · @ and # are cities"
        }
        Layer::Culture => {
            "colours are peoples, not realms · the deeper the colour, the thicker they live"
        }
        Layer::Magic => {
            "purple is wild mana · a realm is tinted by the school it follows · * a school's home"
        }
        Layer::Population => "dark to gold to red as people crowd in · the sea is left bare",
        Layer::Biomes => "flat colours are biomes · green forest · gold desert · grey peaks · blue sea",
    };
    if ascii {
        s.replace('·', "|")
            .replace('×', "x")
            .replace('▲', "^")
            .replace('♣', "T")
            .replace('≈', "~")
    } else {
        s.to_string()
    }
}

pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::Detail;

    #[test]
    fn stability_reads_as_words() {
        assert_eq!(stability(1.0), "steady");
        assert_eq!(stability(0.65), "steady");
        assert_eq!(stability(0.5), "restless");
        assert_eq!(stability(0.3), "troubled");
        assert_eq!(stability(0.0), "on the brink");
    }

    #[test]
    fn treasury_reads_as_words() {
        assert_eq!(treasury(-1.0), "bankrupt");
        assert_eq!(treasury(0.0), "poor");
        assert_eq!(treasury(100.0), "solvent");
        assert_eq!(treasury(600.0), "rich");
    }

    #[test]
    fn sizes_climb_in_order() {
        let ladder = [0usize, 10, 30, 60, 150, 400];
        let words: Vec<&str> = ladder.iter().map(|&c| realm_size(c)).collect();
        for pair in words.windows(2) {
            assert_ne!(pair[0], pair[1]);
        }
        assert_eq!(realm_size(400), "a great power");
        assert_eq!(realm_rank(400), "great");
    }

    #[test]
    fn thousands_get_commas() {
        assert_eq!(folk(0.0), "0");
        assert_eq!(folk(4.0), "4,000");
        assert_eq!(folk(0.4), "400");
        assert_eq!(folk(10.24), "10,200");
        assert_eq!(folk(1234.0), "1,234,000");
    }

    #[test]
    fn every_cell_describes_itself() {
        let mut w = crate::sim::World::new(5, 50, 26, Detail::Low);
        for _ in 0..120 {
            w.tick();
        }
        for i in (0..w.cells.len()).step_by(7) {
            let s = here_sentence(&w, i);
            assert!(s.ends_with('.'), "{}", s);
            assert!(s.chars().next().unwrap().is_uppercase(), "{}", s);
            assert!(!s.contains(" ,"), "{}", s);
        }
    }

    #[test]
    fn legends_fit_on_one_line() {
        for l in Layer::all() {
            for ascii in [false, true] {
                let s = legend(l, ascii);
                assert!(!s.contains('\n'));
                assert!(s.chars().count() < 110, "{}", s);
                if ascii {
                    assert!(s.is_ascii(), "{}", s);
                }
            }
        }
    }

    #[test]
    fn schools_and_armies_have_words() {
        let mut w = crate::sim::World::new(9, 50, 26, Detail::Low);
        for _ in 0..200 {
            w.tick();
        }
        for s in 0..w.schools.len() {
            assert!(!school_reach(&w, s).is_empty());
        }
        for p in w.living_polities() {
            assert!(!army_standing(&w, p).is_empty());
            assert!(!realm_size(w.polities[p].cells).is_empty());
        }
    }
}
