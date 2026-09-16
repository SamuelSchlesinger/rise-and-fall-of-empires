//! Disasters, notable people, wonders and the naming of ages.

use super::chronicle::{EventKind, Ref};
use super::politics;
use super::{Era, Plague, PolityKind, Role, SchoolKind, World};

const PLAGUE_ADJ: &[&str] = &[
    "Grey",
    "Red",
    "Weeping",
    "Sweating",
    "Black",
    "Silent",
    "Blistering",
    "Yellow",
    "Coughing",
    "Shivering",
];
const PLAGUE_NOUN: &[&str] = &[
    "Death", "Plague", "Sickness", "Fever", "Rot", "Pox", "Wasting",
];

pub fn disasters(w: &mut World) {
    let rng = w.rng.clone();
    let tn = w.tuning;
    // Ongoing plagues.
    let mut i = 0;
    while i < w.plagues.len() {
        let polities = w.plagues[i].polities.clone();
        let mut deaths = 0.0f64;
        for c in 0..w.cells.len() {
            if let Some(p) = w.cells[c].owner {
                if polities.contains(&p) {
                    let d = w.cells[c].pop * tn.plague_cell_deaths;
                    w.cells[c].pop -= d;
                    deaths += d as f64;
                    w.cells[c].plague = 2;
                }
            }
        }
        for c in 0..w.cities.len() {
            if w.cities[c].destroyed.is_none()
                && w.cities[c]
                    .polity
                    .map(|p| polities.contains(&p))
                    .unwrap_or(false)
            {
                let d = w.cities[c].pop * tn.plague_city_deaths;
                w.cities[c].pop -= d;
                deaths += d as f64;
            }
        }
        // Spread.
        let mut spread = Vec::new();
        for &p in &polities {
            if !w.polities[p].alive() {
                continue;
            }
            w.polities[p].stability = (w.polities[p].stability - 0.02).max(0.0);
            for &(q, _) in &w.polities[p].neighbors {
                if !polities.contains(&q)
                    && !spread.contains(&q)
                    && w.polities[q].alive()
                    && rng.chance(tn.plague_spread_chance)
                {
                    spread.push(q);
                }
            }
        }
        let pname = w.plagues[i].name.clone();
        if !spread.is_empty() {
            let names: Vec<String> = spread.iter().map(|&q| w.polities[q].name.clone()).collect();
            let text = format!("{} spread into {}.", pname, politics::join_names(&names));
            let refs: Vec<Ref> = spread.iter().map(|&q| Ref::Polity(q)).collect();
            w.log(1, EventKind::Disaster, &refs, None, text);
            w.plagues[i].polities.extend(spread);
        }
        w.plagues[i].deaths += deaths;
        w.plagues[i].years_left -= 1;
        if w.plagues[i].years_left <= 0 {
            let dead = w.plagues[i].deaths;
            let text = format!(
                "{} burned itself out at last, having carried off some {} thousand souls.",
                pname, dead as i64
            );
            w.log(1, EventKind::Disaster, &[], None, text);
            w.plagues.remove(i);
        } else {
            i += 1;
        }
    }
    for c in w.cells.iter_mut() {
        if c.plague > 0 {
            c.plague -= 1;
        }
    }
    // New plague.
    let big_cities: Vec<usize> = w
        .cities
        .iter()
        .filter(|c| c.destroyed.is_none() && c.pop > 6.0 && c.polity.is_some())
        .map(|c| c.id)
        .collect();
    if !big_cities.is_empty()
        && w.plagues.len() < 2
        && rng.chance(tn.plague_chance * (big_cities.len() as f64).sqrt())
    {
        let c = big_cities[rng.below(big_cities.len())];
        let p = w.cities[c].polity.unwrap();
        let name = format!("the {} {}", rng.pick(PLAGUE_ADJ), rng.pick(PLAGUE_NOUN));
        w.plagues.push(Plague {
            name: name.clone(),
            years_left: 3 + rng.below(4) as i32,
            polities: vec![p],
            deaths: 0.0,
        });
        let text = match rng.below(3) {
            0 => format!("{} came to {} with the ships and the caravans. Within a season the streets of {} were empty.", crate::lang::capitalize(&name), w.cities[c].name, w.cities[c].name),
            1 => format!("A sickness the physicians called {} broke out in {}. The dead were buried in pits outside the walls.", name, w.cities[c].name),
            _ => format!("{} began in the poor quarters of {} and spread through {}.", crate::lang::capitalize(&name), w.cities[c].name, w.polities[p].name),
        };
        w.log(
            2,
            EventKind::Disaster,
            &[Ref::City(c), Ref::Polity(p)],
            Some(w.cities[c].cell),
            text,
        );
    }
    // Famine.
    for p in w.living_polities() {
        let pol = &w.polities[p];
        if pol.cells < 5 {
            continue;
        }
        if rng.chance(tn.famine_chance * (1.2 - pol.avg_fertility as f64).max(0.1)) {
            let cells = w.cells_of(p);
            for &c in &cells {
                w.cells[c].pop *= tn.famine_cell_survival;
            }
            for c in pol.cities.clone() {
                w.cities[c].pop *= tn.famine_city_survival;
            }
            w.polities[p].stability = (w.polities[p].stability - 0.08).max(0.0);
            let cause = match rng.below(4) {
                0 => "The rains failed",
                1 => "Locusts came out of the east",
                2 => "A killing frost struck at midsummer",
                _ => "The rivers ran low",
            };
            let text = format!("{} and famine gripped {}. The granaries of {} were emptied and the people ate bark.", cause, w.polities[p].name, w.polities[p].capital.map(|c| w.cities[c].name.clone()).unwrap_or_default());
            w.log(
                1,
                EventKind::Disaster,
                &[Ref::Polity(p)],
                w.capital_cell(p),
                text,
            );
        }
    }
    // City disasters.
    let ncity = w.cities.len();
    for c in 0..ncity {
        if w.cities[c].destroyed.is_some() || w.cities[c].pop < 2.0 {
            continue;
        }
        if !rng.chance(tn.city_disaster_chance) {
            continue;
        }
        let cell = w.cities[c].cell;
        let river = w.terrain.river[cell] >= 2;
        let mountain = w.terrain.neighbors8(cell).any(|n| {
            matches!(
                w.terrain.biome[n],
                crate::geo::Biome::Mountain | crate::geo::Biome::Peak
            )
        });
        let kind = if river && rng.chance(0.4) {
            "flood"
        } else if mountain && rng.chance(0.4) {
            "earthquake"
        } else {
            "fire"
        };
        w.cities[c].pop *= tn.city_disaster_survival;
        let lost = if !w.cities[c].wonders.is_empty() && rng.chance(0.3) {
            Some(w.cities[c].wonders.remove(0))
        } else {
            None
        };
        let mut text = match kind {
            "flood" => format!(
                "The river burst its banks and drowned the lower town of {}.",
                w.cities[c].name
            ),
            "earthquake" => format!(
                "The earth shook beneath {} and its towers fell.",
                w.cities[c].name
            ),
            _ => format!(
                "A great fire swept through {}; it burned for three days.",
                w.cities[c].name
            ),
        };
        if let Some(wn) = lost {
            text.push_str(&format!(" {} was destroyed.", wn));
        }
        let mut refs = vec![Ref::City(c)];
        if let Some(p) = w.cities[c].polity {
            refs.push(Ref::Polity(p));
        }
        w.log(1, EventKind::Disaster, &refs, Some(cell), text);
    }
    // Omens.
    if rng.chance(tn.omen_chance) {
        let text = match rng.below(4) {
            0 => "A comet with a tail like a sword hung in the sky for forty nights. Priests everywhere read it as they pleased.".to_string(),
            1 => "The sun went dark at noon and the birds fell silent.".to_string(),
            2 => "Two moons were seen in the sky, and the second wept.".to_string(),
            _ => "It rained fish upon the coasts, and the fish were alive.".to_string(),
        };
        w.log(1, EventKind::Disaster, &[], None, text);
    }
}

