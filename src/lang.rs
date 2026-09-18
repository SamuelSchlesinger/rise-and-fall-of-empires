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

/// Letter pairs these languages write for one sound.
///
/// The cluster rule has to know about them or it cannot tell a name from a
/// pile-up: `ngo` is three letters and one consonant, while `lmn` is three
/// letters and three consonants and nobody can say it. Counting letters
/// meant the rule had to be loose enough to let every digraph through,
/// which let `nnnd`, `xngolm` and `quthnusuwh` through with them.
const DIGRAPHS: &[&str] = &[
    "mb", "mp", "nd", "nt", "ng", "nk", "nn", "zh", "sh", "ch", "th", "kh", "gh", "ph", "wh", "hr",
    "hl", "hn", "dh", "bh", "ts", "dz", "ll", "rr", "ss", "tt", "qu", "tr", "dr", "pr", "br", "gr",
    "kr", "fr", "st", "sp", "sk", "sl", "sn", "sm", "cl", "pl", "bl", "gl", "fl", "kl",
];

/// Whether the last two letters written are one sound.
fn is_digraph(out: &[char]) -> bool {
    if out.len() < 2 {
        return false;
    }
    // Lower-cased, because `smooth` is handed already-capitalised names: an
    // initial "I" that does not match "aeiou" is read as a consonant, and
    // that is how the Iia became the Iiish.
    let pair: String = out[out.len() - 2..]
        .iter()
        .map(char::to_ascii_lowercase)
        .collect();
    DIGRAPHS.contains(&pair.as_str())
}

/// Smooth out unpronounceable clusters: no triple letters, no doubled
/// consonant at the start, and no run of more than two consonant *sounds*.
fn tidy(w: &str, lang: &Language, rng: &Rng) -> String {
    let is_v = |c: char| "aeiouy'".contains(c);
    let mut out: Vec<char> = Vec::with_capacity(w.len() + 2);
    // Sounds since the last vowel, where a digraph is one sound.
    let mut run = 0usize;
    // Whether the consonant just written could still take a second letter.
    // Without this a digraph chains: in "zastthstead" every letter pairs
    // with the one before it — st, tt, th, hs, st — and the run counter is
    // never allowed to advance at all.
    let mut pair_open = false;
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
            pair_open = false;
            out.push(c);
            continue;
        }
        // A letter that completes a digraph joins the sound before it
        // rather than starting one of its own.
        if pair_open {
            let mut probe = out.clone();
            probe.push(c);
            if is_digraph(&probe) {
                out.push(c);
                pair_open = false;
                continue;
            }
        }
        run += 1;
        if run > 2 {
            let v = &lang.vowels[rng.weighted(&lang.vowel_w)];
            out.extend(v.chars());
            run = 1;
        }
        out.push(c);
        pair_open = true;
    }
    out.into_iter().collect()
}

/// Make a glued-together word sayable without needing the language's own
/// vowels or a random number.
///
/// [`tidy`] runs while a word is being coined, syllable by syllable, and
/// can afford to insert a vowel from the language's own stock. A suffix, a
/// demonym ending or the second half of a compound is glued on afterwards
/// — often in `adjective` and `demonym`, which have no access to the RNG
/// and must be a pure function of their input or two callers will disagree
/// about what a people is called. So this one drops rather than inserts:
/// no letter three times running, and no run of more than two consonant
/// sounds, counting a digraph as one.
fn smooth(w: &str) -> String {
    let is_v = |c: char| "aeiouy'-".contains(c.to_ascii_lowercase());
    let same = |a: char, b: char| a.eq_ignore_ascii_case(&b);
    let mut out: Vec<char> = Vec::with_capacity(w.len());
    let mut run = 0usize;
    let mut pair_open = false;
    for c in w.chars() {
        let n = out.len();
        if n >= 2 && same(out[n - 1], c) && same(out[n - 2], c) {
            continue;
        }
        if is_v(c) {
            run = 0;
            pair_open = false;
            out.push(c);
            continue;
        }
        if pair_open {
            let mut probe = out.clone();
            probe.push(c);
            if is_digraph(&probe) {
                out.push(c);
                pair_open = false;
                continue;
            }
        }
        if run >= 2 {
            // The pile-up is broken by leaving this sound out, which keeps
            // the word shorter as well as sayable.
            continue;
        }
        run += 1;
        out.push(c);
        pair_open = true;
    }
    out.into_iter().collect()
}

