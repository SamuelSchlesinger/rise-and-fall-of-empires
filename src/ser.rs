//! Save files. A single symmetric `Io` trait drives both writing and
//! reading, so every field is visited in the same order in both directions
//! and the format cannot drift between the two.
//!
//! # The format
//!
//! ```text
//! magic "RFE\0" | format version u32 | body length u64 | FNV-1a 64 of the body
//! body: chunk* where chunk = tag u32 | record version u32 | length u64 | payload
//! ```
//!
//! The body is a sequence of tagged, length-prefixed chunks, one per
//! top-level section of the world (see the `sections!` table). A reader
//! takes the chunks it knows by tag, skips the rest, and leaves a section
//! that never turned up at its blank default — so an old build can read a
//! file with new sections in it, and a new build can read a file without
//! them.
//!
//! Inside a chunk the fields of a record stay positional, which is what
//! keeps the file small and the loop fast. Instead each section carries a
//! record version, written once in the chunk header, which the reader hands
//! back through `s.ver()`.
//!
//! **To add a field: bump that section's version in the `sections!` table,
//! write it unconditionally, read it conditionally.**
//!
//! ```ignore
//! fn city<S: Io>(s: &mut S, c: &mut City) {
//!     s.string(&mut c.name);
//!     if s.ver() >= 2 {
//!         s.f32(&mut c.harbour); // absent from version-1 files: stays default
//!     }
//! }
//! ```
//!
//! A reader that meets a *newer* version of a section it knows cannot parse
//! it positionally, so it skips that chunk too and keeps the defaults. The
//! format version itself only changes when the framing changes; version 1
//! (a bare positional body, no chunks) is still read by `read_v1`.

use crate::geo::{Biome, Feature, FeatureKind, Terrain};
use crate::lang::Language;
use crate::sim::chronicle::{Chronicle, Event, EventKind, Ref};
use crate::sim::*;
use std::collections::BTreeMap;

const MAGIC: &[u8; 4] = b"RFE\0";
/// The format this build writes.
pub const VERSION: u32 = 2;
/// The oldest format it still reads.
pub const MIN_VERSION: u32 = 1;
/// Magic, format version, body length, checksum.
pub const HEADER: usize = 24;
/// Tag, record version, payload length.
pub const CHUNK_HEADER: usize = 16;

/// One traversal of a world's fields, used both to write and to read: each
/// section is described once, and [`Writer`] and [`Reader`] play it in
/// opposite directions.
pub trait Io {
    /// True when this traversal is filling the world in from bytes.
    fn reading(&self) -> bool;
    /// The record version of the section being visited: the current one when
    /// writing, the one the file carries when reading. No record needs it
    /// yet — the first one to gain a field will (see the module docs).
    #[allow(dead_code)]
    fn ver(&self) -> u32;
    /// Visit one byte.
    fn u8(&mut self, v: &mut u8);
    /// Visit a length-prefixed run of bytes.
    fn bytes(&mut self, v: &mut Vec<u8>);
    /// Visit a little-endian `u32`.
    fn u32(&mut self, v: &mut u32) {
        let mut b = v.to_le_bytes().to_vec();
        self.fixed(&mut b, 4);
        *v = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    }
    /// Visit a little-endian `u64`.
    fn u64(&mut self, v: &mut u64) {
        let mut b = v.to_le_bytes().to_vec();
        self.fixed(&mut b, 8);
        *v = u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
    }
    /// Visit exactly `n` bytes, with no length prefix.
    fn fixed(&mut self, b: &mut Vec<u8>, n: usize);
    /// Visit an `i32`, two's complement.
    fn i32(&mut self, v: &mut i32) {
        let mut u = *v as u32;
        self.u32(&mut u);
        *v = u as i32;
    }
    /// Visit an `f32` by its bits, so the value survives exactly.
    fn f32(&mut self, v: &mut f32) {
        let mut u = v.to_bits();
        self.u32(&mut u);
        *v = f32::from_bits(u);
    }
    /// Visit an `f64` by its bits, so the value survives exactly.
    fn f64(&mut self, v: &mut f64) {
        let mut u = v.to_bits();
        self.u64(&mut u);
        *v = f64::from_bits(u);
    }
    /// Visit a `usize`, on the wire as a `u64`.
    fn usize(&mut self, v: &mut usize) {
        let mut u = *v as u64;
        self.u64(&mut u);
        *v = u as usize;
    }
    /// Visit a `bool`, on the wire as one byte.
    fn bool(&mut self, v: &mut bool) {
        let mut b = *v as u8;
        self.u8(&mut b);
        *v = b != 0;
    }
    /// Visit a length-prefixed UTF-8 string; invalid bytes are replaced.
    fn string(&mut self, v: &mut String) {
        let mut b = v.as_bytes().to_vec();
        self.bytes(&mut b);
        *v = String::from_utf8_lossy(&b).into_owned();
    }
}

/// An [`Io`] that appends to a buffer.
pub struct Writer {
    /// The bytes written so far.
    pub buf: Vec<u8>,
    ver: u32,
}

impl Io for Writer {
    fn reading(&self) -> bool {
        false
    }
    fn ver(&self) -> u32 {
        self.ver
    }
    fn u8(&mut self, v: &mut u8) {
        self.buf.push(*v);
    }
    fn fixed(&mut self, b: &mut Vec<u8>, _n: usize) {
        self.buf.extend_from_slice(b);
    }
    fn bytes(&mut self, v: &mut Vec<u8>) {
        let mut n = v.len() as u32;
        self.u32(&mut n);
        self.buf.extend_from_slice(v);
    }
}

