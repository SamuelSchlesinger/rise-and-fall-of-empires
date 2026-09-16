//! Procedural languages: each culture gets a phonology and a handful of
//! orthographic habits, from which every name in the world is coined.
//! Daughter languages are produced by applying sound shifts to a parent.

use crate::rng::Rng;

/// One culture's way of making words: what may start a syllable, what may
/// end one, and the habits that turn syllables into names.
#[derive(Clone, Debug)]
pub struct Language {
    /// The language's own name for itself.
    pub name: String,
    pub(crate) onsets: Vec<String>,
    pub(crate) onset_w: Vec<f64>,
    pub(crate) vowels: Vec<String>,
    pub(crate) vowel_w: Vec<f64>,
    pub(crate) codas: Vec<String>,
    pub(crate) coda_w: Vec<f64>,
    pub(crate) coda_chance: f64,
    pub(crate) onset_chance: f64,
    pub(crate) min_syl: usize,
    pub(crate) max_syl: usize,
    pub(crate) place_suffixes: Vec<String>,
    pub(crate) place_suffix_chance: f64,
    pub(crate) compound_chance: f64,
    pub(crate) hyphen: bool,
    pub(crate) apostrophe_chance: f64,
    pub(crate) adj_suffixes: Vec<String>,
    pub(crate) demonym_suffixes: Vec<String>,
    /// What this culture calls a ruler: King, Khan, Hierarch ...
    pub honorific: String,
}

impl Language {
    /// An empty language, filled in by the loader.
    pub fn blank() -> Language {
        Language {
            name: String::new(),
            onsets: vec!["l".into()],
            onset_w: vec![1.0],
            vowels: vec!["a".into()],
            vowel_w: vec![1.0],
            codas: Vec::new(),
            coda_w: Vec::new(),
            coda_chance: 0.3,
            onset_chance: 0.9,
            min_syl: 1,
            max_syl: 2,
            place_suffixes: Vec::new(),
            place_suffix_chance: 0.3,
            compound_chance: 0.1,
            hyphen: false,
            apostrophe_chance: 0.0,
            adj_suffixes: vec!["ish".into()],
            demonym_suffixes: vec!["s".into()],
            honorific: "King".into(),
        }
    }
}

const FAMILIES: &[&[&str]] = &[
    &["l", "m", "n", "r", "s", "v", "w", "y", "h"], // soft
    &["k", "t", "p", "g", "d", "b"],                // stops
    &["kh", "gh", "q", "ch", "x", "hr"],            // guttural
    &["sh", "zh", "z", "ts", "j"],                  // sibilant
    &["th", "dh", "f", "ph", "wh"],                 // aspirate
    &["ng", "ny", "ll", "rr", "mb", "nd"],          // nasal/doubled
];

const CLUSTERS: &[&str] = &[
    "br", "dr", "kr", "tr", "gr", "pr", "st", "sk", "sp", "sl", "sn", "fl", "gl", "kl", "pl",
    "thr", "str", "vr", "zr", "shr", "kw", "tw", "dw",
];

const VOWEL_SETS: &[&[&str]] = &[
    &["a", "e", "i", "o", "u"],
    &["a", "i", "u", "e"],
    &["a", "e", "i", "o", "u", "y", "ae"],
    &["a", "o", "u", "au", "ou"],
    &["e", "i", "a", "ei", "ie"],
    &["a", "i", "o", "ai", "ia", "io"],
];

const CODA_POOL: &[&str] = &[
    "n", "r", "l", "s", "k", "t", "m", "sh", "th", "n", "r", "l", "d", "g", "x", "nd", "rn", "st",
    "ss", "nn", "lm", "rk", "th",
];

const PLACE_SUFFIX_POOL: &[&str] = &[
    "burg", "heim", "gard", "hold", "haven", "mouth", "ford", "wick", "stead", "vale", "dun",
    "abad", "grad", "polis", "keep", "mere", "moor", "rath", "kar", "than", "nor", "sar", "mar",
    "ost", "hal", "ia", "or", "um", "eth", "ath", "is", "on", "ur",
];

const ADJ_SETS: &[&[&str]] = &[
    &["ish", "ian", "ic"],
    &["ian", "ese", "an"],
    &["i", "ari", "ite"],
    &["ic", "ene", "ian"],
    &["an", "ish", "ite"],
    &["ari", "oi", "ean"],
];

const DEMONYM_SETS: &[&[&str]] = &[
    &["s", "ites", "ians"],
    &["i", "im", "ar"],
    &["folk", "men", "s"],
    &["ans", "ari", "oi"],
    &["ites", "ini", "s"],
];

