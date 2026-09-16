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
    // Every section is there, and the chunks tile the body exactly.
    let cs = chunks(&bytes);
    assert_eq!(cs.len(), 18);
    assert_eq!(cs.last().unwrap().3, bytes.len());
    assert!(cs.iter().any(|c| &c.0 == b"terr"));
    assert!(cs.iter().any(|c| &c.0 == b"chrn"));
}

#[test]
fn v1_fixture_loads_and_runs() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/v1.rfe");
    let bytes = std::fs::read(path).expect("fixture present");
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