/// An [`Io`] that consumes a buffer. It never panics on damaged input: it
/// sets `err` and hands out zeroes from then on.
pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
    /// Set once the reader has run off the end of the buffer.
    pub err: bool,
    ver: u32,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> &'a [u8] {
        if self.pos + n > self.buf.len() {
            self.err = true;
            self.pos = self.buf.len();
            return &[];
        }
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        s
    }
}

impl<'a> Io for Reader<'a> {
    fn reading(&self) -> bool {
        true
    }
    fn ver(&self) -> u32 {
        self.ver
    }
    fn u8(&mut self, v: &mut u8) {
        let s = self.take(1);
        *v = s.first().copied().unwrap_or(0);
    }
    fn fixed(&mut self, b: &mut Vec<u8>, n: usize) {
        let s = self.take(n);
        b.clear();
        b.extend_from_slice(s);
        b.resize(n, 0);
    }
    fn bytes(&mut self, v: &mut Vec<u8>) {
        let mut n = 0u32;
        self.u32(&mut n);
        let n = (n as usize).min(self.buf.len().saturating_sub(self.pos));
        *v = self.take(n).to_vec();
    }
}

// -- helpers ------------------------------------------------------------------

fn opt<S: Io, T: Default>(s: &mut S, v: &mut Option<T>, f: impl Fn(&mut S, &mut T)) {
    let mut has = v.is_some();
    s.bool(&mut has);
    if s.reading() {
        if has {
            let mut t = T::default();
            f(s, &mut t);
            *v = Some(t);
        } else {
            *v = None;
        }
    } else if let Some(t) = v.as_mut() {
        f(s, t);
    }
}

fn opt_usize<S: Io>(s: &mut S, v: &mut Option<usize>) {
    opt(s, v, Io::usize);
}
fn opt_i32<S: Io>(s: &mut S, v: &mut Option<i32>) {
    opt(s, v, Io::i32);
}
fn opt_string<S: Io>(s: &mut S, v: &mut Option<String>) {
    opt(s, v, Io::string);
}
fn opt_bool<S: Io>(s: &mut S, v: &mut Option<bool>) {
    opt(s, v, Io::bool);
}

fn seq<S: Io, T>(s: &mut S, v: &mut Vec<T>, blank: impl Fn() -> T, f: impl Fn(&mut S, &mut T)) {
    let mut n = v.len() as u32;
    s.u32(&mut n);
    if s.reading() {
        v.clear();
        let n = (n as usize).min(50_000_000);
        v.reserve(n);
        for _ in 0..n {
            let mut t = blank();
            f(s, &mut t);
            v.push(t);
        }
    } else {
        for t in v.iter_mut() {
            f(s, t);
        }
    }
}

fn vec_usize<S: Io>(s: &mut S, v: &mut Vec<usize>) {
    seq(s, v, || 0, Io::usize);
}
fn vec_string<S: Io>(s: &mut S, v: &mut Vec<String>) {
    seq(s, v, String::new, Io::string);
}
fn vec_f32<S: Io>(s: &mut S, v: &mut Vec<f32>) {
    seq(s, v, || 0.0, Io::f32);
}
fn vec_f64<S: Io>(s: &mut S, v: &mut Vec<f64>) {
    seq(s, v, || 0.0, Io::f64);
}
fn vec_u8<S: Io>(s: &mut S, v: &mut Vec<u8>) {
    s.bytes(v);
}
fn vec_u16<S: Io>(s: &mut S, v: &mut Vec<u16>) {
    let mut b: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    s.bytes(&mut b);
    if s.reading() {
        *v = b
            .as_chunks::<2>()
            .0
            .iter()
            .copied()
            .map(u16::from_le_bytes)
            .collect();
    }
}
fn vec_u32<S: Io>(s: &mut S, v: &mut Vec<u32>) {
    seq(s, v, || 0, Io::u32);
}
fn vec_bool<S: Io>(s: &mut S, v: &mut Vec<bool>) {
    let mut b: Vec<u8> = v.iter().map(|&x| x as u8).collect();
    s.bytes(&mut b);
    if s.reading() {
        *v = b.into_iter().map(|x| x != 0).collect();
    }
}

fn map<S: Io, V: Default + Copy>(
    s: &mut S,
    m: &mut BTreeMap<usize, V>,
    f: impl Fn(&mut S, &mut V),
) {
    let mut pairs: Vec<(usize, V)> = m.iter().map(|(&k, &v)| (k, v)).collect();
    pairs.sort_by_key(|p| p.0);
    seq(
        s,
        &mut pairs,
        || (0, V::default()),
        |s, (k, v)| {
            s.usize(k);
            f(s, v);
        },
    );
    if s.reading() {
        *m = pairs.into_iter().collect();
    }
}

fn enum8<S: Io, T: Copy + PartialEq>(s: &mut S, v: &mut T, all: &[T]) {
    let mut i = all.iter().position(|x| *x == *v).unwrap_or(0) as u8;
    s.u8(&mut i);
    if s.reading() {
        *v = all[(i as usize).min(all.len() - 1)];
    }
}