pub fn notables(w: &mut World) {
    let rate = w.detail_rate();
    if rate <= 0.0 {
        return;
    }
    let rng = w.rng.clone();
    let tn = w.tuning;
    for p in w.living_polities() {
        let pol = &w.polities[p];
        if pol.cells < 6 {
            continue;
        }
        if !rng.chance(tn.notable_chance * rate) {
            continue;
        }
        let at_war = pol.at_war();
        let weights = [
            if at_war { 3.0 } else { 0.3 },
            if pol.treasury > 40.0 { 1.0 } else { 0.3 },
            if pol.seafaring { 0.8 } else { 0.1 },
            w.cultures[pol.culture].values.mysticism as f64 * 0.8,
            if pol.stability < 0.4 { 1.5 } else { 0.1 },
            w.cultures[pol.culture].values.openness as f64 * 0.5,
            0.45 + w.cultures[pol.culture].values.mysticism as f64 * 0.5,
        ];
        let culture = pol.culture;
        match rng.weighted(&weights) {
            0 => {
                let g = w.new_person(
                    culture,
                    Role::General,
                    Some(p),
                    w.year - rng.int(25, 45),
                    None,
                );
                w.persons[g].traits.valor = (w.persons[g].traits.valor + 0.3).min(1.0);
                super::war::role_general(w, p, g);
                let text = match rng.below(3) {
                    0 => format!(
                        "{}, a {} captain of low birth, rose to command the armies of {}.",
                        w.persons[g].name, w.cultures[culture].adj, w.polities[p].short
                    ),
                    1 => format!(
                        "{} was given command of the {} host; the soldiers loved {}.",
                        w.persons[g].name,
                        w.polities[p].adj,
                        w.persons[g].gender.them()
                    ),
                    _ => format!(
                        "The armies of {} marched now under {}, who had never lost a skirmish.",
                        w.polities[p].short, w.persons[g].name
                    ),
                };
                w.log(
                    1,
                    EventKind::Person,
                    &[Ref::Person(g), Ref::Polity(p)],
                    w.capital_cell(p),
                    text,
                );
            }
            1 => {
                let poet =
                    w.new_person(culture, Role::Poet, Some(p), w.year - rng.int(25, 50), None);
                // Sing of something that happened recently to this realm.
                let ids: Vec<usize> = w
                    .chronicle
                    .for_ref(Ref::Polity(p))
                    .iter()
                    .rev()
                    .take(40)
                    .copied()
                    .collect();
                let subject = ids.iter().map(|&i| &w.chronicle.events[i]).find(|e| {
                    e.importance >= 2
                        && w.year - e.year < 60
                        && matches!(
                            e.kind,
                            EventKind::War
                                | EventKind::Battle
                                | EventKind::Death
                                | EventKind::Politics
                                | EventKind::Disaster
                        )
                });
                let text = match subject {
                    Some(e) => {
                        let what = e.refs.iter().find_map(|r| match r {
                            Ref::War(wid) => Some(w.wars[*wid].name.clone()),
                            Ref::Person(pid) => Some(w.persons[*pid].full_name()),
                            _ => None,
                        }).unwrap_or_else(|| "the old days".to_string());
                        let form = *rng.pick(&["Lay", "Song", "Lament", "Epic", "Ballad"]);
                        w.persons[poet].renown += 2.0;
                        format!("{} of {} composed the {} of {}, which is still sung in {}.", w.persons[poet].name, w.polities[p].short, form, what, w.polities[p].capital.map(|c| w.cities[c].name.clone()).unwrap_or_default())
                    }
                    None => format!("{} of {} wrote verses on the rivers and the seasons that were long remembered.", w.persons[poet].name, w.polities[p].short),
                };
                w.log(
                    1,
                    EventKind::Person,
                    &[Ref::Person(poet), Ref::Polity(p)],
                    w.capital_cell(p),
                    text,
                );
            }
            2 => {
                let ex = w.new_person(
                    culture,
                    Role::Explorer,
                    Some(p),
                    w.year - rng.int(20, 40),
                    None,
                );
                // Name an unnamed sea, ocean or island.
                let cap = w.capital_cell(p).unwrap_or(0);
                let mut best = None;
                let mut best_d = usize::MAX;
                for (fi, f) in w.terrain.features.iter().enumerate() {
                    if f.name.is_some()
                        || !matches!(
                            f.kind,
                            crate::geo::FeatureKind::Sea
                                | crate::geo::FeatureKind::Ocean
                                | crate::geo::FeatureKind::Island
                                | crate::geo::FeatureKind::Continent
                        )
                    {
                        continue;
                    }
                    let d = w.terrain.dist(w.terrain.idx(f.center.0, f.center.1), cap);
                    if d < best_d {
                        best_d = d;
                        best = Some(fi);
                    }
                }
                let text = match best {
                    Some(fi) => {
                        let kind = w.terrain.features[fi].kind;
                        let n = w.feature_name(fi, Some(p));
                        w.persons[ex].renown += 2.0;
                        match kind {
                            crate::geo::FeatureKind::Island => format!("{} of {} sailed beyond the known coasts and came upon {}.", w.persons[ex].name, w.polities[p].short, n),
                            crate::geo::FeatureKind::Continent => format!("{} of {} sailed beyond the known coasts and returned with tales of a land called {}.", w.persons[ex].name, w.polities[p].short, n),
                            _ => format!("{} of {} charted the waters of {}.", w.persons[ex].name, w.polities[p].short, n),
                        }
                    }
                    None => format!("{} of {} sailed west for a year and returned with strange fruit and stranger stories.", w.persons[ex].name, w.polities[p].short),
                };
                w.log(
                    1,
                    EventKind::Discovery,
                    &[Ref::Person(ex), Ref::Polity(p)],
                    w.capital_cell(p),
                    text,
                );
            }
            3 => {
                // A prophet or mage strengthens a school, or founds one.
                let schools: Vec<usize> = w
                    .schools
                    .iter()
                    .filter(|s| s.alive() && s.influence.get(&p).copied().unwrap_or(0.0) > 0.1)
                    .map(|s| s.id)
                    .collect();
                if !schools.is_empty() && rng.chance(0.7) {
                    let s = schools[rng.below(schools.len())];
                    let role = match w.schools[s].kind {
                        SchoolKind::Arcane => Role::Mage,
                        SchoolKind::Divine => Role::Prophet,
                        SchoolKind::Philosophical => Role::Philosopher,
                    };
                    let per = w.new_person(culture, role, Some(p), w.year - rng.int(25, 50), None);
                    w.persons[per].school = Some(s);
                    w.persons[per].renown += 1.5;
                    w.persons[per].city = w.polities[p].capital;
                    if role == Role::Prophet && rng.chance(0.5) {
                        super::stories::utter_prophecy(w, per, p);
                        continue;
                    }
                    let e = w.schools[s].influence.entry(p).or_insert(0.0);
                    *e = (*e + 0.15).min(1.0);
                    let text = match role {
                        Role::Mage => format!("{}, an adept of {}, performed wonders before the court of {} and won many to the {}.", w.persons[per].name, w.schools[s].short, w.polities[p].short, w.schools[s].name),
                        Role::Prophet => format!("{} walked the roads of {} preaching {}, and the villages followed {}.", w.persons[per].name, w.polities[p].short, w.schools[s].name, w.persons[per].gender.them()),
                        _ => format!("{} taught {} in the schools of {} to a generation of clerks and princes.", w.persons[per].name, w.schools[s].name, w.polities[p].short),
                    };
                    w.log(
                        1,
                        EventKind::Magic,
                        &[Ref::Person(per), Ref::School(s), Ref::Polity(p)],
                        w.capital_cell(p),
                        text,
                    );
                } else if let Some(cap) = w.polities[p].capital {
                    let kind = if rng.chance(0.5) {
                        SchoolKind::Divine
                    } else {
                        SchoolKind::Arcane
                    };
                    let s = super::magic::found_school(w, cap, kind, None, None);
                    let founder = w.schools[s].founder;
                    let text = format!("{} of {} had a vision in the wilderness and returned to found {}, which {}.", w.persons[founder].name, w.cities[cap].name, w.schools[s].name, w.schools[s].doctrine);
                    w.log(
                        2,
                        EventKind::Magic,
                        &[
                            Ref::School(s),
                            Ref::Person(founder),
                            Ref::Polity(p),
                            Ref::City(cap),
                        ],
                        Some(w.cities[cap].cell),
                        text,
                    );
                }
            }
            4 => {
                // A rebel.
                let leader =
                    w.new_person(culture, Role::Rebel, None, w.year - rng.int(22, 45), None);
                w.persons[leader].traits.ambition =
                    (w.persons[leader].traits.ambition + 0.3).min(1.0);
                if let Some(rebel) = politics::split_off(w, p, Some(leader), None) {
                    let kind = if w.polities[rebel].culture == w.polities[p].culture {
                        super::WarKind::CivilWar
                    } else {
                        super::WarKind::Rebellion
                    };
                    w.wars_start(
                        rebel,
                        p,
                        kind,
                        format!("the grievances of {}", w.persons[leader].name),
                    );
                    let text = format!(
                        "{}, a {} of {} who had been wronged by the crown, gathered the discontented and proclaimed {}.",
                        w.persons[leader].name,
                        rng.pick(&["soldier", "farmer's child", "minor noble", "priest", "bandit", "tax-collector"]),
                        w.polities[p].short,
                        w.polities[rebel].name
                    );
                    w.log(
                        2,
                        EventKind::War,
                        &[Ref::Polity(rebel), Ref::Polity(p), Ref::Person(leader)],
                        w.capital_cell(rebel),
                        text,
                    );
                }
            }
            6 => {
                let seer = w.new_person(
                    culture,
                    Role::Prophet,
                    Some(p),
                    w.year - rng.int(30, 70),
                    None,
                );
                w.persons[seer].city = w.polities[p].capital;
                super::stories::utter_prophecy(w, seer, p);
            }
            _ => {
                let ph = w.new_person(
                    culture,
                    Role::Philosopher,
                    Some(p),
                    w.year - rng.int(30, 55),
                    None,
                );
                w.polities[p].dev = (w.polities[p].dev + 0.04).min(3.0);
                let text = format!(
                    "{} of {} {}.",
                    w.persons[ph].name,
                    w.polities[p].short,
                    rng.pick(&[
                        "reformed the calendar and the weights of the market",
                        "wrote a treatise on the governance of cities that was copied in every court",
                        "taught the use of the arch and the aqueduct",
                        "compiled the laws of the realm into a single book",
                        "mapped the stars and the roads alike"
                    ])
                );
                w.log(
                    1,
                    EventKind::Person,
                    &[Ref::Person(ph), Ref::Polity(p)],
                    w.capital_cell(p),
                    text,
                );
            }
        }
    }
    // Notables age and die; generals may usurp.
    let np = w.persons.len();
    for i in 0..np {
        let per = &w.persons[i];
        if !per.alive() || per.role == Role::Ruler {
            continue;
        }
        let age = (w.year - per.born) as f32;
        let rel = age / w.races[per.race].lifespan;
        let p_die = tn.notable_death_base + tn.notable_death_age_weight * rel.powi(6);
        if rng.chance(p_die as f64) {
            let role = per.role;
            let name = per.full_name();
            w.persons[i].died = Some(w.year);
            w.persons[i].death = "died.".to_string();
            if w.persons[i].renown >= 2.0 && w.detail.level() >= 1 {
                let text = format!(
                    "{} the {} died at the age of {}.",
                    name,
                    role.name(),
                    age as i32
                );
                w.log(0, EventKind::Death, &[Ref::Person(i)], None, text);
            }
        } else if per.role == Role::General && per.traits.ambition > 0.8 {
            if let Some(p) = per.polity {
                if w.polities[p].alive()
                    && w.polities[p].stability < 0.4
                    && per.renown >= 3.0
                    && rng.chance(tn.general_usurp_chance)
                {
                    let old = w.polities[p].ruler;
                    if let Some(r) = old {
                        w.persons[r].died = Some(w.year);
                        w.persons[r].death = format!(
                            "was deposed and killed by the general {}.",
                            w.persons[i].name
                        );
                        w.persons[r].epithet = Some("the Deposed".to_string());
                    }
                    let culture = w.polities[p].culture;
                    let dyn_name = format!("House {}", w.persons[i].name);
                    politics::install_ruler(w, p, i);
                    w.polities[p].generals.retain(|&g| g != i);
                    if w.polities[p].kind.has_dynasty() {
                        w.polities[p].dynasty = dyn_name;
                    }
                    let _ = culture;
                    let text = format!(
                        "The general {} marched on {} and took the throne of {} for {}self.",
                        w.persons[i].name,
                        w.polities[p]
                            .capital
                            .map(|c| w.cities[c].name.clone())
                            .unwrap_or_default(),
                        w.polities[p].short,
                        w.persons[i].gender.them()
                    );
                    let mut refs = vec![Ref::Person(i), Ref::Polity(p)];
                    if let Some(r) = old {
                        refs.push(Ref::Person(r));
                    }
                    w.log(2, EventKind::Politics, &refs, w.capital_cell(p), text);
                }
            }
        }
    }
}