/// Cut a word back to at most `max` characters, at a vowel boundary where
/// there is one, so that the result still reads as a word of the same
/// language rather than a word with its end bitten off.
fn shorten(w: &str, max: usize) -> String {
    let chars: Vec<char> = w.chars().collect();
    if chars.len() <= max {
        return w.to_string();
    }
    // Back off to the last vowel inside the limit, then keep the one
    // consonant after it if there is one: "Loulnyuxngolmu" -> "Loulnyux".
    let is_v = |c: char| "aeiouy".contains(c);
    let mut cut = max;
    while cut > 3 && !is_v(chars[cut - 1]) {
        cut -= 1;
    }
    while cut > 3 && is_v(chars[cut - 1]) && cut > max.saturating_sub(2) {
        cut -= 1;
    }
    let cut = cut.max(3).min(chars.len());
    chars[..cut].iter().collect()
}

/// The longest a coined name may be before it stops being a name and starts
/// being a keyboard. Twelve is about the length of Constantinople, which is
/// as long as anybody has ever needed.
const NAME_MAX: usize = 12;

const AWKWARD: &[&str] = &[
    "ass", "arse", "sex", "cum", "fag", "tit", "cock", "dick", "shit", "fuck", "poo", "pee", "nig",
    "cunt", "twat", "piss", "wank", "anus", "butt", "boob", "crap", "damn", "hell", "kill", "die",
    "nazi", "rape",
];

/// Short English words a coined name must not collide with.
///
/// The phonology is free to produce "The", and did: a khan of Lorker was
/// named The, and every sentence about him read like a capitalisation bug —
/// "Khan The Bloodhand of Lorker died in the bath". The same goes for a
/// person called And, a town called Of, or a people called the Its.
const RESERVED: &[&str] = &[
    "the", "a", "an", "and", "or", "of", "to", "in", "on", "at", "as", "by", "for", "but", "not",
    "no", "so", "if", "is", "it", "its", "be", "he", "she", "her", "his", "him", "we", "us", "you",
    "was", "are", "had", "has", "who", "why", "how", "all", "any", "one", "two", "up", "out", "do",
    "my", "me", "that", "this", "with", "from", "they", "them", "then", "than", "there", "their",
];

