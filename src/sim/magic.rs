//! Schools of thought: arcane orders, faiths and philosophies. They are
//! founded where the land's mana runs high or a people is given to
//! mysticism, spread along trade and conquest, are adopted or persecuted
//! by states, split in schisms, and sometimes unmake the cities that
//! nurtured them.

use super::chronicle::{EventKind, Ref};
use super::{Role, School, SchoolKind, World};
use crate::term::Rgb;

const ASPECTS: &[(&str, &str)] = &[
    ("Flame", "fire"),
    ("Tide", "the waters"),
    ("Stone", "earth and stone"),
    ("Storm", "wind and lightning"),
    ("Shadow", "darkness"),
    ("Light", "light"),
    ("Blood", "blood"),
    ("Bone", "the dead"),
    ("Dream", "dreams"),
    ("Star", "the stars"),
    ("Void", "the emptiness between things"),
    ("Verdant", "growing things"),
    ("Ice", "cold and stillness"),
    ("Iron", "metal and making"),
    ("Mirror", "reflections"),
    ("Song", "sound and silence"),
    ("Ash", "endings"),
    ("Thread", "fate"),
];
const PRACTICES: &[&str] = &["Binding", "Weaving", "Naming", "Shaping", "Warding", "Summoning", "Reading", "Unmaking"];
const ORDERS: &[&str] = &["Order", "Circle", "Covenant", "College", "Conclave", "Lodge", "Tower", "Brotherhood", "Sisterhood"];

const DEITIES: &[&str] = &[
    "the Twin Moons",
    "the Drowned God",
    "the Ever-Burning",
    "the Nameless",
    "the Wheel",
    "the Mother of Rivers",
    "the Horned Lord",
    "the Silent Judge",
    "the Nine",
    "the Sky-Father",
    "the Serpent Below",
    "the First Flame",
    "the Weeping Saint",
    "the Uncounted Dead",
    "the Thousand-Eyed",
    "the Lantern",
    "the Grey Shepherd",
];
const DIVINE_PRACTICE: &[&str] = &["ascetic", "ecstatic", "sacrificial", "monastic", "crusading", "mystery", "prophetic", "penitent"];

const TENET_WORDS: &[&str] = &["Harmony", "the Will", "Duty", "Doubt", "the Cycle", "Silence", "the Golden Mean", "the Question", "Refusal", "the Ledger", "Kindness", "the Void", "the Garden", "the Road"];
const ACADEMIES: &[&str] = &["School", "Academy", "Way", "Path", "Garden", "Society"];

const ARCANE_TENETS: &[&str] = &[
    "Power has a price, and the price is memory.",
    "Names bind; the unnamed is free.",
    "The stars are wounds in the sky, and light bleeds through.",
    "What is shaped can be unshaped.",
    "The dead keep no secrets from those who ask properly.",
    "Every spell is a debt.",
    "The world is a woven thing and can be unpicked.",
    "Fire remembers everything it has eaten.",
];
const DIVINE_TENETS: &[&str] = &[
    "The dead watch.",
    "Fire cleanses.",
    "Kings serve, or they burn.",
    "Give a tenth of all things to the temple.",
    "Bury nothing; the earth is holy.",
    "Speak no falsehood after sunset.",
    "The sea takes what it is owed.",
    "Suffering is the road to the light.",
    "All rivers are one river.",
];
const PHILO_TENETS: &[&str] = &[
    "Nothing is owed.",
    "Doubt everything twice.",
    "The city is a body; the ruler is only its mouth.",
    "Want less and you will have it.",
    "Every law is a fence around a fear.",
    "The wise ruler does nothing, and nothing is left undone.",
    "Count what you have; then count who counts you.",
    "Kindness is the only argument.",
];

fn school_color(id: usize, kind: SchoolKind) -> Rgb {
    let hue = (id as f32 * 71.0 + match kind {
        SchoolKind::Arcane => 260.0,
        SchoolKind::Divine => 40.0,
        SchoolKind::Philosophical => 160.0,
    }) % 360.0;
    Rgb::from_hsv(hue, 0.6, 0.95)
}