const HONORIFICS: &[&str] = &[
    "King",
    "Queen",
    "Prince",
    "Lord",
    "Khan",
    "Rex",
    "Emperor",
    "Empress",
    "Chief",
    "Sovereign",
    "Arch-Lord",
    "Sultan",
    "Tsar",
    "Basileus",
    "Elder",
    "Warden",
    "Great Chief",
    "Hierarch",
    "Dux",
    "Sar",
    "Tagh",
    "Voivode",
];

fn ends_with_vowel(s: &str) -> bool {
    matches!(s.chars().last(), Some('a' | 'e' | 'i' | 'o' | 'u' | 'y'))
}

/// Smooth out unpronounceable clusters: no triple letters, no doubled
/// consonant at the start, and no run of more than three consonants.
fn tidy(w: &str, lang: &Language, rng: &Rng) -> String {
    let is_v = |c: char| "aeiouy'".contains(c);
    let mut out: Vec<char> = Vec::with_capacity(w.len() + 2);
    let mut run = 0usize;
    for c in w.chars() {
        let n = out.len();
        if n >= 2 && out[n - 1] == c && out[n - 2] == c {
            continue;
        }
        if n == 1 && out[0] == c && !is_v(c) {
            continue;
        }
        if is_v(c) {
            run = 0;
        } else {
            run += 1;
            if run > 3 {
                let v = &lang.vowels[rng.weighted(&lang.vowel_w)];
                out.extend(v.chars());
                run = 1;
            }
        }
        out.push(c);
    }
    out.into_iter().collect()
}

const AWKWARD: &[&str] = &[
    "ass", "arse", "sex", "cum", "fag", "tit", "cock", "dick", "shit", "fuck", "poo", "pee", "nig",
    "cunt", "twat", "piss", "wank", "anus", "butt", "boob", "crap", "damn", "hell", "kill", "die",
    "nazi", "rape",
];

/// Whether a coined word reads badly enough to be worth rolling again.
pub(crate) fn awkward(w: &str) -> bool {
    let l = w.to_lowercase();
    AWKWARD
        .iter()
        .any(|a| l == *a || (a.len() >= 4 && l.contains(a)))
}

/// Upper-case the first character, leaving the rest alone.
pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

impl Language {
    /// Roll a whole language: consonant families, vowels, codas and habits.
    pub fn generate(rng: &Rng) -> Language {
        // Choose 2-3 consonant families, weight them.
        let mut fam: Vec<usize> = (0..FAMILIES.len()).collect();
        rng.shuffle(&mut fam);
        let nfam = 2 + rng.below(2);
        let mut onsets: Vec<String> = Vec::new();
        let mut onset_w: Vec<f64> = Vec::new();
        for (k, &fi) in fam.iter().take(nfam).enumerate() {
            let weight = if k == 0 {
                3.0
            } else if k == 1 {
                1.6
            } else {
                0.7
            };
            for &c in FAMILIES[fi] {
                if rng.chance(0.75) {
                    onsets.push(c.to_string());
                    onset_w.push(weight * rng.range(0.5, 1.5));
                }
            }
        }
        // Always a few soft consonants to keep things pronounceable.
        for &c in ["l", "n", "r", "s", "m", "k", "t"].iter() {
            if !onsets.iter().any(|o| o == c) && rng.chance(0.5) {
                onsets.push(c.to_string());
                onset_w.push(1.0);
            }
        }
        if rng.chance(0.55) {
            let ncl = 2 + rng.below(4);
            for _ in 0..ncl {
                let c = *rng.pick(CLUSTERS);
                if !onsets.iter().any(|o| o == c) {
                    onsets.push(c.to_string());
                    onset_w.push(rng.range(0.3, 0.8));
                }
            }
        }
        if onsets.is_empty() {
            onsets.push("l".into());
            onset_w.push(1.0);
        }

        let vs = *rng.pick(VOWEL_SETS);
        let vowels: Vec<String> = vs.iter().map(std::string::ToString::to_string).collect();
        let vowel_w: Vec<f64> = vowels
            .iter()
            .map(|v| {
                if v.len() == 1 {
                    rng.range(0.8, 2.5)
                } else {
                    rng.range(0.2, 0.6)
                }
            })
            .collect();

        let ncoda = 2 + rng.below(6);
        let mut codas: Vec<String> = Vec::new();
        let mut coda_w = Vec::new();
        for _ in 0..ncoda {
            let c = *rng.pick(CODA_POOL);
            if !codas.iter().any(|o| o == c) {
                codas.push(c.to_string());
                coda_w.push(rng.range(0.4, 1.6));
            }
        }
        let nsuf = 2 + rng.below(4);
        let mut place_suffixes = Vec::new();
        for _ in 0..nsuf {
            let s = *rng.pick(PLACE_SUFFIX_POOL);
            if !place_suffixes.iter().any(|o| o == s) {
                place_suffixes.push(s.to_string());
            }
        }
        // Coin one or two native place suffixes as well.
        let min_syl = 1 + rng.below(2);
        let max_syl = (min_syl + 1 + rng.below(2)).min(3);
        let mut lang = Language {
            name: String::new(),
            onsets,
            onset_w,
            vowels,
            vowel_w,
            codas,
            coda_w,
            coda_chance: rng.range(0.1, 0.65),
            onset_chance: rng.range(0.75, 0.97),
            min_syl,
            max_syl,
            place_suffixes,
            place_suffix_chance: rng.range(0.15, 0.55),
            compound_chance: rng.range(0.05, 0.3),
            hyphen: rng.chance(0.35),
            apostrophe_chance: if rng.chance(0.2) {
                rng.range(0.05, 0.2)
            } else {
                0.0
            },
            adj_suffixes: rng
                .pick(ADJ_SETS)
                .iter()
                .map(std::string::ToString::to_string)
                .collect(),
            demonym_suffixes: rng
                .pick(DEMONYM_SETS)
                .iter()
                .map(std::string::ToString::to_string)
                .collect(),
            honorific: rng.pick(HONORIFICS).to_string(),
        };
        for _ in 0..(1 + rng.below(2)) {
            let w = lang.word(rng, 1);
            lang.place_suffixes.push(w);
        }
        lang.name = lang.name(rng);
        lang
    }

