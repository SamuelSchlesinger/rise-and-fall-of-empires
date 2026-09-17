//! The world simulation: entities, per-year tick, aggregates and the
//! helpers every subsystem uses to name things and write history.

pub mod chronicle;
pub mod dynasty;
pub mod events;
pub mod explain;
pub mod genesis;
pub mod magic;
pub mod people;
pub mod politics;
pub mod prose;
pub mod stories;
pub mod tech;
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
fn io_err(what: &str, e: &std::io::Error) -> crate::ser::SaveError {
    crate::ser::SaveError::Io(std::io::Error::new(e.kind(), format!("{}: {}", what, e)))
}

/// How much of what happens gets written down.
///
/// Detail is a *verbosity* setting and nothing more: it decides the least
/// important event the chronicle keeps ([`Detail::min_importance`]) and
/// whether the optional flourishes are appended to a sentence. It never
/// gates a draw from the world's RNG and never changes what happens, so
/// the same seed gives the same history at low, medium and high — only the
/// telling of it is longer or shorter. `src/tests.rs` checks that.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Detail {
    Low,
    Medium,
    High,
}

impl Detail {
    /// The name the config file and `:detail` use.
    pub fn name(self) -> &'static str {
        match self {
            Detail::Low => "low",
            Detail::Medium => "medium",
            Detail::High => "high",
        }
    }
    /// 0, 1 or 2: how talkative this level is.
    pub fn level(self) -> u8 {
        match self {
            Detail::Low => 0,
            Detail::Medium => 1,
            Detail::High => 2,
        }
    }
    /// The next level round the cycle, for the `D` key.
    pub fn next(self) -> Detail {
        match self {
            Detail::Low => Detail::Medium,
            Detail::Medium => Detail::High,
            Detail::High => Detail::Low,
        }
    }
    /// Minimum importance of events that get written at this level: the
    /// more talkative the level, the smaller the events it keeps. This is
    /// the whole of what detail does to the world.
    pub fn min_importance(self) -> u8 {
        2 - self.level()
    }
}

// ---------------------------------------------------------------------------
// Entities
// ---------------------------------------------------------------------------

/// A people: a lifespan, a language and a set of leanings that every
/// culture descended from it starts out with.
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

/// What a culture cares about. Every value is in `[0, 1]`.
#[derive(Clone, Copy, Debug)]
pub struct Values {
    pub militarism: f32,
    pub mysticism: f32,
    pub mercantilism: f32,
    pub tradition: f32,
    pub openness: f32,
}

/// A people as they are now: a language, a set of values and the land they
/// live on. Cultures split, drift and die out.
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
    /// What this people is good at, one number per [`tech::Field`], in
    /// `[0, 1]`.
    ///
    /// A world in which every culture learns everything equally well is a
    /// world that homogenises: knowledge diffuses until the median thing is
    /// known nearly everywhere and no region is distinctive. A people's bent
    /// is drawn from what it *values* — a martial people works out warcraft,
    /// a mystical one ley-craft, a trading one accounts and ships — so what
    /// a place knows tells you who lives there.
    ///
    /// It gates discovery and adoption both, which is why a technique can
    /// stall for ever at a cultural border that a trade good crosses freely.
    pub learning: [f32; tech::FIELDS.len()],
    pub home: usize,
    pub color: Rgb,
    pub extinct: Option<i32>,
    pub cells: usize,
    pub pop: f64,
    pub last_seen: i32,
}

impl Culture {
    /// How readily this people takes to a field of knowledge.
    pub fn bent(&self, f: tech::Field) -> f32 {
        self.learning[f as usize]
    }
    /// The field this people is best at, and how good it is.
    pub fn best_field(&self) -> (tech::Field, f32) {
        let mut best = (tech::FIELDS[0], self.learning[0]);
        for f in tech::FIELDS {
            let v = self.learning[f as usize];
            if v > best.1 {
                best = (f, v);
            }
        }
        best
    }
}

/// A city: a cell, a population and a prosperity, with walls if anyone has
/// built them and a year it was destroyed if it has been.
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

/// What kind of thing a realm is, which decides how far it can reach and
/// how well it holds together.
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
    /// The common noun for this kind of realm.
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
    /// 0 to 3: how grand the kind is, for promotions and demotions.
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
    /// How many more cells this kind of realm can administer.
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
    /// How hard this kind of realm pushes at its borders.
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
    /// Whether rule passes down a family rather than being chosen.
    pub fn has_dynasty(self) -> bool {
        matches!(
            self,
            PolityKind::Kingdom | PolityKind::Empire | PolityKind::Theocracy | PolityKind::Horde
        )
    }
}

/// A ruling family, across every throne it ever held and every century it
/// lasted.
///
/// A dynasty used to be a bare `String` on the realm, which meant it had no
/// identity: two realms ruled by the same family could not be known to be
/// related, a cadet branch that went off and founded its own kingdom was
/// simply a different word, and nothing could be looked up, clicked or
/// followed. A house is an entity so that a thousand years of one family is
/// something the reader can actually open and read down.
///
/// Like every other entity vector this one is append-only: a house that dies
/// out is marked with the year and stays where it is.
pub struct House {
    pub id: usize,
    /// "the Tashkatoi dynasty", "House Dherrandu".
    pub name: String,
    pub culture: usize,
    /// Whoever the house is named for.
    pub founder: Option<usize>,
    pub founded: i32,
    /// The year the last of the line died or lost the last throne.
    pub ended: Option<i32>,
    /// Everyone born or married into it, in the order they joined.
    pub members: Vec<usize>,
    /// Every realm it has ruled, in the order it first ruled them.
    pub realms: Vec<usize>,
    /// Those who actually held a throne, in the order they came to one.
    /// This is the spine of the house, and what the family tree draws.
    pub seniors: Vec<usize>,
    /// The house this one branched off, if it is a cadet line.
    pub parent: Option<usize>,
    /// The most thrones it held at one time.
    pub peak_realms: usize,
}