pub fn found_school(w: &mut World, city: usize, kind: SchoolKind, founder: Option<usize>, parent: Option<usize>) -> usize {
    let rng = w.rng.clone();
    let id = w.schools.len();
    let culture = w.cities[city].culture;
    let polity = w.cities[city].polity;
    let founder = founder.unwrap_or_else(|| {
        let role = match kind {
            SchoolKind::Arcane => Role::Mage,
            SchoolKind::Divine => Role::Prophet,
            SchoolKind::Philosophical => Role::Philosopher,
        };
        w.new_person(culture, role, polity, w.year - rng.int(28, 55), None)
    });
    w.persons[founder].school = Some(id);
    w.persons[founder].city = Some(city);
    w.persons[founder].renown += 3.0;
    let fname = w.persons[founder].name.clone();
    let lang = w.cultures[culture].lang.clone();
    let (aspect, practice) = match parent {
        Some(p) => {
            let ps = &w.schools[p];
            if rng.chance(0.5) {
                (ps.aspect, rng.below(8))
            } else {
                (rng.below(ASPECTS.len()), ps.practice)
            }
        }
        None => (rng.below(ASPECTS.len()), rng.below(8)),
    };
    let (name, short, doctrine, tenets, hostility) = match kind {
        SchoolKind::Arcane => {
            let (asp, desc) = ASPECTS[aspect];
            let prac = PRACTICES[practice % PRACTICES.len()];
            let order = *rng.pick(ORDERS);
            let (name, short) = match rng.below(4) {
                0 => (format!("the {} {}", asp, order), format!("{} {}", asp, order)),
                1 => (format!("the {}s of {}", prac, asp), format!("{}s of {}", prac, asp)),
                2 => (format!("the {} School", lang.adjective(&fname)), format!("{} School", lang.adjective(&fname))),
                _ => (format!("the Order of the {} {}", asp, prac), format!("{} {}", asp, prac)),
            };
            let doctrine = format!("teaches that all magic is the {} of {}", prac.to_lowercase(), desc);
            let mut t: Vec<String> = Vec::new();
            for _ in 0..2 {
                let x = rng.pick(ARCANE_TENETS).to_string();
                if !t.contains(&x) {
                    t.push(x);
                }
            }
            (name, short, doctrine, t, rng.range32(0.2, 0.6))
        }
        SchoolKind::Divine => {
            let deity = if rng.chance(0.5) { rng.pick(DEITIES).to_string() } else { format!("{} of the {}", lang.name(&rng), ASPECTS[aspect].0) };
            let prac = DIVINE_PRACTICE[practice % DIVINE_PRACTICE.len()];
            let (name, short) = match rng.below(4) {
                0 => (format!("the Faith of {}", deity), deity.clone()),
                1 => (format!("the Church of {}", deity), deity.clone()),
                2 => (format!("the Way of {}", deity), deity.clone()),
                _ => (format!("{}ism", fname.trim_end_matches(|c: char| "aeiou".contains(c))), format!("{}ism", fname.trim_end_matches(|c: char| "aeiou".contains(c)))),
            };
            let short = short.trim_start_matches("the ").to_string();
            let doctrine = format!("is a {} faith devoted to {}", prac, deity);
            let mut t: Vec<String> = Vec::new();
            for _ in 0..3 {
                let x = rng.pick(DIVINE_TENETS).to_string();
                if !t.contains(&x) {
                    t.push(x);
                }
            }
            (name, short, doctrine, t, rng.range32(0.4, 0.95))
        }
        SchoolKind::Philosophical => {
            let tenet = TENET_WORDS[aspect % TENET_WORDS.len()];
            let acad = *rng.pick(ACADEMIES);
            let (name, short) = match rng.below(3) {
                0 => (format!("the {} of {}", acad, tenet), format!("{} of {}", acad, tenet)),
                1 => (format!("{}ism", fname.trim_end_matches(|c: char| "aeiou".contains(c))), format!("{}ism", fname.trim_end_matches(|c: char| "aeiou".contains(c)))),
                _ => (format!("the {} {}", lang.adjective(&fname), acad), format!("{} {}", lang.adjective(&fname), acad)),
            };
            let doctrine = format!("holds that the good life is found in {}", tenet.to_lowercase());
            let mut t: Vec<String> = Vec::new();
            for _ in 0..2 {
                let x = rng.pick(PHILO_TENETS).to_string();
                if !t.contains(&x) {
                    t.push(x);
                }
            }
            (name, short, doctrine, t, rng.range32(0.05, 0.4))
        }
    };
    let mut influence = std::collections::BTreeMap::new();
    if let Some(p) = polity {
        influence.insert(p, 0.15);
    }
    w.schools.push(School {
        id,
        name,
        short,
        kind,
        doctrine,
        tenets,
        aspect,
        practice,
        founder,
        founded: w.year,
        home_city: city,
        parent,
        influence,
        extinct: None,
        color: school_color(id, kind),
        hostility,
        peak_polities: 0,
        fading_since: None,
        state_of: Vec::new(),
    });
    w.century_schools += 1;
    id
}

