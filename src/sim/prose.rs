//! The prose layer: every sentence the chronicle prints is written here.
//!
//! The simulation decides what happens; this module decides how to say it.
//! Keeping the two apart means English grammar (articles, plurals,
//! pronouns, lists) is handled once, in tested helpers, instead of being
//! rebuilt by hand in a hundred `format!` calls.
//!
//! Two rules matter for the rest of the tree:
//!
//! * **Determinism.** The simulation must draw the same random numbers in
//!   the same order for a given seed. A prose helper that needs variety
//!   either takes the world RNG and draws *exactly once* per choice
//!   ([`Pick::rolled`]), or draws nothing at all and derives its choice
//!   from the year and an entity id ([`Pick::stable`]).
//! * **Naming.** A realm is introduced by its full name ("the Kingdom of
//!   Velen") the first time it appears in an event and referred to by its
//!   short name ("Velen") afterwards. [`realm_full`] and [`realm`] make
//!   that rule easy to follow.

pub mod circles;
pub mod diplomacy;
pub mod dynasty;
pub mod events;
pub mod genesis;
pub mod magic;
pub mod war;

mod people;
mod politics;
mod stories;
pub mod trade;

pub use circles::*;
pub use diplomacy::*;
pub use dynasty::*;
pub use events::*;
pub use genesis::*;
pub use magic::*;
pub use people::*;
pub use politics::*;
pub use stories::*;
pub use trade::*;
pub use war::*;

use super::{Gender, PolityKind, World};
use crate::rng::Rng;

// ---------------------------------------------------------------------------
// Articles
// ---------------------------------------------------------------------------

/// Words whose spelling and sound disagree about `a` / `an`.
const AN_EXCEPTIONS: &[&str] = &["hour", "honest", "honour", "honor", "heir", "heirloom"];
const A_EXCEPTIONS: &[&str] = &[
    "unicorn",
    "united",
    "union",
    "universal",
    "university",
    "useful",
    "user",
    "eulogy",
    "european",
    "ewe",
    "one",
    "once",
];

/// `"a"` or `"an"`, whichever suits the noun phrase that follows.
pub fn article(noun: &str) -> &'static str {
    let first = noun
        .split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '\''))
        .find(|w| !w.is_empty())
        .unwrap_or("");
    let lower = first.to_lowercase();
    if AN_EXCEPTIONS.iter().any(|&w| lower == w) {
        return "an";
    }
    if A_EXCEPTIONS.iter().any(|&w| lower == w) {
        return "a";
    }
    match lower.chars().next() {
        Some(c) if "aeiou".contains(c) => "an",
        _ => "a",
    }
}

/// A noun phrase with its indefinite article: `a("empire") == "an empire"`.
pub fn a(noun: &str) -> String {
    format!("{} {}", article(noun), noun)
}

/// Like [`a`], but for the start of a sentence: `"An empire"`.
pub fn cap_a(noun: &str) -> String {
    format!("{} {}", cap(article(noun)), noun)
}

// ---------------------------------------------------------------------------
// Capitalisation
// ---------------------------------------------------------------------------

/// Capitalise the first letter, leaving the rest alone, so that
/// `"the Kingdom of Velen"` becomes `"The Kingdom of Velen"`. Leading
/// quotation marks are stepped over.
pub fn cap(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 1);
    let mut done = false;
    for c in s.chars() {
        if done || c == '"' || c == '\'' || c == '(' || c.is_whitespace() {
            out.push(c);
            continue;
        }
        out.extend(c.to_uppercase());
        done = true;
    }
    out
}

// ---------------------------------------------------------------------------
// Number and plurals
// ---------------------------------------------------------------------------

const IRREGULAR: &[(&str, &str)] = &[
    ("person", "people"),
    ("people", "peoples"),
    ("man", "men"),
    ("woman", "women"),
    ("child", "children"),
    ("city", "cities"),
    ("foot", "feet"),
    ("tooth", "teeth"),
    ("ox", "oxen"),
    ("mouse", "mice"),
    ("wife", "wives"),
    ("life", "lives"),
    ("knife", "knives"),
    ("thief", "thieves"),
];

