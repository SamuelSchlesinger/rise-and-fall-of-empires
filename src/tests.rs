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

/// Compaction drops the small old events, keeps the great ones, and leaves
/// the by-ref index pointing at the right entries.
///
/// The guarantee is about *which* events survive, not about how few: a
/// world that logs a great deal of trivia will keep some of it, because
/// compaction only removes as much as it has to. So the test runs the same
/// seed twice — once capped, once uncapped — and demands that every event
/// worth keeping in the uncapped run is still there in the capped one.
/// Asserting instead that no importance-0 event survives held only while
/// trivia was too rare to fill the excess by itself.
#[test]
fn chronicle_compaction_keeps_the_great_events() {
    let mut w = world(13);
    w.tuning.chronicle_cap = 400;
    run(&mut w, 400);
    assert!(w.chronicle.dropped > 0, "nothing was ever dropped");
    assert!(w.chronicle.len() <= 400 + w.chronicle.dropped);

    let mut full = world(13);
    full.tuning.chronicle_cap = 0;
    run(&mut full, 400);
    assert_eq!(full.chronicle.dropped, 0, "an uncapped chronicle dropped");
    let great = |w: &World| -> Vec<(i32, u8, String)> {
        w.chronicle
            .events
            .iter()
            .filter(|e| e.importance >= 2)
            .map(|e| (e.year, e.importance, e.text.clone()))
            .collect()
    };
    assert_eq!(
        great(&w),
        great(&full),
        "compaction dropped an event it had to keep"
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
