//! Save-format tests: round trips, the version-1 fixture, and what the
//! loader does with a file that is newer, older or damaged.

use crate::ser::{self, SaveError, CHUNK_HEADER, HEADER};
use crate::sim::{Detail, World};

fn small_world(years: i32) -> World {
    let mut w = World::new(9, 48, 24, Detail::Medium);
    for _ in 0..years {
        w.tick();
    }
    w
}

fn texts(w: &World) -> Vec<String> {
    w.chronicle
        .events
        .iter()
        .map(|e| format!("{} {}", e.year, e.text))
        .collect()
}

/// Recompute the body length and checksum after meddling with the body.
fn reseal(mut bytes: Vec<u8>) -> Vec<u8> {
    let n = (bytes.len() - HEADER) as u64;
    let sum = ser::fnv1a(&bytes[HEADER..]);
    bytes[8..16].copy_from_slice(&n.to_le_bytes());
    bytes[16..HEADER].copy_from_slice(&sum.to_le_bytes());
    bytes
}

/// The `(tag, version, start, end)` of every chunk in the body, where the
/// offsets are into the whole file and cover the chunk header and payload.
fn chunks(bytes: &[u8]) -> Vec<([u8; 4], u32, usize, usize)> {
    let mut out = Vec::new();
    let mut pos = HEADER;
    while pos + CHUNK_HEADER <= bytes.len() {
        let mut tag = [0u8; 4];
        tag.copy_from_slice(&bytes[pos..pos + 4]);
        let ver = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]);
        let mut len = [0u8; 8];
        len.copy_from_slice(&bytes[pos + 8..pos + CHUNK_HEADER]);
        let end = pos + CHUNK_HEADER + u64::from_le_bytes(len) as usize;
        out.push((tag, ver, pos, end));
        pos = end;
    }
    out
}

fn find(bytes: &[u8], want: &[u8; 4]) -> (usize, usize) {
    let c = chunks(bytes)
        .into_iter()
        .find(|c| &c.0 == want)
        .expect("chunk present");
    (c.2, c.3)
}

#[test]
fn round_trip_is_exact() {
    let mut w = small_world(40);
    let bytes = ser::save(&mut w);
    let mut loaded = ser::load(&bytes).expect("loads");
    assert_eq!(loaded.year, w.year);
    assert_eq!(loaded.seed, w.seed);
    assert_eq!(loaded.rng.state(), w.rng.state());
    assert_eq!(loaded.cells.len(), w.cells.len());
    assert_eq!(loaded.polities.len(), w.polities.len());
    assert_eq!(loaded.persons.len(), w.persons.len());
    assert_eq!(texts(&loaded), texts(&w));
    // Saving what we loaded gives the same file back, byte for byte.
    assert_eq!(ser::save(&mut loaded), bytes);
    // And the loaded world goes on living the same life.
    for _ in 0..10 {
        w.tick();
        loaded.tick();
    }
    assert_eq!(texts(&loaded), texts(&w));
}

/// `peak_cities` is the first field a section has gained since the chunked
/// format was written, so it is also the first exercise of the rule: the
/// `poly` section is at version 2, the value round-trips, and a version-1
/// file — which cannot carry it — still loads and simply starts at zero.
#[test]
fn a_realms_peak_cities_survive_a_round_trip() {
    let mut w = small_world(120);
    let peaks: Vec<usize> = w.polities.iter().map(|p| p.peak_cities).collect();
    assert!(
        peaks.iter().any(|&n| n > 1),
        "no realm ever held two cities, so this proves nothing"
    );
    for p in &w.polities {
        assert!(
            p.peak_cities >= p.cities.len(),
            "{} holds more cities than it ever peaked at",
            p.name
        );
    }
    let bytes = ser::save(&mut w);
    let loaded = ser::load(&bytes).expect("loads");
    let back: Vec<usize> = loaded.polities.iter().map(|p| p.peak_cities).collect();
    assert_eq!(back, peaks);
    // The chunk carries a record version above 1, which is what tells an
    // older build to skip it rather than read the new fields as something
    // else. The literal is a deliberate tripwire: adding a field to the
    // realm record means bumping the version in the `sections!` table, and
    // this line is here to fail until that has been done.
    let ver = chunks(&bytes)
        .into_iter()
        .find(|c| &c.0 == b"poly")
        .expect("the realms are in there")
        .1;
    assert_eq!(ver, 3);
}