impl House {
    /// Whether anyone of the line still lives.
    pub fn alive(&self) -> bool {
        self.ended.is_none()
    }
    /// How long it lasted, or has lasted so far.
    pub fn span(&self, year: i32) -> i32 {
        self.ended.unwrap_or(year) - self.founded
    }
}

/// Where two realms stand with one another, beyond the bare
/// [`Polity::tension`] that decides whether they come to blows.
///
/// Tension is a temperature; a stance is a commitment. They are kept apart
/// because a realm can be furious with an ally and placid towards a rival
/// it has not got round to yet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stance {
    /// No standing arrangement. Never stored: an absent key means this.
    Neutral,
    /// A declared enmity. Rivals join wars against each other's friends.
    Rival,
    /// A defensive tie. Allies are called into each other's wars.
    Allied,
    /// The two houses have married. A marriage can put one realm's heir on
    /// the other's throne, which is how a union happens without a war.
    Married,
}

impl Default for Stance {
    /// An arrangement nobody has made.
    fn default() -> Stance {
        Stance::Neutral
    }
}

impl Stance {
    /// The word the interface uses.
    pub fn name(self) -> &'static str {
        match self {
            Stance::Neutral => "neutral",
            Stance::Rival => "rival",
            Stance::Allied => "allied",
            Stance::Married => "married",
        }
    }
}

/// How a throne passes when its holder dies.
///
/// This is the single most consequential thing about a realm that nothing in
/// the world can see from the outside, and it is what decides whether a
/// conqueror's work outlives them or is divided among their children the
/// year they die.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Inheritance {
    /// The eldest surviving child takes everything.
    Primogeniture,
    /// Every adult child takes a share, and a great realm becomes several.
    Partition,
    /// The strongest claimant takes it, and the others may not accept that.
    Tanistry,
    /// Somebody is chosen, and blood counts for little.
    Elective,
}

impl Inheritance {
    /// The word the interface uses.
    pub fn name(self) -> &'static str {
        match self {
            Inheritance::Primogeniture => "primogeniture",
            Inheritance::Partition => "partition",
            Inheritance::Tanistry => "tanistry",
            Inheritance::Elective => "election",
        }
    }
    /// Whether blood counts at all under this custom.
    pub fn has_heirs(self) -> bool {
        self != Inheritance::Elective
    }
    /// How the detail page explains it in a clause.
    pub fn describe(self) -> &'static str {
        match self {
            Inheritance::Primogeniture => "the eldest child takes the whole",
            Inheritance::Partition => "the realm is divided among the heirs",
            Inheritance::Tanistry => "the strongest kinsman takes the throne",
            Inheritance::Elective => "the throne is not inherited",
        }
    }
}

/// A realm: land, cities, a ruler, an army and a treasury, and the running
/// tallies that decide whether it grows, holds or comes apart.
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
    /// The most cities the realm ever held at once. Kept beside
    /// [`Polity::peak_cells`] so that an epitaph pairs a peak with a peak
    /// rather than a high-water mark with a death-bed count.
    pub peak_cities: usize,
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
    /// Standing arrangements with other realms. An absent key is
    /// [`Stance::Neutral`], so the map holds only what was decided.
    pub stance: BTreeMap<usize, Stance>,
    /// The year each stance was taken, for the prose and the detail page.
    pub stance_since: BTreeMap<usize, i32>,
    /// Whom this realm pays tribute to, if it has been made to.
    pub overlord: Option<usize>,
    /// Realms that pay tribute here. Kept in step with their `overlord`.
    pub tributaries: Vec<usize>,
    /// How its throne passes.
    pub inheritance: Inheritance,
    /// The house on the throne. [`Polity::dynasty`] is its name, kept in
    /// step by `World::seat_house`; this is its identity.
    pub house: Option<usize>,
    /// The share of the settled world this realm held when it was largest.
    /// A hegemon is remembered as one even after it breaks.
    pub peak_share: f32,
    /// The year the world first acknowledged this realm as the power of the
    /// age, if it ever did. Recorded so that the acknowledgement happens
    /// once rather than every year it stays large.
    pub hegemon_since: Option<i32>,
    /// Living children of the ruling house with a claim here, oldest first.
    pub heirs: Vec<usize>,
    /// How far the realm's land lies from its seat, as a multiple of the
    /// reach a realm of its development can govern. 1.0 is a realm whose
    /// average province sits exactly at the edge of comfortable rule.
    ///
    /// This is the brake that size alone never provided. Administrative
    /// capacity grows with cities, and a conqueror takes cities, so conquest
    /// used to pay for its own administration and nothing stopped a realm
    /// that got ahead. Distance does not work that way: taking a province
    /// two months' ride from the capital makes the next one harder, not
    /// easier, and an empire breaks along its far edge first.
    pub sprawl: f32,
    /// What this realm's knowledge adds to what it can govern.
    ///
    /// Kept on the realm because [`Polity::admin_capacity`] has no view of
    /// the world; filled by `recompute` from `polity_known`, and rebuilt on
    /// load because `ser::load` ends by calling `recompute`.
    pub tech_admin: f32,
}