const WONDER_ADJ: &[&str] = &[
    "Great", "Golden", "Black", "Sunken", "Ninefold", "White", "Hanging", "Singing", "Eternal",
    "Iron",
];
const WONDER_KIND: &[&str] = &[
    "Tower",
    "Ziggurat",
    "Library",
    "Colossus",
    "Gardens",
    "Lighthouse",
    "Temple",
    "Walls",
    "Bridge",
    "Aqueduct",
    "Mausoleum",
    "Observatory",
    "Arena",
    "Gate",
];

pub fn wonders(w: &mut World) {
    let rng = w.rng.clone();
    let tn = w.tuning;
    for p in w.living_polities() {
        let pol = &w.polities[p];
        if pol.treasury < tn.wonder_min_treasury || pol.stability < 0.5 || pol.at_war() {
            continue;
        }
        let cap = match pol.capital {
            Some(c) => c,
            None => continue,
        };
        if !rng.chance(tn.wonder_chance) {
            continue;
        }
        let cost = tn.wonder_cost;
        let name = format!(
            "the {} {} of {}",
            rng.pick(WONDER_ADJ),
            rng.pick(WONDER_KIND),
            w.cities[cap].name
        );
        w.polities[p].treasury -= cost;
        w.polities[p].prestige += 20.0;
        w.polities[p].dev = (w.polities[p].dev + 0.05).min(3.0);
        w.cities[cap].wonders.push(name.clone());
        w.cities[cap].prosperity += 0.1;
        let ruler = w.ruler_title(p);
        let text = match rng.below(3) {
            0 => format!("{} raised {}. It took a generation to build and beggared the treasury, but travellers came from every land to see it.", ruler, name),
            1 => format!("{} was completed in the reign of {}.", crate::lang::capitalize(&name), ruler),
            _ => format!("The masons of {} finished {}, greatest of the works of {}.", w.polities[p].short, name, w.polities[p].dynasty.clone().trim_start_matches("the ").to_string()).replace("works of .", "works of the age."),
        };
        let mut refs = vec![Ref::City(cap), Ref::Polity(p)];
        if let Some(r) = w.polities[p].ruler {
            refs.push(Ref::Person(r));
        }
        w.log(2, EventKind::Wonder, &refs, Some(w.cities[cap].cell), text);
        super::stories::artifacts_on_wonder(w, p, cap);
    }
}