/// The plural of a noun: `plural("city") == "cities"`.
pub fn plural(word: &str) -> String {
    let lower = word.to_lowercase();
    if let Some(&(_, p)) = IRREGULAR.iter().find(|(s, _)| *s == lower) {
        return p.to_string();
    }
    if lower.ends_with('s')
        || lower.ends_with('x')
        || lower.ends_with('z')
        || lower.ends_with("ch")
        || lower.ends_with("sh")
    {
        return format!("{}es", word);
    }
    if lower.len() > 1 && lower.ends_with('y') {
        let before = lower.chars().nth(lower.len() - 2).unwrap_or('a');
        if !"aeiou".contains(before) {
            return format!("{}ies", &word[..word.len() - 1]);
        }
    }
    format!("{}s", word)
}

/// `"no battles"`, `"one battle"`, `"3 battles"`.
pub fn count(n: i64, word: &str) -> String {
    match n {
        0 => format!("no {}", plural(word)),
        1 | -1 => format!("one {}", word),
        _ => format!("{} {}", n, plural(word)),
    }
}

const NUMBER_WORDS: &[&str] = &[
    "no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven",
    "twelve",
];

/// Small numbers as words, larger ones as digits.
pub fn number(n: i64) -> String {
    if (0..NUMBER_WORDS.len() as i64).contains(&n) {
        NUMBER_WORDS[n as usize].to_string()
    } else {
        n.to_string()
    }
}

const ORDINAL_WORDS: &[&str] = &[
    "zeroth", "first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth",
    "tenth",
];

/// `"1st"`, `"2nd"`, `"13th"`.
pub fn ordinal(n: i64) -> String {
    let suffix = match (n.abs() % 100, n.abs() % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    format!("{}{}", n, suffix)
}

/// `"first"`, `"second"`, falling back to `"11th"` past the tenth.
pub fn ordinal_word(n: i64) -> String {
    if (0..ORDINAL_WORDS.len() as i64).contains(&n) {
        ORDINAL_WORDS[n as usize].to_string()
    } else {
        ordinal(n)
    }
}

/// A span of time in words: `"a year"`, `"12 years"`, `"a century"`.
pub fn years(n: i64) -> String {
    match n {
        i64::MIN..=0 => "less than a year".to_string(),
        1 => "a year".to_string(),
        100 => "a century".to_string(),
        n if n % 100 == 0 && n <= 1200 => format!("{} centuries", number(n / 100)),
        n => format!("{} years", n),
    }
}

// ---------------------------------------------------------------------------
// Lists
// ---------------------------------------------------------------------------

/// `"a"`, `"a and b"`, `"a, b, and c"`.
pub fn list_with_and<S: AsRef<str>>(items: &[S]) -> String {
    let items: Vec<&str> = items.iter().map(std::convert::AsRef::as_ref).collect();
    match items.len() {
        0 => String::new(),
        1 => items[0].to_string(),
        2 => format!("{} and {}", items[0], items[1]),
        _ => format!(
            "{}, and {}",
            items[..items.len() - 1].join(", "),
            items[items.len() - 1]
        ),
    }
}

/// [`list_with_and`] for the owned strings the simulation tends to hold.
pub fn join_names(names: &[String]) -> String {
    list_with_and(names)
}

// ---------------------------------------------------------------------------
// Pronouns
// ---------------------------------------------------------------------------

/// The pronouns to use for one person, including verb agreement for the
/// singular `they`.
///
/// The set is deliberately complete rather than minimal: prose written
/// later needs every form to hand, and a half-set is how hand-built
/// pronouns go wrong. The tests below exercise all of them.
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct Pronouns {
    /// "she", "he", "they"
    pub subject: &'static str,
    /// "her", "him", "them"
    pub object: &'static str,
    /// "her", "his", "their"
    pub possessive: &'static str,
    /// "hers", "his", "theirs"
    pub possessive_pronoun: &'static str,
    /// "herself", "himself", "themselves"
    pub reflexive: &'static str,
    /// True for the singular "they", which takes plural verbs.
    pub plural_verb: bool,
}

#[allow(dead_code)]
impl Pronouns {
    /// "She", "He", "They"
    pub fn subject_cap(&self) -> String {
        cap(self.subject)
    }
    /// "Her", "Him", "Them"
    pub fn object_cap(&self) -> String {
        cap(self.object)
    }
    /// "Her", "His", "Their"
    pub fn possessive_cap(&self) -> String {
        cap(self.possessive)
    }
    /// "Herself", "Himself", "Themselves"
    pub fn reflexive_cap(&self) -> String {
        cap(self.reflexive)
    }
    /// The right form of a verb: `verb("was", "were")`.
    pub fn verb(&self, singular: &'static str, plural: &'static str) -> &'static str {
        if self.plural_verb {
            plural
        } else {
            singular
        }
    }
    /// "was" or "were".
    pub fn was(&self) -> &'static str {
        self.verb("was", "were")
    }
    /// "has" or "have".
    pub fn has(&self) -> &'static str {
        self.verb("has", "have")
    }
}

