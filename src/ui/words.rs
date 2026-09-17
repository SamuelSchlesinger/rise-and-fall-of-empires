//! Numbers in plain words. The simulation thinks in percentages; a viewer
//! reads "on the brink" faster than "12%". Every mapping here is a pure
//! function so it can be tested and reused by the sidebar, the lists and
//! the detail pages alike.

use super::{Layer, Mode};
use crate::sim::{PolityKind, SchoolKind, World};

/// Short, complete keyboard prompts. Keep the entry points to the richer
/// screens ahead of movement controls so they remain visible on small windows.
pub(super) fn navigation(mode: Mode, paused: bool) -> [&'static str; 2] {
    match mode {
        Mode::Map => [
            "Enter inspect  e browse  r recap  c history  t story  x intervene",
            if paused {
                "Space resume  +/- speed  Arrows move  Tab layers  / search  f follow"
            } else {
                "Space pause  +/- speed  Arrows move  Tab layers  / search  f follow"
            },
        ],
        Mode::List => [
            "Enter inspect  Tab category  / filter  x intervene",
            "Arrows choose  m map  Space pause/resume",
        ],
        Mode::Detail => [
            "[letters] open links  Backspace previous  m map",
            "Arrows scroll  PgUp/PgDn page  ]/[ realms  Space pause/resume",
        ],
        Mode::Chronicle => [
            "/ filter  f importance  Click event to visit",
            "Arrows scroll  PgUp/PgDn page  Space pause/resume",
        ],
        Mode::Recap => [
            ":recap N change years  Arrows scroll",
            "PgUp/PgDn page  Space pause/resume",
        ],
        Mode::Help => [
            "p player guide  t tutorial",
            "Arrows scroll  PgUp/PgDn page",
        ],
        Mode::Guide => [
            "? key reference  t tutorial",
            "Arrows scroll  PgUp/PgDn page",
        ],
        Mode::Fate => ["1-6 choose an intervention", "Esc cancel"],
    }
}

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
        match city.destroyed {
            None => s.push_str(&format!(", where {} stands", city.name)),
            Some(year) => {
                s.push_str(&format!(", where {} stood until {}", city.name, year));
            }
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

/// Where a realm's stability is going: the simulation's own word for it,
/// and the number that word is about.
///
/// `drift` is [`stability_drift`](crate::sim::explain::stability_drift) and
/// `target` is [`stability_target`](crate::sim::explain::stability_target),
/// which is the total the realm page's **Why** block prints. Saying them
/// together is the point: the page used to call a realm "restless (65%),
/// and holding there" three lines above "pulls stability towards 62%",
/// with nothing to say those were the same fact.
pub fn stability_trend(drift: &str, target: f32) -> String {
    let word = drift.trim().trim_start_matches("and ").trim();
    format!(
        "{}: the pull is towards {:.0}%",
        word,
        (target * 100.0).round()
    )
}

/// What a level of the event log is worth in words, so the feed says what
/// it is showing rather than only a number.
///
/// Level 2, the default, is about one line a year at five years a second —
/// a feed a reader can actually follow. Level 1 is everything the chronicle
/// keeps, which at speed scrolls past far faster than anyone can read.
pub fn log_level(min: u8) -> &'static str {
    match min {
        0 | 1 => "everything",
        2 => "the notable",
        _ => "only the great",
    }
}

/// The key to whatever the map is currently showing, cut to the one line
/// of `width` characters it is given under the map.
pub fn legend(layer: Layer, ascii: bool, width: usize) -> String {
    fit(&legend_parts(layer, ascii), width, ascii)
}

/// Every piece of a layer's key, the most worth knowing first, so a narrow
/// terminal loses the tail of the key rather than half of a sentence.
///
/// The glyphs come from [`biome_style`](crate::ui::biome_style), the same
/// call the map draws with, so the key cannot drift from the map.
pub fn legend_parts(layer: Layer, ascii: bool) -> Vec<String> {
    use crate::geo::Biome;
    let g = |b: Biome| crate::ui::biome_style(b, ascii).1;
    let biome = |b: Biome, name: &str| format!("{}{}", g(b), name);
    let river = if ascii { '~' } else { '≈' };
    let ruins = if ascii { 'x' } else { '×' };
    let field = if ascii { '.' } else { '·' };
    match layer {
        Layer::Political => vec![
            "lines are borders".into(),
            format!("{} realm land", field),
            "blank unclaimed".into(),
            "@ capital".into(),
            "# city".into(),
            format!("{} ruins", ruins),
            "! an event".into(),
            format!("{} river", river),
        ],
        Layer::Culture => vec![
            "lines part peoples".into(),
            format!("{} their land", field),
            "blank is empty".into(),
            "deeper colour, more of them".into(),
            "@ # cities".into(),
        ],
        Layer::Terrain => vec![
            biome(Biome::Mountain, " peak"),
            biome(Biome::Hills, " hill"),
            biome(Biome::Forest, " wood"),
            biome(Biome::Taiga, " pine"),
            biome(Biome::Grassland, " grass"),
            biome(Biome::Steppe, " steppe"),
            biome(Biome::Desert, " desert"),
            format!("{} river or lake", river),
            "@ # cities".into(),
            biome(Biome::Tundra, " tundra"),
            biome(Biome::Ice, " ice"),
            biome(Biome::Wastes, " waste"),
            biome(Biome::Ocean, " sea"),
        ],
        Layer::Biomes => vec![
            "flat colours are biomes".into(),
            biome(Biome::Mountain, " peak"),
            biome(Biome::Hills, " hill"),
            biome(Biome::Forest, " wood"),
            biome(Biome::Taiga, " pine"),
            biome(Biome::Grassland, " grass"),
            biome(Biome::Steppe, " steppe"),
            biome(Biome::Desert, " desert"),
            biome(Biome::Tundra, " tundra"),
            biome(Biome::Ice, " ice"),
            biome(Biome::Wastes, " waste"),
            biome(Biome::Ocean, " sea"),
            biome(Biome::Shallows, " shoal or lake"),
        ],
        Layer::Magic => vec![
            "purple is wild mana".into(),
            "a realm takes its school's colour".into(),
            "* a school's home".into(),
            "@ # cities".into(),
        ],
        Layer::Population => vec![
            "dark to gold to red as people crowd in".into(),
            "the sea is left bare".into(),
            "@ # cities".into(),
        ],
    }
}