const BIOMES: [Biome; 18] = [
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
const FEATURE_KINDS: [FeatureKind; 13] = [
    FeatureKind::Ocean,
    FeatureKind::Sea,
    FeatureKind::Lake,
    FeatureKind::Range,
    FeatureKind::Forest,
    FeatureKind::Jungle,
    FeatureKind::Desert,
    FeatureKind::Marsh,
    FeatureKind::Steppe,
    FeatureKind::Island,
    FeatureKind::Continent,
    FeatureKind::River,
    FeatureKind::Nexus,
];
const POLITY_KINDS: [PolityKind; 8] = [
    PolityKind::Tribe,
    PolityKind::Chiefdom,
    PolityKind::Kingdom,
    PolityKind::Empire,
    PolityKind::Republic,
    PolityKind::Theocracy,
    PolityKind::Magocracy,
    PolityKind::Horde,
];
const ROLES: [Role; 9] = [
    Role::Ruler,
    Role::General,
    Role::Mage,
    Role::Prophet,
    Role::Philosopher,
    Role::Poet,
    Role::Rebel,
    Role::Explorer,
    Role::Martyr,
];
const GENDERS: [Gender; 3] = [Gender::F, Gender::M, Gender::N];
const SCHOOL_KINDS: [SchoolKind; 3] = [
    SchoolKind::Arcane,
    SchoolKind::Divine,
    SchoolKind::Philosophical,
];
const WAR_KINDS: [WarKind; 6] = [
    WarKind::Conquest,
    WarKind::Rebellion,
    WarKind::CivilWar,
    WarKind::Holy,
    WarKind::Raid,
    WarKind::Succession,
];
const EVENT_KINDS: [EventKind; 14] = [
    EventKind::Genesis,
    EventKind::Founding,
    EventKind::Politics,
    EventKind::Death,
    EventKind::War,
    EventKind::Battle,
    EventKind::Peace,
    EventKind::Disaster,
    EventKind::Magic,
    EventKind::Culture,
    EventKind::Era,
    EventKind::Wonder,
    EventKind::Discovery,
    EventKind::Person,
];
const DETAILS: [Detail; 3] = [Detail::Low, Detail::Medium, Detail::High];

fn rgb<S: Io>(s: &mut S, c: &mut crate::term::Rgb) {
    s.u8(&mut c.0);
    s.u8(&mut c.1);
    s.u8(&mut c.2);
}

fn reff<S: Io>(s: &mut S, r: &mut Ref) {
    let (mut tag, mut id) = match *r {
        Ref::Polity(i) => (0u8, i),
        Ref::City(i) => (1, i),
        Ref::Culture(i) => (2, i),
        Ref::Person(i) => (3, i),
        Ref::School(i) => (4, i),
        Ref::War(i) => (5, i),
        Ref::Race(i) => (6, i),
        Ref::Feature(i) => (7, i),
        Ref::Artifact(i) => (8, i),
    };
    s.u8(&mut tag);
    s.usize(&mut id);
    if s.reading() {
        *r = match tag {
            0 => Ref::Polity(id),
            1 => Ref::City(id),
            2 => Ref::Culture(id),
            3 => Ref::Person(id),
            4 => Ref::School(id),
            5 => Ref::War(id),
            6 => Ref::Race(id),
            7 => Ref::Feature(id),
            _ => Ref::Artifact(id),
        };
    }
}

fn language<S: Io>(s: &mut S, l: &mut Language) {
    s.string(&mut l.name);
    vec_string(s, &mut l.onsets);
    vec_f64(s, &mut l.onset_w);
    vec_string(s, &mut l.vowels);
    vec_f64(s, &mut l.vowel_w);
    vec_string(s, &mut l.codas);
    vec_f64(s, &mut l.coda_w);
    s.f64(&mut l.coda_chance);
    s.f64(&mut l.onset_chance);
    s.usize(&mut l.min_syl);
    s.usize(&mut l.max_syl);
    vec_string(s, &mut l.place_suffixes);
    s.f64(&mut l.place_suffix_chance);
    s.f64(&mut l.compound_chance);
    s.bool(&mut l.hyphen);
    s.f64(&mut l.apostrophe_chance);
    vec_string(s, &mut l.adj_suffixes);
    vec_string(s, &mut l.demonym_suffixes);
    s.string(&mut l.honorific);
}

fn terrain<S: Io>(s: &mut S, t: &mut Terrain) {
    s.usize(&mut t.w);
    s.usize(&mut t.h);
    vec_f32(s, &mut t.elev);
    s.f32(&mut t.sea);
    vec_f32(s, &mut t.temp);
    vec_f32(s, &mut t.moist);
    seq(
        s,
        &mut t.biome,
        || Biome::Ocean,
        |s, b| enum8(s, b, &BIOMES),
    );
    vec_u8(s, &mut t.river);
    vec_u32(s, &mut t.flow);
    vec_f32(s, &mut t.fertility);
    vec_f32(s, &mut t.minerals);
    vec_f32(s, &mut t.mana);
    vec_bool(s, &mut t.coast);
    vec_u16(s, &mut t.region);
    vec_u16(s, &mut t.river_feat);
    vec_u16(s, &mut t.landmass);
    seq(
        s,
        &mut t.features,
        || Feature {
            kind: FeatureKind::Sea,
            name: None,
            named_by: None,
            cells: Vec::new(),
            center: (0, 0),
        },
        |s, f| {
            enum8(s, &mut f.kind, &FEATURE_KINDS);
            opt_string(s, &mut f.name);
            opt_usize(s, &mut f.named_by);
            vec_usize(s, &mut f.cells);
            s.usize(&mut f.center.0);
            s.usize(&mut f.center.1);
        },
    );
    s.usize(&mut t.land_count);
}

fn values<S: Io>(s: &mut S, v: &mut Values) {
    s.f32(&mut v.militarism);
    s.f32(&mut v.mysticism);
    s.f32(&mut v.mercantilism);
    s.f32(&mut v.tradition);
    s.f32(&mut v.openness);
}

fn traits<S: Io>(s: &mut S, t: &mut Traits) {
    s.f32(&mut t.ambition);
    s.f32(&mut t.valor);
    s.f32(&mut t.wisdom);
    s.f32(&mut t.piety);
    s.f32(&mut t.cruelty);
    s.f32(&mut t.charisma);
}

fn blank_values() -> Values {
    Values {
        militarism: 0.5,
        mysticism: 0.5,
        mercantilism: 0.5,
        tradition: 0.5,
        openness: 0.5,
    }
}
fn blank_traits() -> Traits {
    Traits {
        ambition: 0.5,
        valor: 0.5,
        wisdom: 0.5,
        piety: 0.5,
        cruelty: 0.5,
        charisma: 0.5,
    }
}

fn race<S: Io>(s: &mut S, r: &mut Race) {
    s.usize(&mut r.id);
    s.string(&mut r.name);
    s.string(&mut r.adj);
    s.string(&mut r.plural);
    language(s, &mut r.lang);
    s.f32(&mut r.lifespan);
    for a in r.affinity.iter_mut() {
        s.f32(a);
    }
    s.f32(&mut r.coast_love);
    s.f32(&mut r.martial);
    s.f32(&mut r.mystic);
    s.f32(&mut r.mercantile);
    s.f32(&mut r.fecund);
    s.f32(&mut r.seafaring);
    s.string(&mut r.description);
    s.usize(&mut r.home);
}

fn blank_race() -> Race {
    Race {
        id: 0,
        name: String::new(),
        adj: String::new(),
        plural: String::new(),
        lang: Language::blank(),
        lifespan: 70.0,
        affinity: [0.5; crate::geo::GROUP_COUNT],
        coast_love: 0.5,
        martial: 0.5,
        mystic: 0.5,
        mercantile: 0.5,
        fecund: 0.5,
        seafaring: 0.5,
        description: String::new(),
        home: 0,
    }
}

fn culture<S: Io>(s: &mut S, c: &mut Culture) {
    s.usize(&mut c.id);
    s.string(&mut c.name);
    s.string(&mut c.adj);
    s.string(&mut c.plural);
    s.usize(&mut c.race);
    language(s, &mut c.lang);
    opt_usize(s, &mut c.parent);
    s.i32(&mut c.founded);
    values(s, &mut c.values);
    s.usize(&mut c.home);
    rgb(s, &mut c.color);
    opt_i32(s, &mut c.extinct);
    s.usize(&mut c.cells);
    s.f64(&mut c.pop);
    s.i32(&mut c.last_seen);
}

fn blank_culture() -> Culture {
    Culture {
        id: 0,
        name: String::new(),
        adj: String::new(),
        plural: String::new(),
        race: 0,
        lang: Language::blank(),
        parent: None,
        founded: 0,
        values: blank_values(),
        home: 0,
        color: crate::term::Rgb(0, 0, 0),
        extinct: None,
        cells: 0,
        pop: 0.0,
        last_seen: 0,
    }
}

fn city<S: Io>(s: &mut S, c: &mut City) {
    s.usize(&mut c.id);
    s.string(&mut c.name);
    s.usize(&mut c.cell);
    s.i32(&mut c.founded);
    s.usize(&mut c.culture);
    opt_usize(s, &mut c.polity);
    s.f32(&mut c.pop);
    s.f32(&mut c.prosperity);
    s.f32(&mut c.walls);
    opt_i32(s, &mut c.destroyed);
    vec_string(s, &mut c.wonders);
    s.u32(&mut c.times_sacked);
    opt_usize(s, &mut c.founder);
    s.f32(&mut c.peak_pop);
}

fn blank_city() -> City {
    City {
        id: 0,
        name: String::new(),
        cell: 0,
        founded: 0,
        culture: 0,
        polity: None,
        pop: 0.0,
        prosperity: 0.0,
        walls: 0.0,
        destroyed: None,
        wonders: Vec::new(),
        times_sacked: 0,
        founder: None,
        peak_pop: 0.0,
    }
}

fn polity<S: Io>(s: &mut S, p: &mut Polity) {
    s.usize(&mut p.id);
    s.string(&mut p.name);
    s.string(&mut p.short);
    s.string(&mut p.adj);
    enum8(s, &mut p.kind, &POLITY_KINDS);
    s.usize(&mut p.culture);
    opt_usize(s, &mut p.capital);
    opt_usize(s, &mut p.ruler);
    s.string(&mut p.dynasty);
    s.i32(&mut p.founded);
    opt_i32(s, &mut p.fell);
    s.string(&mut p.fall_cause);
    opt_usize(s, &mut p.parent);
    rgb(s, &mut p.color);
    s.usize(&mut p.cells);
    s.f64(&mut p.pop);
    vec_usize(s, &mut p.cities);
    s.usize(&mut p.peak_cells);
    s.i32(&mut p.peak_year);
    s.f32(&mut p.stability);
    s.f32(&mut p.treasury);
    s.f32(&mut p.army);
    s.f32(&mut p.dev);
    s.f32(&mut p.prestige);
    s.f32(&mut p.exhaustion);
    s.f32(&mut p.decadence);
    s.bool(&mut p.seafaring);
    opt_usize(s, &mut p.school);
    vec_usize(s, &mut p.wars);
    map(s, &mut p.tension, Io::f32);
    seq(
        s,
        &mut p.neighbors,
        || (0, 0),
        |s, (q, l)| {
            s.usize(q);
            s.u32(l);
        },
    );
    s.u32(&mut p.conquered);
    s.i32(&mut p.reign_start);
    s.i32(&mut p.reign_gained);
    s.u32(&mut p.reign_cities);
    s.u32(&mut p.reign_wars_won);
    s.i32(&mut p.last_revolt);
    s.f32(&mut p.foreign_share);
    s.f32(&mut p.avg_fertility);
    s.usize(&mut p.cultures_within);
    vec_usize(s, &mut p.rulers);
    vec_usize(s, &mut p.generals);
    s.i32(&mut p.last_kind_change);
    map(s, &mut p.culture_counts, Io::u32);
    map(s, &mut p.truce, Io::i32);
}

fn blank_polity() -> Polity {
    Polity {
        id: 0,
        name: String::new(),
        short: String::new(),
        adj: String::new(),
        kind: PolityKind::Tribe,
        culture: 0,
        capital: None,
        ruler: None,
        dynasty: String::new(),
        founded: 0,
        fell: None,
        fall_cause: String::new(),
        parent: None,
        color: crate::term::Rgb(0, 0, 0),
        cells: 0,
        pop: 0.0,
        cities: Vec::new(),
        peak_cells: 0,
        peak_year: 0,
        stability: 0.5,
        treasury: 0.0,
        army: 0.0,
        dev: 0.0,
        prestige: 0.0,
        exhaustion: 0.0,
        decadence: 0.0,
        seafaring: false,
        school: None,
        wars: Vec::new(),
        tension: BTreeMap::new(),
        neighbors: Vec::new(),
        conquered: 0,
        reign_start: 0,
        reign_gained: 0,
        reign_cities: 0,
        reign_wars_won: 0,
        last_revolt: 0,
        foreign_share: 0.0,
        avg_fertility: 0.0,
        cultures_within: 0,
        rulers: Vec::new(),
        generals: Vec::new(),
        last_kind_change: 0,
        culture_counts: BTreeMap::new(),
        truce: BTreeMap::new(),
    }
}

fn person<S: Io>(s: &mut S, p: &mut Person) {
    s.usize(&mut p.id);
    s.string(&mut p.name);
    opt_string(s, &mut p.epithet);
    enum8(s, &mut p.gender, &GENDERS);
    s.usize(&mut p.culture);
    s.usize(&mut p.race);
    s.i32(&mut p.born);
    opt_i32(s, &mut p.died);
    s.string(&mut p.death);
    enum8(s, &mut p.role, &ROLES);
    opt_usize(s, &mut p.polity);
    opt_usize(s, &mut p.school);
    opt_usize(s, &mut p.city);
    traits(s, &mut p.traits);
    s.f32(&mut p.renown);
    opt_usize(s, &mut p.parent);
    s.u32(&mut p.battles_won);
}

fn blank_person() -> Person {
    Person {
        id: 0,
        name: String::new(),
        epithet: None,
        gender: Gender::N,
        culture: 0,
        race: 0,
        born: 0,
        died: None,
        death: String::new(),
        role: Role::Ruler,
        polity: None,
        school: None,
        city: None,
        traits: blank_traits(),
        renown: 0.0,
        parent: None,
        battles_won: 0,
    }
}

fn school<S: Io>(s: &mut S, c: &mut School) {
    s.usize(&mut c.id);
    s.string(&mut c.name);
    s.string(&mut c.short);
    enum8(s, &mut c.kind, &SCHOOL_KINDS);
    s.string(&mut c.doctrine);
    vec_string(s, &mut c.tenets);
    s.usize(&mut c.aspect);
    s.usize(&mut c.practice);
    s.usize(&mut c.founder);
    s.i32(&mut c.founded);
    s.usize(&mut c.home_city);
    opt_usize(s, &mut c.parent);
    map(s, &mut c.influence, Io::f32);
    opt_i32(s, &mut c.extinct);
    rgb(s, &mut c.color);
    s.f32(&mut c.hostility);
    s.usize(&mut c.peak_polities);
    opt_i32(s, &mut c.fading_since);
    vec_usize(s, &mut c.state_of);
}

fn blank_school() -> School {
    School {
        id: 0,
        name: String::new(),
        short: String::new(),
        kind: SchoolKind::Arcane,
        doctrine: String::new(),
        tenets: Vec::new(),
        aspect: 0,
        practice: 0,
        founder: 0,
        founded: 0,
        home_city: 0,
        parent: None,
        influence: BTreeMap::new(),
        extinct: None,
        color: crate::term::Rgb(0, 0, 0),
        hostility: 0.0,
        peak_polities: 0,
        fading_since: None,
        state_of: Vec::new(),
    }
}

fn war<S: Io>(s: &mut S, x: &mut War) {
    s.usize(&mut x.id);
    s.usize(&mut x.attacker);
    s.usize(&mut x.defender);
    s.i32(&mut x.started);
    opt_i32(s, &mut x.ended);
    s.string(&mut x.cause);
    s.string(&mut x.name);
    s.f32(&mut x.score);
    s.u32(&mut x.battles);
    s.string(&mut x.result);
    s.i32(&mut x.cells_taken);
    enum8(s, &mut x.kind, &WAR_KINDS);
}

fn blank_war() -> War {
    War {
        id: 0,
        attacker: 0,
        defender: 0,
        started: 0,
        ended: None,
        cause: String::new(),
        name: String::new(),
        score: 0.0,
        battles: 0,
        result: String::new(),
        cells_taken: 0,
        kind: WarKind::Conquest,
    }
}

fn cell<S: Io>(s: &mut S, c: &mut CellState) {
    opt_usize(s, &mut c.owner);
    opt_usize(s, &mut c.culture);
    s.f32(&mut c.pop);
    opt_usize(s, &mut c.city);
    s.i32(&mut c.since);
    s.u8(&mut c.plague);
}

fn holder<S: Io>(s: &mut S, h: &mut Holder) {
    let (mut tag, mut id) = match *h {
        Holder::Polity(i) => (0u8, i),
        Holder::Person(i) => (1, i),
        Holder::City(i) => (2, i),
        Holder::Lost => (3, 0),
    };
    s.u8(&mut tag);
    s.usize(&mut id);
    if s.reading() {
        *h = match tag {
            0 => Holder::Polity(id),
            1 => Holder::Person(id),
            2 => Holder::City(id),
            _ => Holder::Lost,
        };
    }
}

const ARTIFACT_KINDS: [ArtifactKind; 10] = [
    ArtifactKind::Crown,
    ArtifactKind::Blade,
    ArtifactKind::Tome,
    ArtifactKind::Gem,
    ArtifactKind::Banner,
    ArtifactKind::Staff,
    ArtifactKind::Chalice,
    ArtifactKind::Horn,
    ArtifactKind::Mirror,
    ArtifactKind::Shard,
];

fn artifact<S: Io>(s: &mut S, a: &mut Artifact) {
    s.usize(&mut a.id);
    s.string(&mut a.name);
    enum8(s, &mut a.kind, &ARTIFACT_KINDS);
    s.i32(&mut a.made);
    opt_usize(s, &mut a.maker);
    opt_usize(s, &mut a.origin);
    holder(s, &mut a.holder);
    s.f32(&mut a.power);
    s.string(&mut a.description);
    opt_usize(s, &mut a.lost_at);
    s.u32(&mut a.hands);
}

fn blank_artifact() -> Artifact {
    Artifact {
        id: 0,
        name: String::new(),
        kind: ArtifactKind::Crown,
        made: 0,
        maker: None,
        origin: None,
        holder: Holder::Lost,
        power: 0.0,
        description: String::new(),
        lost_at: None,
        hands: 0,
    }
}

fn prophecy_kind<S: Io>(s: &mut S, k: &mut ProphecyKind) {
    let (mut tag, mut a, mut b) = match *k {
        ProphecyKind::RealmFalls(q) => (0u8, q, 0usize),
        ProphecyKind::CrownOfEmpire(q) => (1, q, 0),
        ProphecyKind::CityBurns(c, n) => (2, c, n as usize),
        ProphecyKind::RulerMurdered(q) => (3, q, 0),
        ProphecyKind::FaithSpreads(x, n) => (4, x, n),
        ProphecyKind::RelicReturns(x, q) => (5, x, q),
    };
    s.u8(&mut tag);
    s.usize(&mut a);
    s.usize(&mut b);
    if s.reading() {
        *k = match tag {
            0 => ProphecyKind::RealmFalls(a),
            1 => ProphecyKind::CrownOfEmpire(a),
            2 => ProphecyKind::CityBurns(a, b as u32),
            3 => ProphecyKind::RulerMurdered(a),
            4 => ProphecyKind::FaithSpreads(a, b),
            _ => ProphecyKind::RelicReturns(a, b),
        };
    }
}

fn prophecy<S: Io>(s: &mut S, p: &mut Prophecy) {
    s.usize(&mut p.id);
    s.usize(&mut p.seer);
    s.i32(&mut p.year);
    s.i32(&mut p.deadline);
    prophecy_kind(s, &mut p.kind);
    s.string(&mut p.what);
    opt_bool(s, &mut p.outcome);
    opt_i32(s, &mut p.resolved);
}

fn blank_prophecy() -> Prophecy {
    Prophecy {
        id: 0,
        seer: 0,
        year: 0,
        deadline: 0,
        kind: ProphecyKind::RealmFalls(0),
        what: String::new(),
        outcome: None,
        resolved: None,
    }
}

fn event<S: Io>(s: &mut S, e: &mut Event) {
    s.i32(&mut e.year);
    s.u8(&mut e.importance);
    enum8(s, &mut e.kind, &EVENT_KINDS);
    seq(s, &mut e.refs, || Ref::Polity(0), reff);
    opt_usize(s, &mut e.loc);
    s.string(&mut e.text);
}

fn blank_event() -> Event {
    Event {
        year: 0,
        importance: 0,
        kind: EventKind::Genesis,
        refs: Vec::new(),
        loc: None,
        text: String::new(),
    }
}

fn blank_era() -> Era {
    Era {
        start: 0,
        name: String::new(),
        description: String::new(),
    }
}

fn era<S: Io>(s: &mut S, e: &mut Era) {
    s.i32(&mut e.start);
    s.string(&mut e.name);
    s.string(&mut e.description);
}

fn blank_plague() -> Plague {
    Plague {
        name: String::new(),
        years_left: 0,
        polities: Vec::new(),
        deaths: 0.0,
    }
}

fn plague<S: Io>(s: &mut S, p: &mut Plague) {
    s.string(&mut p.name);
    s.i32(&mut p.years_left);
    vec_usize(s, &mut p.polities);
    s.f64(&mut p.deaths);
}

// -- sections -----------------------------------------------------------------
//
// One function per top-level section. Each is visited with `s.ver()` set to
// that section's record version, so records can read newer fields
// conditionally (see the module docs).

fn head<S: Io>(s: &mut S, w: &mut World) {
    s.u64(&mut w.seed);
    let mut st = w.rng.state();
    for x in st.iter_mut() {
        s.u64(x);
    }
    if s.reading() {
        w.rng.set_state(st);
    }
    s.i32(&mut w.year);
    enum8(s, &mut w.detail, &DETAILS);
}

fn terrain_sec<S: Io>(s: &mut S, w: &mut World) {
    terrain(s, &mut w.terrain);
}
fn cells<S: Io>(s: &mut S, w: &mut World) {
    seq(s, &mut w.cells, CellState::default, cell);
}
fn races<S: Io>(s: &mut S, w: &mut World) {
    seq(s, &mut w.races, blank_race, race);
}
fn cultures<S: Io>(s: &mut S, w: &mut World) {
    seq(s, &mut w.cultures, blank_culture, culture);
}
fn cities<S: Io>(s: &mut S, w: &mut World) {
    seq(s, &mut w.cities, blank_city, city);
}
fn polities<S: Io>(s: &mut S, w: &mut World) {
    seq(s, &mut w.polities, blank_polity, polity);
}
fn persons<S: Io>(s: &mut S, w: &mut World) {
    seq(s, &mut w.persons, blank_person, person);
}
fn schools<S: Io>(s: &mut S, w: &mut World) {
    seq(s, &mut w.schools, blank_school, school);
}
fn wars<S: Io>(s: &mut S, w: &mut World) {
    seq(s, &mut w.wars, blank_war, war);
}
fn eras<S: Io>(s: &mut S, w: &mut World) {
    seq(s, &mut w.eras, blank_era, era);
}
fn plagues<S: Io>(s: &mut S, w: &mut World) {
    seq(s, &mut w.plagues, blank_plague, plague);
}
fn artifacts<S: Io>(s: &mut S, w: &mut World) {
    seq(s, &mut w.artifacts, blank_artifact, artifact);
}
fn prophecies<S: Io>(s: &mut S, w: &mut World) {
    seq(s, &mut w.prophecies, blank_prophecy, prophecy);
}

fn chronicle<S: Io>(s: &mut S, w: &mut World) {
    let mut events: Vec<Event> = if s.reading() {
        Vec::new()
    } else {
        w.chronicle.events.clone()
    };
    seq(s, &mut events, blank_event, event);
    if s.reading() {
        w.chronicle = Chronicle::from_events(events);
    }
}

fn stats<S: Io>(s: &mut S, w: &mut World) {
    s.f64(&mut w.stats.peak_pop);
    vec_f64(s, &mut w.stats.pop_history);
}

fn counters<S: Io>(s: &mut S, w: &mut World) {
    s.u32(&mut w.century_wars);
    s.u32(&mut w.century_schools);
    s.u32(&mut w.century_polities_born);
    s.f64(&mut w.century_pop_start);
}

fn battlefields<S: Io>(s: &mut S, w: &mut World) {
    let mut names: Vec<(usize, String)> = w
        .battlefield_names
        .iter()
        .map(|(&k, v)| (k, v.clone()))
        .collect();
    names.sort_by_key(|x| x.0);
    seq(
        s,
        &mut names,
        || (0, String::new()),
        |s, (k, v)| {
            s.usize(k);
            s.string(v);
        },
    );
    if s.reading() {
        w.battlefield_names = names.into_iter().collect();
    }
}

// -- the chunk table ----------------------------------------------------------

/// A four-byte ASCII chunk tag, little-endian.
const fn tag(b: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*b)
}