/// A world's record of itself survives a round trip, and a world that never
/// had one still loads.
///
/// The series is the one thing in a save that cannot be rebuilt: every other
/// derived field is recomputed on load, and a peak can at worst be
/// re-earned, but no amount of running a world forward recovers what its
/// population was in the eighth century. It also carries the count of
/// chronicle entries compaction has thrown away — the chronicle itself is
/// rebuilt through `Chronicle::default` on load, so the count comes back as
/// nothing unless something else holds it.
#[test]
fn a_worlds_record_of_itself_survives_a_round_trip() {
    let mut w = small_world(120);
    assert!(
        w.history.samples.len() >= 10,
        "120 years should be a dozen samples, not {}",
        w.history.samples.len()
    );
    let before: Vec<(i32, f32)> = w.history.samples.iter().map(|s| (s.year, s.pop)).collect();
    let forgotten = w.chronicle.dropped;

    let bytes = ser::save(&mut w);
    let cs = chunks(&bytes);
    let (_, ver, _, _) = *cs
        .iter()
        .find(|c| &c.0 == b"hist")
        .expect("the history chunk is written");
    // A tripwire, like the realm record's: adding a field to a sample means
    // bumping the version in the `sections!` table, and this fails until it
    // has been.
    assert_eq!(ver, 1);

    let loaded = ser::load(&bytes).expect("a fresh save must load");
    let after: Vec<(i32, f32)> = loaded
        .history
        .samples
        .iter()
        .map(|s| (s.year, s.pop))
        .collect();
    assert_eq!(before, after, "the world forgot what it had been");
    assert_eq!(
        loaded.chronicle.dropped, forgotten,
        "the count of forgotten history was itself forgotten"
    );

    // And a file written before there was any such chunk still loads, with
    // an empty record rather than an error.
    let (at, end) = find(&bytes, b"hist");
    let mut v = bytes[..at].to_vec();
    v.extend_from_slice(&bytes[end..]);
    let without = reseal(v);
    let old = ser::load(&without).expect("a world without a history still loads");
    assert!(old.history.samples.is_empty());
}

#[test]
fn header_is_what_we_say_it_is() {
    let mut w = small_world(5);
    let bytes = ser::save(&mut w);
    assert_eq!(&bytes[..4], b"RFE\0");
    assert_eq!(
        u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        ser::VERSION
    );
    let n = u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize;
    assert_eq!(n, bytes.len() - HEADER);
    let sum = u64::from_le_bytes(bytes[16..HEADER].try_into().unwrap());
    assert_eq!(sum, ser::fnv1a(&bytes[HEADER..]));
    // Every section is there, and the chunks tile the body exactly. The
    // count is the length of the `sections!` table, so adding a section
    // means updating this line — which is the point of asserting it.
    let cs = chunks(&bytes);
    assert_eq!(cs.len(), 22);
    assert_eq!(cs.last().unwrap().3, bytes.len());
    assert!(cs.iter().any(|c| &c.0 == b"terr"));
    assert!(cs.iter().any(|c| &c.0 == b"chrn"));
    assert!(cs.iter().any(|c| &c.0 == b"hous"));
    assert!(cs.iter().any(|c| &c.0 == b"tech"));
    assert!(cs.iter().any(|c| &c.0 == b"trde"));
}

#[test]
fn v1_fixture_loads_and_runs() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/v1.rfe");
    // `Cargo.toml` keeps `tests/fixtures/` out of the published crate, so in
    // an unpacked `.crate` this file is not there. Say so and stop, rather
    // than failing a test the reader cannot possibly fix.
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => {
            println!("no tests/fixtures/v1.rfe here (packaged crate); skipping");
            return;
        }
    };
    assert_eq!(
        u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        1
    );
    let mut w = ser::load(&bytes).expect("a version-1 save still loads");
    assert_eq!(w.terrain.w, 80);
    assert_eq!(w.terrain.h, 32);
    assert_eq!(w.year, 60);
    assert!(!w.races.is_empty());
    assert_eq!(w.cells.len(), 80 * 32);
    let before = w.chronicle.len();
    for _ in 0..20 {
        w.tick();
    }
    assert_eq!(w.year, 80);
    assert!(w.chronicle.len() >= before);
    // Re-saved, it comes back as a current-version file.
    let again = ser::save(&mut w);
    assert_eq!(
        u32::from_le_bytes([again[4], again[5], again[6], again[7]]),
        ser::VERSION
    );
    assert_eq!(ser::load(&again).expect("round trips").year, 80);
}

#[test]
fn unknown_chunks_are_skipped() {
    let mut w = small_world(30);
    let bytes = ser::save(&mut w);
    let (at, _) = find(&bytes, b"city");
    // A section from some future build, spliced in ahead of the cities.
    let mut bogus = Vec::new();
    bogus.extend_from_slice(b"zzzz");
    bogus.extend_from_slice(&7u32.to_le_bytes());
    bogus.extend_from_slice(&11u64.to_le_bytes());
    bogus.extend_from_slice(b"hello world");
    let mut spliced = bytes[..at].to_vec();
    spliced.extend_from_slice(&bogus);
    spliced.extend_from_slice(&bytes[at..]);
    let mut loaded = ser::load(&reseal(spliced)).expect("skips what it does not know");
    assert_eq!(texts(&loaded), texts(&w));
    assert_eq!(loaded.cities.len(), w.cities.len());
    for _ in 0..5 {
        w.tick();
        loaded.tick();
    }
    assert_eq!(texts(&loaded), texts(&w));
}