pub fn eras(w: &mut World) {
    if w.year % 100 != 0 || w.year == 0 {
        return;
    }
    let rng = w.rng.clone();
    let owned = w.stats.owned_cells.max(1);
    let living = w.living_polities();
    let biggest = living.iter().copied().max_by_key(|&p| w.polities[p].cells);
    let share = biggest
        .map(|p| w.polities[p].cells as f32 / owned as f32)
        .unwrap_or(0.0);
    let pop_change = if w.century_pop_start > 0.0 {
        (w.stats.pop - w.century_pop_start) / w.century_pop_start
    } else {
        0.0
    };
    let wars = w.century_wars;
    let schools = w.century_schools;
    let born = w.century_polities_born;
    let realms = living.len().max(4) as u32;
    let warlike = wars > realms * 2 && wars > 15;
    let peaceful = wars * 2 < realms;
    let (name, desc) = if share > 0.35 && w.polities[biggest.unwrap()].kind == PolityKind::Empire {
        let p = biggest.unwrap();
        (
            rng.pick(&[
                format!("the Age of {}", w.polities[p].short),
                format!("the {} Peace", w.polities[p].adj),
                format!("the {} Ascendancy", w.polities[p].adj),
            ])
            .clone(),
            format!(
                "{} ruled a third of the settled world, and the lesser realms lived in its shadow.",
                w.polities[p].name
            ),
        )
    } else if pop_change < -0.15 {
        (
            rng.pick(&["the Silent Years", "the Long Winter", "the Age of Ash", "the Dark Age"]).to_string(),
            format!("The peoples of the world dwindled by a {} part; cities emptied and roads grew over.", if pop_change < -0.3 { "third" } else { "sixth" }),
        )
    } else if warlike {
        (
            rng.pick(&[
                "the Warring Age",
                "the Age of Blood",
                "the Century of Spears",
                "the Age of Iron",
            ])
            .to_string(),
            format!(
                "{} wars were fought in a hundred years, and no border stayed where it was drawn.",
                wars
            ),
        )
    } else if schools >= 4 {
        (
            rng.pick(&["the Age of Wonders", "the Age of the Star-Readers", "the Age of Prophets", "the Enlightenment"]).to_string(),
            format!("{} new schools of thought arose, and the courts of the world argued over doctrine.", schools),
        )
    } else if born > realms {
        (
            rng.pick(&[
                "the Age of Kings",
                "the Age of Petty Kings",
                "the Age of Banners",
            ])
            .to_string(),
            format!(
                "{} new realms were founded as peoples everywhere took up crowns.",
                born
            ),
        )
    } else if peaceful && pop_change > 0.05 {
        (
            rng.pick(&["the Long Peace", "the Age of Plenty", "the Quiet Age", "the Age of Roads"]).to_string(),
            "Harvests were good, the roads were safe, and the chroniclers complained of having little to write.".to_string(),
        )
    } else {
        (
            rng.pick(&[
                "the Middle Years",
                "the Age of Kings",
                "the Age of Walls",
                "the Uncertain Age",
            ])
            .to_string(),
            "The world turned as it always had.".to_string(),
        )
    };
    let text = format!(
        "The chroniclers call the century now ending {}. {}",
        name, desc
    );
    w.log(3, EventKind::Era, &[], None, text);
    w.eras.push(Era {
        start: w.year - 100,
        name,
        description: desc,
    });
    w.century_wars = 0;
    w.century_schools = 0;
    w.century_polities_born = 0;
    w.century_pop_start = w.stats.pop;
}