impl World {
    /// (army multiplier, stability bonus, development bonus, prosperity bonus) from the state school.
    pub fn school_effects(&self, p: usize) -> (f32, f32, f32, f32) {
        match self.polities[p].school {
            Some(s) if self.schools[s].alive() => {
                let infl = self.schools[s].influence.get(&p).copied().unwrap_or(0.0);
                match self.schools[s].kind {
                    SchoolKind::Arcane => (1.0 + 0.3 * infl, 0.0, 0.5 * infl, 0.05 * infl),
                    SchoolKind::Divine => (1.0 + 0.1 * infl, 0.12 * infl, 0.0, 0.0),
                    SchoolKind::Philosophical => (1.0, 0.05 * infl, 0.4 * infl, 0.15 * infl),
                }
            }
            _ => (1.0, 0.0, 0.0, 0.0),
        }
    }
}

pub fn tick(w: &mut World) {
    let rng = w.rng.clone();
    // Founding.
    let ncity = w.cities.len();
    for c in 0..ncity {
        if w.cities[c].destroyed.is_some() {
            continue;
        }
        let cell = w.cities[c].cell;
        let mana = w.terrain.mana[cell];
        let culture = w.cities[c].culture;
        let vals = w.cultures[culture].values;
        let polity = w.cities[c].polity;
        let existing = polity.map(|p| w.schools.iter().filter(|s| s.alive() && s.influence.get(&p).copied().unwrap_or(0.0) > 0.2).count()).unwrap_or(0);
        let base = 0.0015 * (mana as f64 - 0.25).max(0.0) * (0.5 + vals.mysticism as f64 * 2.0) + 0.0004 * (0.5 + vals.openness as f64);
        let p = base * (w.cities[c].pop as f64 / 3.0).min(2.0) / (1.0 + existing as f64);
        if !rng.chance(p) {
            continue;
        }
        let weights = [mana as f64 * 2.0 + vals.mysticism as f64, vals.tradition as f64 + 0.6, vals.openness as f64 + vals.mercantilism as f64 * 0.5];
        let kind = match rng.weighted(&weights) {
            0 => SchoolKind::Arcane,
            1 => SchoolKind::Divine,
            _ => SchoolKind::Philosophical,
        };
        let s = found_school(w, c, kind, None, None);
        let founder = w.schools[s].founder;
        let place = w.place_phrase(cell, polity);
        let text = match kind {
            SchoolKind::Arcane => format!(
                "In {} {}, the mage {} gathered students and founded {}, which {}.",
                w.cities[c].name, place, w.persons[founder].name, w.schools[s].name, w.schools[s].doctrine
            ),
            SchoolKind::Divine => format!(
                "{} of {} began to preach in the streets of {}. {} {}, and its first tenet is: \"{}\"",
                w.persons[founder].name, w.cultures[culture].name, w.cities[c].name, crate::lang::capitalize(&w.schools[s].name), w.schools[s].doctrine, w.schools[s].tenets[0]
            ),
            SchoolKind::Philosophical => format!(
                "{} of {} taught in the markets of {} that \"{}\" From these lessons grew {}, which {}.",
                w.persons[founder].name, w.cities[c].name, w.cities[c].name, w.schools[s].tenets[0], w.schools[s].name, w.schools[s].doctrine
            ),
        };
        let mut refs = vec![Ref::School(s), Ref::Person(founder), Ref::City(c)];
        if let Some(p) = polity {
            refs.push(Ref::Polity(p));
        }
        w.log(2, EventKind::Magic, &refs, Some(cell), text);
        super::stories::artifacts_on_school(w, s);
    }

    // Spread, adoption, persecution, schism, extinction.
    let nschool = w.schools.len();
    for s in 0..nschool {
        if !w.schools[s].alive() {
            continue;
        }
        let kind = w.schools[s].kind;
        let home_polity = w.cities[w.schools[s].home_city].polity;
        let mut influence: Vec<(usize, f32)> = w.schools[s].influence.iter().map(|(&p, &v)| (p, v)).collect();
        let mut gains: Vec<(usize, f32)> = Vec::new();
        for &(p, v) in &influence {
            if !w.polities[p].alive() {
                continue;
            }
            let pol = &w.polities[p];
            let vals = w.cultures[pol.culture].values;
            let is_state = pol.school == Some(s);
            let rival_state = pol.school.is_some() && !is_state;
            let mut d = 0.007 * (0.5 + vals.mysticism) * (0.5 + vals.openness);
            if is_state {
                d += 0.012;
            }
            if rival_state {
                d -= 0.01 * w.schools[pol.school.unwrap()].hostility;
            }
            if home_polity == Some(p) {
                d += 0.01;
            }
            d -= 0.009;
            gains.push((p, d));
            // Spread to neighbours.
            if v > 0.2 {
                for &(q, _) in &pol.neighbors {
                    if !w.polities[q].alive() {
                        continue;
                    }
                    let qv = w.cultures[w.polities[q].culture].values;
                    let at_war = w.war_between(p, q).is_some();
                    let mut sd = 0.002 * v * (0.5 + qv.openness) * (0.5 + qv.mysticism);
                    if at_war {
                        sd *= 0.3;
                    }
                    if w.polities[q].culture == pol.culture {
                        sd *= 1.5;
                    }
                    gains.push((q, sd));
                }
            }
        }
        for (p, d) in gains {
            let e = w.schools[s].influence.entry(p).or_insert(0.0);
            *e = (*e + d).clamp(0.0, 1.0);
        }
        w.schools[s].influence.retain(|p, v| *v > 0.003 && w.polities[*p].alive());
        influence = w.schools[s].influence.iter().map(|(&p, &v)| (p, v)).collect();
        let adherents = influence.iter().filter(|(_, v)| *v > 0.1).count();
        if adherents > w.schools[s].peak_polities {
            w.schools[s].peak_polities = adherents;
        }
        // Adoption and persecution.
        for &(p, v) in &influence {
            if !w.polities[p].alive() {
                continue;
            }
            let pol = &w.polities[p];
            let ruler_t = super::politics::traits_of(w, p);
            let affinity = match kind {
                SchoolKind::Arcane => w.cultures[pol.culture].values.mysticism,
                SchoolKind::Divine => ruler_t.piety,
                SchoolKind::Philosophical => ruler_t.wisdom,
            };
            match pol.school {
                None => {
                    if v > 0.4 && rng.chance(0.04 * (0.3 + affinity as f64)) {
                        w.polities[p].school = Some(s);
                        w.schools[s].state_of.push(p);
                        let text = match kind {
                            SchoolKind::Arcane => format!("{} took the adepts of {} into the royal service; henceforth {} would be the magic of {}.", w.ruler_title(p), w.schools[s].short, w.schools[s].name, w.polities[p].short),
                            SchoolKind::Divine => format!("{} was baptised into {}, and {} became the faith of {}.", w.ruler_title(p), w.schools[s].name, w.schools[s].short, w.polities[p].name),
                            SchoolKind::Philosophical => format!("The court of {} adopted the teachings of {}; its laws were rewritten by the {}.", w.polities[p].short, w.schools[s].name, w.schools[s].kind.follower()),
                        };
                        w.log(2, EventKind::Magic, &[Ref::School(s), Ref::Polity(p)], w.capital_cell(p), text);
                    }
                }
                Some(state) if state != s => {
                    let sv = w.schools[state].influence.get(&p).copied().unwrap_or(0.0);
                    if v > sv + 0.25 && rng.chance(0.05) {
                        // Conversion.
                        w.polities[p].school = Some(s);
                        w.schools[s].state_of.push(p);
                        let text = format!("{} forsook {} for {}.", w.ruler_title(p), w.schools[state].name, w.schools[s].name);
                        w.log(2, EventKind::Magic, &[Ref::School(s), Ref::School(state), Ref::Polity(p)], w.capital_cell(p), text);
                    } else if v > 0.25 && (w.schools[state].hostility + ruler_t.cruelty + ruler_t.piety * 0.5) > 1.2 && rng.chance(0.06) {
                        // Persecution.
                        let e = w.schools[s].influence.get_mut(&p).unwrap();
                        *e *= 0.5;
                        w.polities[p].stability = (w.polities[p].stability - 0.05).max(0.0);
                        let cap = w.polities[p].capital.map(|c| w.cities[c].name.clone()).unwrap_or_default();
                        let mut text = format!("{} outlawed {} throughout {}; its {} were driven from {}.", w.ruler_title(p), w.schools[s].name, w.polities[p].short, w.schools[s].kind.follower(), cap);
                        let mut refs = vec![Ref::School(s), Ref::Polity(p), Ref::School(state)];
                        if w.high_detail() && rng.chance(0.5) {
                            let culture = w.polities[p].culture;
                            let m = w.new_person(culture, Role::Martyr, Some(p), w.year - rng.int(20, 60), None);
                            w.persons[m].school = Some(s);
                            w.persons[m].died = Some(w.year);
                            w.persons[m].death = format!("was burned in the square of {} for the teachings of {}.", cap, w.schools[s].short);
                            w.persons[m].renown = 2.0;
                            text.push_str(&format!(" {} was burned in the square; the {} of {} call {} a martyr.", w.persons[m].name, w.schools[s].kind.follower(), w.schools[s].short, w.persons[m].gender.them()));
                            refs.push(Ref::Person(m));
                        }
                        // Persecution raises tension with states of this school.
                        for q in w.schools[s].state_of.clone() {
                            if w.polities[q].alive() && q != p {
                                let e = w.polities[q].tension.entry(p).or_insert(0.1);
                                *e = (*e + 0.1).min(1.0);
                            }
                        }
                        w.log(1, EventKind::Magic, &refs, w.capital_cell(p), text);
                    }
                }
                _ => {}
            }
        }
        // Schism.
        let age = w.year - w.schools[s].founded;
        if age > 60 && adherents >= 2 && rng.chance(0.006) {
            let home = w.schools[s].home_city;
            // Pick a city in an adherent polity other than the home.
            let candidates: Vec<usize> = influence
                .iter()
                .filter(|(_, v)| *v > 0.2)
                .flat_map(|(p, _)| w.polities[*p].cities.iter().copied())
                .filter(|&c| c != home && w.cities[c].destroyed.is_none())
                .collect();
            if let Some(&city) = candidates.get(rng.below(candidates.len().max(1)).min(candidates.len().saturating_sub(1))) {
                let child = found_school(w, city, kind, None, Some(s));
                // Split influence.
                let mut moved = Vec::new();
                for &(p, v) in &influence {
                    if rng.chance(0.45) {
                        w.schools[child].influence.insert(p, v * 0.6);
                        if let Some(e) = w.schools[s].influence.get_mut(&p) {
                            *e *= 0.5;
                        }
                        moved.push(p);
                    }
                }
                let founder = w.schools[child].founder;
                let dispute = match kind {
                    SchoolKind::Arcane => format!("over whether {} may be practised on the living", PRACTICES[w.schools[child].practice % PRACTICES.len()].to_lowercase()),
                    SchoolKind::Divine => match rng.below(3) {
                        0 => "over the true name of the god".to_string(),
                        1 => format!("over the tenet \"{}\"", w.schools[s].tenets[0]),
                        _ => "over who might sit on the high seat".to_string(),
                    },
                    SchoolKind::Philosophical => "over the meaning of a single sentence of the founder".to_string(),
                };
                let text = format!(
                    "{} split {}. {} of {} led the dissenters out, and their teaching became known as {}.",
                    crate::lang::capitalize(&w.schools[s].name),
                    dispute,
                    w.persons[founder].name,
                    w.cities[city].name,
                    w.schools[child].name
                );
                let mut refs = vec![Ref::School(child), Ref::School(s), Ref::Person(founder), Ref::City(city)];
                for p in moved {
                    refs.push(Ref::Polity(p));
                }
                w.log(2, EventKind::Magic, &refs, Some(w.cities[city].cell), text);
            }
        }
        // Extinction.
        let maxv = influence.iter().map(|(_, v)| *v).fold(0.0, f32::max);
        if maxv < 0.04 && age > 10 {
            match w.schools[s].fading_since {
                None => w.schools[s].fading_since = Some(w.year),
                Some(y) if w.year - y > 15 => {
                    w.schools[s].extinct = Some(w.year);
                    for p in 0..w.polities.len() {
                        if w.polities[p].school == Some(s) {
                            w.polities[p].school = None;
                        }
                    }
                    let text = format!("The last {} of {} died, and its teachings were forgotten.", w.schools[s].kind.follower(), w.schools[s].name);
                    w.log(1, EventKind::Magic, &[Ref::School(s)], None, text);
                }
                _ => {}
            }
        } else {
            w.schools[s].fading_since = None;
        }
        // Arcane catastrophe.
        if kind == SchoolKind::Arcane {
            for p in w.schools[s].state_of.clone() {
                if !w.polities[p].alive() || w.polities[p].school != Some(s) {
                    continue;
                }
                let v = w.schools[s].influence.get(&p).copied().unwrap_or(0.0);
                if let Some(cap) = w.polities[p].capital {
                    let cell = w.cities[cap].cell;
                    let mana = w.terrain.mana[cell];
                    let pdis = 0.0012 * (v as f64) * (mana as f64 + w.polities[p].dev as f64 * 0.3);
                    if rng.chance(pdis) {
                        catastrophe(w, s, p, cap);
                    }
                }
            }
        }
    }
    // Drop state schools that went extinct.
    for p in 0..w.polities.len() {
        if let Some(s) = w.polities[p].school {
            if !w.schools[s].alive() {
                w.polities[p].school = None;
            }
        }
    }
}