/// The pronoun set for a gender.
pub fn pronouns(g: Gender) -> Pronouns {
    match g {
        Gender::F => Pronouns {
            subject: "she",
            object: "her",
            possessive: "her",
            possessive_pronoun: "hers",
            reflexive: "herself",
            plural_verb: false,
        },
        Gender::M => Pronouns {
            subject: "he",
            object: "him",
            possessive: "his",
            possessive_pronoun: "his",
            reflexive: "himself",
            plural_verb: false,
        },
        Gender::N => Pronouns {
            subject: "they",
            object: "them",
            possessive: "their",
            possessive_pronoun: "theirs",
            reflexive: "themselves",
            plural_verb: true,
        },
    }
}

/// The pronoun set for a person in the world.
pub fn who(w: &World, person: usize) -> Pronouns {
    pronouns(w.persons[person].gender)
}

// ---------------------------------------------------------------------------
// Choosing between variants
// ---------------------------------------------------------------------------

/// Chooses between phrasings without disturbing the simulation's stream of
/// random numbers.
///
/// [`Pick::rolled`] consumes exactly one draw per choice, which is what the
/// call sites that used `rng.below(k)` or `rng.pick(..)` always did.
/// [`Pick::stable`] consumes none: it hashes the year and an entity id, so
/// a site that never drew a number still never draws one. A stable pick
/// holds one hash, so several [`Pick::index`] calls on it are correlated;
/// make a fresh one per independent choice.
pub enum Pick<'a> {
    Rolled(&'a Rng),
    Stable(u64),
}

impl<'a> Pick<'a> {
    /// A choice made from the world's RNG: one draw per [`Pick::index`].
    pub fn rolled(rng: &'a Rng) -> Pick<'a> {
        Pick::Rolled(rng)
    }

    /// A choice derived from the year and an entity id: no draws at all.
    pub fn stable(year: i32, id: usize) -> Pick<'static> {
        let mut h = (year as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (id as u64);
        h ^= h >> 30;
        h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h ^= h >> 27;
        h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
        Pick::Stable(h ^ (h >> 31))
    }

    /// An index in `0..n`.
    pub fn index(&self, n: usize) -> usize {
        debug_assert!(n > 0);
        match self {
            Pick::Rolled(rng) => rng.below(n),
            Pick::Stable(h) => (*h % n.max(1) as u64) as usize,
        }
    }

    /// One of a set of variants.
    pub fn of<'t, T>(&self, variants: &'t [T]) -> &'t T {
        &variants[self.index(variants.len())]
    }

    /// One of a set of string variants, as an owned string.
    pub fn text(&self, variants: &[&str]) -> String {
        self.of(variants).to_string()
    }
}

// ---------------------------------------------------------------------------
// Naming things
// ---------------------------------------------------------------------------

/// A realm's full name, for its first mention in an event:
/// `"the Kingdom of Velen"`.
pub fn realm_full(w: &World, p: usize) -> String {
    w.polities[p].name.clone()
}

/// A realm's full name at the start of a sentence: `"The Kingdom of Velen"`.
pub fn realm_full_cap(w: &World, p: usize) -> String {
    cap(&w.polities[p].name)
}

/// A realm's short name, for every mention after the first: `"Velen"`.
pub fn realm(w: &World, p: usize) -> String {
    w.polities[p].short.clone()
}