/// Whether a coined word reads badly enough to be worth rolling again.
pub(crate) fn awkward(w: &str) -> bool {
    let l = w.to_lowercase();
    if RESERVED.contains(&l.as_str()) {
        return true;
    }
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
                // A hyphen is a place to draw breath, so a hyphenated name
                // may run longer than a solid one — but not much longer,
                // and the two halves together still have to fit a sidebar.
                return format!(
                    "{}-{}",
                    capitalize(&shorten(&smooth(&base), 8)),
                    capitalize(&shorten(&smooth(&second), 7))
                );
            }
            // The join is where the pile-ups came from: two words that were
            // each pronounceable were glued without the cluster rule ever
            // seeing the seam, and without anything capping the total. That
            // is how a realm came to be called Loulnyuxngolmungon, and its
            // people the Loulnyuxngolmungonians.
            let joined = smooth(&tidy(
                &format!("{}{}", shorten(&base, 8), second),
                self,
                rng,
            ));
            return capitalize(&shorten(&joined, NAME_MAX));
        }
        if rng.chance(self.place_suffix_chance) && !self.place_suffixes.is_empty() {
            let suf = rng.pick(&self.place_suffixes);
            let mut b = shorten(&base, NAME_MAX - suf.chars().count().min(4));
            // A trailing vowel always goes before a vowel-initial suffix, and
            // sometimes before a consonant.
            let vowel_suffix = suf.starts_with(|c: char| "aeiou".contains(c));
            if ends_with_vowel(&b) && (vowel_suffix || rng.chance(0.3)) {
                b.pop();
            }
            return capitalize(&shorten(&smooth(&format!("{}{}", b, suf)), NAME_MAX));
        }
        capitalize(&shorten(&base, NAME_MAX))
    }

    /// A person's given name.
    pub fn person(&self, rng: &Rng) -> String {
        self.name(rng)
    }

    /// Adjective form, e.g. "Velen" -> "Velenish".
    pub fn adjective(&self, base: &str) -> String {
        let suf = self.adj_suffixes[base.len() % self.adj_suffixes.len()].as_str();
        let mut b = shorten(base, NAME_MAX.saturating_sub(suf.chars().count().min(4)));
        if ends_with_vowel(&b) && suf.starts_with(|c: char| "aeiou".contains(c)) {
            b.pop();
        }
        smooth(&format!("{}{}", b, suf))
    }

    /// Plural demonym, e.g. "Velen" -> "Velenites".
    pub fn demonym(&self, base: &str) -> String {
        let suf = self.demonym_suffixes[(base.len() + 1) % self.demonym_suffixes.len()].as_str();
        let mut b = shorten(base, NAME_MAX.saturating_sub(suf.chars().count().min(4)));
        if suf == "s" {
            // Through the smoother like every other ending. This one path
            // returned early and unsmoothed, so a people of Chinluthhr
            // became the Chinluthhrs.
            if b.ends_with('s') || b.ends_with("sh") || b.ends_with('x') {
                return smooth(&format!("{}es", b));
            }
            return smooth(&format!("{}s", b));
        }
        if ends_with_vowel(&b) && suf.starts_with(|c: char| "aeiou".contains(c)) {
            b.pop();
        }
        smooth(&format!("{}{}", b, suf))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every name in the world has to be one a reader can hold in their
    /// head, because a name they cannot say is a realm they cannot care
    /// about.
    ///
    /// The generator used to coin realms called Loulnyuxngolmungon and
    /// peoples called the Loulnyuxngolmungonians: `place` checked the length
    /// of the first element of a compound and then glued a second one on
    /// without a length check or a cluster check, and `demonym` then added
    /// a suffix to the result. Three separate rules, each of which believed
    /// somebody else was measuring.
    #[test]
    fn coined_names_are_pronounceable_and_short() {
        let rng = Rng::new(9);
        let mut worst = String::new();
        let mut checked = 0usize;
        for _ in 0..60 {
            let lang = Language::generate(&rng);
            for _ in 0..80 {
                for name in [lang.place(&rng), lang.person(&rng), lang.name(&rng)] {
                    let base = name.clone();
                    for n in [base.clone(), lang.adjective(&base), lang.demonym(&base)] {
                        checked += 1;
                        let letters: Vec<char> = n.to_lowercase().chars().collect();
                        assert!(
                            letters.len() <= 16,
                            "{:?} is {} characters long",
                            n,
                            letters.len()
                        );
                        if letters.len() > worst.chars().count() {
                            worst = n.clone();
                        }
                        // No letter written three times running.
                        for w in letters.windows(3) {
                            assert!(
                                !(w[0] == w[1] && w[1] == w[2]),
                                "{:?} has a trebled letter (from base {:?})",
                                n,
                                base
                            );
                        }
                        // No run of five consonant letters: two sounds is
                        // the limit and the longest digraph is two letters,
                        // so four letters is the most a legal run can be.
                        let mut run = 0usize;
                        for c in &letters {
                            if "aeiouy'-".contains(*c) {
                                run = 0;
                            } else {
                                run += 1;
                                assert!(run < 5, "{:?} has a consonant pile-up", n);
                            }
                        }
                    }
                }
            }
        }
        assert!(checked > 10_000, "only {} names checked", checked);
    }
}
