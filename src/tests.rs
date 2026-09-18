//! Automated checks: determinism, save files, configuration, names and
//! geography. Kept to small worlds and few years so `cargo test` is quick.

use crate::config::{self, Config};
use crate::geo;
use crate::lang::Language;
use crate::rng::Rng;
use crate::ser;
use crate::sim::{Detail, World};
use crate::term::Key;

const W: usize = 80;
const H: usize = 32;

fn world(seed: u64) -> World {
    World::new(seed, W, H, Detail::High)
}

fn run(w: &mut World, years: i32) {
    for _ in 0..years {
        w.tick();
    }
}

/// Every event rendered as one line, including year, importance and kind.
fn chronicle(w: &World) -> Vec<String> {
    w.chronicle
        .events
        .iter()
        .map(|e| {
            format!(
                "{}|{}|{}|{:?}|{}",
                e.year,
                e.importance,
                e.kind.name(),
                e.loc,
                e.text
            )
        })
        .collect()
}

/// The world's summary numbers as an exactly comparable string.
fn summary(w: &World) -> String {
    format!(
        "year {} pop {:?} peak {:?} realms {} alive {} cities {} cultures {} schools {} wars {} cells {} persons {} events {}",
        w.year,
        w.stats.pop,
        w.stats.peak_pop,
        w.polities.len(),
        w.stats.polities_alive,
        w.stats.cities_alive,
        w.stats.cultures_alive,
        w.stats.schools_alive,
        w.stats.wars_active,
        w.stats.owned_cells,
        w.persons.len(),
        w.chronicle.len()
    )
}

/// Every cell's population, bit for bit.
fn populations(w: &World) -> Vec<u32> {
    w.cells.iter().map(|c| c.pop.to_bits()).collect()
}

#[test]
fn rng_clones_share_one_stream() {
    // A handle taken with `clone()` must advance the same stream as the
    // original, or a local roll and a nested call would draw in lockstep.
    let a = Rng::new(3);
    let b = a.clone();
    assert_eq!(a.state(), b.state());
    let first = a.next_u64();
    assert_ne!(first, b.next_u64());
    assert_eq!(a.state(), b.state());
    let c = Rng::new(3);
    assert_eq!(first, c.next_u64(), "a fresh Rng is still its own stream");
}

#[test]
fn same_seed_same_history() {
    let mut a = world(7);
    let mut b = world(7);
    run(&mut a, 200);
    run(&mut b, 200);
    assert_eq!(chronicle(&a), chronicle(&b));
    assert_eq!(summary(&a), summary(&b));
    assert_eq!(populations(&a), populations(&b));
    assert_eq!(a.rng.state(), b.rng.state());
    assert!(
        a.chronicle.len() > 50,
        "a 200 year world should have history"
    );
}

/// The owner index has to agree with the cells themselves at every moment a
/// subsystem might read it, not only after `recompute`.
#[test]
fn owner_index_matches_the_map() {
    let mut w = world(5);
    for _ in 0..200 {
        w.tick();
        for p in 0..w.polities.len() {
            let want: Vec<usize> = (0..w.cells.len())
                .filter(|&i| w.cells[i].owner == Some(p))
                .collect();
            assert_eq!(w.cells_of(p), want, "realm {} in year {}", p, w.year);
        }
    }
}

/// Compaction keeps the chronicle near its cap, oldest and least important
/// first, and never touches the handful of age-defining events.
///
/// The promise used to be that nothing of importance 2 or above was ever
/// dropped, and on a long game that made the cap a fiction: a large map at
/// the eightieth century held four hundred thousand events against a cap of
/// sixty thousand, because by then almost nothing left was droppable. So
/// importance 2 is droppable now, oldest first, and only importance 3 — five
/// places in the whole simulation, the naming of an age and the rise and
/// dissolution of a hegemony — is kept whatever happens.
///
/// What the test demands is therefore two things: that the cap is real, and
/// that the events the world is *named* after survive it.
#[test]
fn chronicle_compaction_keeps_the_cap_and_the_greatest_events() {
    let mut w = world(13);
    w.tuning.chronicle_cap = 400;
    run(&mut w, 900);
    assert!(w.chronicle.dropped > 0, "nothing was ever dropped");
    assert!(
        w.chronicle.len() <= 400,
        "{} events against a cap of 400",
        w.chronicle.len()
    );

    let mut full = world(13);
    full.tuning.chronicle_cap = 0;
    run(&mut full, 900);
    assert_eq!(full.chronicle.dropped, 0, "an uncapped chronicle dropped");
    let greatest = |w: &World| -> Vec<(i32, String)> {
        w.chronicle
            .events
            .iter()
            .filter(|e| e.importance >= 3)
            .map(|e| (e.year, e.text.clone()))
            .collect()
    };
    assert_eq!(
        greatest(&w),
        greatest(&full),
        "compaction dropped an event the world is named after"
    );
    // And what it did drop, it dropped from the far end: whatever survives
    // of the ordinary run of history is the recent part of it.
    let oldest_ordinary = w
        .chronicle
        .events
        .iter()
        .filter(|e| e.importance < 3)
        .map(|e| e.year)
        .min()
        .expect("some ordinary history survived");
    assert!(
        oldest_ordinary > 100,
        "the oldest surviving footnote is from year {}, so nothing was trimmed from the front",
        oldest_ordinary
    );

    let mut refs: Vec<crate::sim::chronicle::Ref> = Vec::new();
    for e in &w.chronicle.events {
        refs.extend(e.refs.iter().copied());
    }
    refs.sort();
    refs.dedup();
    for r in refs {
        for &id in w.chronicle.for_ref(r) {
            assert!(
                w.chronicle.events[id].refs.contains(&r),
                "stale by-ref entry after compaction"
            );
        }
    }
}

#[test]
fn different_seeds_differ() {
    let mut a = world(7);
    let mut b = world(8);
    run(&mut a, 60);
    run(&mut b, 60);
    assert_ne!(chronicle(&a), chronicle(&b));
}

#[test]
fn save_round_trip_continues_identically() {
    let mut a = world(11);
    run(&mut a, 120);
    let bytes = ser::save(&mut a);
    let mut b = ser::load(&bytes).expect("a fresh save must load");
    assert_eq!(chronicle(&a), chronicle(&b));
    assert_eq!(populations(&a), populations(&b));
    run(&mut a, 100);
    run(&mut b, 100);
    assert_eq!(chronicle(&a), chronicle(&b));
    assert_eq!(populations(&a), populations(&b));
    assert_eq!(summary(&a), summary(&b));
}

#[test]
fn load_rejects_bad_input() {
    assert!(ser::load(&[]).is_err());
    assert!(ser::load(b"not a save file at all").is_err());
    let garbage: Vec<u8> = (0..4096u32)
        .map(|i| (i.wrapping_mul(2654435761) >> 13) as u8)
        .collect();
    assert!(ser::load(&garbage).is_err());

    let mut w = world(5);
    run(&mut w, 30);
    let good = ser::save(&mut w);
    assert!(ser::load(&good).is_ok());

    // Right length, wrong magic.
    let mut wrong_magic = good.clone();
    wrong_magic[0] = b'X';
    assert!(ser::load(&wrong_magic).is_err());

    // A version this build does not know.
    let mut wrong_version = good.clone();
    wrong_version[4] = 99;
    assert!(ser::load(&wrong_version).is_err());

    // Truncations must return an error rather than panic.
    for cut in [8, 64, 1000, good.len() / 3, good.len() / 2, good.len() - 1] {
        if cut < good.len() {
            assert!(
                ser::load(&good[..cut]).is_err(),
                "truncating to {} bytes should fail cleanly",
                cut
            );
        }
    }
}

#[test]
fn key_names_round_trip() {
    let keys = [
        Key::Char('j'),
        Key::Char('G'),
        Key::Char('0'),
        Key::Char(' '),
        Key::Char('<'),
        Key::Ctrl('d'),
        Key::Ctrl('u'),
        Key::Up,
        Key::Down,
        Key::Left,
        Key::Right,
        Key::ShiftUp,
        Key::ShiftDown,
        Key::ShiftLeft,
        Key::ShiftRight,
        Key::PageUp,
        Key::PageDown,
        Key::Home,
        Key::End,
        Key::Enter,
        Key::Esc,
        Key::Tab,
        Key::BackTab,
        Key::Backspace,
        Key::Delete,
        Key::F(1),
        Key::F(12),
    ];
    for k in &keys {
        let name = config::key_name(k);
        assert_eq!(
            config::parse_key(&name),
            Some(k.clone()),
            "'{}' should parse back to {:?}",
            name,
            k
        );
    }
    // Spellings a user might write by hand.
    assert_eq!(config::parse_key("<CR>"), Some(Key::Enter));
    assert_eq!(config::parse_key("<escape>"), Some(Key::Esc));
    assert_eq!(config::parse_key("<space>"), Some(Key::Char(' ')));
    assert_eq!(config::parse_key("<pgup>"), Some(Key::PageUp));
    assert_eq!(config::parse_key("<c-D>"), Some(Key::Ctrl('d')));
    assert_eq!(config::parse_key(""), None);
    assert_eq!(config::parse_key("<Nonsense>"), None);
    assert_eq!(config::parse_key("<Up"), None);
}

#[test]
fn config_rejects_bad_values() {
    let mut c = Config::default();
    assert!(c.set("detail", "high").is_ok());
    assert_eq!(c.detail, Some(Detail::High));
    assert!(c.set("speed", "25").is_ok());
    assert!(c.set("mouse", "off").is_ok());
    assert_eq!(c.mouse, Some(false));
    assert!(c.set("zoom", "2").is_ok());

    assert!(c.set("detail", "enormous").is_err());
    assert!(c.set("speed", "quick").is_err());
    assert!(c.set("mouse", "maybe").is_err());
    assert!(c.set("ascii", "sometimes").is_err());
    assert!(c.set("autosave", "often").is_err());
    assert!(c.set("width", "wide").is_err());
    assert!(c.set("log", "-").is_err());
    assert!(c.set("follow", "later").is_err());
    assert!(c.set("nonsense", "1").is_err());
    // A rejected value leaves the old one in place.
    assert_eq!(c.detail, Some(Detail::High));
    assert_eq!(c.mouse, Some(false));
}