fn catastrophe(w: &mut World, s: usize, p: usize, cap: usize) {
    let rng = w.rng.clone();
    let cell = w.cities[cap].cell;
    let name = w.cities[cap].name.clone();
    let (x, y) = w.terrain.xy(cell);
    let mut cells = Vec::new();
    for dy in -2i32..=2 {
        for dx in -4i32..=4 {
            let cx = x as i32 + dx;
            let cy = y as i32 + dy;
            if cx < 0 || cy < 0 || cx >= w.terrain.w as i32 || cy >= w.terrain.h as i32 {
                continue;
            }
            if (dx * dx) as f32 / 16.0 + (dy * dy) as f32 / 4.0 <= 1.0 {
                cells.push(w.terrain.idx(cx as usize, cy as usize));
            }
        }
    }
    w.terrain.blight(&cells);
    let mut dead = 0.0;
    for &c in &cells {
        dead += w.cells[c].pop;
        w.cells[c].pop *= 0.1;
        if let Some(ci) = w.cells[c].city {
            if w.cities[ci].destroyed.is_none() {
                dead += w.cities[ci].pop;
                w.cities[ci].destroyed = Some(w.year);
                w.cities[ci].pop = 0.0;
                w.cities[ci].polity = None;
                w.cells[c].city = None;
            }
        }
    }
    // A new feature: the blighted place.
    let fid = w.terrain.features.len();
    let lang = w.polity_lang(p).clone();
    let fname = crate::geo::name_feature(crate::geo::FeatureKind::Nexus, &lang, &rng);
    let fname = match rng.below(3) {
        0 => format!("the Scar of {}", name),
        1 => format!("the {} Waste", name),
        _ => fname,
    };
    w.terrain.features.push(crate::geo::Feature { kind: crate::geo::FeatureKind::Desert, name: Some(fname.clone()), named_by: Some(w.polities[p].culture), cells: cells.clone(), center: (x, y) });
    for &c in &cells {
        w.terrain.region[c] = (fid + 1) as u16;
    }
    // The polity reels.
    w.polities[p].stability = (w.polities[p].stability - 0.5).max(0.0);
    w.polities[p].school = None;
    let remaining: Vec<usize> = w.polities[p].cities.iter().copied().filter(|&c| w.cities[c].destroyed.is_none()).collect();
    w.polities[p].capital = remaining.iter().copied().max_by(|&a, &b| w.cities[a].pop.partial_cmp(&w.cities[b].pop).unwrap());
    // The school is discredited everywhere.
    for v in w.schools[s].influence.values_mut() {
        *v *= 0.3;
    }
    let ruler_dead = rng.chance(0.7);
    let mut text = format!(
        "The adepts of {} in {} reached too far. In a single night the city was unmade: a great working went wrong, and where {} had stood there was only glass, ash and silence. The land for miles about was blighted, and is called {}. Some {} thousand souls perished.",
        w.schools[s].short, name, name, fname, (dead as i32).max(1)
    );
    if ruler_dead {
        if let Some(r) = w.polities[p].ruler {
            let rn = w.persons[r].name.clone();
            text.push_str(&format!(" {} died with the city.", rn));
            super::politics::ruler_dies(w, p, format!("perished in the unmaking of {}.", name), 2);
        }
    }
    if w.polities[p].capital.is_none() {
        text.push_str(&format!(" {} did not survive the loss.", crate::lang::capitalize(&w.polities[p].name)));
        super::politics::fall(w, p, format!("perished with {}.", name), None, 3);
    }
    w.log(3, EventKind::Disaster, &[Ref::School(s), Ref::Polity(p), Ref::City(cap), Ref::Feature(fid)], Some(cell), text);
    super::stories::artifacts_on_catastrophe(w, cell, s);
}