/// As many `parts` as `width` has room for, two spaces apart, ending in an
/// ellipsis when some had to be left out.
pub fn fit(parts: &[String], width: usize, ascii: bool) -> String {
    let sep = "  ";
    let more = if ascii { "..." } else { "…" };
    let len = |s: &str| s.chars().count();
    let mut used = 0;
    let mut taken = 0;
    for (i, p) in parts.iter().enumerate() {
        let add = len(p) + if i == 0 { 0 } else { sep.len() };
        if used + add > width {
            break;
        }
        used += add;
        taken += 1;
    }
    if taken == parts.len() {
        return parts.join(sep);
    }
    // Something was left out, so say so — dropping one more part if that is
    // what it takes to make room for the mark.
    while taken > 0 && used + 1 + len(more) > width {
        taken -= 1;
        used = used.saturating_sub(len(&parts[taken]) + if taken == 0 { 0 } else { sep.len() });
    }
    if taken == 0 {
        return parts
            .first()
            .map(|p| {
                let room = width.saturating_sub(len(more));
                p.chars().take(room).collect::<String>() + more
            })
            .unwrap_or_default();
    }
    format!("{} {}", parts[..taken].join(sep), more)
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

    /// The word and the number are one sentence, so the page cannot say a
    /// realm is holding steady beside a target it is nowhere near.
    #[test]
    fn the_trend_carries_the_number_it_is_about() {
        assert_eq!(
            stability_trend("and holding there", 0.62),
            "holding there: the pull is towards 62%"
        );
        assert_eq!(
            stability_trend("and getting worse", 0.2),
            "getting worse: the pull is towards 20%"
        );
        assert!(stability_trend("and recovering", 0.344).contains("34%"));
    }

    #[test]
    fn log_levels_have_words() {
        assert_eq!(log_level(1), "everything");
        assert_eq!(log_level(2), "the notable");
        assert_eq!(log_level(3), "only the great");
        assert_eq!(log_level(0), log_level(1));
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
        assert_eq!(realm_size(0), "a realm of no land at all");
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

    /// The widths the key is actually given under the map: a 150-, a 120-,
    /// a 100- and an 80-column terminal, less the sidebar, the layer's name
    /// and the margins.
    const LEGEND_WIDTHS: [usize; 4] = [102, 72, 51, 66];

    #[test]
    fn legends_fit_on_one_line() {
        for l in Layer::all() {
            for ascii in [false, true] {
                for w in LEGEND_WIDTHS {
                    let s = legend(l, ascii, w);
                    assert!(!s.contains('\n'));
                    assert!(
                        s.chars().count() <= w,
                        "{:?} at {}: {:?} is {} wide",
                        l,
                        w,
                        s,
                        s.chars().count()
                    );
                    assert!(!s.is_empty(), "{:?} at {} says nothing", l, w);
                    if ascii {
                        assert!(s.is_ascii(), "{}", s);
                    }
                }
            }
        }
    }

    /// A key cut short says so, and the part it does show is whole.
    #[test]
    fn a_shortened_legend_is_marked() {
        let parts = legend_parts(Layer::Terrain, false);
        let full = fit(&parts, 400, false);
        for p in &parts {
            assert!(full.contains(p.as_str()), "{} missing from {}", p, full);
        }
        let short = fit(&parts, 30, false);
        assert!(short.ends_with('…'), "{}", short);
        assert!(short.starts_with(&parts[0]), "{}", short);
        assert!(fit(&parts, 30, true).ends_with("..."));
    }

    /// Every glyph the terrain and biome layers draw is named in their key.
    #[test]
    fn the_map_glyphs_are_all_in_the_key() {
        use crate::geo::Biome;
        let biomes = [
            Biome::DeepOcean,
            Biome::Ocean,
            Biome::Shallows,
            Biome::Lake,
            Biome::Ice,
            Biome::Tundra,
            Biome::Taiga,
            Biome::Steppe,
            Biome::Grassland,
            Biome::Forest,
            Biome::Jungle,
            Biome::Savanna,
            Biome::Desert,
            Biome::Swamp,
            Biome::Hills,
            Biome::Mountain,
            Biome::Peak,
            Biome::Wastes,
        ];
        for ascii in [false, true] {
            for layer in [Layer::Terrain, Layer::Biomes] {
                let key = legend_parts(layer, ascii).join(" ");
                for b in biomes {
                    let g = crate::ui::biome_style(b, ascii).1;
                    assert!(
                        key.contains(g),
                        "{:?}: {:?} is in no key: {}",
                        layer,
                        g,
                        key
                    );
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