impl Polity {
    /// Whether the realm still stands.
    pub fn alive(&self) -> bool {
        self.fell.is_none()
    }
    /// Whether it is fighting anyone.
    pub fn at_war(&self) -> bool {
        !self.wars.is_empty()
    }
    /// How many cells it can govern before it starts to fray.
    pub fn admin_capacity(&self, t: &Tuning) -> f32 {
        t.admin_capacity_base
            + self.kind.admin_bonus()
            + self.dev * t.admin_capacity_dev_weight
            + self.cities.len() as f32 * t.admin_capacity_city_weight
            + self.tech_admin
    }
    /// Land held over [`Polity::admin_capacity`]: 1.0 is exactly at the limit.
    pub fn overextension(&self, t: &Tuning) -> f32 {
        (self.cells as f32 / self.admin_capacity(t)).max(0.0)
    }
}

/// What a person is remembered as.
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
    /// Born to a ruling house and never came to a throne. Appended last so
    /// that the save format's role table keeps its existing positions.
    Noble,
}

impl Role {
    /// The common noun for this role.
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
            Role::Noble => "noble",
        }
    }
}

/// A person's gender, which chooses their honorific.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gender {
    F,
    M,
    N,
}

/// What a person is like. Every trait is in `[0, 1]`.
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
    /// A fresh set of traits, each near the middle of its range.
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
    /// A child's traits: the parent's, pulled back towards the middle.
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
    /// The two or three traits that stand out, in words.
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

/// Somebody worth remembering: a ruler, a general, a mage, a prophet.
///
/// A person carries their *own* tally of what they did, beside the realm's
/// [`Polity::reign_gained`] and friends. The realm's counters reset at every
/// accession, which is what they are for; a life has to outlast the throne
/// it sat on, because the whole point of a figure like a great conqueror is
/// that the chronicle still knows what they did four hundred years later.
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
    // --- kin ---
    /// Who they married, if anyone. Symmetric: both halves point at each other.
    pub spouse: Option<usize>,
    /// Their children, oldest first, living and dead.
    pub children: Vec<usize>,
    /// The house they belong to, which is not always the house they founded.
    pub house: Option<usize>,
    // --- what they did, for as long as the world remembers ---
    /// Land the realm gained while they held it, less what it lost —
    /// settlement and conquest together.
    pub gained: i32,
    /// Of that, the land taken from somebody else in war. Kept apart
    /// because a conqueror and a coloniser are not the same figure, and
    /// "took 50 lands in war" must not be said of a ruler who cleared
    /// forest for fifty years.
    pub taken: i32,
    pub cities_founded: u32,
    pub cities_taken: u32,
    pub wars_won: u32,
    /// Years on the throne, totalled across every realm they ruled.
    pub reign_years: i32,
    /// The year they first came to any throne. The realm's `reign_start`
    /// only describes whoever sits there *now*, so it cannot date a dead
    /// ruler's accession — which is what a line of succession is drawn
    /// along.
    pub crowned: Option<i32>,
    /// A style won rather than inherited: "the Great", "Emperor of the West".
    pub title: Option<String>,
    /// How far they stand above their contemporaries, in points. Recomputed
    /// every year they live; frozen at death.
    pub greatness: f32,
    /// The year the world first called them great, if it ever did. This is
    /// the difference between a figure and an obituary.
    pub acclaimed: Option<i32>,
    /// A contemporary they are measured against.
    pub rival: Option<usize>,
    /// The ruler they rose under, which is how a general becomes a successor.
    pub served: Option<usize>,
}

impl Person {
    /// Name and epithet together: "Aurel the Patient".
    pub fn full_name(&self) -> String {
        match &self.epithet {
            Some(e) => format!("{} {}", self.name, e),
            None => self.name.clone(),
        }
    }
    /// Whether they are still living.
    pub fn alive(&self) -> bool {
        self.died.is_none()
    }
    /// Their age in `year`, or their age at death if they are dead.
    pub fn age(&self, year: i32) -> i32 {
        self.died.unwrap_or(year) - self.born
    }
    /// Whether the world has called them great in their own lifetime.
    pub fn is_acclaimed(&self) -> bool {
        self.acclaimed.is_some()
    }
    /// "she", "he" or "they", for prose that has to refer back to them.
    pub fn they(&self) -> &'static str {
        match self.gender {
            Gender::F => "she",
            Gender::M => "he",
            Gender::N => "they",
        }
    }
    /// "her", "his" or "their".
    pub fn their(&self) -> &'static str {
        match self.gender {
            Gender::F => "her",
            Gender::M => "his",
            Gender::N => "their",
        }
    }
    /// "her", "him" or "them".
    pub fn them(&self) -> &'static str {
        match self.gender {
            Gender::F => "her",
            Gender::M => "him",
            Gender::N => "them",
        }
    }
}

/// What a school teaches: magic, a faith, or a way of thinking.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SchoolKind {
    Arcane,
    Divine,
    Philosophical,
}

impl SchoolKind {
    /// The common noun for this kind of school.
    pub fn name(self) -> &'static str {
        match self {
            SchoolKind::Arcane => "arcane school",
            SchoolKind::Divine => "faith",
            SchoolKind::Philosophical => "philosophy",
        }
    }
    /// What its followers are called.
    pub fn follower(self) -> &'static str {
        match self {
            SchoolKind::Arcane => "adepts",
            SchoolKind::Divine => "faithful",
            SchoolKind::Philosophical => "disciples",
        }
    }
}

