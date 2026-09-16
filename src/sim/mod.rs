//! The world simulation: entities, per-year tick, aggregates and the
//! helpers every subsystem uses to name things and write history.

pub mod chronicle;
pub mod events;
pub mod genesis;
pub mod magic;
pub mod people;
pub mod politics;
pub mod prose;
pub mod stories;
pub mod tuning;
pub mod war;

use crate::geo::{self, FeatureKind, Terrain, GROUP_COUNT};
use crate::lang::Language;
use crate::rng::Rng;
use crate::term::Rgb;
use chronicle::{Chronicle, Event, EventKind, Ref};
use std::collections::BTreeMap;
use tuning::Tuning;

/// A file error with the path it happened to folded into its message.
fn io_err(what: &str, e: std::io::Error) -> crate::ser::SaveError {
    crate::ser::SaveError::Io(std::io::Error::new(e.kind(), format!("{}: {}", what, e)))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Detail {
    Low,
    Medium,
    High,
}

impl Detail {
    pub fn name(self) -> &'static str {
        match self {
            Detail::Low => "low",
            Detail::Medium => "medium",
            Detail::High => "high",
        }
    }
    pub fn level(self) -> u8 {
        match self {
            Detail::Low => 0,
            Detail::Medium => 1,
            Detail::High => 2,
        }
    }
    pub fn next(self) -> Detail {
        match self {
            Detail::Low => Detail::Medium,
            Detail::Medium => Detail::High,
            Detail::High => Detail::Low,
        }
    }
    /// Minimum importance of events that get written at this level.
    pub fn min_importance(self) -> u8 {
        match self {
            Detail::Low => 2,
            Detail::Medium => 1,
            Detail::High => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Entities
// ---------------------------------------------------------------------------

pub struct Race {
    #[allow(dead_code)]
    pub id: usize,
    pub name: String,
    pub adj: String,
    pub plural: String,
    pub lang: Language,
    pub lifespan: f32,
    pub affinity: [f32; GROUP_COUNT],
    pub coast_love: f32,
    pub martial: f32,
    pub mystic: f32,
    pub mercantile: f32,
    pub fecund: f32,
    pub seafaring: f32,
    pub description: String,
    pub home: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct Values {
    pub militarism: f32,
    pub mysticism: f32,
    pub mercantilism: f32,
    pub tradition: f32,
    pub openness: f32,
}

pub struct Culture {
    pub id: usize,
    pub name: String,
    pub adj: String,
    pub plural: String,
    pub race: usize,
    pub lang: Language,
    pub parent: Option<usize>,
    pub founded: i32,
    pub values: Values,
    pub home: usize,
    pub color: Rgb,
    pub extinct: Option<i32>,
    pub cells: usize,
    pub pop: f64,
    pub last_seen: i32,
}

pub struct City {
    pub id: usize,
    pub name: String,
    pub cell: usize,
    pub founded: i32,
    pub culture: usize,
    pub polity: Option<usize>,
    pub pop: f32,
    pub prosperity: f32,
    pub walls: f32,
    pub destroyed: Option<i32>,
    pub wonders: Vec<String>,
    pub times_sacked: u32,
    pub founder: Option<usize>,
    pub peak_pop: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PolityKind {
    Tribe,
    Chiefdom,
    Kingdom,
    Empire,
    Republic,
    Theocracy,
    Magocracy,
    Horde,
}

impl PolityKind {
    pub fn name(self) -> &'static str {
        match self {
            PolityKind::Tribe => "tribe",
            PolityKind::Chiefdom => "chiefdom",
            PolityKind::Kingdom => "kingdom",
            PolityKind::Empire => "empire",
            PolityKind::Republic => "republic",
            PolityKind::Theocracy => "theocracy",
            PolityKind::Magocracy => "magocracy",
            PolityKind::Horde => "horde",
        }
    }
    pub fn rank(self) -> u8 {
        match self {
            PolityKind::Tribe => 0,
            PolityKind::Chiefdom => 1,
            PolityKind::Horde => 2,
            PolityKind::Kingdom
            | PolityKind::Republic
            | PolityKind::Theocracy
            | PolityKind::Magocracy => 2,
            PolityKind::Empire => 3,
        }
    }
    pub fn admin_bonus(self) -> f32 {
        match self {
            PolityKind::Tribe => 0.0,
            PolityKind::Chiefdom => 6.0,
            PolityKind::Kingdom => 30.0,
            PolityKind::Empire => 100.0,
            PolityKind::Republic => 20.0,
            PolityKind::Theocracy => 18.0,
            PolityKind::Magocracy => 16.0,
            PolityKind::Horde => 30.0,
        }
    }
    pub fn expansion_mult(self) -> f32 {
        match self {
            PolityKind::Tribe => 0.6,
            PolityKind::Chiefdom => 0.8,
            PolityKind::Kingdom => 1.0,
            PolityKind::Empire => 1.25,
            PolityKind::Republic => 0.85,
            PolityKind::Theocracy => 0.9,
            PolityKind::Magocracy => 0.8,
            PolityKind::Horde => 1.8,
        }
    }
    pub fn has_dynasty(self) -> bool {
        matches!(
            self,
            PolityKind::Kingdom | PolityKind::Empire | PolityKind::Theocracy | PolityKind::Horde
        )
    }
}

pub struct Polity {
    pub id: usize,
    pub name: String,
    pub short: String,
    pub adj: String,
    pub kind: PolityKind,
    pub culture: usize,
    pub capital: Option<usize>,
    pub ruler: Option<usize>,
    pub dynasty: String,
    pub founded: i32,
    pub fell: Option<i32>,
    pub fall_cause: String,
    pub parent: Option<usize>,
    pub color: Rgb,
    pub cells: usize,
    pub pop: f64,
    pub cities: Vec<usize>,
    pub peak_cells: usize,
    pub peak_year: i32,
    pub stability: f32,
    pub treasury: f32,
    pub army: f32,
    pub dev: f32,
    pub prestige: f32,
    pub exhaustion: f32,
    pub decadence: f32,
    pub seafaring: bool,
    pub school: Option<usize>,
    pub wars: Vec<usize>,
    pub tension: BTreeMap<usize, f32>,
    pub neighbors: Vec<(usize, u32)>,
    pub conquered: u32,
    pub reign_start: i32,
    pub reign_gained: i32,
    pub reign_cities: u32,
    pub reign_wars_won: u32,
    pub last_revolt: i32,
    pub foreign_share: f32,
    pub avg_fertility: f32,
    pub cultures_within: usize,
    pub rulers: Vec<usize>,
    pub generals: Vec<usize>,
    pub last_kind_change: i32,
    pub culture_counts: BTreeMap<usize, u32>,
    pub truce: BTreeMap<usize, i32>,
}

impl Polity {
    pub fn alive(&self) -> bool {
        self.fell.is_none()
    }
    pub fn at_war(&self) -> bool {
        !self.wars.is_empty()
    }
    pub fn admin_capacity(&self, t: &Tuning) -> f32 {
        t.admin_capacity_base
            + self.kind.admin_bonus()
            + self.dev * t.admin_capacity_dev_weight
            + self.cities.len() as f32 * t.admin_capacity_city_weight
    }
    pub fn overextension(&self, t: &Tuning) -> f32 {
        (self.cells as f32 / self.admin_capacity(t)).max(0.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Ruler,
    General,
    Mage,
    Prophet,
    Philosopher,
    Poet,
    Rebel,
    Explorer,
    Martyr,
}

impl Role {
    pub fn name(self) -> &'static str {
        match self {
            Role::Ruler => "ruler",
            Role::General => "general",
            Role::Mage => "mage",
            Role::Prophet => "prophet",
            Role::Philosopher => "philosopher",
            Role::Poet => "poet",
            Role::Rebel => "rebel",
            Role::Explorer => "explorer",
            Role::Martyr => "martyr",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gender {
    F,
    M,
    N,
}

#[derive(Clone, Copy, Debug)]
pub struct Traits {
    pub ambition: f32,
    pub valor: f32,
    pub wisdom: f32,
    pub piety: f32,
    pub cruelty: f32,
    pub charisma: f32,
}

impl Traits {
    pub fn random(rng: &Rng) -> Traits {
        Traits {
            ambition: rng.trait_value(0.5, 0.22),
            valor: rng.trait_value(0.5, 0.22),
            wisdom: rng.trait_value(0.5, 0.22),
            piety: rng.trait_value(0.45, 0.22),
            cruelty: rng.trait_value(0.35, 0.22),
            charisma: rng.trait_value(0.5, 0.22),
        }
    }
    pub fn inherit(&self, rng: &Rng) -> Traits {
        let mix = |v: f32| rng.trait_value(v as f64 * 0.6 + 0.2, 0.2);
        Traits {
            ambition: mix(self.ambition),
            valor: mix(self.valor),
            wisdom: mix(self.wisdom),
            piety: mix(self.piety),
            cruelty: mix(self.cruelty),
            charisma: mix(self.charisma),
        }
    }
    pub fn describe(&self) -> String {
        let mut parts: Vec<&str> = Vec::new();
        if self.ambition > 0.72 {
            parts.push("ambitious");
        } else if self.ambition < 0.28 {
            parts.push("content");
        }
        if self.valor > 0.72 {
            parts.push("valiant");
        } else if self.valor < 0.28 {
            parts.push("timid");
        }
        if self.wisdom > 0.72 {
            parts.push("wise");
        } else if self.wisdom < 0.28 {
            parts.push("foolish");
        }
        if self.piety > 0.72 {
            parts.push("pious");
        } else if self.piety < 0.25 {
            parts.push("irreverent");
        }
        if self.cruelty > 0.7 {
            parts.push("cruel");
        } else if self.cruelty < 0.2 {
            parts.push("gentle");
        }
        if self.charisma > 0.72 {
            parts.push("beloved");
        } else if self.charisma < 0.28 {
            parts.push("cold");
        }
        if parts.is_empty() {
            "unremarkable".to_string()
        } else {
            parts.join(", ")
        }
    }
}

pub struct Person {
    #[allow(dead_code)]
    pub id: usize,
    pub name: String,
    pub epithet: Option<String>,
    pub gender: Gender,
    pub culture: usize,
    pub race: usize,
    pub born: i32,
    pub died: Option<i32>,
    pub death: String,
    pub role: Role,
    pub polity: Option<usize>,
    pub school: Option<usize>,
    pub city: Option<usize>,
    pub traits: Traits,
    pub renown: f32,
    pub parent: Option<usize>,
    pub battles_won: u32,
}

impl Person {
    pub fn full_name(&self) -> String {
        match &self.epithet {
            Some(e) => format!("{} {}", self.name, e),
            None => self.name.clone(),
        }
    }
    pub fn alive(&self) -> bool {
        self.died.is_none()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SchoolKind {
    Arcane,
    Divine,
    Philosophical,
}

impl SchoolKind {
    pub fn name(self) -> &'static str {
        match self {
            SchoolKind::Arcane => "arcane school",
            SchoolKind::Divine => "faith",
            SchoolKind::Philosophical => "philosophy",
        }
    }
    pub fn follower(self) -> &'static str {
        match self {
            SchoolKind::Arcane => "adepts",
            SchoolKind::Divine => "faithful",
            SchoolKind::Philosophical => "disciples",
        }
    }
}

pub struct School {
    pub id: usize,
    pub name: String,
    pub short: String,
    pub kind: SchoolKind,
    pub doctrine: String,
    pub tenets: Vec<String>,
    pub aspect: usize,
    pub practice: usize,
    pub founder: usize,
    pub founded: i32,
    pub home_city: usize,
    pub parent: Option<usize>,
    pub influence: BTreeMap<usize, f32>,
    pub extinct: Option<i32>,
    pub color: Rgb,
    pub hostility: f32,
    pub peak_polities: usize,
    pub fading_since: Option<i32>,
    pub state_of: Vec<usize>,
}

impl School {
    pub fn alive(&self) -> bool {
        self.extinct.is_none()
    }
    pub fn total_influence(&self) -> f32 {
        self.influence.values().sum()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WarKind {
    Conquest,
    Rebellion,
    CivilWar,
    Holy,
    Raid,
    Succession,
}

pub struct War {
    #[allow(dead_code)]
    pub id: usize,
    pub attacker: usize,
    pub defender: usize,
    pub started: i32,
    pub ended: Option<i32>,
    pub cause: String,
    pub name: String,
    pub score: f32,
    pub battles: u32,
    pub result: String,
    pub cells_taken: i32,
    pub kind: WarKind,
}

impl War {
    pub fn alive(&self) -> bool {
        self.ended.is_none()
    }
}

#[derive(Clone, Copy, Default)]
pub struct CellState {
    pub owner: Option<usize>,
    pub culture: Option<usize>,
    pub pop: f32,
    pub city: Option<usize>,
    pub since: i32,
    pub plague: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArtifactKind {
    Crown,
    Blade,
    Tome,
    Gem,
    Banner,
    Staff,
    Chalice,
    Horn,
    Mirror,
    Shard,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Holder {
    Polity(usize),
    Person(usize),
    City(usize),
    Lost,
}

pub struct Artifact {
    pub id: usize,
    pub name: String,
    pub kind: ArtifactKind,
    pub made: i32,
    pub maker: Option<usize>,
    pub origin: Option<usize>,
    pub holder: Holder,
    pub power: f32,
    pub description: String,
    pub lost_at: Option<usize>,
    pub hands: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProphecyKind {
    RealmFalls(usize),
    CrownOfEmpire(usize),
    CityBurns(usize, u32),
    RulerMurdered(usize),
    FaithSpreads(usize, usize),
    RelicReturns(usize, usize),
}

pub struct Prophecy {
    #[allow(dead_code)]
    pub id: usize,
    pub seer: usize,
    pub year: i32,
    pub deadline: i32,
    pub kind: ProphecyKind,
    pub what: String,
    pub outcome: Option<bool>,
    pub resolved: Option<i32>,
}

pub struct Era {
    #[allow(dead_code)]
    pub start: i32,
    pub name: String,
    pub description: String,
}

pub struct Plague {
    pub name: String,
    pub years_left: i32,
    pub polities: Vec<usize>,
    pub deaths: f64,
}

#[derive(Default, Clone)]
pub struct Stats {
    pub pop: f64,
    pub peak_pop: f64,
    pub polities_alive: usize,
    pub wars_active: usize,
    pub cities_alive: usize,
    pub owned_cells: usize,
    pub cultures_alive: usize,
    pub schools_alive: usize,
    pub pop_history: Vec<f64>,
}

/// The phases of a tick, in the order `World::tick` runs them.
pub const PHASES: [&str; 19] = [
    "grow_and_migrate",
    "form_polities",
    "expand",
    "found_cities",
    "economy",
    "diplomacy",
    "resolve_wars",
    "rulers",
    "unrest",
    "magic",
    "culture_drift",
    "disasters",
    "notables",
    "wonders",
    "artifacts",
    "prophecies",
    "legends",
    "recompute",
    "eras",
];

/// Per-phase timings, accumulated only while `on` is set. The flag is
/// checked once per phase, so leaving it off costs nothing measurable.
pub struct Prof {
    pub on: bool,
    pub ms: [f64; PHASES.len()],
}

impl Default for Prof {
    fn default() -> Prof {
        Prof {
            on: false,
            ms: [0.0; PHASES.len()],
        }
    }
}

// ---------------------------------------------------------------------------
// The world
// ---------------------------------------------------------------------------

pub struct World {
    pub seed: u64,
    pub rng: Rng,
    pub year: i32,
    pub detail: Detail,
    pub tuning: Tuning,
    pub terrain: Terrain,
    pub cells: Vec<CellState>,
    pub races: Vec<Race>,
    pub cultures: Vec<Culture>,
    pub cities: Vec<City>,
    pub polities: Vec<Polity>,
    pub persons: Vec<Person>,
    pub schools: Vec<School>,
    pub wars: Vec<War>,
    pub eras: Vec<Era>,
    pub plagues: Vec<Plague>,
    pub artifacts: Vec<Artifact>,
    pub prophecies: Vec<Prophecy>,
    pub chronicle: Chronicle,
    pub stats: Stats,
    pub century_wars: u32,
    pub century_schools: u32,
    pub century_polities_born: u32,
    pub century_pop_start: f64,
    pub battlefield_names: BTreeMap<usize, String>,
    pub ticks_ms: f64,
    /// Cells owned by each polity, ascending. Rebuilt by `recompute` and kept
    /// exact in between by `claim` and `fall`, so that no subsystem has to
    /// scan the whole map to find one realm's land.
    pub owner_cells: Vec<Vec<usize>>,
    pub prof: Prof,
}

impl World {
    /// An empty world for the loader to fill in.
    pub fn blank() -> World {
        World {
            seed: 0,
            rng: Rng::new(0),
            year: 0,
            detail: Detail::Medium,
            tuning: Tuning::default(),
            terrain: Terrain::empty(),
            cells: Vec::new(),
            races: Vec::new(),
            cultures: Vec::new(),
            cities: Vec::new(),
            polities: Vec::new(),
            persons: Vec::new(),
            schools: Vec::new(),
            wars: Vec::new(),
            eras: Vec::new(),
            plagues: Vec::new(),
            artifacts: Vec::new(),
            prophecies: Vec::new(),
            chronicle: Chronicle::default(),
            stats: Stats::default(),
            century_wars: 0,
            century_schools: 0,
            century_polities_born: 0,
            century_pop_start: 0.0,
            battlefield_names: BTreeMap::new(),
            ticks_ms: 0.0,
            owner_cells: Vec::new(),
            prof: Prof::default(),
        }
    }

    pub fn save_to(&mut self, path: &std::path::Path) -> Result<usize, crate::ser::SaveError> {
        let bytes = crate::ser::save(self);
        if let Some(dir) = path.parent() {
            if !dir.as_os_str().is_empty() {
                std::fs::create_dir_all(dir)
                    .map_err(|e| io_err(&format!("cannot create {}", dir.display()), e))?;
            }
        }
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &bytes)
            .map_err(|e| io_err(&format!("cannot write {}", tmp.display()), e))?;
        std::fs::rename(&tmp, path)
            .map_err(|e| io_err(&format!("cannot rename to {}", path.display()), e))?;
        Ok(bytes.len())
    }

    pub fn load_from(path: &std::path::Path) -> Result<World, crate::ser::SaveError> {
        let bytes = std::fs::read(path)
            .map_err(|e| io_err(&format!("cannot read {}", path.display()), e))?;
        crate::ser::load(&bytes)
    }

    pub fn new(seed: u64, w: usize, h: usize, detail: Detail) -> World {
        let rng = Rng::new(seed);
        let terrain = geo::generate(&rng, w, h);
        let n = terrain.n();
        let mut world = World {
            seed,
            rng,
            year: 0,
            detail,
            tuning: Tuning::default(),
            terrain,
            cells: vec![CellState::default(); n],
            races: Vec::new(),
            cultures: Vec::new(),
            cities: Vec::new(),
            polities: Vec::new(),
            persons: Vec::new(),
            schools: Vec::new(),
            wars: Vec::new(),
            eras: Vec::new(),
            plagues: Vec::new(),
            artifacts: Vec::new(),
            prophecies: Vec::new(),
            chronicle: Chronicle::default(),
            stats: Stats::default(),
            century_wars: 0,
            century_schools: 0,
            century_polities_born: 0,
            century_pop_start: 0.0,
            battlefield_names: BTreeMap::new(),
            ticks_ms: 0.0,
            owner_cells: Vec::new(),
            prof: Prof::default(),
        };
        genesis::populate(&mut world);
        world.recompute();
        world
    }

    // -- logging ----------------------------------------------------------

    pub fn log(
        &mut self,
        importance: u8,
        kind: EventKind,
        refs: &[Ref],
        loc: Option<usize>,
        text: String,
    ) -> Option<usize> {
        if importance < self.detail.min_importance() {
            return None;
        }
        let ev = Event {
            year: self.year,
            importance,
            kind,
            refs: refs.to_vec(),
            loc,
            text,
        };
        Some(self.chronicle.push(ev))
    }

    pub fn high_detail(&self) -> bool {
        self.detail == Detail::High
    }

    /// Multiplier for optional narrative activity (notables, flourishes).
    pub fn detail_rate(&self) -> f64 {
        match self.detail {
            Detail::Low => 0.0,
            Detail::Medium => 0.4,
            Detail::High => 1.0,
        }
    }

    // -- naming helpers -----------------------------------------------------

    pub fn polity_lang(&self, p: usize) -> &Language {
        &self.cultures[self.polities[p].culture].lang
    }

    /// "King Aldo the Bold of Velen"
    pub fn ruler_title(&self, p: usize) -> String {
        match self.polities[p].ruler {
            Some(r) => {
                let per = &self.persons[r];
                let hon = self.honorific(p, per.gender);
                format!("{} {} of {}", hon, per.full_name(), self.polities[p].short)
            }
            None => format!("the leaderless {}", self.polities[p].name),
        }
    }

    pub fn ruler_short(&self, p: usize) -> String {
        match self.polities[p].ruler {
            Some(r) => {
                let per = &self.persons[r];
                format!("{} {}", self.honorific(p, per.gender), per.full_name())
            }
            None => "the council".to_string(),
        }
    }

    pub fn honorific(&self, p: usize, g: Gender) -> String {
        let pol = &self.polities[p];
        let lang = self.polity_lang(p);
        match pol.kind {
            PolityKind::Tribe => "Chief".to_string(),
            PolityKind::Chiefdom => "Chieftain".to_string(),
            PolityKind::Empire => match g {
                Gender::F => "Empress".to_string(),
                _ => "Emperor".to_string(),
            },
            PolityKind::Republic => "Consul".to_string(),
            PolityKind::Theocracy => "Hierarch".to_string(),
            PolityKind::Magocracy => "Archmage".to_string(),
            PolityKind::Horde => "Khan".to_string(),
            PolityKind::Kingdom => match (lang.honorific.as_str(), g) {
                ("King", Gender::F) => "Queen".to_string(),
                ("Queen", Gender::M) => "King".to_string(),
                ("Prince", Gender::F) => "Princess".to_string(),
                ("Emperor", Gender::F) => "Empress".to_string(),
                ("Empress", Gender::M) => "Emperor".to_string(),
                ("Emperor", _) | ("Empress", _) => "King".to_string(),
                (h, _) => h.to_string(),
            },
        }
    }

    pub fn new_person(
        &mut self,
        culture: usize,
        role: Role,
        polity: Option<usize>,
        born: i32,
        traits: Option<Traits>,
    ) -> usize {
        let id = self.persons.len();
        let race = self.cultures[culture].race;
        let name = self.cultures[culture].lang.person(&self.rng);
        let gender = match self.rng.below(20) {
            0 => Gender::N,
            1..=10 => Gender::F,
            _ => Gender::M,
        };
        let traits = traits.unwrap_or_else(|| Traits::random(&self.rng));
        self.persons.push(Person {
            id,
            name,
            epithet: None,
            gender,
            culture,
            race,
            born,
            died: None,
            death: String::new(),
            role,
            polity,
            school: None,
            city: None,
            traits,
            renown: 0.0,
            parent: None,
            battles_won: 0,
        });
        id
    }

    /// Ensure a feature has a name, coining one in the polity's language.
    pub fn feature_name(&mut self, f: usize, by_polity: Option<usize>) -> String {
        if let Some(n) = &self.terrain.features[f].name {
            return n.clone();
        }
        let culture = match by_polity {
            Some(p) => self.polities[p].culture,
            None => {
                // Nearest culture by home distance, else first.
                let center = self.terrain.features[f].center;
                let ci = self.terrain.idx(center.0, center.1);
                let mut best = 0;
                let mut best_d = usize::MAX;
                for c in &self.cultures {
                    let d = self.terrain.dist(c.home, ci);
                    if d < best_d {
                        best_d = d;
                        best = c.id;
                    }
                }
                best
            }
        };
        let kind = self.terrain.features[f].kind;
        let name = geo::name_feature(kind, &self.cultures[culture].lang, &self.rng);
        self.terrain.features[f].name = Some(name.clone());
        self.terrain.features[f].named_by = Some(culture);
        if let Some(p) = by_polity {
            let plural = self.cultures[culture].plural.clone();
            let text = match kind {
                FeatureKind::River => format!("The {} came to call the river {}.", plural, name),
                FeatureKind::Sea | FeatureKind::Ocean => format!(
                    "Sailors of {} named the waters {}.",
                    self.polities[p].short, name
                ),
                FeatureKind::Nexus => format!(
                    "The {} found a place of terrible power and named it {}.",
                    plural, name
                ),
                FeatureKind::Continent => format!("The {} called their land {}.", plural, name),
                _ => format!("The {} named the {} {}.", plural, kind.label(), name),
            };
            self.log(
                0,
                EventKind::Discovery,
                &[Ref::Polity(p), Ref::Feature(f)],
                None,
                text,
            );
        }
        name
    }

    /// A short phrase locating a cell: "on the banks of the Ashwater".
    pub fn place_phrase(&mut self, cell: usize, by_polity: Option<usize>) -> String {
        let t = &self.terrain;
        if let Some(r) = t.river_at(cell) {
            let n = self.feature_name(r, by_polity);
            return format!("on the banks of {}", n);
        }
        if self.terrain.coast[cell] {
            let t = &self.terrain;
            for nb in t.neighbors8(cell).collect::<Vec<_>>() {
                if let Some(f) = self.terrain.feature_at(nb) {
                    if matches!(
                        self.terrain.features[f].kind,
                        FeatureKind::Sea | FeatureKind::Ocean
                    ) {
                        let n = self.feature_name(f, by_polity);
                        return format!("on the shores of {}", n);
                    }
                }
            }
        }
        if let Some(f) = self.terrain.feature_at(cell) {
            let kind = self.terrain.features[f].kind;
            let n = self.feature_name(f, by_polity);
            return match kind {
                FeatureKind::Range => format!("high in {}", n),
                FeatureKind::Forest | FeatureKind::Jungle => format!("deep in {}", n),
                FeatureKind::Desert => format!("at an oasis in {}", n),
                FeatureKind::Marsh => format!("amid {}", n),
                FeatureKind::Steppe => format!("upon {}", n),
                FeatureKind::Island => format!("on {}", n),
                FeatureKind::Lake => format!("beside {}", n),
                _ => format!("in {}", n),
            };
        }
        // Nearby feature within 3 cells.
        let t = &self.terrain;
        let (x, y) = t.xy(cell);
        let mut found: Option<(usize, &'static str)> = None;
        'outer: for r in 1..=3i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let cx = x as i32 + dx;
                    let cy = y as i32 + dy;
                    if cx < 0 || cy < 0 || cx >= t.w as i32 || cy >= t.h as i32 {
                        continue;
                    }
                    let i = t.idx(cx as usize, cy as usize);
                    if let Some(f) = t.feature_at(i) {
                        let phrase = match t.features[f].kind {
                            FeatureKind::Range => "in the shadow of",
                            FeatureKind::Forest | FeatureKind::Jungle => "at the edge of",
                            FeatureKind::Desert => "on the fringe of",
                            FeatureKind::Marsh => "near",
                            FeatureKind::Sea | FeatureKind::Ocean => "near the coast of",
                            FeatureKind::Lake => "near",
                            _ => continue,
                        };
                        found = Some((f, phrase));
                        break 'outer;
                    }
                }
            }
        }
        if let Some((f, phrase)) = found {
            let n = self.feature_name(f, by_polity);
            return format!("{} {}", phrase, n);
        }
        let b = self.terrain.biome[cell];
        format!("in the open {}", b.name())
    }

    pub fn polity_color(&self, id: usize) -> Rgb {
        let hue = (id as f32 * 137.508) % 360.0;
        let sat = if id % 3 == 0 { 0.75 } else { 0.55 };
        let val = if id % 2 == 0 { 0.85 } else { 0.65 };
        Rgb::from_hsv(hue, sat, val)
    }

    // -- aggregates -------------------------------------------------------

    pub fn recompute(&mut self) {
        for p in self.polities.iter_mut() {
            p.cells = 0;
            p.pop = 0.0;
            p.cities.clear();
            p.neighbors.clear();
            p.cultures_within = 0;
            p.culture_counts.clear();
            p.foreign_share = 0.0;
            p.avg_fertility = 0.0;
        }
        for c in self.cultures.iter_mut() {
            c.cells = 0;
            c.pop = 0.0;
        }
        let mut foreign: Vec<u32> = vec![0; self.polities.len()];
        let mut fert: Vec<f32> = vec![0.0; self.polities.len()];
        // Every border crossing, collected flat and counted afterwards: a
        // sort beats a map lookup per cell.
        let mut borders: Vec<(usize, usize)> = Vec::new();
        let n = self.cells.len();
        let w = self.terrain.w;
        let mut total_pop = 0.0;
        let mut owned = 0usize;
        self.owner_cells.resize_with(self.polities.len(), Vec::new);
        for v in self.owner_cells.iter_mut() {
            v.clear();
        }
        for i in 0..n {
            let cs = self.cells[i];
            total_pop += cs.pop as f64;
            if let Some(c) = cs.culture {
                self.cultures[c].cells += 1;
                self.cultures[c].pop += cs.pop as f64;
            }
            if let Some(p) = cs.owner {
                owned += 1;
                self.owner_cells[p].push(i);
                let pol = &mut self.polities[p];
                pol.cells += 1;
                pol.pop += cs.pop as f64;
                fert[p] += self.terrain.fertility[i];
                if let Some(c) = cs.culture {
                    *pol.culture_counts.entry(c).or_insert(0) += 1;
                    if c != pol.culture {
                        foreign[p] += 1;
                    }
                }
                // Neighbour detection (east and south suffice for symmetric pairs).
                let x = i % w;
                let y = i / w;
                if x + 1 < w {
                    if let Some(q) = self.cells[i + 1].owner {
                        if q != p {
                            borders.push((p.min(q), p.max(q)));
                        }
                    }
                }
                if y + 1 < self.terrain.h {
                    if let Some(q) = self.cells[i + w].owner {
                        if q != p {
                            borders.push((p.min(q), p.max(q)));
                        }
                    }
                }
            }
        }
        borders.sort_unstable();
        let mut at = 0;
        while at < borders.len() {
            let (p, q) = borders[at];
            let mut len = 0u32;
            while at < borders.len() && borders[at] == (p, q) {
                len += 1;
                at += 1;
            }
            self.polities[p].neighbors.push((q, len));
            self.polities[q].neighbors.push((p, len));
        }
        for c in &self.cities {
            if c.destroyed.is_some() {
                continue;
            }
            if let Some(p) = c.polity {
                self.polities[p].cities.push(c.id);
                self.polities[p].pop += c.pop as f64;
            }
            self.cultures[c.culture].pop += c.pop as f64;
            total_pop += c.pop as f64;
        }
        for p in self.polities.iter_mut() {
            if p.cells > 0 {
                p.foreign_share = foreign[p.id] as f32 / p.cells as f32;
                p.avg_fertility = fert[p.id] / p.cells as f32;
            }
            p.cultures_within = p.culture_counts.len();
            if p.cells > p.peak_cells {
                p.peak_cells = p.cells;
                p.peak_year = self.year;
            }
        }
        for c in self.cultures.iter_mut() {
            if c.cells > 0 || c.pop > 0.0 {
                c.last_seen = self.year;
            }
        }
        self.stats.pop = total_pop;
        if total_pop > self.stats.peak_pop {
            self.stats.peak_pop = total_pop;
        }
        self.stats.polities_alive = self.polities.iter().filter(|p| p.alive()).count();
        self.stats.wars_active = self.wars.iter().filter(|w| w.alive()).count();
        self.stats.cities_alive = self.cities.iter().filter(|c| c.destroyed.is_none()).count();
        self.stats.cultures_alive = self.cultures.iter().filter(|c| c.extinct.is_none()).count();
        self.stats.schools_alive = self.schools.iter().filter(|s| s.alive()).count();
        self.stats.owned_cells = owned;
    }

    // -- tick -------------------------------------------------------------

    pub fn tick(&mut self) {
        let t0 = std::time::Instant::now();
        self.year += 1;
        if self.year == 1 {
            self.century_pop_start = self.stats.pop;
        }
        // Run one phase of the tick, charging its time to the profile when
        // profiling is on.
        macro_rules! phase {
            ($i:expr, $call:expr) => {{
                if self.prof.on {
                    let t = std::time::Instant::now();
                    $call;
                    self.prof.ms[$i] += t.elapsed().as_secs_f64() * 1000.0;
                } else {
                    $call;
                }
            }};
        }
        phase!(0, people::grow_and_migrate(self));
        phase!(1, politics::form_polities(self));
        phase!(2, politics::expand(self));
        phase!(3, politics::found_cities(self));
        phase!(4, politics::economy(self));
        phase!(5, war::diplomacy(self));
        phase!(6, war::resolve_wars(self));
        phase!(7, politics::rulers(self));
        phase!(8, politics::unrest(self));
        phase!(9, magic::tick(self));
        phase!(10, people::culture_drift(self));
        phase!(11, events::disasters(self));
        phase!(12, events::notables(self));
        phase!(13, events::wonders(self));
        phase!(14, stories::tick_artifacts(self));
        phase!(15, stories::tick_prophecies(self));
        phase!(16, stories::tick_legends(self));
        phase!(17, self.recompute());
        phase!(18, events::eras(self));
        self.chronicle.compact(self.tuning.chronicle_cap);
        if self.year % 10 == 0 {
            self.stats.pop_history.push(self.stats.pop);
        }
        self.ticks_ms = self.ticks_ms * 0.9 + t0.elapsed().as_secs_f64() * 1000.0 * 0.1;
    }

    // -- the owner index --------------------------------------------------

    /// Cells owned by polity `p`, ascending.
    pub fn cells_of_ref(&self, p: usize) -> &[usize] {
        self.owner_cells.get(p).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Cells owned by polity `p`, ascending, as an owned list. Callers that
    /// go on to mutate the world need this; the rest want `cells_of_ref`.
    pub fn cells_of(&self, p: usize) -> Vec<usize> {
        self.cells_of_ref(p).to_vec()
    }

    /// Give `cell` to `p`, keeping the owner index sorted.
    pub fn own_cell(&mut self, cell: usize, p: usize) {
        if let Some(old) = self.cells[cell].owner {
            if old == p {
                return;
            }
            self.unown_cell(cell, old);
        }
        self.cells[cell].owner = Some(p);
        if self.owner_cells.len() <= p {
            self.owner_cells.resize_with(p + 1, Vec::new);
        }
        let v = &mut self.owner_cells[p];
        if let Err(at) = v.binary_search(&cell) {
            v.insert(at, cell);
        }
    }

    fn unown_cell(&mut self, cell: usize, from: usize) {
        if let Some(v) = self.owner_cells.get_mut(from) {
            if let Ok(at) = v.binary_search(&cell) {
                v.remove(at);
            }
        }
    }

    /// Hand every cell of `p` to `to` (or to nobody) and return the list, in
    /// ascending order. Both index entries stay sorted.
    pub fn transfer_cells(&mut self, p: usize, to: Option<usize>) -> Vec<usize> {
        if self.owner_cells.len() <= p {
            return Vec::new();
        }
        let moved = std::mem::take(&mut self.owner_cells[p]);
        for &i in &moved {
            self.cells[i].owner = to;
        }
        if let Some(q) = to {
            if self.owner_cells.len() <= q {
                self.owner_cells.resize_with(q + 1, Vec::new);
            }
            let old = std::mem::take(&mut self.owner_cells[q]);
            let mut merged = Vec::with_capacity(old.len() + moved.len());
            let (mut a, mut b) = (0, 0);
            while a < old.len() && b < moved.len() {
                if old[a] <= moved[b] {
                    merged.push(old[a]);
                    a += 1;
                } else {
                    merged.push(moved[b]);
                    b += 1;
                }
            }
            merged.extend_from_slice(&old[a..]);
            merged.extend_from_slice(&moved[b..]);
            self.owner_cells[q] = merged;
        }
        moved
    }

    pub fn living_polities(&self) -> Vec<usize> {
        self.polities
            .iter()
            .filter(|p| p.alive())
            .map(|p| p.id)
            .collect()
    }

    pub fn capital_cell(&self, p: usize) -> Option<usize> {
        self.polities[p].capital.map(|c| self.cities[c].cell)
    }
}