/// The sections of the body, in the order the writer emits them, each with
/// the current version of its records. Bump a version here when a section
/// gains a field; never reuse or renumber a tag.
macro_rules! sections {
    ($($name:ident = $bytes:literal, $ver:literal, $f:ident;)*) => {
        $(const $name: u32 = tag($bytes);)*

        /// Write every section as a tagged, length-prefixed chunk.
        fn write_body(wr: &mut Writer, w: &mut World) {
            $(write_chunk(wr, $name, $ver, |s| $f(s, w));)*
        }

        /// Read one chunk into `w`. Returns false for a tag this build does
        /// not know, or one whose records are newer than it can parse; the
        /// caller then skips the chunk and the section keeps its default.
        fn read_chunk(rd: &mut Reader, tag: u32, ver: u32, w: &mut World) -> bool {
            match tag {
                $($name if ver <= $ver => { $f(rd, w); true })*
                _ => false,
            }
        }
    };
}

sections! {
    T_HEAD = b"head", 1, head;
    T_TERR = b"terr", 1, terrain_sec;
    T_CELL = b"cell", 1, cells;
    T_RACE = b"race", 1, races;
    T_CULT = b"cult", 1, cultures;
    T_CITY = b"city", 1, cities;
    T_POLY = b"poly", 1, polities;
    T_PERS = b"pers", 1, persons;
    T_SCHL = b"schl", 1, schools;
    T_WARS = b"wars", 1, wars;
    T_ERAS = b"eras", 1, eras;
    T_PLAG = b"plag", 1, plagues;
    T_ARTI = b"arti", 1, artifacts;
    T_PROP = b"prop", 1, prophecies;
    T_CHRN = b"chrn", 1, chronicle;
    T_STAT = b"stat", 1, stats;
    T_CTRS = b"ctrs", 1, counters;
    T_BFNM = b"bfnm", 1, battlefields;
}