/// An order, faith or academy: a doctrine, a home city and a hold over
/// however many realms will listen to it.
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
    /// Whether anyone still teaches it.
    pub fn alive(&self) -> bool {
        self.extinct.is_none()
    }
    /// Its hold over every realm added together.
    pub fn total_influence(&self) -> f32 {
        self.influence.values().sum()
    }
}

/// What a war is being fought over.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WarKind {
    Conquest,
    Rebellion,
    CivilWar,
    Holy,
    Raid,
    Succession,
}

/// A war: two realms, a running score, and how it ended.
pub struct War {
    #[allow(dead_code)]
    pub id: usize,
    /// The realm that declared it, and the principal on the other side.
    /// These two name the war and settle the peace; `allies_a` and
    /// `allies_d` are everyone else who came in.
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
    /// Realms fighting alongside the attacker, in the order they joined.
    pub allies_a: Vec<usize>,
    /// Realms fighting alongside the defender.
    pub allies_d: Vec<usize>,
    /// What the attacker says it is for, and what a peace has to settle.
    pub aim: WarAim,
    /// Whether the aim was met when the war ended.
    pub aim_met: bool,
}

/// What a war is actually *for*: the thing a peace has to resolve.
///
/// A war without an aim can only end in a draw worth no sentence, which is
/// what made most of them forgettable. An aim gives the declaration a
/// promise, the peace a verdict, and the loser something to remember.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WarAim {
    /// Take and keep a stretch of the enemy's border.
    Border,
    /// Take one named city.
    City(usize),
    /// Make the enemy a tributary rather than take its land.
    Vassalage,
    /// Throw off an overlord.
    Independence,
    /// Put a particular claimant on the enemy's throne.
    Claimant(usize),
    /// Make the enemy accept the attacker's school.
    Faith,
    /// Seize a particular relic.
    Relic(usize),
    /// Loot, and go home.
    Plunder,
    /// Break the strongest realm in the world before it swallows everyone.
    Containment,
}

impl War {
    /// Whether the fighting is still going on.
    pub fn alive(&self) -> bool {
        self.ended.is_none()
    }
}

/// What the simulation keeps for each map cell, on top of its terrain.
#[derive(Clone, Copy, Default)]
pub struct CellState {
    pub owner: Option<usize>,
    pub culture: Option<usize>,
    pub pop: f32,
    pub city: Option<usize>,
    pub since: i32,
    pub plague: u8,
}

/// What sort of thing a relic is.
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

/// Who has a relic now, if anyone.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Holder {
    Polity(usize),
    Person(usize),
    City(usize),
    Lost,
}

/// A relic: made once, passed from hand to hand, and lost often enough to
/// be worth going looking for.
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

/// What a seer foretold, in a form the simulation can check.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProphecyKind {
    RealmFalls(usize),
    CrownOfEmpire(usize),
    CityBurns(usize, u32),
    RulerMurdered(usize),
    FaithSpreads(usize, usize),
    RelicReturns(usize, usize),
}

/// A prophecy, its deadline, and whether it came true.
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

/// A named age of the world, given to it in hindsight.
pub struct Era {
    #[allow(dead_code)]
    pub start: i32,
    pub name: String,
    pub description: String,
}

/// A pestilence running its course through some realms.
pub struct Plague {
    pub name: String,
    pub years_left: i32,
    pub polities: Vec<usize>,
    pub deaths: f64,
}

/// World totals, recomputed each year for the sidebar and the balance harness.
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

/// How much of the distance to an overseas province a realm with total
/// mastery of the sea is forgiven. At 0.5, land across water is half as far
/// from the capital as the map says.
const SEA_BINDS_FACTOR: f32 = 0.5;

/// The water a realm can cross the moment it first builds sea-going hulls,
/// before its people's seafaring and its own development are counted.
const SEA_REACH_BASE: f32 = 3.0;