#[test]
fn generated_names_are_pronounceable() {
    let rng = Rng::new(4242);
    let mut checked = 0;
    for _ in 0..10 {
        let lang = Language::generate(&rng);
        for _ in 0..100 {
            let name = lang.name(&rng);
            let n = name.chars().count();
            assert!(
                (3..=10).contains(&n),
                "name '{}' has {} characters",
                name,
                n
            );
            let first = name.chars().next().unwrap();
            assert!(first.is_uppercase(), "name '{}' is not capitalised", name);
            assert!(
                name.chars()
                    .skip(1)
                    .all(|c| c.is_lowercase() || c == '\'' || c == '-'),
                "name '{}' should be capitalised only at the start",
                name
            );
            let chars: Vec<char> = name.chars().collect();
            assert!(
                !chars.windows(3).any(|t| t[0] == t[1] && t[1] == t[2]),
                "name '{}' has a triple letter",
                name
            );
            assert!(
                !crate::lang::awkward(&name),
                "name '{}' is on the blocklist",
                name
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 1000);
}

#[test]
fn terrain_is_sane() {
    for seed in [1u64, 2, 3, 99, 12345] {
        let rng = Rng::new(seed);
        let t = geo::generate(&rng, W, H);
        let n = t.n();
        assert_eq!(n, W * H);
        assert_eq!(t.biome.len(), n);
        assert_eq!(t.river.len(), n);
        assert_eq!(t.coast.len(), n);

        let land = (0..n).filter(|&i| t.is_land(i)).count();
        assert_eq!(land, t.land_count, "land_count should match the biomes");
        let frac = land as f64 / n as f64;
        assert!(
            (0.25..=0.65).contains(&frac),
            "seed {}: land fraction {:.3} is outside 25%-65%",
            seed,
            frac
        );

        for i in 0..n {
            if t.river[i] > 0 {
                assert!(t.is_land(i), "seed {}: river cell {} is water", seed, i);
            }
            if t.coast[i] {
                assert!(t.is_land(i), "seed {}: coast cell {} is water", seed, i);
                assert!(
                    t.neighbors8(i).any(|nb| t.biome[nb].is_sea()),
                    "seed {}: coast cell {} has no sea neighbour",
                    seed,
                    i
                );
            }
        }
        assert!(t.river.iter().any(|&r| r > 0), "seed {}: no rivers", seed);
        assert!(t.coast.iter().any(|&c| c), "seed {}: no coast", seed);
    }
}

/// The smallest world the command line allows, `--width 40 --height 20`, has
/// little room for realms: some seeds leave one with no cities, no neighbours
/// or no land worth the name. Nothing in a tick may fall over on that.
#[test]
fn tiny_worlds_run_without_panicking() {
    for seed in 1..=6u64 {
        for detail in [Detail::Low, Detail::Medium, Detail::High] {
            let mut w = World::new(seed, 40, 20, detail);
            run(&mut w, 150);
            assert_eq!(w.year, 150, "seed {} should have reached year 150", seed);
            assert_eq!(w.cells.len(), w.terrain.n());
            for p in w.living_polities() {
                assert!(
                    w.polities[p].cells <= w.terrain.n(),
                    "seed {}: realm {} holds more land than the world has",
                    seed,
                    p
                );
            }
        }
    }
}

/// Detail is a verbosity setting: it decides what gets written down and
/// nothing else. Low, medium and high must therefore live exactly the same
/// history from the same seed, down to the state of the RNG, and differ
/// only in how many events came out of it.
#[test]
fn detail_changes_only_what_is_written() {
    let mut worlds: Vec<World> = [Detail::Low, Detail::Medium, Detail::High]
        .iter()
        .map(|&d| World::new(19, 48, 24, d))
        .collect();
    for w in worlds.iter_mut() {
        run(w, 200);
    }
    for w in &worlds[1..] {
        assert_eq!(
            summary_without_events(w),
            summary_without_events(&worlds[0])
        );
        assert_eq!(populations(w), populations(&worlds[0]));
        assert_eq!(w.rng.state(), worlds[0].rng.state());
    }
    // And the point of the setting: more detail, more written down.
    assert!(
        worlds[2].chronicle.len() > worlds[1].chronicle.len(),
        "high detail should write more than medium"
    );
    assert!(
        worlds[1].chronicle.len() > worlds[0].chronicle.len(),
        "medium detail should write more than low"
    );
}

/// [`summary`] without the event count, which detail is allowed to change.
fn summary_without_events(w: &World) -> String {
    let s = summary(w);
    match s.rfind(" events ") {
        Some(at) => s[..at].to_string(),
        None => s,
    }
}

/// Two living realms may not answer to the same name: "Bordering Zhi
/// (wary), Zhi (calm)" is not a thing a reader can act on.
#[test]
fn living_realms_have_distinct_short_names() {
    for seed in [4u64, 11, 42] {
        let mut w = world(seed);
        run(&mut w, 600);
        let mut seen: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        for p in w.living_polities() {
            let key = w.polities[p].short.to_lowercase();
            assert!(
                !key.trim().is_empty(),
                "seed {}: realm {} has no name",
                seed,
                p
            );
            if let Some(&other) = seen.get(&key) {
                panic!(
                    "seed {}: realms {} ({}) and {} ({}) both answer to '{}'",
                    seed, other, w.polities[other].name, p, w.polities[p].name, key
                );
            }
            seen.insert(key, p);
        }
        // And the full name must agree with the short one, or a ruler's
        // title names a realm the map has never heard of.
        for p in w.living_polities() {
            let pol = &w.polities[p];
            if pol.kind != crate::sim::PolityKind::Tribe {
                assert!(
                    pol.name.contains(&pol.short) || pol.name.contains(&pol.adj),
                    "seed {}: '{}' is not recognisably '{}'",
                    seed,
                    pol.name,
                    pol.short
                );
            }
        }
    }
}

/// A world does not call two of its centuries by the same name.
#[test]
fn era_names_are_unique_within_a_world() {
    for seed in [4u64, 11, 42] {
        let mut w = world(seed);
        run(&mut w, 1200);
        let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for e in &w.eras {
            assert!(
                seen.insert(e.name.as_str()),
                "seed {}: '{}' names two centuries",
                seed,
                e.name
            );
        }
        assert!(w.eras.len() >= 10, "seed {}: too few eras to judge", seed);
    }
}

/// Plurals every count in the chronicle should have been through: a bare
/// "1" followed by a plural noun. "31 years" is fine; "1 years" is not.
const PLURALS_OF_ONE: &[&str] = &[
    "1 years",
    "1 entries",
    "1 battles",
    "1 lands",
    "1 cities",
    "1 wars",
    "1 realms",
    "1 generals",
    "1 nobles",
    "1 schools",
    "1 times",
];

/// Whether `text` says "one" of something in digits, with the plural noun.
///
/// Only a standalone 1 counts, so "31 years" is left alone — and so is
/// "3.1 lands a year", where the 1 is the tail of a decimal and the noun is
/// correctly plural. A digit or a decimal point before the 1 means it is
/// not the number one.
fn says_one_as_a_plural(text: &str) -> Option<&'static str> {
    let bytes = text.as_bytes();
    PLURALS_OF_ONE.iter().copied().find(|&bad| {
        let mut from = 0;
        while let Some(at) = text[from..].find(bad) {
            let at = from + at;
            let joined = at > 0 && (bytes[at - 1].is_ascii_digit() || bytes[at - 1] == b'.');
            if !joined {
                return true;
            }
            from = at + 1;
        }
        false
    })
}

/// Counts in the chronicle go through `prose::count` and `prose::years`, so
/// "1 years of fighting" and "1 entries" cannot get out.
#[test]
fn no_event_says_one_of_anything_in_digits() {
    let mut w = world(23);
    run(&mut w, 400);
    assert!(w.chronicle.len() > 200, "too little history to judge");
    for e in &w.chronicle.events {
        assert!(
            says_one_as_a_plural(&e.text).is_none(),
            "year {}: {:?} in {:?}",
            e.year,
            says_one_as_a_plural(&e.text),
            e.text
        );
    }
    // The explanations are read the same way, so they are held to it too.
    for p in w.living_polities() {
        for f in crate::sim::explain::stability_factors(&w, p) {
            assert!(says_one_as_a_plural(&f.text).is_none(), "{}", f.text);
        }
        assert!(says_one_as_a_plural(crate::sim::explain::stability_drift(&w, p)).is_none());
    }
    for x in 0..w.wars.len() {
        let v = crate::sim::explain::war_view(&w, x);
        for r in v.reasons.iter().chain(std::iter::once(&v.peace)) {
            assert!(says_one_as_a_plural(r).is_none(), "{}", r);
        }
    }
    for i in 0..w.persons.len() {
        let s = crate::sim::explain::standing(&w, i);
        assert!(says_one_as_a_plural(&s).is_none(), "{}", s);
    }
}

/// A queen is not a king. A kingdom whose tongue calls its ruler an
/// emperor still crowns a queen when the ruler is a woman.
#[test]
fn honorifics_follow_gender() {
    use crate::sim::{Gender, PolityKind};
    let mut w = world(31);
    run(&mut w, 300);
    assert!(!w.living_polities().is_empty());
    for p in w.living_polities() {
        let kind = w.polities[p].kind;
        let (f, m) = (w.honorific(p, Gender::F), w.honorific(p, Gender::M));
        // No realm ever calls a woman "King" or a man "Queen".
        assert_ne!(f, "King", "realm {} titles a woman King", p);
        assert_ne!(m, "Queen", "realm {} titles a man Queen", p);
        assert_ne!(f, "Emperor");
        assert_ne!(m, "Empress");
        if kind == PolityKind::Empire {
            assert_eq!((f.as_str(), m.as_str()), ("Empress", "Emperor"));
        }
        // A kingdom is a kingdom, whatever word its language reaches for.
        if kind == PolityKind::Kingdom {
            let lang = &w.cultures[w.polities[p].culture].lang;
            if matches!(lang.honorific.as_str(), "Emperor" | "Empress") {
                assert_eq!((f.as_str(), m.as_str()), ("Queen", "King"));
            }
        }
    }
    // Every honorific in the world is one a person could be addressed by.
    for p in w.living_polities() {
        for g in [Gender::F, Gender::M, Gender::N] {
            let h = w.honorific(p, g);
            assert!(!h.is_empty());
            assert!(h.chars().next().unwrap().is_uppercase(), "{}", h);
        }
    }
}

/// A tiny world left running long enough for realms to rise, fall and run out
/// of cities altogether, and then saved and reloaded.
#[test]
fn a_cramped_world_survives_a_long_run() {
    let mut w = World::new(3, 40, 20, Detail::High);
    run(&mut w, 800);
    assert_eq!(w.year, 800);
    let bytes = ser::save(&mut w);
    let back = ser::load(&bytes).expect("a tiny world should reload");
    assert_eq!(back.year, w.year);
    assert_eq!(back.polities.len(), w.polities.len());
}

/// A dynasty is a tree, not a thread.
///
/// Succession used to consult the dead ruler's own children and nothing
/// else. Nobody but the reigning ruler married, so nobody but the reigning
/// ruler bred, so a king who died childless ended his house however many
/// brothers survived him — and the chronicle became a column of
/// "the old ruler left no clear heir". A world of twelve centuries should
/// hold at least one house that ruled across twenty reigns, and crowns
/// should pass sideways to kin far more often than they are seized by
/// strangers.
#[test]
fn a_house_outlives_its_founder_many_times_over() {
    let mut w = world(7);
    run(&mut w, 1200);
    let longest = (0..w.houses.len())
        .map(|h| w.houses[h].seniors.len())
        .max()
        .unwrap_or(0);
    assert!(
        longest >= 20,
        "the greatest house in twelve centuries managed {} rulers; \
         dynasties are threads again",
        longest
    );
    // Kin succession is not a curiosity: it should be the ordinary way a
    // throne passes when the ruler leaves no grown child.
    let kin = w
        .chronicle
        .events
        .iter()
        .filter(|e| {
            e.text.contains("went sideways")
                || e.text.contains("without issue, and the throne passed")
                || e.text.contains("took the throne of")
        })
        .count();
    let seized = w
        .chronicle
        .events
        .iter()
        .filter(|e| e.text.contains("left no clear heir"))
        .count();
    assert!(
        kin > seized,
        "{} thrones passed to kin against {} seized by strangers",
        kin,
        seized
    );
}

/// Kinship reads the family tree the way a herald would: a brother is two
/// steps, an uncle three, a first cousin four, and the words match.
#[test]
fn kinship_names_the_relation_it_measures() {
    use crate::sim::dynasty::kinship;
    let mut w = world(3);
    run(&mut w, 200);
    // Find a ruler with a sibling, by way of a shared parent.
    let mut checked = 0;
    for i in 0..w.persons.len() {
        // Children who name *this* person as their parent. A child is
        // listed by both of its parents but names only one of them, so
        // somebody who had children by two partners lists two people who
        // are not siblings at all --- which is exactly the relation this
        // test exists to measure, and exactly the way to get it wrong.
        let kids: Vec<usize> = w.persons[i]
            .children
            .iter()
            .copied()
            .filter(|&c| w.persons[c].parent == Some(i))
            .collect();
        if kids.len() < 2 {
            continue;
        }
        let k = kinship(&w, kids[0], kids[1]).expect("siblings are kin");
        assert_eq!((k.up, k.down), (1, 1), "siblings are one step each way");
        assert_eq!(k.name(crate::sim::Gender::M), "brother");
        // And a nephew, if the elder has children of their own. A child is
        // listed by both parents but names only one of them as `parent`,
        // and it is that one the blood is traced through.
        if let Some(&grand) = w.persons[kids[0]]
            .children
            .iter()
            .find(|&&g| w.persons[g].parent == Some(kids[0]))
        {
            let k = kinship(&w, kids[1], grand).expect("an uncle knows his nephew");
            assert_eq!((k.up, k.down), (1, 2));
            assert_eq!(k.name(crate::sim::Gender::F), "niece");
        }
        checked += 1;
        if checked >= 3 {
            break;
        }
    }
    assert!(
        checked > 0,
        "no siblings in two centuries of a peopled world"
    );
}

/// A house page holds together at millennia scale: the line of succession
/// is in order, every ruler on it belongs to the house, and nothing on the
/// page runs past the width it was given.
///
/// This is the test that would have caught the three things wrong with the
/// first version of the family tree — accessions dated from the wrong
/// realm's `reign_start` and so printed out of order, elected rulers
/// enrolled in houses they were not born to, and non-ruling kin who never
/// died and so appeared as thousand-year-old children.
#[test]
fn a_house_page_reads_down_the_centuries() {
    let mut w = world(7);
    run(&mut w, 1200);
    let big = (0..w.houses.len())
        .max_by_key(|&h| w.houses[h].seniors.len())
        .expect("a world of twelve centuries has ruling houses in it");
    assert!(
        w.houses[big].seniors.len() >= 5,
        "no house ever managed five rulers, so this proves nothing"
    );
    for h in 0..w.houses.len() {
        let ho = &w.houses[h];
        // Everyone who ruled for the house is of the house.
        for &r in &ho.seniors {
            assert!(
                ho.members.contains(&r),
                "{} ruled for {} without belonging to it",
                w.persons[r].name,
                ho.name
            );
            assert_eq!(
                w.persons[r].house,
                Some(h),
                "{} is on two houses' lists",
                w.persons[r].name
            );
        }
        // The line runs forwards in time.
        let mut years: Vec<i32> = ho
            .seniors
            .iter()
            .map(|&r| w.persons[r].crowned.unwrap_or(w.persons[r].born))
            .collect();
        let sorted = {
            let mut v = years.clone();
            v.sort_unstable();
            v
        };
        years.sort_unstable();
        assert_eq!(years, sorted);
        // Nobody outlives their own people by a wide margin.
        for &m in &ho.members {
            let per = &w.persons[m];
            let span = w.races[per.race].lifespan;
            assert!(
                per.age(w.year) as f32 <= span * 3.0,
                "{} is {} years old and their people live about {:.0}",
                per.name,
                per.age(w.year),
                span
            );
        }
    }
    // The page itself renders, and fits.
    for width in [60usize, 96, 200] {
        let ls = crate::ui::detail::detail_lines(
            &w,
            crate::sim::chronicle::Ref::House(big),
            width,
            false,
        );
        assert!(ls.len() > 8, "the page came out empty");
        for l in &ls {
            assert!(
                l.text.chars().count() <= width,
                "a line ran past {} columns: {:?}",
                width,
                l.text
            );
        }
    }
}

/// Every landmass worth settling can be reached from the largest one by a
/// chain of sea crossings, so a realm that masters the sea can in principle
/// reach the whole world.
///
/// This is the property the sea exists for. Without it a fifth of some
/// worlds sat two cells of water away and no army could ever go there.
#[test]
fn the_sea_joins_the_world() {
    for seed in [7u64, 1, 42, 99, 3, 128] {
        let w = World::new(seed, 160, 64, Detail::Medium);
        let t = &w.terrain;
        // Landmass sizes, and which are worth reaching at all.
        let mut size: std::collections::BTreeMap<u16, usize> = Default::default();
        for i in 0..t.w * t.h {
            if t.landmass[i] != 0 {
                *size.entry(t.landmass[i]).or_insert(0) += 1;
            }
        }
        let biggest = *size.iter().max_by_key(|(_, &s)| s).expect("land").0;
        // Flood the landmass graph through crossings.
        let mut seen: std::collections::BTreeSet<u16> = Default::default();
        seen.insert(biggest);
        let mut grew = true;
        while grew {
            grew = false;
            for c in &t.crossings {
                let (a, b) = (t.landmass[c.from as usize], t.landmass[c.to as usize]);
                if seen.contains(&a) && !seen.contains(&b) {
                    seen.insert(b);
                    grew = true;
                }
            }
        }
        let stranded: Vec<(u16, usize)> = size
            .iter()
            .filter(|(lm, &s)| s >= 20 && !seen.contains(lm))
            .map(|(&lm, &s)| (lm, s))
            .collect();
        let land: usize = size.values().sum();
        let reachable: usize = size
            .iter()
            .filter(|(lm, _)| seen.contains(lm))
            .map(|(_, &s)| s)
            .sum();
        assert!(
            stranded.is_empty(),
            "seed {}: landmasses of {:?} cells cannot be reached by sea at all \
             ({:.0}% of land reachable)",
            seed,
            stranded.iter().map(|&(_, s)| s).collect::<Vec<_>>(),
            reachable as f32 / land as f32 * 100.0
        );
        assert!(
            !t.crossings.is_empty(),
            "seed {} has no sea crossings at all",
            seed
        );
    }
}

/// Realms actually cross the water: they settle across it and they fight
/// across it. Both roads have to work, because a crossing whose far shore is
/// empty is a colony and one whose far shore is somebody else's is a war,
/// and a world only reaches its islands if it can do both.
#[test]
fn the_sea_is_crossed_both_ways() {
    let mut colonies = 0;
    let mut landings = 0;
    let mut straddled = 0;
    for seed in [7u64, 1, 42, 99] {
        let mut w = World::new(seed, 160, 64, Detail::Medium);
        run(&mut w, 1400);
        landings += w
            .chronicle
            .events
            .iter()
            .filter(|e| e.text.contains("come by sea"))
            .count();
        for p in 0..w.polities.len() {
            let mut on: Vec<u16> = w
                .cells_of_ref(p)
                .iter()
                .map(|&i| w.terrain.landmass[i])
                .filter(|&l| l != 0)
                .collect();
            on.sort_unstable();
            on.dedup();
            if on.len() > 1 {
                straddled += 1;
            }
        }
        // Somebody peopled an island their ancestors could not have walked
        // to: a landmass other than the biggest that carries population.
        let mut lm: std::collections::BTreeMap<u16, (usize, f32)> = Default::default();
        for i in 0..w.terrain.w * w.terrain.h {
            let l = w.terrain.landmass[i];
            if l == 0 {
                continue;
            }
            let e = lm.entry(l).or_insert((0, 0.0));
            e.0 += 1;
            e.1 += w.cells[i].pop;
        }
        let biggest = *lm.iter().max_by_key(|(_, &(s, _))| s).expect("land").0;
        colonies += lm
            .iter()
            .filter(|(&l, &(s, pop))| l != biggest && s >= 8 && pop > 0.0)
            .count();
    }
    assert!(
        landings > 0,
        "no army in four worlds ever landed on a hostile shore"
    );
    assert!(
        straddled > 0,
        "no realm in four worlds ever held land on two landmasses"
    );
    assert!(
        colonies > 0,
        "no island in four worlds was ever peopled across water"
    );
}

/// Blood behaves like blood: variation survives the generations, a strain
/// can hide and resurface, and marrying close costs a line its vigour.
///
/// The model it replaced blended a single parent's value toward the middle,
/// so four generations after a remarkable ruler the line was unremarkable
/// and nothing could ever skip a generation.
#[test]
fn a_line_keeps_its_blood() {
    use crate::sim::blood;
    let mut w = world(19);
    run(&mut w, 900);

    // Variation has not collapsed toward the middle.
    let born_here: Vec<usize> = (0..w.persons.len())
        .filter(|&i| w.persons[i].parent.is_some())
        .collect();
    assert!(born_here.len() > 40, "too few children to judge");
    let spread = born_here
        .iter()
        .filter(|&&i| {
            let t = w.persons[i].traits;
            t.ambition > 0.75 || t.ambition < 0.25 || t.cruelty > 0.75 || t.wisdom > 0.75
        })
        .count();
    assert!(
        spread * 8 >= born_here.len(),
        "only {} of {} children were remarkable in any trait: the line has flattened",
        spread,
        born_here.len()
    );

    // Somebody carries something they do not show.
    let carriers = (0..w.persons.len())
        .filter(|&i| !blood::carried(&w.persons[i].genes).is_empty())
        .count();
    assert!(carriers > 0, "no unexpressed strain exists anywhere");

    // Crossing two carriers of the same rare allele can double it.
    let rng = crate::rng::Rng::new(5);
    let mut a = [128u8; blood::GENES];
    let mut b = [128u8; blood::GENES];
    a[0] = 250;
    b[0] = 250;
    let mut doubled = 0;
    for _ in 0..200 {
        if blood::surfaced(&blood::cross(&a, &b, &rng)) == Some(0) {
            doubled += 1;
        }
    }
    assert!(
        doubled > 20,
        "two carriers produced a doubled recessive only {} times in 200",
        doubled
    );

    // And a child of close kin is the weaker for it.
    let close: Vec<usize> = (0..w.persons.len())
        .filter(|&i| w.persons[i].inbred > 0.3)
        .collect();
    for &i in close.iter().take(20) {
        assert!(
            w.persons[i].vigour < 1.0,
            "a child of close kin paid nothing for it"
        );
    }
}

/// Trade does what trade is for: goods sit where the ground puts them,
/// routes join places that want what each other has, position on the map is
/// worth money, and closing a road is felt at both ends.
#[test]
fn trade_makes_position_worth_something() {
    let mut w = world(31);
    run(&mut w, 700);

    // The ground produces, and produces different things in different places.
    let kinds: std::collections::BTreeSet<&str> =
        w.goods.iter().flatten().map(|g| g.name()).collect();
    assert!(
        kinds.len() >= 6,
        "only {} kinds of good exist in the whole world",
        kinds.len()
    );

    // Routes exist, and some go by sea.
    assert!(!w.routes.is_empty(), "no city trades with any other");
    assert!(
        w.routes.iter().any(|r| r.by_sea),
        "nothing is carried by water"
    );

    // A route joins cities whose hinterlands differ: that is the whole
    // reason for one to exist.
    for r in w.routes.iter().take(30) {
        let a: std::collections::BTreeSet<&str> =
            w.city_goods(r.a).iter().map(|g| g.name()).collect();
        let b: std::collections::BTreeSet<&str> =
            w.city_goods(r.b).iter().map(|g| g.name()).collect();
        assert!(
            a.difference(&b).next().is_some() && b.difference(&a).next().is_some(),
            "{} and {} trade but want nothing from each other",
            w.cities[r.a].name,
            w.cities[r.b].name
        );
    }

    // Wealth is positional: some cities take far more than the median.
    //
    // Measured across several worlds, because how lopsided any one world's
    // geography is depends on that world's geography — a single seed that
    // happens to lay its goods out evenly proves nothing either way.
    let mut ratios: Vec<f32> = Vec::new();
    for seed in [31u64, 7, 42] {
        let mut v = world(seed);
        run(&mut v, 700);
        let mut takings: Vec<f32> = v
            .cities
            .iter()
            .filter(|c| c.destroyed.is_none())
            .map(|c| v.city_trade(c.id))
            .collect();
        takings.sort_by(f32::total_cmp);
        let median = takings[takings.len() / 2].max(0.1);
        ratios.push(takings.last().copied().unwrap_or(0.0) / median);
    }
    assert!(
        ratios.iter().all(|&r| r > 1.5) && ratios.iter().any(|&r| r > 2.0),
        "the busiest city barely beats the median anywhere ({:?}): position is worth nothing",
        ratios
    );

    // A war between two realms closes the roads between them.
    let warring = w
        .wars
        .iter()
        .find(|x| x.alive())
        .map(|x| (x.attacker, x.defender));
    if let Some((a, b)) = warring {
        assert_eq!(
            w.trade_between(a, b),
            0.0,
            "two realms at war are still trading with each other"
        );
    }
}

/// The weather moves, and it moves the land's capacity with it.
///
/// Terrain is immutable, which is right for elevation and wrong for
/// rainfall: without this the steppe is as dry in year nine thousand as in
/// year one and no region ever has a bad century.
#[test]
fn the_weather_turns_over_centuries() {
    let mut w = world(23);
    run(&mut w, 40);
    let early: Vec<f32> = w.climate.at.clone();
    assert!(!early.is_empty(), "the world has no weather at all");
    assert!(
        early.iter().any(|&v| v.abs() > 0.15),
        "the weather is flat everywhere"
    );
    run(&mut w, 900);
    let late = &w.climate.at;
    // Somewhere has genuinely changed.
    let moved = early
        .iter()
        .zip(late.iter())
        .filter(|(a, b)| (*a - *b).abs() > 0.3)
        .count();
    assert!(
        moved > early.len() / 50,
        "only {} cells of {} saw their weather change in nine centuries",
        moved,
        early.len()
    );
    // And it is the same weather a reloaded world gets back, including
    // partway through an epoch.
    for extra in [0usize, 3, 5] {
        let mut a = world(23);
        run(&mut a, 200 + extra as i32);
        let bytes = crate::ser::save(&mut a);
        let b = crate::ser::load(&bytes).expect("loads");
        assert_eq!(
            a.climate.at, b.climate.at,
            "the weather did not survive a save taken {} years into an epoch",
            extra
        );
    }
}

// ---------------------------------------------------------------------------
// Probes
// ---------------------------------------------------------------------------
//
// The `#[ignore]` tests below are instruments, not checks: they print and
// assert nothing, and `cargo test` skips them. They are kept because every
// scaling fault this game has had was found by one of them and none was
// found by reading the code — the schools that compounded, the martyrs that
// piled up, the treasury with no drain, the prosperity that pinned at its
// ceiling, and the brake that covered only one of two doors. A checked
// invariant tells you when something has broken; these tell you what shape
// it broke in, which is the part that takes the time.
//
// Run one with, for example:
//
//     C=100 cargo test --release time_each_century -- --ignored --nocapture

/// Print the cost of a year, century by century, with the counts that
/// explain it. Not a check — a probe, for when the game feels slow.
///
/// `W`, `H`, `C` and `DETAIL` set the map and how far to run:
/// `W=400 H=160 C=50 cargo test --release time_each_century -- --ignored --nocapture`
///
/// Living counts are printed beside the ever-lived ones on purpose. Almost
/// every slowdown this game has had was one of two things — a loop walking
/// the dead, or a population that was supposed to reach an equilibrium and
/// did not — and the two are told apart at a glance by whether the left-hand
/// number is climbing with the right.
#[test]
#[ignore]
fn time_each_century() {
    let detail = match std::env::var("DETAIL").as_deref() {
        Ok("high") => Detail::High,
        Ok("low") => Detail::Low,
        _ => Detail::Medium,
    };
    let env = |k: &str, d: usize| -> usize {
        std::env::var(k)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(d)
    };
    let mut w = World::new(7, env("W", 288), env("H", 144), detail);
    let mut worst = 0.0f64;
    for century in 0..env("C", 80) {
        let t = std::time::Instant::now();
        for _ in 0..100 {
            w.tick();
        }
        let ms = t.elapsed().as_secs_f64() * 1000.0 / 100.0;
        worst = worst.max(ms);
        if century % 5 == 0 || ms > 2.0 {
            println!(
                "year {:>5}: {:>7.3} ms/year | persons {:>5}/{:<8} realms {:>4}/{:<6} \
                 schools {:>4}/{:<5} wars {:>4}/{:<6} events {:>6} cities {:>4}",
                (century + 1) * 100,
                ms,
                w.alive_persons.len(),
                w.persons.len(),
                w.alive_polities.len(),
                w.polities.len(),
                w.schools.iter().filter(|s| s.alive()).count(),
                w.schools.len(),
                w.alive_wars.len(),
                w.wars.len(),
                w.chronicle.len(),
                w.cities.len(),
            );
        }
    }
    println!("worst century: {:.3} ms/year", worst);
}

/// The stability breakdown must name the number the realm is drifting
/// towards.
///
/// `explain::stability_factors` mirrors the target that `politics::economy`
/// computes each year, and the module says so at the top of the file — but
/// saying so is not a guarantee, and the mirror had already drifted. Its
/// treasury term was a second copy of a ratio against a ceiling of six
/// hundred. When the ceiling was removed the simulation moved to a
/// saturating curve and the copy went on dividing, so a realm holding three
/// thousand was told its full treasury was worth a hundred and four points
/// of stability — five times more than anything in the list can be worth,
/// and printed at the top of the page as the reason for everything.
///
/// A drift like that is invisible in the simulation and plain in the
/// explanation, which is the argument for checking the explanation against
/// the simulation rather than against itself.
///
/// Two demands, because stability has many writers and income has few. A
/// rebellion, a persecution, a rival's knife and a dozen other things move
/// it directly, so a single year cannot be held to the target exactly — the
/// *typical* year can, which is what the median is for. And no single pull
/// on a number that lives between nought and one may be worth more than a
/// good fraction of that range, which is the bound the broken term failed
/// by a factor of two and which needs no statistics at all.
#[test]
fn the_stability_breakdown_names_the_right_target() {
    use crate::sim::explain;
    let mut w = world(43);
    run(&mut w, 200);
    // A realm richer than the old ceiling, because that is the regime the
    // drift showed up in and a young world never reaches it on its own. With
    // the broken term this one line is enough: three thousand against a
    // divisor of six hundred is a pull worth one whole point of stability.
    {
        let rich = *w
            .alive_polities
            .first()
            .expect("a 200 year world has realms");
        for t in [3_000.0f32, 50_000.0] {
            w.polities[rich].treasury = t;
            for f in explain::stability_factors(&w, rich) {
                assert!(
                    f.weight.abs() <= 0.6,
                    "a realm holding {:.0} is told {:?} is worth {:.2} of a \
                     stability range of 1.0",
                    t,
                    f.text,
                    f.weight
                );
            }
        }
        w.polities[rich].treasury = 0.0;
    }

    let mut errors: Vec<f32> = Vec::new();
    let mut widest = (0.0f32, String::new());
    for _ in 0..80 {
        let before: Vec<(usize, f32, f32)> = w
            .alive_polities
            .iter()
            .copied()
            .map(|p| (p, w.polities[p].stability, explain::stability_target(&w, p)))
            .collect();
        // No pull on stability may be worth more than a good fraction of the
        // range stability lives in. The broken treasury term was worth 104
        // points out of 100.
        for &(p, _, _) in &before {
            for f in explain::stability_factors(&w, p) {
                if f.weight.abs() > widest.0 {
                    widest = (f.weight.abs(), f.text.clone());
                }
            }
        }
        w.tick();
        for (p, was, target) in before {
            if !w.polities[p].alive() {
                continue;
            }
            let now = w.polities[p].stability;
            // Clamped at both ends, and jostled by a small random nudge each
            // year, so a year spent against either limit says nothing.
            if !(0.02..=0.98).contains(&now) || !(0.02..=0.98).contains(&was) {
                continue;
            }
            let want = (target - was) * w.tuning.stability_adjust_rate;
            errors.push((now - was - want).abs());
        }
    }
    assert!(
        widest.0 <= 0.6,
        "a single pull on stability is worth {:.2} of a range of 1.0: {:?}",
        widest.0,
        widest.1
    );
    assert!(
        errors.len() > 300,
        "only {} realm-years were checked",
        errors.len()
    );
    errors.sort_by(f32::total_cmp);
    let median = errors[errors.len() / 2];
    // The yearly nudge alone is worth 0.02, so this is the nudge and little
    // else.
    assert!(
        median <= 0.022,
        "the typical year missed the breakdown's target by {:.4}",
        median
    );
}

/// Trade must keep its place in the economy as the economy grows.
///
/// A route's value came from the goods on the ground and the distance
/// between the two cities — both fixed for ever — while city income grows
/// with population, prosperity and development, all of which compound. So
/// the whole trade of a world was a constant in a growing economy, and
/// trade's share of what realms earned fell from a quarter in the second
/// century to a fortieth by the twenty-eighth, purely by standing still.
///
/// Two things fix it and this holds both to account: the mass term of a
/// gravity model, so a road between two great cities carries more than one
/// between two villages, and a kind of innovation that improves the
/// carrying trade — which had no place among the twelve effects a
/// generated tree could hand out, so a realm could invent the ocean-going
/// ship and its caravans carried what they carried in year one.
#[test]
fn trade_keeps_its_place_as_the_world_grows() {
    use crate::sim::explain;
    let share = |w: &World| -> f32 {
        let mut gross = 0.0f32;
        let mut tolls = 0.0f32;
        for &p in &w.alive_polities {
            gross += explain::income_factors(w, p)
                .iter()
                .filter(|f| f.weight > 0.0)
                .map(|f| f.weight)
                .sum::<f32>();
            tolls += w.realm_trade(p) * w.tuning.trade_toll_factor;
        }
        tolls / gross.max(0.001)
    };
    let mut w = World::new(53, 160, 64, Detail::Medium);
    run(&mut w, 500);
    let young = share(&w);
    run(&mut w, 1200);
    let old = share(&w);
    assert!(
        young > 0.005,
        "trade was never worth anything ({:.4})",
        young
    );
    assert!(
        old > young * 0.5,
        "trade fell from {:.2}% of what realms earn to {:.2}% as the world \
         grew: a route's value is not keeping up with the economy",
        young * 100.0,
        old * 100.0
    );
}

/// A world can learn to carry goods better.
///
/// The twelve kinds of effect a generated tree could hand out were settled
/// before trade grew into anything, and none of them touched a road. This
/// checks the whole path: that a tree offers commerce at all, that the
/// multiplier it produces is bounded, and that realms in a played-out world
/// have actually picked some up.
#[test]
fn commerce_is_something_a_world_can_learn() {
    use crate::sim::tech::Effect;
    let mut w = World::new(59, 160, 64, Detail::Medium);
    // The tree must contain some, or the path is decorative.
    let offered = w
        .techs
        .iter()
        .filter(|t| matches!(t.effect, Effect::Commerce(_)))
        .count();
    assert!(
        offered > 0,
        "a tree of {} innovations offers no way to improve the carrying trade",
        w.techs.len()
    );
    // Bounded, and never below where it starts: an unknown art cannot make
    // a realm's roads worse.
    for &p in &w.alive_polities {
        let m = w.tech_trade_mult(p);
        assert!((1.0..3.0).contains(&m), "trade multiplier of {}", m);
    }
    run(&mut w, 900);
    let best = w
        .alive_polities
        .iter()
        .map(|&p| w.tech_trade_mult(p))
        .fold(1.0f32, f32::max);
    assert!(
        best > 1.0,
        "after nine centuries no realm in the world had learned anything \
         about carrying goods"
    );
}

/// A treasury must settle at what a realm earns, not climb with the years.
///
/// Gold used to be pinned by a ceiling of six hundred, and the ceiling
/// destroyed every fact about wealth above it: eight of the ten richest
/// realms in a world sat at exactly the cap, drawing the identical bonus to
/// their stability. What the cap was standing in for is a drain — a
/// treasury's only cost was upkeep, which scales with the army and the land
/// and not at all with how full the coffers are, so gold was an accumulator
/// with no matching outflow.
///
/// Three drains replace it, all proportional to wealth: a court that spends
/// what it has, corruption that skims the tax roll, and soldiers bought with
/// gold who then cost upkeep. The invariant they buy is the one that matters
/// — the same one the schools and the martyrs needed — that the quantity
/// settles where its inflow and outflow meet rather than growing with the
/// length of the game.
///
/// So this asserts the settling point: a treasury near what the court's
/// share implies, and not a figure that depends on how long the world has
/// run. The second half of the test switches the drains off and demands that
/// the bound *fails*, because a bound nothing can breach proves nothing —
/// and without them a world reaches a million and a half.
#[test]
fn a_treasury_settles_at_what_a_realm_earns() {
    use crate::sim::explain;
    let bound = |w: &World| -> (f32, f32, usize) {
        // Where the court's share puts the equilibrium: the war chest, plus
        // what a realm earns divided by the fraction spent above it. Six
        // times over is generous for the richest realm in a world.
        let share = w.tuning.court_spend_share.max(0.0001);
        let mut worst = (0.0f32, 0.0f32, 0usize);
        for &p in &w.alive_polities {
            let t = w.polities[p].treasury;
            let gross: f32 = explain::income_factors(w, p)
                .iter()
                .filter(|f| f.weight > 0.0)
                .map(|f| f.weight)
                .sum();
            let ceiling = w.tuning.court_reserve + gross / share * 6.0;
            if t / ceiling.max(1.0) > worst.0 / worst.1.max(1.0) {
                worst = (t, ceiling, p);
            }
        }
        worst
    };

    let mut w = world(37);
    run(&mut w, 1500);
    let (held, ceiling, p) = bound(&w);
    assert!(
        held <= ceiling,
        "realm {} holds {:.0} against a settling point of {:.0}",
        p,
        held,
        ceiling
    );
    // And that the bound is being tested against a world with money in it.
    // The realm above is the one furthest past its own settling point,
    // which on a small map can be a poor realm with a low one, so the
    // richest is asked separately.
    let richest_tuned = w
        .alive_polities
        .iter()
        .map(|&q| w.polities[q].treasury)
        .fold(0.0f32, f32::max);
    assert!(
        richest_tuned > w.tuning.court_reserve,
        "no realm in a 1500 year world got past its war chest ({:.0} held)",
        richest_tuned
    );

    // And the same world with nothing draining it must break that bound,
    // because otherwise the bound is not measuring the drains.
    let mut loose = world(37);
    loose.tuning.court_spend_share = 0.0;
    loose.tuning.corruption_decadence_weight = 0.0;
    loose.tuning.corruption_sprawl_weight = 0.0;
    loose.tuning.army_gold_weight = 0.0;
    run(&mut loose, 1500);
    let richest = loose
        .alive_polities
        .iter()
        .map(|&p| loose.polities[p].treasury)
        .fold(0.0f32, f32::max);
    // Compared against the tuned world's richest, since the loose world has
    // no share to divide by and so no settling point of its own.
    let tuned_ceiling = (richest_tuned * 3.0).max(w.tuning.court_reserve * 4.0);
    assert!(
        richest > tuned_ceiling,
        "with no drains the richest realm held only {:.0}, under {:.0} — the \
         bound above is not measuring anything",
        richest,
        tuned_ceiling
    );
}

/// Wealth rots a court, and the rot takes its cut of the next tax roll.
///
/// The point of spending gold on splendour is that it is not free: it buys
/// prestige and it buys decadence, and decadence is worth a great deal of
/// stability and a share of the revenue. Before this, decadence grew from a
/// realm's *age* and its overextension and never from its riches, so an
/// empire at the height of its wealth rotted at exactly the same rate as one
/// scraping by.
#[test]
fn a_rich_court_rots_and_skims() {
    use crate::sim::{corruption_share, court_spending};
    let w = world(41);
    let tn = &w.tuning;

    // A court spends only what is above the war chest, and more of it the
    // richer it is.
    assert_eq!(court_spending(tn, tn.court_reserve - 1.0), 0.0);
    assert_eq!(court_spending(tn, 0.0), 0.0);
    let modest = court_spending(tn, tn.court_reserve + 100.0);
    let grand = court_spending(tn, tn.court_reserve + 1000.0);
    assert!(modest > 0.0 && grand > modest * 5.0);

    // Rot and distance both take a cut, and together they are bounded: a
    // realm that collected nothing at all would simply dissolve.
    assert_eq!(corruption_share(tn, 0.0, 1.0), 0.0);
    assert!(corruption_share(tn, 1.0, 1.0) > corruption_share(tn, 0.3, 1.0));
    assert!(corruption_share(tn, 0.0, 2.0) > corruption_share(tn, 0.0, 1.0));
    assert!(corruption_share(tn, 1.0, 9.0) <= 0.75);

    // And a realm really does rot as it spends. Compared within one world
    // rather than between two, because two worlds given different rules
    // diverge on the first die roll and are then not comparable at all: the
    // first attempt at this ran a spendthrift world against a frugal one and
    // found the frugal one *more* decadent, for the good reason that a court
    // which hoards its gold buys an army, conquers more than it can govern
    // and rots from overextension instead.
    //
    // So one world is forked through a save file, which is the only way to
    // get two identical copies of it, and each fork runs a single year.
    let mut seed_world = world(41);
    run(&mut seed_world, 300);
    let bytes = ser::save(&mut seed_world);
    let rich = *seed_world
        .alive_polities
        .first()
        .expect("a 300 year world has a realm");

    let rot = |spend: f32| -> f32 {
        let mut w = ser::load(&bytes).expect("a fresh save must load");
        w.tuning.court_spend_share = spend;
        // Given far more than any war chest needs, so the court has
        // something to be extravagant with.
        w.polities[rich].treasury = w.tuning.court_reserve + 2000.0;
        let before = w.polities[rich].decadence;
        w.tick();
        w.polities[rich].decadence - before
    };
    let spending = rot(0.25);
    let frugal = rot(0.0);
    assert!(
        spending > frugal,
        "a court given two thousand to spend rotted by {:.5}, and one \
         forbidden to spend it by {:.5}",
        spending,
        frugal
    );
}

/// Gold buys confidence on a curve, not against a ceiling.
#[test]
fn wealth_saturates_rather_than_capping() {
    use crate::sim::{treasury_confidence, wealth_reach};
    let w = world(3);
    let tn = &w.tuning;
    // Monotonic, and it never reaches one however much is hoarded — so
    // there is a gradient all the way up and no cliff at the top. The ratio
    // this replaced gave every wealthy realm the identical bonus.
    let mut last = treasury_confidence(tn, 0.0);
    for t in [50.0f32, 200.0, 600.0, 3000.0, 100_000.0] {
        let v = treasury_confidence(tn, t);
        assert!(v > last, "confidence fell from {} to {} at {}", last, v, t);
        assert!(v < 1.0);
        last = v;
    }
    // Debt is worth something on its own account, and bounded.
    assert!(treasury_confidence(tn, -10.0) < 0.0);
    assert!(treasury_confidence(tn, -1.0e9) >= -0.2);
    // And what a hoard can buy is bounded too, or a rich realm fields an
    // army its people could not feed.
    assert!(wealth_reach(tn, 0.0) == 0.0);
    assert!(wealth_reach(tn, 1.0e9) < 1.0);
    assert!(wealth_reach(tn, tn.court_reserve) > 0.4);
}

/// The prosperity breakdown must name where a city is actually heading.
///
/// Prosperity became the most interesting number in the game when trade fed
/// it and a city's income came to depend on it, and it was the only such
/// number with no explanation. The weights mirror `politics::economy`, so
/// the same rule applies as for income: a duplicate with nothing holding it
/// to account drifts.
///
/// Checked through the movement rather than the value, because prosperity
/// is a stock that eases towards its target at a tuned rate: one year's
/// step must be that rate times the distance the breakdown says remains.
#[test]
fn the_prosperity_breakdown_names_the_right_target() {
    use crate::sim::explain;
    let mut w = world(59);
    run(&mut w, 200);
    let mut errors: Vec<f32> = Vec::new();
    for _ in 0..60 {
        let before: Vec<(usize, f32, f32)> = (0..w.cities.len())
            .filter(|&c| w.cities[c].destroyed.is_none())
            .map(|c| (c, w.cities[c].prosperity, explain::prosperity_target(&w, c)))
            .collect();
        w.tick();
        for (c, was, target) in before {
            if w.cities[c].destroyed.is_some() {
                continue;
            }
            let now = w.cities[c].prosperity;
            // Clamped at both ends, and the Hand of Fate and the wonder
            // builder both add to it directly, so a year against a limit
            // says nothing about the arithmetic.
            if !(0.06..=2.49).contains(&now) || !(0.06..=2.49).contains(&was) {
                continue;
            }
            let want = (target - was) * w.tuning.prosperity_adjust_rate;
            errors.push((now - was - want).abs());
        }
    }
    assert!(
        errors.len() > 400,
        "only {} city-years checked",
        errors.len()
    );
    errors.sort_by(f32::total_cmp);
    let median = errors[errors.len() / 2];
    // A city's people and its realm's development move within the tick as
    // well, so the typical year is close rather than exact — and a missing
    // term is nowhere near this.
    assert!(
        median <= 0.004,
        "the typical city missed its breakdown's target by {:.5}",
        median
    );
}

/// A realm's books at the start of a year, for [`the_income_breakdown_adds_up`].
struct Books {
    realm: usize,
    treasury: f32,
    /// Wonders standing in its cities: paying for one is not a cost of
    /// government.
    wonders: usize,
    /// How many towns it holds, and how many wars it has ever declared.
    /// Either changing means the tax base moved under it mid-tick.
    cities: usize,
    wars: usize,
    /// What the breakdown says the year will bring, and what passes through
    /// the treasury to bring it.
    forecast: f32,
    gross: f32,
}

/// The income breakdown must add up to what the treasury actually gains.
///
/// `explain` duplicates the arithmetic in `politics::economy` on purpose —
/// an explanation has to name its parts, and the simulation only wants the
/// sum — but a duplicate with nothing holding it to account drifts, and a
/// breakdown that disagrees with the number beside it is worse than no
/// breakdown at all.
///
/// So this reads the treasury before and after a tick and demands the
/// difference. Only realms where nothing *else* touched the money are
/// compared: tribute, sacking, peace terms and paying for a wonder are all
/// treasury writers, and none of them are income.
///
/// Writing it cost three changes to the simulation, because three inputs to
/// the income arithmetic were read at one moment and used at another, and an
/// explanation can only read the world as it stands:
///
/// * A city was taxed on the prosperity the same loop was about to give it,
///   so the crown's income depended on an adjustment made in the same
///   breath. It is taxed on the year's opening prosperity now.
/// * Towns grew in the first phase and were taxed in the fifth, so a
///   booming city paid on people who had arrived that same year. Growth is
///   its own phase now, and it runs after the assessment.
/// * And trade stays *after* the economy rather than before it, which looks
///   like the wrong way round and is not: a toll roll is assessed in
///   arrears. Moving it earlier was tried and made this uncheckable, since
///   a forecast cannot know a toll roll that has not been counted yet.
///
/// With those, every input is a year's opening value and the parts add up to
/// the whole. What tolerance is left is float arithmetic and the order of a
/// summation — which is the point: a term left out, a weight wrong or a cost
/// counted as income is off by tens of percent and cannot hide in it.
#[test]
fn the_income_breakdown_adds_up() {
    use crate::sim::explain;
    let mut w = world(7);
    run(&mut w, 150);
    let mut checked = 0;
    for _ in 0..200 {
        let before: Vec<Books> = w
            .alive_polities
            .iter()
            .copied()
            // Not at war (sacking, peace terms and loot), paying no
            // tribute and — the one that took a while to find — collecting
            // none either. An overlord's treasury grows by its tributaries'
            // taxes, which is somebody else's income.
            .filter(|&p| {
                !w.polities[p].at_war()
                    && w.polities[p].overlord.is_none()
                    && !w
                        .alive_polities
                        .iter()
                        .any(|&q| w.polities[q].overlord == Some(p))
            })
            .map(|p| {
                let wonders: usize = w.polities[p]
                    .cities
                    .iter()
                    .map(|&c| w.cities[c].wonders.len())
                    .sum();
                // The tolerance scales with what passes through the
                // treasury, not with what is left in it: a realm whose
                // income nearly cancels its upkeep has a net near zero, and
                // a percent of its gross is a large share of that.
                let f = explain::income_factors(&w, p);
                let net: f32 = f.iter().map(|x| x.weight).sum();
                let gross: f32 = f.iter().map(|x| x.weight.abs()).sum();
                Books {
                    realm: p,
                    treasury: w.polities[p].treasury,
                    wonders,
                    cities: w.polities[p].cities.len(),
                    wars: w.polities[p].wars.len(),
                    forecast: net,
                    gross,
                }
            })
            .collect();
        w.tick();
        for b in before {
            let (p, was, predicted, gross) = (b.realm, b.treasury, b.forecast, b.gross);
            let (wonders_before, cities_before, wars_before) = (b.wonders, b.cities, b.wars);
            if !w.polities[p].alive() || w.polities[p].at_war() {
                continue;
            }
            let wonders: usize = w.polities[p]
                .cities
                .iter()
                .map(|&c| w.cities[c].wonders.len())
                .sum();
            if wonders != wonders_before {
                continue;
            }
            // Nor a realm whose tax base moved under it. `recompute` rebuilds
            // a realm's city list from who owns the ground, so a town gained
            // or lost this year is taxed on one side of the comparison and
            // not the other.
            if w.polities[p].cities.len() != cities_before {
                continue;
            }
            // Nor one that fought a whole war inside the tick. A realm's war
            // list only grows, so a change of length means a declaration —
            // and a war declared in the fifth phase can be lost, looted and
            // settled by the eighth, which leaves `at_war` false at both
            // ends of the comparison and somebody else's gold in the
            // treasury.
            if w.polities[p].wars.len() != wars_before {
                continue;
            }
            let now = w.polities[p].treasury;
            // The treasury is clamped at both ends, and a clamped value
            // tells us nothing about the arithmetic that produced it.
            if !(-59.9..=599.9).contains(&now) {
                continue;
            }
            let got = now - was;
            assert!(
                (got - predicted).abs() <= 0.05 + gross * 0.04,
                "realm {} in year {}: treasury moved {:.4} but the breakdown \
                 totals {:.4} (gross {:.4})",
                p,
                w.year,
                got,
                predicted,
                gross
            );
            checked += 1;
        }
    }
    assert!(
        checked > 200,
        "only {} realm-years were clean enough to check",
        checked
    );
}

/// No living population may grow without bound.
///
/// Every population here is a queue: members arrive, members leave, and by
/// Little's law the number alive at any moment is the arrival rate times the
/// mean lifetime. The vectors behind them are append-only, so their *length*
/// is meant to grow with the length of history — but the number alive has to
/// settle, because what the tick does each year is proportional to that and
/// not to the length of the log.
///
/// The schools were neither. Arrivals scaled with the population, because
/// every school rolled the same yearly chance to schism — so this was not a
/// queue at all but a birth process, and the count grew by a fixed fraction
/// per century for ever. This is the half of that bug a short test can
/// catch: turn the schism chance up and see whether the count still answers
/// to the size of the world that carries it.
///
/// The other half — a class of member with no way *out* — accumulates far
/// too slowly to show here, and is guarded by
/// [`a_school_with_nowhere_to_go_is_forgotten`] below and by the census in
/// `time_each_century`.
#[test]
fn living_schools_answer_to_the_size_of_the_world() {
    // A tradition can start two ways — a schism from an existing one, or a
    // teaching founded where the ley runs strong — and the brake has to
    // cover both. It first covered only schism, which held while the world
    // had a few hundred cities and failed the moment it had fifteen
    // hundred: schools climbed to six hundred against two hundred and
    // seventy realms, the martyrs came back with them, and a large map went
    // from three milliseconds a year to eighteen. So each door is turned up
    // in turn, and then both together.
    for (schism, founding) in [(0.9, 1.0), (0.006, 4000.0), (0.9, 4000.0)] {
        let mut w = World::new(19, 200, 80, Detail::Medium);
        w.tuning.school_schism_chance = schism;
        w.tuning.school_schism_min_age = 5;
        w.tuning.school_schism_min_adherents = 2;
        w.tuning.school_found_mana_rate *= founding;
        w.tuning.school_found_open_rate *= founding;
        run(&mut w, 1400);
        let living = w.schools.iter().filter(|s| s.alive()).count();
        let realms = w.alive_polities.len().max(1);
        let room = (realms as f64 * w.tuning.schools_per_realm).max(1.0);
        assert!(living > 0, "no school survived at all");
        assert!(
            (living as f64) < room * 3.0,
            "with schism {} and founding x{}: {} living schools against {} \
             realms, so the traditions are compounding",
            schism,
            founding,
            living,
            realms
        );
        // And they must actually be retired, not merely stop being founded.
        assert!(
            w.schools.len() > living * 4,
            "{} schools ever against {} alive: almost nothing is being retired",
            w.schools.len(),
            living
        );
    }
}

/// A teaching that has been fading for generations, with no larger cousin to
/// be folded into, is forgotten rather than kept alive for ever.
///
/// This is the service-rate half of the schools bug, and the reason it went
/// unnoticed for so long is that it is invisible over any short run. The
/// ordinary way a school ends is absorption, which needs a bigger school of
/// the same kind holding ground where the dying one still stands. A school
/// that had retreated somewhere no cousin reached met that condition never
/// — and it never fell below the extinction floor either, because it still
/// held a tenth of one realm. So it failed to be absorbed every year for
/// ever. One immortal school is nothing; they accumulate linearly, and six
/// hundred of them had piled up by the hundredth century, each one rolling
/// against every realm it touched.
///
/// Tested directly rather than by simulation, because the accumulation is
/// slow and the mechanism is exact: a lone school of its kind has nowhere to
/// go by construction, so it must die of its own accord.
#[test]
fn a_school_with_nowhere_to_go_is_forgotten() {
    let mut w = world(5);
    run(&mut w, 120);
    let city = (0..w.cities.len())
        .find(|&c| w.cities[c].destroyed.is_none() && w.cities[c].polity.is_some())
        .expect("a world of 120 years has a city");
    // Its own kind, and the only one of it: nothing can absorb it.
    let s = crate::sim::magic::found_school(
        &mut w,
        city,
        crate::sim::SchoolKind::Philosophical,
        None,
        None,
    );
    let alone = |w: &World| -> bool {
        !w.schools
            .iter()
            .any(|x| x.id != s && x.alive() && x.kind == crate::sim::SchoolKind::Philosophical)
    };
    // Held in exactly the state that used to be eternal: enough of a
    // following to stay above the extinction floor, and far too little to
    // stand anywhere. Pinned every year because a school left alone in its
    // own realm legitimately flourishes, and a flourishing school is not
    // what this is about.
    let p = w.cities[city].polity.expect("the city has a realm");
    let bar = w.tuning.school_fading_years * 5 + 40;
    for _ in 0..bar {
        w.schools[s].influence.clear();
        w.schools[s].influence.insert(p, 0.2);
        w.tick();
        if !w.schools[s].alive() {
            return;
        }
        if !alone(&w) {
            // A cousin appeared and could legitimately absorb it, so the
            // test no longer proves anything. Not a failure.
            return;
        }
    }
    panic!(
        "a school with nowhere to go was still alive after {} years, holding {:?}",
        bar,
        w.schools[s]
            .influence
            .values()
            .fold(0.0f32, |a, &b| a.max(b))
    );
}

#[test]
#[ignore]
fn time_saving_and_the_chronicle() {
    for years in [1000i32, 3000, 5000, 8000] {
        let mut w = World::new(7, 160, 64, Detail::Medium);
        run(&mut w, years);
        let t = std::time::Instant::now();
        let bytes = crate::ser::save(&mut w);
        let save_ms = t.elapsed().as_secs_f64() * 1000.0;
        let t = std::time::Instant::now();
        let _ = crate::ser::load(&bytes).expect("loads");
        let load_ms = t.elapsed().as_secs_f64() * 1000.0;
        // The chronicle screen, unfiltered and filtered.
        let t = std::time::Instant::now();
        for _ in 0..20 {
            std::hint::black_box(crate::ui::detail::list_rows(&w, 5));
        }
        let wars_ms = t.elapsed().as_secs_f64() * 1000.0 / 20.0;
        let frame = crate::ui::time_frames(
            {
                let mut v = World::new(7, 160, 64, Detail::Medium);
                run(&mut v, years);
                v
            },
            160,
            45,
            40,
        );
        println!(
            "year {:>5}: save {:>8.1} ms | load {:>8.1} ms | wars list {:>6.3} ms | frame {:>6.2} ms | {} events ({} KB)",
            years,
            save_ms,
            load_ms,
            wars_ms,
            frame,
            w.chronicle.len(),
            bytes.len() / 1024
        );
    }
}

#[test]
#[ignore]
fn time_the_interface() {
    let env = |k: &str, d: usize| -> usize {
        std::env::var(k)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(d)
    };
    let (wide, high) = (env("W", 288), env("H", 144));
    for years in [1000i32, 3000, 5000, 8000] {
        let mut w = World::new(7, wide, high, Detail::Medium);
        run(&mut w, years);
        let time = |f: &dyn Fn()| -> f64 {
            let t = std::time::Instant::now();
            for _ in 0..20 {
                f();
            }
            t.elapsed().as_secs_f64() * 1000.0 / 20.0
        };
        let tab = |n: usize| {
            time(&|| {
                std::hint::black_box(crate::ui::detail::list_rows(&w, n));
            })
        };
        let stories = time(&|| {
            std::hint::black_box(crate::ui::stories(&w));
        });
        println!(
            "year {:>5} ({:>7} persons {:>6} wars {:>5} realms): stories {:>7.3} \
             | realms {:>7.3} | persons {:>7.3} | wars {:>7.3} | figures {:>7.3} ms/frame",
            years,
            w.persons.len(),
            w.wars.len(),
            w.polities.len(),
            stories,
            tab(0),
            tab(4),
            tab(5),
            tab(9),
        );
    }
}

/// A list filter can ask what things are, not only what they are called.
///
/// The filter matched the text of a rendered row, which answers one question
/// well and every other question not at all: "which realms are running a
/// deficit" is not in the text of a row and is in the world.
#[test]
fn a_list_can_be_asked_a_question() {
    use crate::sim::chronicle::Ref;
    use crate::ui::query::{self, Atom, Op};
    let mut w = world(31);
    run(&mut w, 400);

    // Parsing: a comparison, and anything else as a word to look for.
    match &query::parse("lands>200")[0].any[0] {
        Atom::Compare { field, op, value } => {
            assert_eq!(field, "lands");
            assert_eq!(*op, Op::Gt);
            assert_eq!(*value, 200.0);
        }
        other => panic!("expected a comparison, got {:?}", other),
    }
    // `>=` must not parse as `>` and leave an `=` behind.
    match &query::parse("stability>=30")[0].any[0] {
        Atom::Compare { op, value, .. } => {
            assert_eq!(*op, Op::Ge);
            assert_eq!(*value, 30.0);
        }
        other => panic!("expected a comparison, got {:?}", other),
    }
    // A half-typed query is a word, not a term that matches nothing: the
    // filter is typed one character at a time.
    assert!(matches!(&query::parse("lands>")[0].any[0], Atom::Text(_)));

    // A term may be inverted, and may offer alternatives.
    assert!(query::parse("!coast")[0].not);
    assert_eq!(query::parse("wine|spice|salt")[0].any.len(), 3);
    // And a half-typed `!` excludes nothing rather than everything.
    assert!(query::parse("!").is_empty());

    // And the comparisons agree with the world. The bar is taken from the
    // world rather than written down, so the test does not depend on how
    // large a realm grows on a test-sized map.
    let largest = w
        .alive_polities
        .iter()
        .map(|&p| w.polities[p].cells)
        .max()
        .expect("a 400 year world has realms");
    let bar = largest / 2;
    let big = query::parse(&format!("lands>{}", bar));
    let mut over = 0;
    for &p in &w.alive_polities {
        let r = Ref::Polity(p);
        let admitted = query::matches(&w, r, "", &big);
        assert_eq!(
            admitted,
            w.polities[p].cells > bar,
            "realm {} holds {} lands against a bar of {}",
            p,
            w.polities[p].cells,
            bar
        );
        over += usize::from(admitted);
    }
    assert!(over > 0, "nothing passed a bar of half the largest realm");

    // Terms combine, and a field that does not apply to a kind of thing
    // matches nothing rather than everything.
    let both = query::parse(&format!("lands>{} stability<200", bar));
    assert!(w
        .alive_polities
        .iter()
        .any(|&p| query::matches(&w, Ref::Polity(p), "", &both)));
    let wrong = query::parse("prosperity>0");
    assert!(
        !w.alive_polities
            .iter()
            .any(|&p| query::matches(&w, Ref::Polity(p), "", &wrong)),
        "a realm answered a question about prosperity, which is a city's"
    );

    // Negation really inverts, and alternatives really widen: over the
    // living realms, `!big` must be exactly the complement of `big`, and
    // `big|tiny` must admit everything either does and nothing neither
    // does.
    let big = query::parse(&format!("lands>{}", bar));
    let not_big = query::parse(&format!("!lands>{}", bar));
    let either = query::parse(&format!("lands>{}|lands<2", bar));
    let mut inverted = 0;
    for &p in &w.alive_polities {
        let r = Ref::Polity(p);
        let yes = query::matches(&w, r, "", &big);
        assert_ne!(
            yes,
            query::matches(&w, r, "", &not_big),
            "realm {} is on both sides of a negation",
            p
        );
        let small = w.polities[p].cells < 2;
        assert_eq!(
            query::matches(&w, r, "", &either),
            yes || small,
            "realm {} disagrees with its own alternatives",
            p
        );
        inverted += usize::from(!yes);
    }
    assert!(
        inverted > 0,
        "nothing was excluded, so negation proves nothing"
    );

    // Every page that advertises fields must advertise ones that work.
    for tab in 0..crate::ui::detail::LIST_TABS.len() {
        let (rows, _) = crate::ui::detail::list_rows(&w, tab);
        for field in query::fields_for(tab).split_whitespace() {
            assert!(
                rows.iter()
                    .any(|(_, r)| query::value_of(&w, *r, field).is_some()),
                "page {} offers {:?} and no row of it answers to that",
                crate::ui::detail::LIST_TABS[tab],
                field
            );
        }
    }
}

#[test]
#[ignore]
fn probe_treasuries() {
    let env = |k: &str, d: usize| -> usize {
        std::env::var(k)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(d)
    };
    let mut w = World::new(7, env("W", 288), env("H", 144), Detail::Medium);
    // With NEUTRAL=1 the new drains are switched off, which isolates what
    // they did to the world from what the world was already doing.
    if std::env::var("NEUTRAL").is_ok() {
        w.tuning.court_spend_share = 0.0;
        w.tuning.corruption_decadence_weight = 0.0;
        w.tuning.corruption_sprawl_weight = 0.0;
        w.tuning.army_gold_weight = 0.0;
    }
    for c in 0..env("C", 50) {
        run(&mut w, 100);
        let mut t: Vec<f32> = w
            .alive_polities
            .iter()
            .map(|&p| w.polities[p].treasury)
            .collect();
        t.sort_by(f32::total_cmp);
        let n = t.len().max(1);
        let pick = |q: f32| {
            t.get(((n as f32 - 1.0) * q) as usize)
                .copied()
                .unwrap_or(0.0)
        };
        let mean_dec: f32 = w
            .alive_polities
            .iter()
            .map(|&p| w.polities[p].decadence)
            .sum::<f32>()
            / n as f32;
        let mut st: Vec<f32> = w
            .alive_polities
            .iter()
            .map(|&p| w.polities[p].stability)
            .collect();
        st.sort_by(f32::total_cmp);
        let sq = |q: f32| {
            st.get(((st.len().max(1) as f32 - 1.0) * q) as usize)
                .copied()
                .unwrap_or(0.0)
        };
        let mean_army: f32 = w
            .alive_polities
            .iter()
            .map(|&p| w.polities[p].army)
            .sum::<f32>()
            / n as f32;
        if c % 5 == 0 || c + 1 == env("C", 50) {
            println!(
                "year {:>5}: realms {:>4} treasury med {:>9.0} p90 {:>9.0} max {:>10.0} \
                 | decadence {:.2} stability p10 {:.2} med {:.2} p90 {:.2} army {:>7.1}",
                w.year,
                n,
                pick(0.5),
                pick(0.9),
                t.last().copied().unwrap_or(0.0),
                mean_dec,
                sq(0.1),
                sq(0.5),
                sq(0.9),
                mean_army
            );
        }
    }
}

#[test]
#[ignore]
fn probe_trade_share() {
    use crate::sim::explain;
    let mut w = World::new(7, 288, 144, Detail::Medium);
    for c in 0..14 {
        run(&mut w, 200);
        // What share of a realm's gross income the roads account for, both
        // directly as tolls and indirectly through the prosperity that
        // trade buys its cities.
        let mut direct = 0.0f64;
        let mut gross = 0.0f64;
        let mut via_prosp = 0.0f64;
        for &p in &w.alive_polities {
            let g: f32 = explain::income_factors(&w, p)
                .iter()
                .filter(|f| f.weight > 0.0)
                .map(|f| f.weight)
                .sum();
            gross += g as f64;
            direct += (w.realm_trade(p) * w.tuning.trade_toll_factor) as f64;
            // The prosperity a city owes to what passes through it, as a
            // share of the prosperity it has.
            for &ci in &w.polities[p].cities {
                let from_trade = (w.city_trade(ci) * 0.06).min(0.6);
                let share = (from_trade / w.cities[ci].prosperity.max(0.01)).min(1.0);
                via_prosp += (w.cities[ci].pop
                    * w.cities[ci].prosperity
                    * w.tuning.city_income_factor
                    * share) as f64;
            }
        }
        if c % 3 == 0 || c == 13 {
            println!(
                "year {:>5}: gross {:>10.0} | tolls {:>8.0} ({:>4.1}%) | via prosperity {:>8.0} ({:>4.1}%)",
                w.year, gross, direct, direct / gross.max(1.0) * 100.0,
                via_prosp, via_prosp / gross.max(1.0) * 100.0
            );
        }
    }
}

/// Every intervention, on every kind of thing, must do something and must
/// leave the world standing.
///
/// The Hand of Fate reached realms and nothing else, so the only way to
/// touch a city was to find whichever realm happened to hold it and act on
/// the whole of that instead — and a city is the unit most of this world's
/// history actually happens to. It reaches towns, people and regions now,
/// which is four times as many ways to be wrong.
///
/// Each one is worked on a world forked through a save file, so that
/// twenty-four interventions are twenty-four independent experiments rather
/// than one long compounding one, and each is asked for three things: that
/// it says what it did, that the world survives a further century, and that
/// something actually changed.
#[test]
fn every_intervention_does_something_and_breaks_nothing() {
    use crate::sim::chronicle::Ref;
    use crate::ui::detail::{fate_menu_for, fate_reaches, hand_of_fate_on};
    let mut seed_world = world(67);
    run(&mut seed_world, 500);
    let bytes = ser::save(&mut seed_world);

    // One target of each kind, chosen from the world rather than assumed.
    let realm = *seed_world
        .alive_polities
        .first()
        .expect("a 500 year world has realms");
    let city = seed_world.polities[realm]
        .cities
        .first()
        .copied()
        .or_else(|| {
            (0..seed_world.cities.len()).find(|&c| seed_world.cities[c].destroyed.is_none())
        })
        .expect("a 500 year world has cities");
    let person = *seed_world
        .alive_persons
        .last()
        .expect("a 500 year world has people");
    let region = (0..seed_world.terrain.features.len())
        .find(|&f| {
            seed_world.terrain.features[f].name.is_some()
                && seed_world.terrain.features[f]
                    .cells
                    .iter()
                    .any(|&i| seed_world.terrain.is_land(i))
        })
        .expect("a world has a named region with land in it");

    for target in [
        Ref::Polity(realm),
        Ref::City(city),
        Ref::Person(person),
        Ref::Feature(region),
    ] {
        // Every reachable kind must offer a menu, and the menu must have a
        // line for each of the six keys that select from it.
        let menu = fate_menu_for(&seed_world, target);
        assert!(
            menu.len() >= 8,
            "{:?} offers {} lines, which is not a title, a blank and six \
             choices",
            target,
            menu.len()
        );
        for choice in 0..6u8 {
            let mut w = ser::load(&bytes).expect("a fresh save must load");
            assert!(fate_reaches(&w, target), "{:?} is out of reach", target);
            // Acts cost now, and this test is about what they *do*: fund it
            // so a refusal for want of devotion cannot be mistaken for an
            // intervention that quietly does nothing.
            w.fate.power = crate::sim::fate::CEILING;
            let before = summary(&w);
            let said = hand_of_fate_on(&mut w, target, choice);
            assert!(
                !said.is_empty() && said != "that is beyond reach now",
                "{:?} choice {} said {:?}",
                target,
                choice,
                said
            );
            // The world has to survive being interfered with, including the
            // derived indexes an intervention may have invalidated.
            w.recompute();
            run(&mut w, 100);
            let after = summary(&w);
            assert_ne!(
                before, after,
                "{:?} choice {} ({}) left the world exactly as it was",
                target, choice, said
            );
            // And a save written after an intervention must still load.
            let bytes_after = ser::save(&mut w);
            assert!(
                ser::load(&bytes_after).is_ok(),
                "{:?} choice {} produced a world that cannot be saved",
                target,
                choice
            );
        }
    }
}

/// A covenant can be lost, and losing it reads as a loss.
///
/// This is the whole of the stake. A people can be conquered, scattered and
/// absorbed — the simulation was always capable of dealing that hand — and
/// when it does, the altars go cold, the chronicle says so once, and what
/// the watcher had drains away instead of vanishing, so that somebody who
/// has just lost everything still has one last thing to do with it.
#[test]
fn a_people_lost_is_a_covenant_lost() {
    use crate::sim::fate;
    let mut w = world(37);
    run(&mut w, 150);
    let people = (0..w.cultures.len())
        .find(|&c| w.cultures[c].extinct.is_none())
        .expect("a peopled world");
    fate::bind(&mut w, people);
    run(&mut w, 60);
    let held = w.fate.power;
    assert!(held > 1.0, "a covenant gathered nothing to lose");
    assert!(w.fate.forsaken.is_none());

    // The world takes them.
    w.cultures[people].extinct = Some(w.year);
    w.cultures[people].pop = 0.0;
    run(&mut w, 1);
    assert_eq!(w.fate.forsaken, Some(w.year));
    let cold = w
        .chronicle
        .events
        .iter()
        .filter(|e| e.text.contains("altars had been for"))
        .count();
    assert_eq!(cold, 1, "the loss was announced {} times", cold);

    // It drains rather than vanishing, and it is said only once.
    run(&mut w, 20);
    assert!(
        w.fate.power < held,
        "{:.1} held against {:.1} before the loss",
        w.fate.power,
        held
    );
    assert!(w.fate.power > 0.0, "everything went at once");
    assert_eq!(
        w.chronicle
            .events
            .iter()
            .filter(|e| e.text.contains("altars had been for"))
            .count(),
        1,
        "the loss was announced again"
    );
    // And it reaches nothing eventually rather than going negative.
    run(&mut w, 400);
    assert_eq!(w.fate.power, 0.0);
}

/// The covenant pays out at a rate a player can feel.
///
/// Too slow and the first act is twenty minutes away; too fast and the
/// ceiling is always full and nothing is ever a choice. At the default two
/// years a second, a cheap act should be seconds away and the heaviest a
/// minute or two, over the whole life of a world rather than only at its
/// start — a people's share of the world falls as the world fills, and the
/// square root in `fate::tick` is what stops that from starving the loop.
#[test]
fn a_covenant_pays_out_at_a_playable_rate() {
    use crate::sim::fate;
    let mut w = world(29);
    run(&mut w, 60);
    let people = (0..w.cultures.len())
        .filter(|&c| w.cultures[c].extinct.is_none())
        .max_by(|&a, &b| w.cultures[a].pop.total_cmp(&w.cultures[b].pop))
        .expect("a peopled world");
    fate::bind(&mut w, people);
    let mut samples: Vec<(i32, f32)> = Vec::new();
    for _ in 0..12 {
        run(&mut w, 100);
        samples.push((w.year, w.fate.income));
        // Kept spent so the ceiling does not mask the rate.
        w.fate.power = 0.0;
    }
    if std::env::var("SHOW_FATE").is_ok() {
        for (y, inc) in &samples {
            println!(
                "year {:5}  {:+.2}/yr  ({:.0}s to the cheapest act)",
                y,
                inc,
                10.0 / (inc * 2.0)
            );
        }
    }
    for (y, inc) in samples {
        assert!(
            (0.25..=3.2).contains(&inc),
            "in year {} a covenant paid {:.2} a year, which is {} to be a game",
            y,
            inc,
            if inc < 0.25 { "too slow" } else { "too fast" }
        );
    }
}

/// Every kind of god is one some world will offer you.
///
/// The aspect is derived from a people's values rather than chosen, which
/// makes the opening question a real decision — but only if the four kinds
/// actually turn up. A table that reads "a god of endings" for three
/// peoples in five is one choice wearing four names.
#[test]
fn all_four_aspects_are_reachable_and_none_dominates() {
    use crate::sim::fate::{aspect_of, Domain};
    use std::collections::HashMap;
    let mut seen: HashMap<&'static str, usize> = HashMap::new();
    let mut total = 0usize;
    for seed in 0..40u64 {
        let mut w = world(seed);
        run(&mut w, 120);
        for c in 0..w.cultures.len() {
            if w.cultures[c].extinct.is_some() {
                continue;
            }
            *seen.entry(aspect_of(&w, c).title()).or_insert(0) += 1;
            total += 1;
        }
    }
    if std::env::var("SHOW_FATE").is_ok() {
        let mut rows: Vec<_> = seen.iter().collect();
        rows.sort_by_key(|&(_, n)| std::cmp::Reverse(*n));
        for (k, n) in rows {
            println!("{:5.1}%  {}", 100.0 * *n as f64 / total as f64, k);
        }
    }
    for d in [Domain::Growth, Domain::Ruin, Domain::Craft, Domain::Strife] {
        let n = seen.get(d.title()).copied().unwrap_or(0);
        let share = n as f64 / total as f64;
        assert!(
            (0.12..=0.42).contains(&share),
            "{:.0}% of peoples make {}, out of {} peoples",
            share * 100.0,
            d.title(),
            total
        );
    }
}

/// An act of fate is findable afterwards.
///
/// Six interventions in a world of fifty thousand events are invisible
/// unless something marks them, and an act you cannot find afterwards did
/// not feel like an act. The mark has to survive a save, too, or a world
/// put away and taken out again forgets everything the player ever did.
#[test]
fn what_the_watcher_did_is_marked_and_kept() {
    use crate::sim::chronicle::Ref;
    use crate::ui::detail::hand_of_fate_on;
    let mut w = world(43);
    run(&mut w, 250);
    assert!(
        w.chronicle.events.iter().all(|e| !e.by_fate),
        "the world marked its own doing as the watcher's"
    );
    let city = (0..w.cities.len())
        .find(|&c| w.cities[c].destroyed.is_none())
        .expect("a world has cities");
    w.fate.power = crate::sim::fate::CEILING;
    let said = hand_of_fate_on(&mut w, Ref::City(city), 0);
    assert!(!said.starts_with("not enough"), "{}", said);
    let marked: Vec<&str> = w
        .chronicle
        .events
        .iter()
        .filter(|e| e.by_fate)
        .map(|e| e.text.as_str())
        .collect();
    assert_eq!(marked.len(), 1, "an act left {} marks", marked.len());
    let text = marked[0].to_string();

    // The mark must not leak into whatever the world does next.
    run(&mut w, 40);
    assert_eq!(
        w.chronicle.events.iter().filter(|e| e.by_fate).count(),
        1,
        "the mark stayed on after the act was over"
    );
    // And it has to survive being put away.
    let bytes = ser::save(&mut w);
    let back = ser::load(&bytes).expect("loads");
    let kept: Vec<&str> = back
        .chronicle
        .events
        .iter()
        .filter(|e| e.by_fate)
        .map(|e| e.text.as_str())
        .collect();
    assert_eq!(kept, vec![text.as_str()]);
}

/// The covenant card asks a question a new player can answer.
///
/// It is the first screen after the title card, so it carries the whole
/// weight of explaining what the player is. It has to offer real peoples,
/// number them so one keystroke chooses, and fit an eighty-column window.
#[test]
fn the_covenant_card_offers_a_choice_that_fits() {
    use crate::ui::detail::{covenant_card, covenant_choices};
    let mut w = world(23);
    run(&mut w, 60);
    let choices = covenant_choices(&w);
    assert!(
        (2..=9).contains(&choices.len()),
        "{} peoples to choose between",
        choices.len()
    );
    for &(c, _) in &choices {
        assert!(w.cultures[c].extinct.is_none(), "a dead people was offered");
    }
    let card = covenant_card(&w);
    // Every people on the card says what it would make you, so the choice
    // is between kinds of god and not only between population figures.
    for &(c, ref line) in &choices {
        assert!(
            line.contains(crate::sim::fate::aspect_of(&w, c).title()),
            "{:?} does not say what that people makes of a watcher",
            line
        );
    }
    if std::env::var("SHOW_FATE").is_ok() {
        for l in &card {
            println!("|{}|", l);
        }
    }
    let widest = card.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    assert!(
        widest + 6 <= 80,
        "the covenant card needs {} columns: {:?}",
        widest + 6,
        card.iter().max_by_key(|l| l.chars().count())
    );
    // Every offered people is numbered, and declining is always possible.
    for n in 1..=choices.len() {
        assert!(
            card.iter().any(|l| l.starts_with(&format!("{}  ", n))),
            "no line numbered {} in {:#?}",
            n,
            card
        );
    }
    assert!(
        card.iter().any(|l| l.starts_with("0  ")),
        "no way to decline"
    );
}

/// The Hand of Fate panel fits the smallest terminal the game supports.
///
/// The panel sizes itself to its longest line and is centred on the map, so
/// a line a few characters too long does not wrap — it puts the left edge
/// at zero and the right edge off the screen. Adding the covenant's balance
/// to the top of it made that a live risk.
#[test]
fn the_hand_of_fate_fits_a_small_screen() {
    use crate::sim::chronicle::Ref;
    use crate::ui::detail::fate_menu_for;
    let mut w = world(19);
    run(&mut w, 200);
    let realm = *w.alive_polities.first().expect("a world has realms");
    let city = (0..w.cities.len())
        .find(|&c| w.cities[c].destroyed.is_none())
        .expect("a world has cities");
    let person = *w.alive_persons.last().expect("a world has people");
    for target in [Ref::Polity(realm), Ref::City(city), Ref::Person(person)] {
        for bound in [None, Some(0usize)] {
            w.fate.patron = bound;
            w.fate.power = 61.0;
            let menu = fate_menu_for(&w, target);
            if std::env::var("SHOW_FATE").is_ok() {
                println!("--- {:?} bound={:?}", target, bound);
                for l in &menu {
                    println!("|{}|", l);
                }
            }
            // A title, a balance, a blank and six choices.
            assert_eq!(menu.len(), 9, "{:?}: {:#?}", target, menu);
            let widest = menu.iter().map(|l| l.chars().count()).max().unwrap_or(0);
            assert!(
                widest + 4 <= 76,
                "{:?} needs {} columns, which will not fit an 80-column \
                 terminal beside a map: {:?}",
                target,
                widest + 4,
                menu.iter().max_by_key(|l| l.chars().count())
            );
        }
    }
}

/// The covenant is an economy, not a cheat menu.
///
/// The Hand of Fate used to be twenty-four free, unlimited acts: no reason
/// to choose between them and no reason not to use all of them at once,
/// which is not a decision and so not a game. Three things have to hold for
/// it to be one — a price, a purse, and a refusal when the purse is empty.
#[test]
fn an_act_of_fate_costs_what_it_says_and_is_refused_when_it_cannot_be_paid() {
    use crate::sim::chronicle::Ref;
    use crate::sim::fate;
    use crate::ui::detail::{fate_menu_for, hand_of_fate_on};
    let mut w = world(31);
    run(&mut w, 300);
    let realm = *w.alive_polities.first().expect("a world has realms");
    let target = Ref::Polity(realm);

    // An empty purse buys nothing, and says so rather than failing quietly.
    w.fate.power = 0.0;
    let before = summary(&w);
    let said = hand_of_fate_on(&mut w, target, 0);
    assert!(
        said.starts_with("not enough"),
        "a watcher with nothing was allowed to act: {:?}",
        said
    );
    assert_eq!(summary(&w), before, "a refused act still changed the world");
    assert_eq!(w.fate.acts, 0);

    // A full one buys exactly one act, at exactly the advertised price —
    // the price for *this* watcher, which is not the abstract one: an act
    // within your nature costs less and one outside it costs more.
    let cost = fate::price(&w, target, 0);
    w.fate.power = fate::CEILING;
    let said = hand_of_fate_on(&mut w, target, 0);
    assert!(!said.starts_with("not enough"), "{:?}", said);
    assert!(
        (w.fate.power - (fate::CEILING - cost)).abs() < 0.01,
        "an act priced at {} took {}",
        cost,
        fate::CEILING - w.fate.power
    );
    assert_eq!(w.fate.acts, 1);
    assert!((w.fate.spent - cost).abs() < 0.01);

    // And the menu tells the truth about what can be afforded.
    w.fate.power = 0.0;
    let menu = fate_menu_for(&w, target);
    assert!(
        menu.iter().filter(|l| l.starts_with('\u{b7}')).count() == 6,
        "a penniless watcher was shown affordable choices: {:#?}",
        menu
    );
}

/// What you are changes what you can afford.
///
/// Without this, "whose god will you be?" has one sane answer — whichever
/// people is largest — and the opening question is not a question. A
/// people's values decide the watcher's nature, and the nature decides
/// which of the twenty-four acts are cheap.
#[test]
fn a_gods_nature_changes_what_an_act_costs() {
    use crate::sim::chronicle::Ref;
    use crate::sim::fate::{self, aspect_of, domain_of};
    let mut w = world(17);
    run(&mut w, 200);
    let target = Ref::City(
        (0..w.cities.len())
            .find(|&c| w.cities[c].destroyed.is_none())
            .expect("a world has cities"),
    );
    // Unbound, an act costs what the table says and nothing modifies it.
    w.fate.patron = None;
    for choice in 0..6u8 {
        assert_eq!(
            fate::price(&w, target, choice),
            fate::cost_of(target, choice)
        );
    }
    // Bound, at least one act is cheaper than the table and at least one is
    // dearer, and which is which follows the watcher's nature.
    let people = (0..w.cultures.len())
        .find(|&c| w.cultures[c].extinct.is_none())
        .expect("a peopled world");
    w.fate.patron = Some(people);
    let mine = aspect_of(&w, people);
    let (mut cheaper, mut dearer) = (0, 0);
    for choice in 0..6u8 {
        let base = fate::cost_of(target, choice);
        let paid = fate::price(&w, target, choice);
        if domain_of(target, choice) == mine {
            assert!(
                paid < base,
                "an act of your own nature cost {} of {}",
                paid,
                base
            );
            cheaper += 1;
        } else {
            assert!(
                paid > base,
                "an act outside your nature cost {} of {}",
                paid,
                base
            );
            dearer += 1;
        }
    }
    assert!(
        cheaper > 0 && dearer > 0,
        "{} cheap, {} dear",
        cheaper,
        dearer
    );
}

/// A covenant is worth making: a people's devotion outpaces what the world
/// gives a watcher who is bound to nobody, and their extinction takes it
/// away again.
#[test]
fn a_people_is_the_source_of_the_power_spent_on_them() {
    use crate::sim::fate;
    let mut bound = world(53);
    let mut unbound = world(53);
    run(&mut bound, 120);
    run(&mut unbound, 120);
    let people = (0..bound.cultures.len())
        .filter(|&c| bound.cultures[c].extinct.is_none())
        .max_by(|&a, &b| bound.cultures[a].pop.total_cmp(&bound.cultures[b].pop))
        .expect("a peopled world");
    fate::bind(&mut bound, people);
    bound.fate.power = 0.0;
    unbound.fate.power = 0.0;
    run(&mut bound, 200);
    run(&mut unbound, 200);
    // Compared by rate, not by what has piled up: both reach the ceiling in
    // a couple of centuries and the ceiling hides the difference.
    assert!(
        bound.fate.income > unbound.fate.income * 2.0,
        "a covenant with the largest people in the world pays {:.2} a year \
         against {:.2} for no covenant at all",
        bound.fate.income,
        unbound.fate.income
    );
    // And a watcher bound to nobody is poorer, not powerless: this game was
    // a passive simulator with free interventions before it was anything
    // else, and somebody who answers "simply watch" should still be able to
    // reach into the world now and then.
    assert!(
        unbound.fate.power >= fate::cost_of(crate::sim::chronicle::Ref::City(0), 2),
        "two centuries of watching bought {:.1}, which is not even walls",
        unbound.fate.power
    );
    // The old sandbox is one line of config away.
    let mut free = world(53);
    free.tuning.fate_trickle = 60.0;
    run(&mut free, 3);
    assert!(free.fate.power >= fate::CEILING);
}

/// A world can be written out for somebody who does not have the game.
///
/// Everything the interface knows is on a screen that scrolls away, and the
/// only way out was a save file, which needs this binary to read. A world
/// run for five thousand years is a thing people want to keep and show to
/// other people.
#[test]
fn a_world_can_be_written_out() {
    let dir = std::env::temp_dir().join(format!("empires-export-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a temp directory");
    let mut ui = crate::ui::for_test(world(71), 100, 40);
    run(&mut ui.world, 400);

    // Every shape, and each one has to produce something a reader could use.
    for (kind, must_contain) in [
        ("chronicle", "## Years"),
        ("timeline", "# The rise and fall of"),
        ("series", "year,people,realms"),
        ("house", "# "),
        ("map", "<!doctype html>"),
        ("realms", "name,lands,people"),
        ("wealth", "treasury"),
        ("cities", "prosperity"),
        ("roads", "name,"),
    ] {
        let path = dir.join(format!("{}.out", kind));
        let said = ui.export(kind, path.to_str().expect("a utf-8 path"));
        assert!(said.starts_with("wrote "), "{}: {}", kind, said);
        let body = std::fs::read_to_string(&path).expect("the file it said it wrote");
        assert!(
            body.contains(must_contain),
            "{} export has no {:?} in it:\n{}",
            kind,
            must_contain,
            &body[..body.len().min(300)]
        );
        // Big enough to be a real answer rather than a header and nothing.
        assert!(body.len() > 200, "{} export is {} bytes", kind, body.len());
    }

    // A shape nobody knows about says so rather than writing an empty file.
    let said = ui.export("moon-phases", dir.join("x").to_str().unwrap());
    assert!(said.starts_with("usage:"), "{}", said);

    // The chronicle honours what the reader has chosen to see: raising the
    // bar must not make the file longer.
    let path = dir.join("chron.md");
    let p = path.to_str().unwrap();
    ui.chron_min = 1;
    ui.export("chronicle", p);
    let all = std::fs::read_to_string(&path).unwrap().len();
    ui.chron_min = 3;
    ui.export("chronicle", p);
    let great = std::fs::read_to_string(&path).unwrap().len();
    assert!(
        great < all,
        "asking for only the great events gave {} bytes against {}",
        great,
        all
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// A quantity built by adding up advantages must not pile against its
/// ceiling.
///
/// This simulation made the same mistake in four places: sum everything
/// that makes a thing prosperous, or rich, or well-positioned, and then
/// clamp the sum. A clamp is invisible until enough things reach it and
/// then it silently destroys the difference between them. Realms tied at
/// exactly six hundred in the treasury; more than half the cities in the
/// world at exactly two and a half in prosperity, with the *median* city at
/// the maximum, so prosperity had stopped distinguishing anything and city
/// income was decided by population alone.
///
/// `sim::soft_ceiling` is the shape that fixes it: below a knee nothing
/// changes, above it the remaining room is approached and never reached. So
/// the sum can go on growing and the result can go on answering to it.
#[test]
fn prosperity_does_not_pile_against_its_ceiling() {
    use crate::sim::soft_ceiling;

    // The curve itself: unchanged below the knee, bounded above it, and
    // always increasing, since a city that gains an advantage must not lose
    // prosperity for it.
    assert_eq!(soft_ceiling(0.5, 1.5, 2.5), 0.5);
    assert_eq!(soft_ceiling(1.5, 1.5, 2.5), 1.5);
    let mut last = 1.5f32;
    for v in [1.6f32, 2.0, 3.0, 8.0] {
        let got = soft_ceiling(v, 1.5, 2.5);
        assert!(got > last, "{} mapped to {} after {}", v, got, last);
        assert!(got < 2.5, "{} mapped to {}, over the ceiling", v, got);
        last = got;
    }
    // A sum hundreds of times the room available touches the ceiling,
    // because the exponential underflows, and must never pass it.
    assert!(soft_ceiling(500.0, 1.5, 2.5) <= 2.5);
    assert!(soft_ceiling(f32::MAX, 1.5, 2.5) <= 2.5);
    // A degenerate ceiling must not produce nonsense.
    assert_eq!(soft_ceiling(3.0, 2.0, 1.0), 3.0);

    // And in the world: cities must not collect at the top.
    let pinned = |w: &World| -> (f32, f32) {
        let live: Vec<f32> = (0..w.cities.len())
            .filter(|&i| w.cities[i].destroyed.is_none())
            .map(|i| w.cities[i].prosperity)
            .collect();
        let n = live.len().max(1);
        let at_top = live.iter().filter(|&&v| v >= 2.49).count();
        let mut sorted = live;
        sorted.sort_by(f32::total_cmp);
        (
            at_top as f32 / n as f32,
            sorted.get(n / 2).copied().unwrap_or(0.0),
        )
    };
    let mut w = World::new(73, 160, 64, Detail::Medium);
    run(&mut w, 1800);
    let (share, median) = pinned(&w);
    assert!(
        share < 0.25,
        "{:.0}% of cities sit at the top of the prosperity range",
        share * 100.0
    );
    assert!(
        median < 2.45,
        "the median city has prosperity {:.2}, which is the maximum: the \
         measure has stopped telling cities apart",
        median
    );
}

/// A plague must be visible at the speed a plague happens.
///
/// It lasts three to six years, which at twenty-five years a second is a
/// hundred and fifty milliseconds and at a hundred is forty. That was a
/// wash of green over a whole realm and nothing else, so what a player saw
/// was a flash of colour with no explanation — and on three of the newer
/// layers the green meant something already. On the settling layer it means
/// people *arriving*, so a plague was washing the ground in the colour of
/// the opposite thing.
#[test]
fn a_plague_is_marked_on_every_layer_that_shows_it() {
    use crate::ui::Layer;
    let mut w = World::new(79, 120, 60, Detail::Medium);
    run(&mut w, 300);
    // Put one where it can be found rather than waiting for one.
    // On open country rather than in a town: a standing city keeps its own
    // glyph and shows a plague by its colour alone, which a text buffer
    // cannot be asked about.
    let cell = (0..w.cells.len())
        .find(|&i| w.terrain.is_land(i) && w.cells[i].city.is_none() && w.cells[i].pop > 0.1)
        .expect("a 300 year world has peopled open country");
    w.cells[cell].plague = 3;

    for layer in Layer::all() {
        // Biomes draws flat biome colours and deliberately carries no
        // overlays at all.
        if layer == Layer::Biomes {
            continue;
        }
        let mut ui = crate::ui::for_test_at(w_clone(&mut w), 80, 30, layer, cell);
        let marked = ui.frame_shows('\u{2020}');
        assert!(
            marked,
            "a plague leaves no mark on the {} layer",
            layer.name()
        );
    }
}

/// Duplicate a world through a save file, since `World` is not `Clone`.
fn w_clone(w: &mut World) -> World {
    let bytes = ser::save(w);
    ser::load(&bytes).expect("a fresh save must load")
}

/// The relations layer must draw every kind of tie there is.
///
/// The political layer says who owns what and nothing about who is sworn to
/// whom, which left the whole of diplomacy — tension, stances, tributaries,
/// hegemony — readable one realm page at a time and nowhere else, though it
/// is among the largest systems here.
///
/// Each tie is set up deliberately rather than waited for, because a world
/// that happens to contain a marriage and a tributary and a declared rival
/// all at once, all bordering the realm the test picked, is not a world a
/// test can rely on turning up.
#[test]
fn the_relations_layer_draws_every_kind_of_tie() {
    use crate::sim::chronicle::Ref;
    use crate::sim::{dynasty, Stance};
    use crate::ui::Layer;
    let mut w = World::new(83, 140, 70, Detail::Medium);
    run(&mut w, 600);

    // The largest realm, and four neighbours to stand in each relation to
    // it. Neighbours, so that they are on screen beside it.
    let mut realms = w.alive_polities.clone();
    realms.sort_by_key(|&p| std::cmp::Reverse(w.polities[p].cells));
    let me = realms[0];
    let neighbours: Vec<usize> = w.polities[me]
        .neighbors
        .iter()
        .map(|&(q, _)| q)
        .filter(|&q| w.polities[q].alive())
        .collect();
    assert!(
        neighbours.len() >= 4,
        "the largest realm has only {} living neighbours to relate to",
        neighbours.len()
    );

    dynasty::set_stance(&mut w, me, neighbours[0], Stance::Allied);
    dynasty::set_stance(&mut w, me, neighbours[1], Stance::Married);
    dynasty::set_stance(&mut w, me, neighbours[2], Stance::Rival);
    w.polities[neighbours[3]].overlord = Some(me);
    // And a war, which has its own machinery rather than a field to set.
    w.wars_start(
        me,
        neighbours[0],
        crate::sim::WarKind::Conquest,
        "a test".into(),
    );
    w.recompute();

    let capital = w.capital_cell(me).expect("the largest realm has a capital");
    // A screen big enough for the whole map, so that a tie is missing from
    // the frame only if it is not drawn — not because the realm holding it
    // happened to lie beyond the viewport.
    let mut ui = crate::ui::for_test_at(w, 190, 90, Layer::Relations, capital);
    ui.selected = Some(Ref::Polity(me));

    // War wins over a sworn friendship, deliberately: what a reader needs to
    // know about somebody they are fighting is that they are fighting them.
    for (mark, what) in [
        ('\u{2715}', "at war"),
        ('\u{2740}', "married in"),
        ('\u{2260}', "a declared rival"),
        ('\u{25bc}', "paying tribute"),
    ] {
        assert!(
            ui.frame_shows(mark),
            "nothing on the relations layer shows {}",
            what
        );
    }
    // And the realm being asked about carries no glyph of its own: five
    // hundred marks over its own territory is noise, not information.
    assert!(
        !ui.frame_shows('\u{25c6}'),
        "the selected realm is marked on every cell it holds"
    );
}

#[test]
#[ignore]
fn probe_stability_ceiling() {
    let mut w = World::new(7, 288, 144, Detail::Medium);
    for c in 0..8 {
        run(&mut w, 400);
        let mut st: Vec<f32> = w
            .alive_polities
            .iter()
            .map(|&p| w.polities[p].stability)
            .collect();
        st.sort_by(f32::total_cmp);
        let n = st.len().max(1);
        let q = |f: f32| {
            st.get(((n as f32 - 1.0) * f) as usize)
                .copied()
                .unwrap_or(0.0)
        };
        let pinned = st.iter().filter(|&&v| v >= 0.985).count();
        if c % 2 == 0 || c == 7 {
            println!(
                "year {:>5}: realms {:>4} | stability p50 {:.2} p90 {:.2} max {:.2} | at the ceiling {:>4} ({:.0}%)",
                w.year,
                n,
                q(0.5),
                q(0.9),
                st.last().copied().unwrap_or(0.0),
                pinned,
                pinned as f32 / n as f32 * 100.0
            );
        }
    }
}

/// A wonder costs what a realm can afford, and is not always raised in the
/// same city.
///
/// Both halves were flat. A monument cost a hundred whoever built it, so a
/// crown sitting on twelve thousand got one for inside the noise of a
/// year's court spending while a realm on a hundred and thirty was nearly
/// bankrupted by it — the reward worth far more to the poor realm and the
/// cost nothing at all to the rich one. And it was only ever raised where
/// the crown sat, so the second city of an empire could never have one
/// however large it grew, while one capital accumulated seventeen and drew
/// a hundred and thirty-six points of prosperity from them.
#[test]
fn a_wonder_costs_what_a_realm_can_afford() {
    let mut seed_world = world(61);
    run(&mut seed_world, 400);
    let bytes = ser::save(&mut seed_world);
    // A realm that can actually build: two towns, a capital, and not at
    // war. Asking only for two towns meant the test depended on whether the
    // realm it happened to pick was fighting that year, which is a fact
    // about the whole simulation rather than about wonders.
    let realm = *seed_world
        .alive_polities
        .iter()
        .find(|&&p| {
            let pol = &seed_world.polities[p];
            pol.cities.len() >= 2 && pol.capital.is_some() && !pol.at_war()
        })
        .expect("a 400 year world has a realm at peace with two towns");

    // A rich realm pays a real share; a poor one pays the flat price.
    let spent = |treasury: f32| -> f32 {
        let mut w = ser::load(&bytes).expect("a fresh save must load");
        w.tuning.wonder_chance = 1.0;
        w.polities[realm].treasury = treasury;
        w.polities[realm].stability = 0.9;
        let before = w.polities[realm].treasury;
        let wonders = |w: &World| -> usize {
            w.polities[realm]
                .cities
                .iter()
                .map(|&c| w.cities[c].wonders.len())
                .sum()
        };
        let had = wonders(&w);
        crate::sim::events::wonders(&mut w);
        assert!(wonders(&w) > had, "no wonder was built at {}", treasury);
        before - w.polities[realm].treasury
    };
    let poor = spent(200.0);
    let rich = spent(10_000.0);
    assert!(
        rich > poor * 5.0,
        "a realm with fifty times the money paid {:.0} against {:.0}",
        rich,
        poor
    );
    // And never more than it has.
    assert!(spent(130.0) <= 130.0);

    // Over many realms and years, wonders do not all land in capitals.
    let mut w = ser::load(&bytes).expect("a fresh save must load");
    w.tuning.wonder_chance = 0.5;
    run(&mut w, 200);
    let mut elsewhere = 0;
    for &p in &w.alive_polities {
        for &c in &w.polities[p].cities {
            if w.polities[p].capital != Some(c) && !w.cities[c].wonders.is_empty() {
                elsewhere += 1;
            }
        }
    }
    assert!(
        elsewhere > 0,
        "every wonder in the world stands in a capital"
    );
}

/// A war that shuts a road marks the whole corridor, not its two ends.
///
/// The trade network already knew when a road opened and closed — it is
/// what stops the chronicle reporting the same two cities every three years
/// — and the map had no way to show it. A reader watching a war spread
/// should be able to see the corridor go quiet.
#[test]
fn shutting_a_road_marks_the_road() {
    // Three worlds, not one. This test broke twice on changes made
    // elsewhere in the simulation, both times because it was really a
    // statement about one particular road in one particular world.
    for seed in [67u64, 7, 23] {
        shutting_a_road_marks_the_road_in(seed);
    }
}

fn shutting_a_road_marks_the_road_in(seed: u64) {
    let mut w = world(seed);
    run(&mut w, 400);
    // The busiest open road between two realms that are not yet fighting.
    // The busiest rather than the first: a thin road's closure is a small
    // number against whatever else is moving through the same country, and
    // this test is about whether the corridor is marked at all.
    let (route, a, b) = w
        .routes
        .iter()
        .filter_map(|r| {
            let (pa, pb) = (w.cities[r.a].polity?, w.cities[r.b].polity?);
            (r.open
                && pa != pb
                && w.polities[pa].alive()
                && w.polities[pb].alive()
                && w.war_between(pa, pb).is_none())
            .then_some((*r, pa, pb))
        })
        .max_by(|x, y| x.0.value.total_cmp(&y.0.value))
        .unwrap_or_else(|| panic!("seed {}: no open road between two realms at peace", seed));
    let path = w
        .terrain
        .cells_between(w.cities[route.a].cell, w.cities[route.b].cell);
    assert!(
        path.len() > 2,
        "seed {}: the two towns are the same place",
        seed
    );

    let before: f32 = path.iter().map(|&c| w.flows.carried[c]).sum();
    w.wars_start(a, b, crate::sim::WarKind::Conquest, "a test".into());

    // The trade phase alone, not a whole year of the world.
    //
    // A full tick also resolves the war, marches the armies and may take
    // one of the two cities, any of which can leave the road open or leave
    // it out of the network entirely --- so the old version of this test
    // passed or failed on what else the year happened to do, and a change
    // anywhere in the simulation could tip it either way. What is being
    // tested here is one thing: that a road which shuts marks its whole
    // corridor rather than its two ends.
    crate::sim::trade::tick(&mut w);
    assert!(
        w.routes
            .iter()
            .any(|r| r.a == route.a && r.b == route.b && !r.open),
        "seed {}: the war did not shut the road, so there is nothing to look for",
        seed
    );

    let after: f32 = path.iter().map(|&c| w.flows.carried[c]).sum();
    assert!(
        after < before,
        "seed {}: a road shut and the map says nothing: {:.3} to {:.3}",
        seed,
        before,
        after
    );
    // Along its length, not only at the two towns. Measured over the
    // interior of the path rather than at one cell of it: a single midpoint
    // may lie on another road that is still open, and then it carries that
    // one's traffic and says nothing about this one.
    let interior = &path[1..path.len() - 1];
    let marked = interior
        .iter()
        .filter(|&&c| w.flows.carried[c] < 0.0)
        .count();
    assert!(
        marked * 2 >= interior.len(),
        "seed {}: only {} of {} cells along the road were marked",
        seed,
        marked,
        interior.len()
    );
    // And the mark fades rather than staying dark for ever. The decay
    // phase on its own, for the same reason as above.
    let deep: f32 = interior.iter().map(|&c| w.flows.carried[c]).sum();
    for _ in 0..40 {
        crate::sim::flows::tick(&mut w);
    }
    let now: f32 = interior.iter().map(|&c| w.flows.carried[c]).sum();
    assert!(
        now > deep,
        "seed {}: the mark never faded: {:.3} to {:.3}",
        seed,
        deep,
        now
    );
}

/// A good nobody else has is worth more than one every town offers.
///
/// A good's price was a constant of the universe, so a world where one
/// river valley grew the only spice and a world where it grew everywhere
/// paid exactly the same for it. The Goods layer could show a reader where
/// the salt was and say nothing at all about whether that mattered.
///
/// Compared between two copies of one world rather than against a formula
/// written out again in the test, which would only check my arithmetic
/// against itself: the same world is priced with the premium and without
/// it, and the roads carrying what is rare must be the ones that gain.
#[test]
fn rarity_is_worth_something() {
    use crate::sim::trade::{Good, GOODS};
    let mut seed_world = World::new(89, 160, 64, Detail::Medium);
    run(&mut seed_world, 600);
    let bytes = ser::save(&mut seed_world);

    let live: Vec<usize> = (0..seed_world.cities.len())
        .filter(|&c| seed_world.cities[c].destroyed.is_none())
        .collect();
    assert!(live.len() > 20, "need a peopled world");
    let offered = |w: &World, g: Good| -> usize {
        live.iter()
            .filter(|&&c| w.city_goods(c).contains(&g))
            .count()
    };
    let counts: Vec<(Good, usize)> = GOODS
        .into_iter()
        .map(|g| (g, offered(&seed_world, g)))
        .collect();
    let common = *counts
        .iter()
        .max_by_key(|&&(_, n)| n)
        .expect("twelve goods");
    let rare = *counts
        .iter()
        .filter(|&&(_, n)| n > 0)
        .min_by_key(|&&(_, n)| n)
        .expect("something is rare");
    assert!(
        common.1 > rare.1 * 2,
        "every good is about as common as every other, so this proves nothing"
    );

    // What the roads touching a given good are worth, on average, in a
    // world priced one way or the other.
    let mean_for = |premium: f32, g: Good| -> f32 {
        let mut w = ser::load(&bytes).expect("a fresh save must load");
        w.tuning.scarcity_premium = premium;
        crate::sim::trade::refresh(&mut w);
        let vals: Vec<f32> = w
            .routes
            .iter()
            .filter(|r| w.city_goods(r.a).contains(&g) || w.city_goods(r.b).contains(&g))
            .map(|r| r.value)
            .collect();
        if vals.is_empty() {
            0.0
        } else {
            vals.iter().sum::<f32>() / vals.len() as f32
        }
    };
    let flat_rare = mean_for(0.0, rare.0);
    let flat_common = mean_for(0.0, common.0);
    let priced_rare = mean_for(seed_world.tuning.scarcity_premium, rare.0);
    let priced_common = mean_for(seed_world.tuning.scarcity_premium, common.0);
    assert!(flat_rare > 0.0 && flat_common > 0.0, "no roads to compare");

    // Both rise, because every good gains something from being scarce
    // somewhere — but what almost nobody has must rise further.
    let lift = |after: f32, before: f32| after / before;
    assert!(
        lift(priced_rare, flat_rare) > lift(priced_common, flat_common),
        "{:?}, offered by {} towns, gained x{:.3}; {:?}, offered by {}, \
         gained x{:.3}",
        rare.0,
        rare.1,
        lift(priced_rare, flat_rare),
        common.0,
        common.1,
        lift(priced_common, flat_common)
    );
}

/// Every fallen realm gets an epitaph, and a standing one gets none.
///
/// A realm's page could say when it ended and not why, though the world
/// records everything needed to say it: how long it stood, how far past its
/// height it was, and what the chronicle remembers of its last years. A
/// reader who has just watched a five-hundred-year empire vanish deserves
/// better than a date.
#[test]
fn a_fallen_realm_is_given_an_epitaph() {
    use crate::sim::explain;
    let mut w = world(97);
    run(&mut w, 700);
    let fallen: Vec<usize> = (0..w.polities.len())
        .filter(|&p| w.polities[p].fell.is_some())
        .collect();
    assert!(
        fallen.len() > 20,
        "a 700 year world should have buried some realms"
    );
    for &p in &fallen {
        let said = explain::epitaph(&w, p).expect("a fallen realm has an epitaph");
        assert!(said.starts_with("It stood for "), "{:?}", said);
        assert!(said.ends_with('.'), "{:?}", said);
        // No sentence may say a realm stood for a negative or absurd span,
        // and none may run past what a page can hold.
        assert!(!said.contains("-"), "{:?}", said);
        assert!(said.len() < 240, "{} characters: {:?}", said.len(), said);
    }
    // And the living are not eulogised.
    for &p in &w.alive_polities {
        assert!(
            explain::epitaph(&w, p).is_none(),
            "realm {} is still standing and has an epitaph",
            p
        );
    }
}