/// The version-1 body: the same sections, positional, with no chunk frames.
/// Frozen; new sections belong in the table above, not here.
fn read_v1(rd: &mut Reader, w: &mut World) {
    head(rd, w);
    terrain_sec(rd, w);
    cells(rd, w);
    races(rd, w);
    cultures(rd, w);
    cities(rd, w);
    polities(rd, w);
    persons(rd, w);
    schools(rd, w);
    wars(rd, w);
    eras(rd, w);
    plagues(rd, w);
    artifacts(rd, w);
    prophecies(rd, w);
    chronicle(rd, w);
    stats(rd, w);
    counters(rd, w);
    battlefields(rd, w);
}

fn write_chunk(wr: &mut Writer, tag: u32, ver: u32, f: impl FnOnce(&mut Writer)) {
    let mut t = tag;
    wr.u32(&mut t);
    let mut v = ver;
    wr.u32(&mut v);
    let at = wr.buf.len();
    let mut len = 0u64;
    wr.u64(&mut len);
    let outer = wr.ver;
    wr.ver = ver;
    f(wr);
    wr.ver = outer;
    let n = (wr.buf.len() - at - 8) as u64;
    wr.buf[at..at + 8].copy_from_slice(&n.to_le_bytes());
}

// -- errors -------------------------------------------------------------------