/// Whether a realm's name takes a plural verb. "The Free Cities of Velen
/// **were** conquered", but "The Kingdom of Velen **was** conquered".
///
/// The head noun of a name is the word before "of" when there is one and
/// the last word otherwise, and a tribe is always its people, so it is
/// plural whatever its name looks like.
pub fn name_is_plural(name: &str, kind: PolityKind) -> bool {
    if kind == PolityKind::Tribe {
        return true;
    }
    let head = match name.split(" of ").next() {
        Some(before) if before.len() < name.len() => before,
        _ => name,
    };
    let last = head.split_whitespace().last().unwrap_or("");
    let lower = last.to_lowercase();
    lower.len() > 2 && lower.ends_with('s') && !lower.ends_with("ss") && !lower.ends_with("us")
}

/// Whether this realm takes plural verbs and pronouns.
pub fn realm_plural(w: &World, p: usize) -> bool {
    name_is_plural(&w.polities[p].name, w.polities[p].kind)
}

/// "was" or "were", to agree with a realm's name.
pub fn realm_was(w: &World, p: usize) -> &'static str {
    if realm_plural(w, p) {
        "were"
    } else {
        "was"
    }
}

/// "it" or "they", to agree with a realm's name.
pub fn realm_it(w: &World, p: usize) -> &'static str {
    if realm_plural(w, p) {
        "they"
    } else {
        "it"
    }
}

/// "its" or "their", to agree with a realm's name.
pub fn realm_its(w: &World, p: usize) -> &'static str {
    if realm_plural(w, p) {
        "their"
    } else {
        "its"
    }
}

/// A realm's adjective: `"Velenic"`.
pub fn realm_adj(w: &World, p: usize) -> String {
    w.polities[p].adj.clone()
}

/// A realm's capital, or its short name if it has none.
pub fn capital_name(w: &World, p: usize) -> String {
    w.polities[p]
        .capital
        .filter(|&c| w.cities[c].destroyed.is_none())
        .map(|c| w.cities[c].name.clone())
        .unwrap_or_else(|| w.polities[p].short.clone())
}

/// A school's full name, for its first mention: `"the Ashen Circle"`.
pub fn school_full(w: &World, s: usize) -> String {
    w.schools[s].name.clone()
}