/// The phases of a tick, in the order `World::tick` runs them.
pub const PHASES: [&str; 23] = [
    "grow_and_migrate",
    "form_polities",
    "expand",
    "found_cities",
    "economy",
    "diplomacy",
    "alliances",
    "tribute",
    "resolve_wars",
    "dynasty",
    "rulers",
    "unrest",
    "magic",
    "culture_drift",
    "tech",
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

/// Everything there is: the map, everyone on it, and all that has happened.
///
/// Entities live in flat `Vec`s and refer to one another by index, so that
/// any part of the world can be reached from any other without a borrow
/// fight. An index is never reused: a realm that falls keeps its slot.
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
    pub houses: Vec<House>,
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
    /// Every realm still standing, ascending by id.
    ///
    /// The same bargain as [`World::alive_persons`]: the polity vector is
    /// append-only and holds every realm that has ever existed, so scanning
    /// it to find the living costs the whole history of the world — and
    /// `living_polities` is called several times a tick by several phases.
    ///
    /// Appended to by `politics::found_polity` and pruned by
    /// `politics::fall` and `recompute`.
    pub alive_polities: Vec<usize>,
    /// What the ground itself knows: a bitset over this world's own
    /// [`tech::Innovation`] tree, one `u128` per cell.
    ///
    /// Held by the land rather than by the realm on it, which is the whole
    /// point: a realm that falls takes its treasury and its army with it
    /// and leaves its irrigation, its roads and its writing behind for
    /// whoever comes next. This is the only thing in the world that
    /// accumulates across the fall of the state that built it.
    pub known: Vec<u128>,
    /// What each realm knows anywhere in its lands, gathered by `recompute`
    /// so that `knows` is a bit test rather than a walk over its territory.
    pub polity_known: Vec<u128>,
    /// What each cell's knowledge is worth to its harvests, cached.
    ///
    /// `cell_capacity` is called for every cell every year, and working the
    /// multiplier out from the bitset means walking the whole tree each
    /// time — ten thousand cells times a hundred and twenty innovations,
    /// every year, which was six times the cost of the phase it sat in.
    /// Knowledge changes rarely, so it is recomputed where it changes
    /// instead: see `World::refresh_yield`.
    pub cell_yield: Vec<f32>,
    /// This world's own tree of innovations, grown from its seed. No two
    /// worlds have the same one.
    pub techs: Vec<tech::Innovation>,
    /// Whether each innovation has been worked out anywhere yet, so that
    /// the first time is told as news and the hundredth is not.
    pub tech_seen: Vec<bool>,
    /// The year each was first worked out, for the naming of ages.
    pub tech_first_year: Vec<i32>,
    /// Everyone still living, ascending by id.
    ///
    /// Kept beside `persons` for the same reason `owner_cells` is kept
    /// beside `cells`: the person vector is append-only and holds everybody
    /// who has ever lived, so a phase that walks it costs the whole history
    /// of the world every year. Mortality and the yearly scoring of
    /// standing both walk the living, which is a bounded set.
    ///
    /// Appended to by `new_person` and pruned once a tick by
    /// `World::forget_the_dead`.
    pub alive_persons: Vec<usize>,
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
            houses: Vec::new(),
            persons: Vec::new(),
            known: Vec::new(),
            polity_known: Vec::new(),
            cell_yield: Vec::new(),
            techs: Vec::new(),
            tech_seen: Vec::new(),
            tech_first_year: Vec::new(),
            alive_persons: Vec::new(),
            alive_polities: Vec::new(),
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

    /// Write the world to `path` through a temporary file, so an interrupted
    /// save cannot leave a half-written one behind. Returns the byte count.
    ///
    /// # Errors
    ///
    /// [`crate::ser::SaveError::Io`] if the directory or the file cannot be
    /// written or the rename into place fails.
    pub fn save_to(&mut self, path: &std::path::Path) -> Result<usize, crate::ser::SaveError> {
        let bytes = crate::ser::save(self);
        if let Some(dir) = path.parent() {
            if !dir.as_os_str().is_empty() {
                std::fs::create_dir_all(dir)
                    .map_err(|e| io_err(&format!("cannot create {}", dir.display()), &e))?;
            }
        }
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &bytes)
            .map_err(|e| io_err(&format!("cannot write {}", tmp.display()), &e))?;
        std::fs::rename(&tmp, path)
            .map_err(|e| io_err(&format!("cannot rename to {}", path.display()), &e))?;
        Ok(bytes.len())
    }

    /// Read a world back from `path`.
    ///
    /// # Errors
    ///
    /// [`crate::ser::SaveError`] if the file cannot be read, is not a save, or
    /// does not survive its own checksum.
    pub fn load_from(path: &std::path::Path) -> Result<World, crate::ser::SaveError> {
        let bytes = std::fs::read(path)
            .map_err(|e| io_err(&format!("cannot read {}", path.display()), &e))?;
        crate::ser::load(&bytes)
    }

    /// Raise a world `w` by `h` from `seed` and populate its first age.
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
            houses: Vec::new(),
            persons: Vec::new(),
            known: vec![0; n],
            polity_known: Vec::new(),
            cell_yield: Vec::new(),
            techs: Vec::new(),
            tech_seen: Vec::new(),
            tech_first_year: Vec::new(),
            alive_persons: Vec::new(),
            alive_polities: Vec::new(),
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
        // The world's own tree of ideas, grown before its peoples so that
        // the first of them can already be working something out. A hundred
        // and twenty innovations is enough to keep a world learning for ten
        // thousand years without the tree ever running out.
        world.techs = tech::grow_tree(&world.rng, tech::MAX_TECHS.min(120));
        world.tech_seen = vec![false; world.techs.len()];
        world.tech_first_year = vec![0; world.techs.len()];
        genesis::populate(&mut world);
        world.recompute();
        world
    }

    // -- logging ----------------------------------------------------------

    /// Record an event, unless the current [`Detail`] does not care for it.
    /// Returns the event's id when one was kept.
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

    /// Whether the world is running at the highest detail, which is the one
    /// thing detail may be asked: whether to append a flourish to a
    /// sentence. It must never gate a draw from the RNG (see [`Detail`]).
    pub fn high_detail(&self) -> bool {
        self.detail == Detail::High
    }

    // -- naming helpers -----------------------------------------------------

    /// The language of the realm's culture, for naming what it finds.
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

    /// The ruler's name and title, short enough for a sidebar line.
    pub fn ruler_short(&self, p: usize) -> String {
        match self.polities[p].ruler {
            Some(r) => {
                let per = &self.persons[r];
                format!("{} {}", self.honorific(p, per.gender), per.full_name())
            }
            None => "the council".to_string(),
        }
    }

    /// What this realm calls a ruler of that gender: King, Khan, Hierarch.
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
            // A kingdom does not style its ruler an emperor, whatever the
            // tongue's own word is, so an imperial honorific falls back to
            // the royal one — by gender, not blindly to "King".
            PolityKind::Kingdom => match (lang.honorific.as_str(), g) {
                ("King", Gender::F) => "Queen".to_string(),
                ("Queen", Gender::M) => "King".to_string(),
                ("Prince", Gender::F) => "Princess".to_string(),
                ("Emperor" | "Empress", Gender::F) => "Queen".to_string(),
                ("Emperor" | "Empress", _) => "King".to_string(),
                (h, _) => h.to_string(),
            },
        }
    }

    /// Drop the dead from [`World::alive_persons`], once a tick.
    ///
    /// Costs the number of living rather than the number who have ever
    /// lived, and keeps the list sorted, which several phases rely on for
    /// a stable visit order.
    pub fn forget_the_dead(&mut self) {
        let persons = &self.persons;
        self.alive_persons.retain(|&i| persons[i].alive());
    }

    // -- the sea ----------------------------------------------------------

    /// How wide a stretch of open water this realm can carry a colony or an
    /// army across, in cells. Zero until it has taken to the sea at all.
    ///
    /// Reach is what turns the sea from a wall into a distance. A realm that
    /// has only just learned to build sea-going hulls can cross a strait; an
    /// old, developed, seafaring power can cross an ocean, and only such a
    /// power can reach the far isles. It is capped at
    /// [`crate::geo::MAX_CROSSING`] because that is the widest water the map
    /// records a route over.
    pub fn sea_reach(&self, p: usize) -> u16 {
        let pol = &self.polities[p];
        if !pol.seafaring || !pol.alive() {
            return 0;
        }
        let race = &self.races[self.cultures[pol.culture].race];
        let kind_bonus = match pol.kind {
            // A republic on the water is a thalassocracy: its whole reason
            // for existing is the carrying trade.
            PolityKind::Republic => 3.0,
            PolityKind::Empire => 2.0,
            PolityKind::Horde => -2.0,
            _ => 0.0,
        };
        let reach = SEA_REACH_BASE
            + race.seafaring * 8.0
            + pol.dev * 2.5
            + kind_bonus
            + self.tech_sea_bonus(p);
        reach.clamp(1.0, crate::geo::MAX_CROSSING as f32) as u16
    }

    /// Every crossing that leaves land this realm holds, with the cell on
    /// the far side. Only those within its reach.
    ///
    /// Cheap: the route graph is static and sorted, so this walks the
    /// realm's own coast rather than searching the map.
    pub fn sea_sorties(&self, p: usize) -> Vec<crate::geo::Crossing> {
        let reach = self.sea_reach(p);
        if reach == 0 {
            return Vec::new();
        }
        let mut out = Vec::new();
        for &i in self.cells_of_ref(p) {
            if !self.terrain.coast[i] {
                continue;
            }
            for c in self.terrain.crossings_from(i) {
                if c.width <= reach {
                    out.push(*c);
                }
            }
        }
        out
    }

    // -- houses -----------------------------------------------------------

    /// Found a house and return its index.
    pub fn new_house(&mut self, name: String, culture: usize, founder: Option<usize>) -> usize {
        let id = self.houses.len();
        self.houses.push(House {
            id,
            name,
            culture,
            founder,
            founded: self.year,
            ended: None,
            members: Vec::new(),
            realms: Vec::new(),
            seniors: Vec::new(),
            parent: None,
            peak_realms: 0,
        });
        if let Some(f) = founder {
            // A house is as old as its founder's reign, not as old as the
            // moment the world got round to naming it: a chiefdom that
            // becomes a kingdom names the dynasty of a ruler who has
            // already been on the throne for years, and dating it from the
            // promotion put the founder's accession before the founding of
            // his own house.
            if let Some(crowned) = self.persons[f].crowned {
                self.houses[id].founded = crowned.min(self.houses[id].founded);
            }
            // Somebody who already belonged to a house and now founds one
            // is starting a cadet branch, not appearing from nowhere. The
            // link is what lets the family tree show where it forked.
            let old = self.persons[f].house;
            self.houses[id].parent = old;
            if let Some(o) = old {
                // They keep their place among the old house's *members* —
                // blood does not change — but they come off its line of
                // succession, because from here they rule as the first of
                // their own. Left on both, they appeared twice in two
                // different families' trees.
                self.houses[o].seniors.retain(|&x| x != f);
            }
            self.join_house(f, id);
        }
        id
    }

    /// Enrol somebody in a house, once.
    pub fn join_house(&mut self, person: usize, house: usize) {
        if self.persons[person].house == Some(house) {
            return;
        }
        self.persons[person].house = Some(house);
        if !self.houses[house].members.contains(&person) {
            self.houses[house].members.push(person);
        }
        // Somebody joining is proof the line is not extinct after all,
        // which happens when a cadet branch outlives the senior one.
        self.houses[house].ended = None;
    }

    /// Seat a house on a realm's throne, keeping the realm's display name
    /// for the dynasty in step with the house's own.
    ///
    /// Every write to `Polity::dynasty` goes through here. The string and
    /// the index are two views of one fact, and the only way to keep them
    /// from drifting is to have one place that sets both.
    pub fn seat_house(&mut self, p: usize, house: usize) {
        self.polities[p].house = Some(house);
        self.polities[p].dynasty = self.houses[house].name.clone();
        if !self.houses[house].realms.contains(&p) {
            self.houses[house].realms.push(p);
        }
        let held = self.houses[house]
            .realms
            .iter()
            .filter(|&&q| self.polities[q].alive() && self.polities[q].house == Some(house))
            .count();
        if held > self.houses[house].peak_realms {
            self.houses[house].peak_realms = held;
        }
        self.houses[house].ended = None;
    }

    /// Record that somebody of this house has come to a throne. The list is
    /// the spine the family tree is drawn along.
    pub fn house_accession(&mut self, person: usize) {
        if let Some(h) = self.persons[person].house {
            if !self.houses[h].seniors.contains(&person) {
                self.houses[h].seniors.push(person);
            }
            // The realm goes on the house's list whether or not it formally
            // took the house's name: a reader looking at the line of
            // succession sees the realm each ruler held, and every one of
            // those has to be a throne the house is listed as having held.
            if let Some(p) = self.persons[person].polity {
                if !self.houses[h].realms.contains(&p) {
                    self.houses[h].realms.push(p);
                }
            }
        }
    }

    /// Mark a house as ended, if nothing of it is left.
    ///
    /// A house is not finished merely because it has lost a throne — cadet
    /// branches and living members outlast a deposition — so this checks
    /// both before writing the year down.
    pub fn close_house_if_spent(&mut self, house: usize) {
        if self.houses[house].ended.is_some() {
            return;
        }
        let holds_throne = self.houses[house]
            .realms
            .iter()
            .any(|&p| self.polities[p].alive() && self.polities[p].house == Some(house));
        if holds_throne {
            return;
        }
        let living = self.houses[house]
            .members
            .iter()
            .any(|&m| self.persons[m].alive());
        if !living {
            self.houses[house].ended = Some(self.year);
        }
    }

    // -- crediting a life -------------------------------------------------
    //
    // Every deed is recorded twice: once against the realm, whose counters
    // reset at the next accession, and once against the person, whose do
    // not. The pair is what lets the chronicle still know four centuries
    // later which ruler took the land.

    /// Land won or lost, credited to the realm and to whoever rules it.
    pub fn credit_land(&mut self, p: usize, delta: i32) {
        self.polities[p].reign_gained += delta;
        if let Some(r) = self.polities[p].ruler {
            self.persons[r].gained += delta;
        }
    }

    /// Land taken from, or lost to, another realm in war. Counted against
    /// the total as well, so `taken` is always a subset of `gained`.
    pub fn credit_conquest(&mut self, p: usize, delta: i32) {
        self.credit_land(p, delta);
        if let Some(r) = self.polities[p].ruler {
            self.persons[r].taken += delta;
        }
    }

    /// A city founded by this realm.
    pub fn credit_city_founded(&mut self, p: usize) {
        self.polities[p].reign_cities += 1;
        if let Some(r) = self.polities[p].ruler {
            self.persons[r].cities_founded += 1;
        }
    }

    /// A city stormed or taken by this realm.
    pub fn credit_city_taken(&mut self, p: usize) {
        if let Some(r) = self.polities[p].ruler {
            self.persons[r].cities_taken += 1;
        }
    }

    /// A war brought to a victorious peace by this realm.
    pub fn credit_war_won(&mut self, p: usize) {
        self.polities[p].reign_wars_won += 1;
        if let Some(r) = self.polities[p].ruler {
            self.persons[r].wars_won += 1;
        }
    }

    /// Bring somebody into the world and return their index.
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
            spouse: None,
            children: Vec::new(),
            house: None,
            gained: 0,
            taken: 0,
            cities_founded: 0,
            cities_taken: 0,
            wars_won: 0,
            reign_years: 0,
            crowned: None,
            title: None,
            greatness: 0.0,
            acclaimed: None,
            rival: None,
            served: None,
        });
        // Ids only ever increase, so appending keeps the list sorted.
        self.alive_persons.push(id);
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

    /// A realm's colour on the map, spread around the hue circle by index so
    /// that neighbours rarely clash.
    pub fn polity_color(&self, id: usize) -> Rgb {
        let hue = (id as f32 * 137.508) % 360.0;
        let sat = if id % 3 == 0 { 0.75 } else { 0.55 };
        let val = if id % 2 == 0 { 0.85 } else { 0.65 };
        Rgb::from_hsv(hue, sat, val)
    }

    // -- aggregates -------------------------------------------------------

    /// Rebuild every aggregate: cell counts, populations, neighbours and the
    /// owner index. Run once a year and after loading.
    pub fn recompute(&mut self) {
        self.forget_the_fallen();
        self.forget_the_dead();
        // Knowledge is parallel to the cells; a world assembled from an
        // older file may not have brought one for every cell.
        if self.known.len() != self.cells.len() {
            self.known.resize(self.cells.len(), 0);
        }
        if self.cell_yield.len() != self.cells.len() {
            self.cell_yield = (0..self.cells.len())
                .map(|i| self.tech_capacity_mult(i))
                .collect();
        }
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
        // What each realm knows, gathered in the sweep that is already
        // walking every cell, so that `knows` is a bit test afterwards
        // rather than a walk over the realm's own land.
        let mut known_by: Vec<u128> = vec![0; self.polities.len()];
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
                known_by[p] |= self.known[i];
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
        // Neighbours across water.
        //
        // Without this the sea is not a distance but a wall: two realms on
        // opposite shores were not neighbours at all, so no tension ever
        // built between them, so no war was ever declared, so an army could
        // see a coast it could never be sent to. A crossing only counts when
        // somebody's reach can actually make it, and it counts for little —
        // a sea border is one cell of contact, so tension over water rises
        // slowly, the way a quarrel with a realm you rarely meet should.
        let reaches: Vec<u16> = (0..self.polities.len())
            .map(|p| self.sea_reach(p))
            .collect();
        for c in &self.terrain.crossings {
            let (a, b) = (c.from as usize, c.to as usize);
            let (Some(p), Some(q)) = (self.cells[a].owner, self.cells[b].owner) else {
                continue;
            };
            if p == q {
                continue;
            }
            // One of them has to be able to cross it for the two to be in
            // any kind of contact.
            if c.width > reaches[p].max(reaches[q]) {
                continue;
            }
            borders.push((p.min(q), p.max(q)));
        }
        self.polity_known = known_by;
        for p in 0..self.polities.len() {
            self.polities[p].tech_admin = self.tech_admin_bonus(p);
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
        // Mean distance of a realm's land from its seat. One pass over the
        // owner index, which is already built, so this costs the map once
        // rather than the map times the number of realms.
        let mut sprawl_sum: Vec<f64> = Vec::with_capacity(self.polities.len());
        for p in 0..self.polities.len() {
            sprawl_sum.push(match self.capital_cell(p) {
                Some(seat) => {
                    // Land across water counts for less the better the realm
                    // sails. For most of history the sea was the fast road:
                    // a province a month's sail away was closer to the
                    // capital than one a month's march inland, and an empire
                    // that controlled the water was *bound* by it rather
                    // than stretched across it.
                    //
                    // Without this the sea could be opened and would then be
                    // punished — every overseas holding would drive up
                    // `sprawl`, cost stability, and fray the realm that took
                    // it, so nothing would ever be held for long.
                    let home = self.terrain.landmass[seat];
                    let reach = self.sea_reach(p) as f32;
                    let over_sea =
                        1.0 - SEA_BINDS_FACTOR * (reach / crate::geo::MAX_CROSSING as f32).min(1.0);
                    self.owner_cells[p]
                        .iter()
                        .map(|&i| {
                            let d = self.terrain.dist(seat, i) as f64;
                            if self.terrain.landmass[i] == home {
                                d
                            } else {
                                d * over_sea as f64
                            }
                        })
                        .sum()
                }
                None => 0.0,
            });
        }
        let reach_base = self.tuning.admin_reach_base;
        let reach_dev = self.tuning.admin_reach_dev_weight;
        for p in self.polities.iter_mut() {
            if p.cells > 0 {
                p.foreign_share = foreign[p.id] as f32 / p.cells as f32;
                p.avg_fertility = fert[p.id] / p.cells as f32;
                let mean = (sprawl_sum[p.id] / p.cells as f64) as f32;
                let reach = (reach_base + p.dev * reach_dev).max(1.0);
                p.sprawl = mean / reach;
            } else {
                p.sprawl = 0.0;
            }
            p.cultures_within = p.culture_counts.len();
            if p.cells > p.peak_cells {
                p.peak_cells = p.cells;
                p.peak_year = self.year;
            }
            if p.cities.len() > p.peak_cities {
                p.peak_cities = p.cities.len();
            }
            if owned > 0 {
                let share = p.cells as f32 / owned as f32;
                if share > p.peak_share {
                    p.peak_share = share;
                }
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

    /// Advance the world one year, running the phases in [`PHASES`] order.
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
        phase!(6, war::alliances(self));
        phase!(7, war::tribute(self));
        phase!(8, war::resolve_wars(self));
        // Houses run before rulers: a succession has to be able to draw on
        // heirs who were born, named and grown up in front of the reader,
        // and who are current as of this year.
        phase!(9, dynasty::tick(self));
        phase!(10, politics::rulers(self));
        phase!(11, politics::unrest(self));
        phase!(12, magic::tick(self));
        phase!(13, people::culture_drift(self));
        phase!(14, tech::tick(self));
        phase!(15, events::disasters(self));
        phase!(16, events::notables(self));
        phase!(17, events::wonders(self));
        phase!(18, stories::tick_artifacts(self));
        phase!(19, stories::tick_prophecies(self));
        phase!(20, stories::tick_legends(self));
        phase!(21, self.recompute());
        phase!(22, events::eras(self));
        self.chronicle.compact(self.tuning.chronicle_cap);
        if self.year % 10 == 0 {
            self.stats.pop_history.push(self.stats.pop);
        }
        self.ticks_ms = self.ticks_ms * 0.9 + t0.elapsed().as_secs_f64() * 1000.0 * 0.1;
    }

    // -- the owner index --------------------------------------------------

    /// Cells owned by polity `p`, ascending.
    pub fn cells_of_ref(&self, p: usize) -> &[usize] {
        self.owner_cells
            .get(p)
            .map(std::vec::Vec::as_slice)
            .unwrap_or(&[])
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

    /// The indices of every realm that still stands.
    pub fn living_polities(&self) -> Vec<usize> {
        self.alive_polities.clone()
    }

    /// Drop the fallen from [`World::alive_polities`].
    pub fn forget_the_fallen(&mut self) {
        let polities = &self.polities;
        self.alive_polities.retain(|&p| polities[p].alive());
    }

    /// The cell the realm's capital sits on, if it has one standing.
    pub fn capital_cell(&self, p: usize) -> Option<usize> {
        self.polities[p].capital.map(|c| self.cities[c].cell)
    }
}