/// Everything that can go wrong reading or writing a save.
#[derive(Debug)]
pub enum SaveError {
    /// The magic bytes are missing: this is not a save file at all.
    NotASave,
    /// A format too new (or too old) for this build.
    UnsupportedVersion { found: u32, supported: u32 },
    /// The file ends in the middle of a record.
    Truncated,
    /// The body does not match the checksum in the header.
    Checksum,
    /// The file parses but describes an impossible world.
    Inconsistent,
    /// The file could not be read or written.
    Io(std::io::Error),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::NotASave => write!(f, "not a Rise and Fall of Empires save file"),
            SaveError::UnsupportedVersion { found, supported } => write!(
                f,
                "save file version {} is not supported (this build reads versions {}..={})",
                found, MIN_VERSION, supported
            ),
            SaveError::Truncated => write!(f, "save file is truncated"),
            SaveError::Checksum => write!(f, "save file is corrupt (checksum mismatch)"),
            SaveError::Inconsistent => write!(f, "save file is inconsistent"),
            SaveError::Io(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for SaveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SaveError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for SaveError {
    fn from(e: std::io::Error) -> SaveError {
        SaveError::Io(e)
    }
}

// -- file ---------------------------------------------------------------------

/// FNV-1a, 64 bit: a few lines, and enough to catch a damaged save.
pub fn fnv1a(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &x in b {
        h ^= x as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// The whole world as bytes: header, then one tagged chunk per section.
pub fn save(w: &mut World) -> Vec<u8> {
    let mut wr = Writer {
        buf: Vec::with_capacity(1 << 20),
        ver: VERSION,
    };
    wr.buf.extend_from_slice(MAGIC);
    wr.buf.extend_from_slice(&VERSION.to_le_bytes());
    wr.buf.resize(HEADER, 0); // body length and checksum, patched below
    write_body(&mut wr, w);
    let mut buf = wr.buf;
    let n = (buf.len() - HEADER) as u64;
    let sum = fnv1a(&buf[HEADER..]);
    buf[8..16].copy_from_slice(&n.to_le_bytes());
    buf[16..HEADER].copy_from_slice(&sum.to_le_bytes());
    buf
}

/// Rebuild a world from [`save`]'s bytes.
///
/// # Errors
///
/// [`SaveError`] if the bytes are not a save, are of an unreadable version,
/// are truncated, fail the checksum, or describe an impossible world.
pub fn load(bytes: &[u8]) -> Result<World, SaveError> {
    if bytes.len() < 8 || &bytes[..4] != MAGIC {
        return Err(SaveError::NotASave);
    }
    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let mut w = World::blank();
    match version {
        1 => {
            let mut rd = Reader {
                buf: &bytes[8..],
                pos: 0,
                err: false,
                ver: 1,
            };
            read_v1(&mut rd, &mut w);
            if rd.err {
                return Err(SaveError::Truncated);
            }
        }
        VERSION => read_body(bytes, &mut w)?,
        found => {
            return Err(SaveError::UnsupportedVersion {
                found,
                supported: VERSION,
            })
        }
    }
    if w.terrain.w == 0 || w.terrain.h == 0 || w.cells.len() != w.terrain.w * w.terrain.h {
        return Err(SaveError::Inconsistent);
    }
    w.recompute();
    Ok(w)
}

/// Walk the chunks of a current-version file, skipping the tags we do not know.
fn read_body(bytes: &[u8], w: &mut World) -> Result<(), SaveError> {
    if bytes.len() < HEADER {
        return Err(SaveError::Truncated);
    }
    let mut n = [0u8; 8];
    n.copy_from_slice(&bytes[8..16]);
    let n = u64::from_le_bytes(n) as usize;
    let mut sum = [0u8; 8];
    sum.copy_from_slice(&bytes[16..HEADER]);
    let sum = u64::from_le_bytes(sum);
    let body = bytes
        .get(HEADER..)
        .and_then(|b| b.get(..n))
        .ok_or(SaveError::Truncated)?;
    if fnv1a(body) != sum {
        return Err(SaveError::Checksum);
    }
    let mut pos = 0usize;
    while pos < body.len() {
        if pos + CHUNK_HEADER > body.len() {
            return Err(SaveError::Truncated);
        }
        let mut hd = Reader {
            buf: &body[pos..pos + CHUNK_HEADER],
            pos: 0,
            err: false,
            ver: 0,
        };
        let (mut tag, mut ver, mut len) = (0u32, 0u32, 0u64);
        hd.u32(&mut tag);
        hd.u32(&mut ver);
        hd.u64(&mut len);
        pos += CHUNK_HEADER;
        let end = (len as usize)
            .checked_add(pos)
            .filter(|e| *e <= body.len())
            .ok_or(SaveError::Truncated)?;
        let mut rd = Reader {
            buf: &body[pos..end],
            pos: 0,
            err: false,
            ver,
        };
        if read_chunk(&mut rd, tag, ver, w) && rd.err {
            return Err(SaveError::Truncated);
        }
        pos = end;
    }
    Ok(())
}