/// A school's short name with the article its full name carries, for use
/// after a preposition: "the faithful of the Nameless", but
/// "the disciples of Raiwhism".
pub fn school_the(w: &World, s: usize) -> String {
    let short = &w.schools[s].short;
    if w.schools[s].name.starts_with("the ") {
        format!("the {}", short)
    } else {
        short.clone()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{Detail, World};

    #[test]
    fn articles_follow_sound_not_just_spelling() {
        assert_eq!(article("kingdom"), "a");
        assert_eq!(article("empire"), "an");
        assert_eq!(article("arcane school"), "an");
        assert_eq!(article("hour"), "an");
        assert_eq!(article("united league"), "a");
        assert_eq!(a("arcane school"), "an arcane school");
        assert_eq!(a("faith"), "a faith");
        assert_eq!(cap_a("arcane school"), "An arcane school");
        assert_eq!(cap_a("philosophy"), "A philosophy");
    }

    #[test]
    fn cap_handles_articles_and_quotes() {
        assert_eq!(cap("the Kingdom of Velen"), "The Kingdom of Velen");
        assert_eq!(cap("velen"), "Velen");
        assert_eq!(cap(""), "");
        assert_eq!(cap("\"doubt everything\""), "\"Doubt everything\"");
    }

    #[test]
    fn plurals_and_counts() {
        assert_eq!(plural("battle"), "battles");
        assert_eq!(plural("city"), "cities");
        assert_eq!(plural("church"), "churches");
        assert_eq!(plural("valley"), "valleys");
        assert_eq!(plural("person"), "people");
        assert_eq!(count(3, "battle"), "3 battles");
        assert_eq!(count(1, "battle"), "one battle");
        assert_eq!(count(0, "battle"), "no battles");
        assert_eq!(count(2, "city"), "2 cities");
    }

    #[test]
    fn numbers_and_ordinals() {
        assert_eq!(number(0), "no");
        assert_eq!(number(7), "seven");
        assert_eq!(number(40), "40");
        assert_eq!(ordinal(1), "1st");
        assert_eq!(ordinal(2), "2nd");
        assert_eq!(ordinal(3), "3rd");
        assert_eq!(ordinal(11), "11th");
        assert_eq!(ordinal(22), "22nd");
        assert_eq!(ordinal_word(2), "second");
        assert_eq!(ordinal_word(40), "40th");
    }

    #[test]
    fn plural_realm_names_take_plural_verbs() {
        use crate::sim::PolityKind::{Chiefdom, Kingdom, Republic, Tribe};
        assert!(!name_is_plural("the Kingdom of Velen", Kingdom));
        assert!(!name_is_plural("the Velenic Empire", Kingdom));
        assert!(name_is_plural("the Velenic Clans", Chiefdom));
        assert!(name_is_plural("the Free Cities of Velen", Republic));
        assert!(name_is_plural("the Towers of Velen", Republic));
        // A tribe is its people, whatever the name looks like.
        assert!(name_is_plural("the Safar", Tribe));
        // "-ss" and "-us" endings are singular.
        assert!(!name_is_plural("the Blessed Land of Velen", Kingdom));
    }

    #[test]
    fn spans_of_years() {
        assert_eq!(years(0), "less than a year");
        assert_eq!(years(1), "a year");
        assert_eq!(years(12), "12 years");
        assert_eq!(years(100), "a century");
        assert_eq!(years(300), "three centuries");
        assert_eq!(years(137), "137 years");
    }

    #[test]
    fn lists_read_as_english() {
        let none: [&str; 0] = [];
        assert_eq!(list_with_and(&none), "");
        assert_eq!(list_with_and(&["Velen"]), "Velen");
        assert_eq!(list_with_and(&["Velen", "Ashur"]), "Velen and Ashur");
        assert_eq!(
            list_with_and(&["Velen", "Ashur", "Kor"]),
            "Velen, Ashur, and Kor"
        );
        assert_eq!(
            join_names(&["Velen".to_string(), "Ashur".to_string()]),
            "Velen and Ashur"
        );
    }

    #[test]
    fn pronoun_sets() {
        let f = pronouns(Gender::F);
        assert_eq!((f.subject, f.object, f.possessive), ("she", "her", "her"));
        assert_eq!(f.reflexive, "herself");
        assert_eq!(f.was(), "was");
        let m = pronouns(Gender::M);
        assert_eq!((m.subject, m.object, m.possessive), ("he", "him", "his"));
        assert_eq!(m.possessive_pronoun, "his");
        let n = pronouns(Gender::N);
        assert_eq!(
            (n.subject, n.object, n.possessive),
            ("they", "them", "their")
        );
        assert_eq!(n.reflexive, "themselves");
        assert_eq!(n.was(), "were");
        assert_eq!(n.has(), "have");
        assert_eq!(n.subject_cap(), "They");
        assert_eq!(f.possessive_cap(), "Her");
    }

    #[test]
    fn picks_are_stable_without_drawing() {
        let a = Pick::stable(120, 7).index(5);
        let b = Pick::stable(120, 7).index(5);
        assert_eq!(a, b);
        assert!(a < 5);
        // Different years and ids do not all collapse to the same variant.
        let spread: std::collections::BTreeSet<usize> = (0..40)
            .map(|i| Pick::stable(100 + i, i as usize).index(5))
            .collect();
        assert!(spread.len() > 1);
    }

    #[test]
    fn rolled_picks_consume_exactly_one_draw() {
        let rng = crate::rng::Rng::new(9);
        let before = rng.state();
        let _ = Pick::rolled(&rng).index(4);
        let after_one = rng.state();
        rng.set_state(before);
        let _ = rng.below(4);
        assert_eq!(rng.state(), after_one);
    }

    /// The whole point of the layer: a century of history that reads as
    /// English. Checks are generic so new prose is covered automatically.
    #[test]
    fn a_century_of_chronicle_is_well_formed() {
        // Long enough to reach the events that only a developed world
        // produces — rivalries, patronage, trade closing, a strain
        // surfacing, an innovation named. A hundred years reached none of
        // them, so a lost line continuation in one of those sentences sat
        // in the tree until somebody read the chronicle by eye.
        // Several worlds, not one. Every phrasing is behind a random or a
        // hashed choice, so one seed reads perhaps a third of them, and the
        // variants added to break up the chronicle's repetition are exactly
        // the sentences a single-seed test is least likely to reach. A
        // lower-case realm name at the head of one of them survived this
        // test twice before a reader found it.
        let mut events: Vec<crate::sim::chronicle::Event> = Vec::new();
        for seed in [11u64, 7, 23, 41] {
            let mut w = World::new(seed, 96, 48, Detail::High);
            for _ in 0..400 {
                w.tick();
            }
            assert!(w.chronicle.len() > 50, "the world should have a history");
            events.extend(w.chronicle.events.iter().cloned());
        }
        for e in &events {
            let t = &e.text;
            let ctx = format!("year {}: {:?}", e.year, t);
            assert!(!t.is_empty(), "empty event text at {}", ctx);
            assert!(!t.contains("  "), "double space in {}", ctx);
            assert!(!t.contains(" a a "), "stuttered article in {}", ctx);
            assert!(!t.contains("A arcane"), "wrong article in {}", ctx);
            assert!(!t.contains("a arcane"), "wrong article in {}", ctx);
            assert!(!t.contains("a empire"), "wrong article in {}", ctx);
            assert!(!t.contains(" of ."), "dangling 'of' in {}", ctx);
            assert!(!t.contains(" of ,"), "dangling 'of' in {}", ctx);
            assert!(!t.contains(" of the ."), "dangling 'of the' in {}", ctx);
            assert!(!t.contains(" ."), "space before a full stop in {}", ctx);
            assert!(!t.contains(" ,"), "space before a comma in {}", ctx);
            assert!(!t.contains(".."), "doubled full stop in {}", ctx);
            assert!(!t.contains(",,"), "doubled comma in {}", ctx);
            // A capitalised article in the middle of a sentence.
            //
            // Every realm name comes in two forms — `realm_full` gives "the
            // Kingdom of Sha" and `realm_full_cap` gives "The Kingdom of
            // Sha" — and a phrasing that uses the opening form where the
            // name is not opening the sentence reads "the call went to The
            // Kingdom of Sha". Two of the variants added to break up the
            // chronicle's repetition did exactly that, which is what a
            // family of near-identical helpers will always eventually cost.
            // Nothing in this world is named "The ...", so a capitalised
            // article after a lower-case letter or a comma is always wrong.
            let chars: Vec<char> = t.chars().collect();
            for (k, w) in chars.windows(5).enumerate() {
                let opens = k == 0 || {
                    let before = chars[k - 1];
                    before.is_lowercase() || before == ','
                };
                let article = matches!(w, ['T', 'h', 'e', ' ', _])
                    || matches!(w, ['A', ' ', _, _, _])
                    || matches!(w, ['A', 'n', ' ', _, _]);
                if k > 0 && opens && article && chars[k - 1] != '.' {
                    let around: String = chars[k.saturating_sub(24)..(k + 24).min(chars.len())]
                        .iter()
                        .collect();
                    panic!(
                        "capitalised article mid-sentence in {}: ...{}...",
                        ctx, around
                    );
                }
            }
            // Every sentence in an entry, not only the first. An event is
            // built by appending clauses --- a battle, then a sack, then a
            // wonder cast down --- and a clause written to sit mid-sentence
            // that ends up opening one reads as a bug however correct the
            // sentence before it was.
            let mut after_stop = false;
            for (k, c) in chars.iter().enumerate() {
                if after_stop && !c.is_whitespace() {
                    assert!(
                        c.is_uppercase() || c.is_ascii_digit() || *c == '"' || *c == '\'',
                        "lower-case sentence start in {}: ...{}...",
                        ctx,
                        chars[k.saturating_sub(20)..(k + 20).min(chars.len())]
                            .iter()
                            .collect::<String>()
                    );
                    after_stop = false;
                }
                // A full stop ends a sentence unless it is an initial or a
                // decimal point, neither of which is followed by a space.
                if *c == '.' || *c == '!' || *c == '?' {
                    after_stop = chars.get(k + 1).is_some_and(|n| *n == ' ');
                }
            }
            let first = t.chars().next().unwrap();
            // A digit is a fine way to begin a sentence — "26% of the
            // world's settled land now lay under the Lafulannic Kingdom" —
            // and the rule rejected it. Nothing noticed until a change
            // elsewhere moved that line into the century this test reads,
            // which is the trouble with a rule that is nearly right.
            assert!(
                first.is_uppercase() || first == '"' || first.is_ascii_digit(),
                "lower-case sentence start in {}",
                ctx
            );
            let last = t.chars().last().unwrap();
            assert!(
                matches!(last, '.' | '"' | '!' | '?'),
                "unterminated sentence in {}",
                ctx
            );
        }
    }

    /// A joined list of names takes a plural verb.
    ///
    /// Two helpers here return something that may be singular or plural ---
    /// `join_names`, and a realm whose name is a people ("the Tsishan
    /// Clans") --- and a clause written for the singular case reads as a
    /// mistake the moment the other turns up: "Tseizri and Roksnio was in
    /// the line beside them", "The Tsishan Clans was inherited by a child".
    /// Checked at the helpers rather than by pattern-matching the chronicle,
    /// because "the Chasimban attack on the Ngeese Clans was thrown back" is
    /// perfectly correct and no substring rule can tell the two apart.
    #[test]
    fn a_list_of_allies_takes_a_plural_verb() {
        let mut w = World::new(5, 80, 40, Detail::Medium);
        for _ in 0..300 {
            w.tick();
        }
        let realms: Vec<usize> = w.alive_polities.iter().copied().take(6).collect();
        assert!(realms.len() >= 4, "too few realms to make a coalition");
        // One ally on the winning side, then two.
        let one = diplomacy::battle_allies(&w, &realms[..2], &realms[2..3]);
        let two = diplomacy::battle_allies(&w, &realms[..3], &realms[3..4]);
        for (text, allies) in [(&one, 1), (&two, 2)] {
            if text.is_empty() {
                continue;
            }
            let plural = text.contains("were") || text.contains("have ");
            let singular = text.contains(" was ") || text.contains(" has ");
            if allies == 2 && singular && !plural {
                panic!("two allies took a singular verb: {:?}", text);
            }
        }
        // And a realm whose name is a people is plural wherever it is the
        // subject.
        for p in w.alive_polities.iter().copied() {
            let was = realm_was(&w, p);
            let it = realm_it(&w, p);
            assert_eq!(
                was == "were",
                it == "they",
                "{} disagrees with itself about number",
                w.polities[p].name
            );
        }
    }

    /// No one sentence may be the world's whole voice.
    ///
    /// The chronicle is the product. When a phrase fires on every battle of
    /// every coalition war it is not colour, it is a stuck record: the
    /// clause "was no longer a quarrel between two realms" once appeared
    /// seven hundred and fifty-six times in eight centuries, six percent of
    /// every line the reader would ever see, and four of the next five
    /// worst offenders were the same sentence about a horse.
    ///
    /// This measures the thing directly rather than the code that causes
    /// it: every seven-word run of common words in a long history, and the
    /// share of all events the commonest of them accounts for. It is a
    /// ratchet — if a new event type is added and given one phrasing, this
    /// is what says so.
    #[test]
    fn no_phrase_becomes_the_whole_chronicle() {
        use std::collections::HashMap;
        let mut w = World::new(7, 200, 100, Detail::High);
        for _ in 0..800 {
            w.tick();
        }
        let lines: Vec<&str> = w.chronicle.events.iter().map(|e| e.text.as_str()).collect();
        assert!(lines.len() > 2000, "too short a history to measure");
        let mut counts: HashMap<String, usize> = HashMap::new();
        for line in &lines {
            // Proper nouns are the part that *does* vary, so a run
            // containing one says nothing about repetition. Only runs made
            // entirely of ordinary words are counted.
            let words: Vec<&str> = line
                .split(|c: char| !c.is_alphabetic() && c != '\'')
                .filter(|s| !s.is_empty())
                .collect();
            let mut seen_here: Vec<String> = Vec::new();
            for run in words.windows(7) {
                if run
                    .iter()
                    .any(|word| word.chars().next().is_some_and(char::is_uppercase))
                {
                    continue;
                }
                let key = run.join(" ");
                // Once per line: a sentence that says the same thing twice
                // is one repetition, not two.
                if seen_here.contains(&key) {
                    continue;
                }
                seen_here.push(key.clone());
                *counts.entry(key).or_insert(0) += 1;
            }
        }
        let (worst, &n) = counts
            .iter()
            .max_by_key(|&(_, n)| n)
            .expect("a history has words in it");
        let share = n as f64 / lines.len() as f64;
        assert!(
            share < 0.03,
            "{:.1}% of all {} events contain the same seven words: {:?}",
            share * 100.0,
            lines.len(),
            worst
        );
    }
}