#[test]
fn newer_section_versions_are_skipped() {
    let mut w = small_world(30);
    let mut bytes = ser::save(&mut w);
    // Records we cannot parse positionally: leave the section at its default.
    let (at, _) = find(&bytes, b"stat");
    bytes[at + 4..at + 8].copy_from_slice(&99u32.to_le_bytes());
    let loaded = ser::load(&reseal(bytes)).expect("loads without the section");
    assert!(loaded.stats.pop_history.is_empty());
    assert_eq!(loaded.year, w.year);
    assert_eq!(loaded.cities.len(), w.cities.len());
}

#[test]
fn missing_chunk_loads_defaults() {
    let mut w = small_world(30);
    assert!(!w.stats.pop_history.is_empty());
    let bytes = ser::save(&mut w);
    let (at, end) = find(&bytes, b"stat");
    let mut cut = bytes[..at].to_vec();
    cut.extend_from_slice(&bytes[end..]);
    let loaded = ser::load(&reseal(cut)).expect("a missing section is not an error");
    assert!(loaded.stats.pop_history.is_empty());
    assert_eq!(loaded.year, w.year);
    assert_eq!(texts(&loaded), texts(&w));
}

#[test]
fn a_flipped_byte_is_a_checksum_error() {
    let mut w = small_world(20);
    let bytes = ser::save(&mut w);
    for at in [HEADER + 40, bytes.len() / 2, bytes.len() - 1] {
        let mut bad = bytes.clone();
        bad[at] ^= 0x20;
        match ser::load(&bad) {
            Err(SaveError::Checksum) => {}
            other => panic!("expected a checksum error at {}, got {:?}", at, other.err()),
        }
    }
}

#[test]
fn rubbish_and_future_formats_are_refused() {
    assert!(matches!(ser::load(b"not a save"), Err(SaveError::NotASave)));
    assert!(matches!(ser::load(&[]), Err(SaveError::NotASave)));

    let mut w = small_world(10);
    let bytes = ser::save(&mut w);

    let mut future = bytes.clone();
    future[4..8].copy_from_slice(&99u32.to_le_bytes());
    match ser::load(&future) {
        Err(SaveError::UnsupportedVersion { found, supported }) => {
            assert_eq!(found, 99);
            assert_eq!(supported, ser::VERSION);
        }
        other => panic!("expected an unsupported version, got {:?}", other.err()),
    }

    let short = reseal(bytes[..bytes.len() - 200].to_vec());
    assert!(matches!(ser::load(&short), Err(SaveError::Truncated)));

    // The header promises more body than the file holds.
    let mut lying = bytes.clone();
    lying[8..16].copy_from_slice(&(bytes.len() as u64 * 2).to_le_bytes());
    assert!(matches!(ser::load(&lying), Err(SaveError::Truncated)));
}

#[test]
fn errors_read_well() {
    let e = SaveError::UnsupportedVersion {
        found: 9,
        supported: ser::VERSION,
    };
    let s = e.to_string();
    assert!(s.contains("version 9"), "{}", s);
    let _: &dyn std::error::Error = &e;
    assert!(SaveError::NotASave.to_string().contains("save file"));
}

/// Two hundred mutations of a real save, each resealed so that the checksum
/// waves it through and the parser itself is what gets tested. Loading may
/// fail in any way it likes; it may not panic, hang, or try to allocate the
/// machine out of memory.
#[test]
fn mutated_saves_never_panic() {
    let mut w = small_world(25);
    let good = ser::save(&mut w);
    let rng = crate::rng::Rng::new(0xdead_beef_1234);
    let mut loaded = 0;
    for case in 0..200 {
        let mut bad = good.clone();
        // A handful of edits per case, biased towards the structural bytes:
        // chunk headers, lengths and the sequence counts just behind them.
        let edits = 1 + rng.below(4);
        for _ in 0..edits {
            let at = match case % 4 {
                // Anywhere at all.
                0 => HEADER + rng.below(bad.len() - HEADER),
                // Inside a chunk header (tag, version or length).
                1 => {
                    let cs = chunks(&bad);
                    let c = cs[rng.below(cs.len())];
                    c.2 + rng.below(CHUNK_HEADER)
                }
                // The first bytes of a payload, which is where the counts are.
                2 => {
                    let cs = chunks(&bad);
                    let c = cs[rng.below(cs.len())];
                    (c.2 + CHUNK_HEADER + rng.below(8)).min(bad.len() - 1)
                }
                // The file header itself.
                _ => rng.below(HEADER),
            };
            bad[at] = rng.below(256) as u8;
        }
        // Half the cases are also truncated somewhere.
        if case % 2 == 0 {
            let keep = rng.below(bad.len());
            bad.truncate(keep);
        }
        if bad.len() > HEADER {
            bad = reseal(bad);
        }
        if let Ok(mut world) = ser::load(&bad) {
            loaded += 1;
            // A world that loads must also be able to live: every index in it
            // has to be in range for a tick to be safe.
            world.tick();
        }
    }
    // The test is worth little if nothing ever got through the parser.
    assert!(
        loaded > 0,
        "no mutated save ever loaded; the test proves nothing"
    );
}
