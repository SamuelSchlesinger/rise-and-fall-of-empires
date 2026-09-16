//! Disasters, notable people, wonders and the naming of ages.

use super::chronicle::{EventKind, Ref};
use super::politics;
use super::prose::{self, Pick};
use super::{Era, Plague, PolityKind, Role, SchoolKind, World};

pub fn disasters(w: &mut World) {
    let rng = w.rng.clone();
    let tn = w.tuning;
    // Ongoing plagues.
    let mut i = 0;
    while i < w.plagues.len() {
        let polities = w.plagues[i].polities.clone();
        let mut deaths = 0.0f64;
        // The sick realms' land, in map order, so the death toll adds up the
        // same way a walk of the whole map would.
        let mut sick: Vec<usize> = Vec::new();
        for &p in &polities {
            sick.extend_from_slice(w.cells_of_ref(p));
        }
        sick.sort_unstable();
        for &c in &sick {
            let d = w.cells[c].pop * tn.plague_cell_deaths;
            w.cells[c].pop -= d;
            deaths += d as f64;
            w.cells[c].plague = 2;
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
            let text = prose::plague_spread(&pname, &names);
            let refs: Vec<Ref> = spread.iter().map(|&q| Ref::Polity(q)).collect();
            w.log(1, EventKind::Disaster, &refs, None, text);
            w.plagues[i].polities.extend(spread);
        }
        w.plagues[i].deaths += deaths;
        w.plagues[i].years_left -= 1;
        if w.plagues[i].years_left <= 0 {
            let dead = w.plagues[i].deaths;
            let text = prose::plague_ended(&pname, dead as i64);
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
        let name = prose::plague_name(&Pick::rolled(&rng));
        w.plagues.push(Plague {
            name: name.clone(),
            years_left: 3 + rng.below(4) as i32,
            polities: vec![p],
            deaths: 0.0,
        });
        let text = prose::plague_began(w, &name, c, p, &Pick::rolled(&rng));
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
            for k in 0..w.cells_of_ref(p).len() {
                let c = w.cells_of_ref(p)[k];
                w.cells[c].pop *= tn.famine_cell_survival;
            }
            for c in pol.cities.clone() {
                w.cities[c].pop *= tn.famine_city_survival;
            }
            w.polities[p].stability = (w.polities[p].stability - 0.08).max(0.0);
            let cause = prose::famine_cause(&Pick::rolled(&rng));
            let text = prose::famine(w, p, &cause);
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
        let text = prose::city_disaster(w, c, kind, lost.as_deref());
        let mut refs = vec![Ref::City(c)];
        if let Some(p) = w.cities[c].polity {
            refs.push(Ref::Polity(p));
        }
        w.log(1, EventKind::Disaster, &refs, Some(cell), text);
    }
    // Omens.
    if rng.chance(tn.omen_chance) {
        let text = prose::omen(&Pick::rolled(&rng));
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
                let text = prose::general_appointed(w, p, g, &Pick::rolled(&rng));
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
                        let what = e
                            .refs
                            .iter()
                            .find_map(|r| match r {
                                Ref::War(wid) => Some(w.wars[*wid].name.clone()),
                                Ref::Person(pid) => Some(w.persons[*pid].full_name()),
                                _ => None,
                            })
                            .unwrap_or_else(|| prose::nothing_in_particular().to_string());
                        let form = *rng.pick(&["Lay", "Song", "Lament", "Epic", "Ballad"]);
                        w.persons[poet].renown += 2.0;
                        prose::poet_sings(w, p, poet, form, &what)
                    }
                    None => prose::poet_idle(w, p, poet),
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
                        let what = match kind {
                            crate::geo::FeatureKind::Island => "island",
                            crate::geo::FeatureKind::Continent => "continent",
                            _ => "waters",
                        };
                        prose::explorer_found(w, p, ex, what, &n)
                    }
                    None => prose::explorer_empty_handed(w, p, ex),
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
                    let text = prose::school_champion(w, p, s, per);
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
                    let text = prose::school_from_vision(w, s, founder, cap);
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
                    let cause = prose::grievances_of(&w.persons[leader].name);
                    w.wars_start(rebel, p, kind, cause);
                    let text =
                        prose::rebellion_of_the_wronged(w, p, rebel, leader, &Pick::rolled(&rng));
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
                let text = prose::reformer(w, p, ph, &Pick::rolled(&rng));
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
                let text = prose::notable_died(&name, role.name(), age as i32);
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
                        w.persons[r].death = prose::deposed_by(&w.persons[i].name.clone());
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
                    let capital = prose::capital_name(w, p);
                    let text = prose::general_usurps(w, p, i, &capital);
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
        let city = w.cities[cap].name.clone();
        let name = prose::wonder_name(&city, &Pick::rolled(&rng));
        w.polities[p].treasury -= cost;
        w.polities[p].prestige += 20.0;
        w.polities[p].dev = (w.polities[p].dev + 0.05).min(3.0);
        w.cities[cap].wonders.push(name.clone());
        w.cities[cap].prosperity += 0.1;
        let ruler = w.ruler_title(p);
        let text = prose::wonder_built(w, p, &name, &ruler, &Pick::rolled(&rng));
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
    // One realm holding better than a third of the world's land names the age.
    let hegemon = biggest.filter(|&p| share > 0.35 && w.polities[p].kind == PolityKind::Empire);
    let (name, desc) = if let Some(p) = hegemon {
        (
            prose::era_name_empire(w, p, &Pick::rolled(&rng)),
            prose::era_of_empire(w, p),
        )
    } else if pop_change < -0.15 {
        (
            prose::era_name_dying(&Pick::rolled(&rng)),
            prose::era_of_dying(pop_change < -0.3),
        )
    } else if warlike {
        (
            prose::era_name_war(&Pick::rolled(&rng)),
            prose::era_of_war(wars),
        )
    } else if schools >= 4 {
        (
            prose::era_name_schools(&Pick::rolled(&rng)),
            prose::era_of_schools(schools),
        )
    } else if born > realms {
        (
            prose::era_name_crowns(&Pick::rolled(&rng)),
            prose::era_of_crowns(born),
        )
    } else if peaceful && pop_change > 0.05 {
        (
            prose::era_name_peace(&Pick::rolled(&rng)),
            prose::era_of_peace().to_string(),
        )
    } else {
        (
            prose::era_name_ordinary(&Pick::rolled(&rng)),
            prose::era_ordinary().to_string(),
        )
    };
    let text = prose::era_named(&name, &desc);
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