    /// Derive a daughter language by shifting sounds and habits.
    pub fn mutate(&self, rng: &Rng) -> Language {
        let mut l = self.clone();
        let shifts: &[(&str, &str)] = &[
            ("p", "f"),
            ("k", "ch"),
            ("t", "th"),
            ("b", "v"),
            ("d", "dh"),
            ("g", "gh"),
            ("s", "sh"),
            ("kh", "h"),
            ("f", "p"),
            ("ch", "k"),
            ("th", "t"),
            ("v", "w"),
            ("sh", "s"),
            ("w", "v"),
            ("r", "l"),
            ("l", "r"),
            ("z", "s"),
            ("q", "k"),
            ("y", "j"),
            ("j", "y"),
        ];
        // Two or three shifts, not one: a daughter language that differs in
        // a single consonant coins names its parent could have coined, and
        // a coastline of "Zh-" peoples is the result.
        let nshift = 2 + rng.below(2);
        for _ in 0..nshift {
            let (from, to) = *rng.pick(shifts);
            for o in l.onsets.iter_mut() {
                if o == from {
                    *o = to.to_string();
                }
            }
            for c in l.codas.iter_mut() {
                if c == from {
                    *c = to.to_string();
                }
            }
        }
        // Drop or add a consonant.
        if l.onsets.len() > 4 && rng.chance(0.5) {
            let i = rng.below(l.onsets.len());
            l.onsets.remove(i);
            l.onset_w.remove(i);
        }
        if rng.chance(0.5) {
            let fam = *rng.pick(FAMILIES);
            let c = *rng.pick(fam);
            if !l.onsets.iter().any(|o| o == c) {
                l.onsets.push(c.to_string());
                l.onset_w.push(rng.range(0.5, 1.5));
            }
        }
        // Vowel shift. Two thirds of the time the whole vowel set goes,
        // which is what makes a daughter tongue sound like another people
        // rather than the same people with a lisp. The weights are rolled
        // again with it, since they have to match the set's length.
        // Indexed rather than `pick(pick(..))`: old compilers (MSRV 1.70)
        // cannot infer through the nested deref coercion.
        if rng.chance(0.65) {
            let set: &[&str] = VOWEL_SETS[rng.below(VOWEL_SETS.len())];
            l.vowels = set.iter().map(std::string::ToString::to_string).collect();
            l.vowel_w = l
                .vowels
                .iter()
                .map(|v| {
                    if v.len() == 1 {
                        rng.range(0.8, 2.5)
                    } else {
                        rng.range(0.2, 0.6)
                    }
                })
                .collect();
        } else {
            let i = rng.below(l.vowels.len());
            let set: &[&str] = VOWEL_SETS[rng.below(VOWEL_SETS.len())];
            let nv = *rng.pick(set);
            l.vowels[i] = nv.to_string();
        }
        l.coda_chance = (l.coda_chance + rng.range(-0.2, 0.2)).clamp(0.05, 0.7);
        if rng.chance(0.4) {
            l.adj_suffixes = rng
                .pick(ADJ_SETS)
                .iter()
                .map(std::string::ToString::to_string)
                .collect();
        }
        if rng.chance(0.4) {
            l.demonym_suffixes = rng
                .pick(DEMONYM_SETS)
                .iter()
                .map(std::string::ToString::to_string)
                .collect();
        }
        if rng.chance(0.5) {
            l.honorific = rng.pick(HONORIFICS).to_string();
        }
        if rng.chance(0.5) {
            let w = l.word(rng, 1);
            l.place_suffixes.push(w);
            if l.place_suffixes.len() > 6 {
                l.place_suffixes.remove(0);
            }
        }
        l.hyphen = if rng.chance(0.3) { !l.hyphen } else { l.hyphen };
        l.name = l.name(rng);
        l
    }

