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
    for k in keys {
        let name = config::key_name(k.clone());
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