    fn syllable(&self, rng: &Rng, first: bool, last: bool) -> String {
        let mut s = String::new();
        let onset_p = if first { self.onset_chance } else { 0.95 };
        if rng.chance(onset_p) {
            s.push_str(&self.onsets[rng.weighted(&self.onset_w)]);
        }
        s.push_str(&self.vowels[rng.weighted(&self.vowel_w)]);
        let coda_p = if last {
            self.coda_chance * 1.4
        } else {
            self.coda_chance * 0.6
        };
        if !self.codas.is_empty() && rng.chance(coda_p) {
            s.push_str(&self.codas[rng.weighted(&self.coda_w)]);
        }
        s
    }

    /// A lowercase word of `syl` syllables.
    pub fn word(&self, rng: &Rng, syl: usize) -> String {
        let mut w = String::new();
        let n = syl.max(1);
        for i in 0..n {
            let s = self.syllable(rng, i == 0, i + 1 == n);
            // Avoid ugly triple consonant pile-ups at syllable joins.
            if let (Some(a), Some(b)) = (w.chars().last(), s.chars().next()) {
                if !"aeiouy".contains(a) && !"aeiouy".contains(b) && rng.chance(0.5) {
                    w.push_str(&self.vowels[rng.weighted(&self.vowel_w)]);
                }
            }
            w.push_str(&s);
            if self.apostrophe_chance > 0.0 && i + 1 < n && rng.chance(self.apostrophe_chance) {
                w.push('\'');
            }
        }
        tidy(&w, self, rng)
    }

    fn syl_count(&self, rng: &Rng) -> usize {
        self.min_syl + rng.below(self.max_syl - self.min_syl + 1)
    }

    /// A capitalised name-like word, kept to a pronounceable length.
    pub fn name(&self, rng: &Rng) -> String {
        for _ in 0..8 {
            let w = self.word(rng, self.syl_count(rng));
            let n = w.chars().count();
            if (3..=10).contains(&n) && !awkward(&w) {
                return capitalize(&w);
            }
        }
        capitalize(&self.word(rng, 2))
    }

    /// A place name, which may be a compound or take a suffix.
    pub fn place(&self, rng: &Rng) -> String {
        let mut base = self.word(rng, self.syl_count(rng));
        for _ in 0..6 {
            let n = base.chars().count();
            if (3..=9).contains(&n) && !awkward(&base) {
                break;
            }
            base = self.word(rng, self.syl_count(rng));
        }
        if rng.chance(self.compound_chance) {
            let second = self.word(rng, 1 + rng.below(2));
            if self.hyphen {
                return format!("{}-{}", capitalize(&base), capitalize(&second));
            }
            return capitalize(&format!("{}{}", base, second));
        }
        if rng.chance(self.place_suffix_chance) && !self.place_suffixes.is_empty() {
            let suf = rng.pick(&self.place_suffixes);
            let mut b = base.clone();
            // A trailing vowel always goes before a vowel-initial suffix, and
            // sometimes before a consonant.
            let vowel_suffix = suf.starts_with(|c: char| "aeiou".contains(c));
            if ends_with_vowel(&b) && (vowel_suffix || rng.chance(0.3)) {
                b.pop();
            }
            return capitalize(&format!("{}{}", b, suf));
        }
        capitalize(&base)
    }

    /// A person's given name.
    pub fn person(&self, rng: &Rng) -> String {
        self.name(rng)
    }

    /// Adjective form, e.g. "Velen" -> "Velenish".
    pub fn adjective(&self, base: &str) -> String {
        let suf = self.adj_suffixes[base.len() % self.adj_suffixes.len()].as_str();
        let mut b = base.to_string();
        if ends_with_vowel(&b) && suf.starts_with(|c: char| "aeiou".contains(c)) {
            b.pop();
        }
        format!("{}{}", b, suf)
    }

    /// Plural demonym, e.g. "Velen" -> "Velenites".
    pub fn demonym(&self, base: &str) -> String {
        let suf = self.demonym_suffixes[(base.len() + 1) % self.demonym_suffixes.len()].as_str();
        let mut b = base.to_string();
        if suf == "s" {
            if b.ends_with('s') || b.ends_with("sh") || b.ends_with('x') {
                return format!("{}es", b);
            }
            return format!("{}s", b);
        }
        if ends_with_vowel(&b) && suf.starts_with(|c: char| "aeiou".contains(c)) {
            b.pop();
        }
        format!("{}{}", b, suf)
    }
}
