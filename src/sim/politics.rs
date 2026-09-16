//! States: how tribes form, expand across terrain, found cities, keep or
//! lose stability, change rulers, and eventually fragment or fall.

use super::chronicle::{EventKind, Ref};
use super::prose::{self, Pick};
use super::{City, Polity, PolityKind, Role, Traits, World};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Naming
// ---------------------------------------------------------------------------

pub fn make_name(
    w: &World,
    kind: PolityKind,
    short: &str,
    culture: usize,
    capital_name: Option<&str>,
) -> String {
    let lang = &w.cultures[culture].lang;
    let adj = lang.adjective(short);
    let rng = &w.rng;
    let cap = capital_name.unwrap_or(short);
    match kind {
        PolityKind::Tribe => format!("the {}", lang.demonym(short)),
        PolityKind::Chiefdom => rng
            .pick(&[
                format!("the {} Chiefdom", adj),
                format!("the {} Clans", adj),
                format!("the Chiefdom of {}", cap),
            ])
            .clone(),
        PolityKind::Kingdom => rng
            .pick(&[
                format!("the Kingdom of {}", short),
                format!("the {} Kingdom", adj),
                format!("the Realm of {}", short),
                format!("the Crown of {}", short),
                format!("the Kingdom of {}", cap),
            ])
            .clone(),
        PolityKind::Empire => rng
            .pick(&[
                format!("the {} Empire", adj),
                format!("the Empire of {}", short),
                format!("the {} Dominion", adj),
                format!("the {} Imperium", adj),
            ])
            .clone(),
        PolityKind::Republic => rng
            .pick(&[
                format!("the Republic of {}", cap),
                format!("the {} League", adj),
                format!("the Free Cities of {}", short),
                format!("the {} Commonwealth", adj),
            ])
            .clone(),
        PolityKind::Theocracy => rng
            .pick(&[
                format!("the Holy {} Realm", adj),
                format!("the {} Theocracy", adj),
                format!("the See of {}", cap),
                format!("the Blessed Land of {}", short),
            ])
            .clone(),
        PolityKind::Magocracy => rng
            .pick(&[
                format!("the {} Conclave", adj),
                format!("the Magisterium of {}", short),
                format!("the {} Covenant", adj),
                format!("the Towers of {}", cap),
            ])
            .clone(),
        PolityKind::Horde => rng
            .pick(&[
                format!("the {} Horde", adj),
                format!("the Horde of {}", short),
                format!("the {} Riders", adj),
            ])
            .clone(),
    }
}

fn dynasty_name(w: &World, culture: usize, founder_name: &str) -> String {
    let lang = &w.cultures[culture].lang;
    match w.rng.below(3) {
        0 => format!("House {}", founder_name),
        1 => format!("the {} dynasty", lang.adjective(founder_name)),
        _ => format!("the line of {}", founder_name),
    }
}

// ---------------------------------------------------------------------------
// Creation
// ---------------------------------------------------------------------------

pub fn found_city(w: &mut World, p: Option<usize>, cell: usize, culture: usize) -> usize {
    let id = w.cities.len();
    let name = w.cultures[culture].lang.place(&w.rng);
    let local_pop = w.cells[cell].pop;
    let city = City {
        id,
        name,
        cell,
        founded: w.year,
        culture,
        polity: p,
        pop: 0.4 + local_pop * 0.4,
        prosperity: 0.4,
        walls: 0.0,
        destroyed: None,
        wonders: Vec::new(),
        times_sacked: 0,
        founder: p,
        peak_pop: 0.0,
    };
    w.cities.push(city);
    w.cells[cell].city = Some(id);
    if w.cells[cell].culture.is_none() {
        w.cells[cell].culture = Some(culture);
    }
    id
}

pub fn claim(w: &mut World, p: usize, cell: usize) {
    let culture = w.polities[p].culture;
    w.cells[cell].owner = Some(p);
    w.cells[cell].since = w.year;
    if w.cells[cell].culture.is_none() {
        w.cells[cell].culture = Some(culture);
        w.cells[cell].pop = w.cells[cell].pop.max(0.05);
    }
    if let Some(c) = w.cells[cell].city {
        w.cities[c].polity = Some(p);
    }
    // Discover and name features.
    if w.detail.level() >= 1 {
        if let Some(f) = w.terrain.feature_at(cell) {
            if w.terrain.features[f].name.is_none() {
                w.feature_name(f, Some(p));
            }
        }
        if let Some(r) = w.terrain.river_at(cell) {
            if w.terrain.features[r].name.is_none() {
                w.feature_name(r, Some(p));
            }
        }
    }
}

/// Create a polity from a set of cells with a seat at `seat`.
pub fn found_polity(
    w: &mut World,
    culture: usize,
    seat: usize,
    kind: PolityKind,
    parent: Option<usize>,
    cells: &[usize],
    leader: Option<usize>,
) -> usize {
    let id = w.polities.len();
    let short = w.cultures[culture].lang.name(&w.rng);
    let color = w.polity_color(id);
    let ruler = match leader {
        Some(l) => l,
        None => w.new_person(
            culture,
            Role::Ruler,
            Some(id),
            w.year - w.rng.int(22, 45),
            None,
        ),
    };
    w.persons[ruler].polity = Some(id);
    w.persons[ruler].role = Role::Ruler;
    let dynasty = if kind.has_dynasty() {
        dynasty_name(w, culture, &w.persons[ruler].name.clone())
    } else {
        String::new()
    };
    let adj = w.cultures[culture].lang.adjective(&short);
    w.polities.push(Polity {
        id,
        name: String::new(),
        short: short.clone(),
        adj,
        kind,
        culture,
        capital: None,
        ruler: Some(ruler),
        dynasty,
        founded: w.year,
        fell: None,
        fall_cause: String::new(),
        parent,
        color,
        cells: 0,
        pop: 0.0,
        cities: Vec::new(),
        peak_cells: 0,
        peak_year: w.year,
        stability: 0.6,
        treasury: 5.0,
        army: 0.5,
        dev: parent.map(|pp| w.polities[pp].dev * 0.8).unwrap_or(0.0),
        prestige: 0.0,
        exhaustion: 0.0,
        decadence: 0.0,
        seafaring: parent.map(|pp| w.polities[pp].seafaring).unwrap_or(false),
        school: None,
        wars: Vec::new(),
        tension: BTreeMap::new(),
        neighbors: Vec::new(),
        conquered: 0,
        reign_start: w.year,
        reign_gained: 0,
        reign_cities: 0,
        reign_wars_won: 0,
        last_revolt: w.year,
        foreign_share: 0.0,
        avg_fertility: 0.0,
        cultures_within: 1,
        rulers: vec![ruler],
        generals: Vec::new(),
        last_kind_change: w.year,
        culture_counts: BTreeMap::new(),
        truce: BTreeMap::new(),
    });
    for &c in cells {
        claim(w, id, c);
    }
    w.polities[id].cells = cells.len();
    // Capital: existing city at seat, else found one.
    let cap = match w.cells[seat].city {
        Some(c) if w.cities[c].destroyed.is_none() => {
            w.cities[c].polity = Some(id);
            c
        }
        _ => found_city(w, Some(id), seat, culture),
    };
    w.polities[id].capital = Some(cap);
    let cap_name = w.cities[cap].name.clone();
    w.polities[id].name = make_name(w, kind, &short, culture, Some(&cap_name));
    w.century_polities_born += 1;
    id
}

// ---------------------------------------------------------------------------
// Formation of new states from stateless peoples
// ---------------------------------------------------------------------------

pub fn form_polities(w: &mut World) {
    let rng = w.rng.clone();
    let tn = w.tuning;
    let n = w.cells.len();
    let tries = (n / 40).max(50);
    let mut founded = 0;
    for _ in 0..tries {
        if founded >= 2 {
            break;
        }
        let i = rng.below(n);
        let cs = w.cells[i];
        if cs.owner.is_some() || cs.pop < tn.polity_form_min_pop || !w.terrain.is_land(i) {
            continue;
        }
        let culture = match cs.culture {
            Some(c) => c,
            None => continue,
        };
        // No existing state nearby.
        let (x, y) = w.terrain.xy(i);
        let mut near_state = false;
        'scan: for dy in -tn.polity_form_radius_y..=tn.polity_form_radius_y {
            for dx in -tn.polity_form_radius_x..=tn.polity_form_radius_x {
                let cx = x as i32 + dx;
                let cy = y as i32 + dy;
                if cx < 0 || cy < 0 || cx >= w.terrain.w as i32 || cy >= w.terrain.h as i32 {
                    continue;
                }
                if w.cells[w.terrain.idx(cx as usize, cy as usize)]
                    .owner
                    .is_some()
                {
                    near_state = true;
                    break 'scan;
                }
            }
        }
        if near_state {
            continue;
        }
        let mil = w.cultures[culture].values.militarism;
        if !rng.chance(tn.polity_form_chance * (0.5 + mil as f64) * (cs.pop as f64).min(3.0)) {
            continue;
        }
        // Gather nearby cells of the same culture.
        let mut cells = Vec::new();
        for dy in -1i32..=1 {
            for dx in -2i32..=2 {
                let cx = x as i32 + dx;
                let cy = y as i32 + dy;
                if cx < 0 || cy < 0 || cx >= w.terrain.w as i32 || cy >= w.terrain.h as i32 {
                    continue;
                }
                let j = w.terrain.idx(cx as usize, cy as usize);
                if w.cells[j].owner.is_none()
                    && w.terrain.is_land(j)
                    && w.terrain.biome[j].move_cost().is_some()
                    && w.cells[j].culture == Some(culture)
                {
                    cells.push(j);
                }
            }
        }
        if cells.len() < 3 {
            continue;
        }
        let kind = if w.cultures[culture].values.militarism > 0.8
            && matches!(
                w.terrain.biome[i],
                crate::geo::Biome::Steppe | crate::geo::Biome::Savanna
            )
            && rng.chance(0.5)
        {
            PolityKind::Horde
        } else {
            PolityKind::Tribe
        };
        let p = found_polity(w, culture, i, kind, None, &cells, None);
        founded += 1;
        let place = w.place_phrase(i, Some(p));
        let capital = w.polities[p].capital.unwrap();
        let text = prose::polity_founded(w, p, kind, capital, culture, &place);
        w.log(
            1,
            EventKind::Founding,
            &[
                Ref::Polity(p),
                Ref::Culture(culture),
                Ref::City(capital),
            ],
            Some(i),
            text,
        );
    }
}

// ---------------------------------------------------------------------------
// Expansion
// ---------------------------------------------------------------------------

pub fn expand(w: &mut World) {
    let rng = w.rng.clone();
    let tn = w.tuning;
    let n = w.cells.len();
    let np = w.polities.len();
    let mut cand: Vec<Vec<(f32, usize, f32)>> = vec![Vec::new(); np];
    let capitals: Vec<Option<usize>> = (0..np).map(|p| w.capital_cell(p)).collect();
    let alive: Vec<bool> = w.polities.iter().map(|p| p.alive()).collect();
    let mut seen: Vec<usize> = Vec::with_capacity(8);
    for i in 0..n {
        if w.cells[i].owner.is_some() || !w.terrain.is_land(i) {
            continue;
        }
        let b = w.terrain.biome[i];
        let base_cost = match b.move_cost() {
            Some(c) => c,
            None => continue,
        };
        seen.clear();
        for nb in w.terrain.neighbors8(i) {
            if let Some(p) = w.cells[nb].owner {
                if !alive[p] || seen.contains(&p) {
                    continue;
                }
                seen.push(p);
                let score = score_cell(w, p, i, capitals[p], base_cost, &rng);
                if let Some((s, c)) = score {
                    cand[p].push((s, i, c));
                }
            }
        }
        // Overseas colonisation for seafaring states.
        if w.terrain.coast[i] {
            let (x, y) = w.terrain.xy(i);
            for dy in -2i32..=2 {
                for dx in -4i32..=4 {
                    if dx.abs() <= 1 && dy.abs() <= 1 {
                        continue;
                    }
                    let cx = x as i32 + dx;
                    let cy = y as i32 + dy;
                    if cx < 0 || cy < 0 || cx >= w.terrain.w as i32 || cy >= w.terrain.h as i32 {
                        continue;
                    }
                    let j = w.terrain.idx(cx as usize, cy as usize);
                    if !w.terrain.coast[j] {
                        continue;
                    }
                    if let Some(p) = w.cells[j].owner {
                        if !alive[p] || !w.polities[p].seafaring || seen.contains(&p) {
                            continue;
                        }
                        seen.push(p);
                        if let Some((s, c)) =
                            score_cell(w, p, i, capitals[p], base_cost + 4.0, &rng)
                        {
                            cand[p].push((s - 0.5, i, c));
                        }
                    }
                }
            }
        }
    }
    for p in 0..np {
        if cand[p].is_empty() {
            continue;
        }
        let pol = &w.polities[p];
        let ambition = pol
            .ruler
            .map(|r| w.persons[r].traits.ambition)
            .unwrap_or(0.5);
        let over = pol.overextension(&tn);
        let over_pen = if over > 1.0 { 1.0 / (over * over) } else { 1.0 };
        let war_pen = if pol.at_war() {
            tn.expand_war_penalty
        } else {
            1.0
        };
        let mut budget = (tn.expand_budget_base
            + (pol.pop as f32).sqrt() * tn.expand_budget_pop_factor)
            * pol.kind.expansion_mult()
            * (tn.expand_ambition_base + ambition)
            * (tn.expand_stability_base + pol.stability)
            * over_pen
            * war_pen;
        let list = &mut cand[p];
        list.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        let mut taken = 0;
        let claims: Vec<usize> = {
            let mut out = Vec::new();
            for &(score, cell, cost) in list.iter() {
                if score < -0.6 && taken > 0 {
                    break;
                }
                if budget < cost && taken > 0 {
                    break;
                }
                if score < -1.5 {
                    break;
                }
                budget -= cost;
                out.push(cell);
                taken += 1;
                if taken >= tn.expand_max_claims {
                    break;
                }
            }
            out
        };
        for cell in claims {
            if w.cells[cell].owner.is_none() {
                claim(w, p, cell);
                w.polities[p].reign_gained += 1;
            }
        }
    }
}

fn score_cell(
    w: &World,
    p: usize,
    i: usize,
    capital: Option<usize>,
    base_cost: f32,
    rng: &crate::rng::Rng,
) -> Option<(f32, f32)> {
    let pol = &w.polities[p];
    let t = &w.terrain;
    let f = t.fertility[i];
    let river = t.river[i] as f32 * 0.25;
    let coast = if t.coast[i] {
        0.25 + if pol.seafaring { 0.2 } else { 0.0 }
    } else {
        0.0
    };
    let minerals = t.minerals[i] * 0.35;
    let cs = &w.cells[i];
    let pop = cs.pop * 0.25;
    let reach = w.tuning.expand_reach_base
        + pol.dev * w.tuning.expand_reach_dev_weight
        + if pol.seafaring { 5.0 } else { 0.0 }
        + match pol.kind {
            PolityKind::Empire => 12.0,
            PolityKind::Horde => 14.0,
            PolityKind::Kingdom
            | PolityKind::Republic
            | PolityKind::Theocracy
            | PolityKind::Magocracy => 5.0,
            _ => 0.0,
        };
    let d = capital.map(|c| t.dist(c, i)).unwrap_or(5) as f32;
    let dist_pen = (d / reach).powi(2) * w.tuning.expand_distance_penalty;
    let culture_bonus = match cs.culture {
        Some(c) if c == pol.culture => 0.5,
        Some(c) => {
            if w.cultures[c].race == w.cultures[pol.culture].race {
                -0.1
            } else {
                -0.3
            }
        }
        None => 0.0,
    };
    let race = w.cultures[pol.culture].race;
    let aff = w.affinity(race, i);
    let score = f * w.tuning.expand_fertility_weight
        + river
        + coast
        + minerals
        + pop
        + culture_bonus
        + aff * 0.6
        - base_cost * w.tuning.expand_terrain_cost_weight
        - dist_pen
        + rng.range32(-0.2, 0.2);
    let cost = base_cost * (1.0 + dist_pen * w.tuning.expand_distance_cost_weight);
    Some((score, cost))
}

// ---------------------------------------------------------------------------
// Cities
// ---------------------------------------------------------------------------

pub fn found_cities(w: &mut World) {
    let rng = w.rng.clone();
    let tn = w.tuning;
    let n = w.cells.len();
    let np = w.polities.len();
    let mut by_owner: Vec<Vec<usize>> = vec![Vec::new(); np];
    for i in 0..n {
        if let Some(p) = w.cells[i].owner {
            by_owner[p].push(i);
        }
    }
    let city_cells: Vec<usize> = w
        .cities
        .iter()
        .filter(|c| c.destroyed.is_none())
        .map(|c| c.cell)
        .collect();
    for p in 0..np {
        if !w.polities[p].alive() || by_owner[p].is_empty() {
            continue;
        }
        let pol = &w.polities[p];
        let max_cities = 1 + pol.cells / tn.city_cells_per_city as usize + (pol.dev * 2.0) as usize;
        if pol.cities.len() >= max_cities {
            continue;
        }
        if !rng.chance(tn.city_found_chance * (0.5 + pol.dev as f64) * (0.4 + pol.stability as f64))
        {
            continue;
        }
        let mut best = None;
        let mut best_s = tn.city_site_min_score;
        let cells = &by_owner[p];
        let samples = cells.len().min(80);
        for _ in 0..samples {
            let i = cells[rng.below(cells.len())];
            if w.cells[i].city.is_some() || !w.terrain.is_land(i) {
                continue;
            }
            let (x, y) = w.terrain.xy(i);
            let too_close = city_cells
                .iter()
                .chain(w.polities[p].cities.iter().map(|&c| &w.cities[c].cell))
                .any(|&cc| {
                    let (cx, cy) = w.terrain.xy(cc);
                    (cx as i32 - x as i32).abs() <= tn.city_spacing_x
                        && (cy as i32 - y as i32).abs() <= tn.city_spacing_y
                });
            if too_close {
                continue;
            }
            let t = &w.terrain;
            let s = t.fertility[i]
                + t.river[i] as f32 * 0.3
                + if t.coast[i] { 0.5 } else { 0.0 }
                + w.cells[i].pop * 0.3
                + t.minerals[i] * 0.3;
            if s > best_s {
                best_s = s;
                best = Some(i);
            }
        }
        if let Some(i) = best {
            let culture = w.cells[i].culture.unwrap_or(w.polities[p].culture);
            let c = found_city(w, Some(p), i, culture);
            w.polities[p].reign_cities += 1;
            let place = w.place_phrase(i, Some(p));
            let text = prose::city_founded(w, p, c, &place, &Pick::rolled(&rng));
            w.log(
                1,
                EventKind::Founding,
                &[Ref::City(c), Ref::Polity(p)],
                Some(i),
                text,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Economy, stability, development
// ---------------------------------------------------------------------------

pub fn economy(w: &mut World) {
    let rng = w.rng.clone();
    let tn = w.tuning;
    let np = w.polities.len();
    let neighbor_dev: Vec<f32> = (0..np)
        .map(|p| {
            w.polities[p]
                .neighbors
                .iter()
                .map(|&(q, _)| w.polities[q].dev)
                .fold(0.0, f32::max)
        })
        .collect();
    for p in 0..np {
        if !w.polities[p].alive() {
            continue;
        }
        let (army_mult, stab_bonus, dev_bonus, prosp_bonus) = w.school_effects(p);
        let art_mult = w.artifact_army_mult(p);
        let culture = w.polities[p].culture;
        let vals = w.cultures[culture].values;
        let at_war = w.polities[p].at_war();
        let peace_neighbors = w.polities[p]
            .neighbors
            .iter()
            .filter(|&&(q, _)| {
                !w.wars.iter().any(|wr| {
                    wr.alive()
                        && ((wr.attacker == p && wr.defender == q)
                            || (wr.attacker == q && wr.defender == p))
                })
            })
            .count() as f32;
        // Cities.
        let cities = w.polities[p].cities.clone();
        let mut prosp_sum = 0.0;
        let mut income = 0.0;
        let dev = w.polities[p].dev;
        let is_capital = w.polities[p].capital;
        for &c in &cities {
            let cell = w.cities[c].cell;
            let t = &w.terrain;
            let mut minerals = 0.0;
            for nb in t.neighbors8(cell) {
                minerals += t.minerals[nb];
            }
            minerals /= 8.0;
            let mut target =
                0.3 + if t.coast[cell] {
                    0.25 * (0.5 + vals.mercantilism)
                } else {
                    0.0
                } + t.river[cell] as f32 * 0.07
                    + minerals * 0.25
                    + dev * 0.3
                    + (peace_neighbors * 0.04).min(0.2)
                    + prosp_bonus;
            if is_capital == Some(c) {
                target += 0.1;
            }
            if at_war {
                target -= 0.15;
            }
            target += w.cities[c].wonders.len() as f32 * 0.08;
            let city = &mut w.cities[c];
            city.prosperity += (target - city.prosperity) * tn.prosperity_adjust_rate;
            city.prosperity = city.prosperity.clamp(0.05, 2.5);
            city.walls += (dev * 0.8 - city.walls) * 0.03;
            prosp_sum += city.prosperity;
            income += city.pop * city.prosperity * tn.city_income_factor;
        }
        let avg_prosp = if cities.is_empty() {
            0.3
        } else {
            prosp_sum / cities.len() as f32
        };
        let pol = &mut w.polities[p];
        income += pol.cells as f32 * tn.cell_income_factor * (1.0 + pol.dev);
        let upkeep = pol.army * tn.army_upkeep_factor + pol.cells as f32 * tn.cell_upkeep_factor;
        pol.treasury = (pol.treasury + income - upkeep).clamp(-60.0, 600.0);
        // Army.
        let kind_mult = match pol.kind {
            PolityKind::Horde => 2.2,
            PolityKind::Tribe => 1.3,
            PolityKind::Chiefdom => 1.2,
            PolityKind::Republic => 0.85,
            PolityKind::Magocracy => 0.8,
            _ => 1.0,
        };
        let target_army = pol.pop as f32
            * (tn.army_pop_factor + vals.militarism * tn.army_militarism_weight)
            * kind_mult
            * (1.0 + pol.dev * 0.5)
            * army_mult
            * art_mult;
        let rate = if at_war {
            tn.army_build_rate_war
        } else {
            tn.army_build_rate_peace
        };
        pol.army += (target_army - pol.army) * rate;
        if pol.treasury < 0.0 {
            pol.army *= 0.96;
        }
        pol.army = pol.army.max(0.1);
        // Development.
        let stab = pol.stability;
        let mut ddev = tn.dev_growth_rate * (avg_prosp + vals.openness * 0.5 + dev_bonus);
        if stab < 0.3 {
            ddev -= 0.0015;
        }
        if neighbor_dev[p] > pol.dev {
            ddev += (neighbor_dev[p] - pol.dev) * 0.004 * (0.5 + vals.openness);
        }
        pol.dev = (pol.dev + ddev).clamp(0.0, 3.0);
        // Seafaring.
        if !pol.seafaring {
            let coastal_city = cities.iter().any(|&c| w.terrain.coast[w.cities[c].cell]);
            let sea = w.races[w.cultures[culture].race].seafaring;
            if coastal_city && rng.chance(0.01 * (sea as f64 + pol.dev as f64 * 0.5)) {
                pol.seafaring = true;
                let text = prose::learned_seafaring(w, p);
                w.log(1, EventKind::Discovery, &[Ref::Polity(p)], None, text);
            }
        }
        let pol = &mut w.polities[p];
        // Prestige.
        pol.prestige = pol.prestige * 0.985 + pol.cells as f32 * 0.002 + cities.len() as f32 * 0.01;
        // Stability.
        let (wisdom, charisma) = pol
            .ruler
            .map(|r| (w.persons[r].traits.wisdom, w.persons[r].traits.charisma))
            .unwrap_or((0.3, 0.3));
        let over = pol.overextension(&tn);
        let kind_stab = match pol.kind {
            PolityKind::Kingdom | PolityKind::Republic => 0.05,
            PolityKind::Empire => -0.08,
            PolityKind::Horde => -0.1,
            PolityKind::Theocracy => 0.03,
            _ => 0.0,
        };
        let target = tn.stability_base
            + wisdom * tn.stability_wisdom_weight
            + charisma * tn.stability_charisma_weight
            + stab_bonus
            + kind_stab
            - (over - 1.0).max(0.0) * tn.stability_overextension_weight
            - pol.foreign_share * tn.stability_foreign_weight
            - pol.exhaustion * tn.stability_exhaustion_weight
            - pol.decadence * tn.stability_decadence_weight
            + (avg_prosp - 0.4) * 0.15
            + vals.tradition * 0.05
            + (pol.treasury / 600.0).max(-0.2) * 0.2;
        pol.stability +=
            (target - pol.stability) * tn.stability_adjust_rate + rng.range32(-0.02, 0.02);
        pol.stability = pol.stability.clamp(0.0, 1.0);
        // Exhaustion and decadence.
        if at_war {
            pol.exhaustion = (pol.exhaustion + tn.exhaustion_war_growth).min(1.5);
        } else {
            pol.exhaustion = (pol.exhaustion - tn.exhaustion_peace_decay).max(0.0);
        }
        if pol.kind.rank() >= 2 && w.year - pol.founded > tn.decadence_min_age {
            let kind_mult = if pol.kind == PolityKind::Empire {
                tn.decadence_empire_mult
            } else {
                1.0
            };
            pol.decadence += (tn.decadence_growth * (1.0 + over)
                - wisdom * tn.decadence_wisdom_relief)
                * kind_mult;
            pol.decadence = pol.decadence.clamp(0.0, 1.0);
        }
    }
}

// ---------------------------------------------------------------------------
// Rulers
// ---------------------------------------------------------------------------

fn epithet_for(w: &World, p: usize, r: usize) -> Option<String> {
    let pol = &w.polities[p];
    let per = &w.persons[r];
    let t = per.traits;
    let rng = &w.rng;
    let length = w.year - pol.reign_start;
    let mut opts: Vec<&str> = Vec::new();
    if pol.reign_gained > 50 {
        opts.extend(["the Conqueror", "the Great", "Widereach", "the Hammer"]);
    }
    if pol.reign_gained < -25 {
        opts.extend(["the Unlucky", "the Lesser", "the Landless", "the Unready"]);
    }
    if pol.reign_cities >= 3 {
        opts.extend(["the Builder", "the Founder"]);
    }
    if pol.reign_wars_won >= 2 {
        opts.extend(["the Victorious", "the Lion", "Ironhand"]);
    }
    if t.cruelty > 0.75 {
        opts.extend(["the Cruel", "the Terrible", "Bloodhand", "the Butcher"]);
    }
    if t.piety > 0.78 {
        opts.extend(["the Pious", "the Devout", "the Blessed"]);
    }
    if t.wisdom > 0.78 {
        opts.extend(["the Wise", "the Just", "the Lawgiver"]);
    }
    if t.wisdom < 0.22 {
        opts.extend(["the Fool", "the Simple", "the Unready"]);
    }
    if t.charisma > 0.78 {
        opts.extend(["the Beloved", "Goldentongue", "the Fair"]);
    }
    if t.valor > 0.78 {
        opts.extend(["the Bold", "the Fearless", "Stormcrow"]);
    }
    if t.ambition > 0.8 && pol.reign_gained < 10 {
        opts.extend(["the Restless", "the Dreamer"]);
    }
    if length > 45 {
        opts.extend(["the Old", "the Long-Reigning", "the Patient"]);
    }
    if length < 3 {
        opts.extend(["the Brief"]);
    }
    if opts.is_empty() {
        if rng.chance(0.3) {
            opts.extend([
                "the Red",
                "the Quiet",
                "Fairhair",
                "Longshanks",
                "the Fat",
                "the Pale",
                "One-Eye",
                "the Stammerer",
                "the Grey",
                "the Hunter",
            ]);
        } else {
            return None;
        }
    }
    Some(rng.pick(&opts).to_string())
}

pub fn ruler_dies(w: &mut World, p: usize, cause: String, importance: u8) {
    let r = match w.polities[p].ruler {
        Some(r) => r,
        None => return,
    };
    let epithet = epithet_for(w, p, r);
    w.persons[r].epithet = epithet;
    w.persons[r].died = Some(w.year);
    w.persons[r].death = cause.clone();
    let length = w.year - w.polities[p].reign_start;
    let mut text = prose::ruler_died(w, p, r, &cause, length);
    if w.high_detail() {
        let flourish = prose::ruler_death_flourish(w, p, r, length, &Pick::rolled(&w.rng.clone()));
        text.push_str(&flourish);
    }
    w.log(
        importance,
        EventKind::Death,
        &[Ref::Person(r), Ref::Polity(p)],
        w.capital_cell(p),
        text,
    );
    super::stories::artifacts_on_ruler_death(w, p, r);
    w.polities[p].ruler = None;
    succession(w, p, r);
}

fn succession(w: &mut World, p: usize, old: usize) {
    let rng = w.rng.clone();
    let pol = &w.polities[p];
    let culture = pol.culture;
    let kind = pol.kind;
    let stability = pol.stability;
    let old_traits = w.persons[old].traits;
    let dynasty = pol.dynasty.clone();
    if !kind.has_dynasty() {
        let heir = w.new_person(
            culture,
            Role::Ruler,
            Some(p),
            w.year - rng.int(25, 50),
            None,
        );
        install_ruler(w, p, heir);
        let text = prose::elected(w, p, kind, heir);
        w.log(
            1,
            EventKind::Politics,
            &[Ref::Person(heir), Ref::Polity(p)],
            w.capital_cell(p),
            text,
        );
        return;
    }
    let p_smooth = w.tuning.succession_smooth_base
        + stability as f64 * w.tuning.succession_smooth_stability_weight;
    if rng.chance(p_smooth) {
        let heir = w.new_person(
            culture,
            Role::Ruler,
            Some(p),
            w.year - rng.int(16, 40),
            Some(old_traits.inherit(&rng)),
        );
        w.persons[heir].parent = Some(old);
        install_ruler(w, p, heir);
        let text = prose::succession_smooth(w, p, heir, &dynasty, &Pick::rolled(&rng));
        w.log(
            1,
            EventKind::Politics,
            &[Ref::Person(heir), Ref::Polity(p)],
            w.capital_cell(p),
            text,
        );
        return;
    }
    // Crisis.
    let roll = rng.f64();
    if roll < 0.4 || w.polities[p].cells < 25 {
        // Usurper founds a new dynasty.
        let usurper = w.new_person(
            culture,
            Role::Ruler,
            Some(p),
            w.year - rng.int(25, 50),
            None,
        );
        w.persons[usurper].traits.ambition = (w.persons[usurper].traits.ambition + 0.3).min(1.0);
        let dyn_name = dynasty_name(w, culture, &w.persons[usurper].name.clone());
        w.polities[p].dynasty = dyn_name.clone();
        install_ruler(w, p, usurper);
        w.polities[p].stability = (w.polities[p].stability - 0.2).max(0.0);
        let text = prose::succession_usurped(w, p, usurper, &dyn_name, &dynasty);
        w.log(
            2,
            EventKind::Politics,
            &[Ref::Person(usurper), Ref::Polity(p)],
            w.capital_cell(p),
            text,
        );
    } else if roll < 0.75 {
        // Succession war: the realm splits.
        let claimant_a = w.new_person(
            culture,
            Role::Ruler,
            Some(p),
            w.year - rng.int(20, 45),
            Some(old_traits.inherit(&rng)),
        );
        w.persons[claimant_a].parent = Some(old);
        install_ruler(w, p, claimant_a);
        let claimant_b = w.new_person(
            culture,
            Role::Rebel,
            None,
            w.year - rng.int(20, 45),
            Some(old_traits.inherit(&rng)),
        );
        w.persons[claimant_b].parent = Some(old);
        let name_a = w.persons[claimant_a].name.clone();
        let name_b = w.persons[claimant_b].name.clone();
        if let Some(rebel) = split_off(w, p, Some(claimant_b), None) {
            let claim = prose::succession_claim(w, p, &name_b);
            w.wars_start(rebel, p, super::WarKind::Succession, claim);
            let seat_a = w.cities[w.polities[p].capital.unwrap()].name.clone();
            let text = prose::succession_war(w, p, rebel, &name_a, &name_b, &seat_a);
            w.log(
                2,
                EventKind::Politics,
                &[
                    Ref::Polity(p),
                    Ref::Polity(rebel),
                    Ref::Person(claimant_a),
                    Ref::Person(claimant_b),
                ],
                w.capital_cell(p),
                text,
            );
        } else {
            let text = prose::succession_disputed(w, p, &name_a);
            w.log(
                1,
                EventKind::Politics,
                &[Ref::Person(claimant_a), Ref::Polity(p)],
                w.capital_cell(p),
                text,
            );
        }
    } else {
        // Weak regency.
        let regent = w.new_person(
            culture,
            Role::Ruler,
            Some(p),
            w.year - rng.int(8, 14),
            Some(old_traits.inherit(&rng)),
        );
        w.persons[regent].parent = Some(old);
        install_ruler(w, p, regent);
        w.polities[p].stability = (w.polities[p].stability - 0.25).max(0.0);
        let text = prose::succession_regency(w, p, regent);
        w.log(
            1,
            EventKind::Politics,
            &[Ref::Person(regent), Ref::Polity(p)],
            w.capital_cell(p),
            text,
        );
    }
}

pub fn install_ruler(w: &mut World, p: usize, r: usize) {
    w.persons[r].polity = Some(p);
    w.persons[r].role = Role::Ruler;
    let pol = &mut w.polities[p];
    pol.ruler = Some(r);
    pol.reign_start = w.year;
    pol.reign_gained = 0;
    pol.reign_cities = 0;
    pol.reign_wars_won = 0;
    pol.rulers.push(r);
    let wisdom = w.persons[r].traits.wisdom;
    if wisdom > 0.6 {
        w.polities[p].decadence *= 0.7;
    }
}

pub fn rulers(w: &mut World) {
    let rng = w.rng.clone();
    let tn = w.tuning;
    for p in w.living_polities() {
        let r = match w.polities[p].ruler {
            Some(r) => r,
            None => {
                let culture = w.polities[p].culture;
                let heir = w.new_person(
                    culture,
                    Role::Ruler,
                    Some(p),
                    w.year - rng.int(25, 45),
                    None,
                );
                install_ruler(w, p, heir);
                continue;
            }
        };
        let per = &w.persons[r];
        let age = (w.year - per.born) as f32;
        let lifespan = w.races[per.race].lifespan;
        let rel = age / lifespan;
        let p_nat = tn.ruler_death_base + tn.ruler_death_age_weight * rel.powi(6);
        let stability = w.polities[p].stability;
        let p_assassin = tn.ruler_assassination_base
            * (1.0 + per.traits.cruelty * tn.ruler_assassination_cruelty_weight)
            + (0.5 - stability).max(0.0) * tn.ruler_assassination_unrest_weight;
        let roll = rng.f64();
        if roll < p_nat as f64 {
            let cause = prose::natural_death(age as i32, &Pick::rolled(&rng));
            ruler_dies(w, p, cause, 1);
        } else if roll < (p_nat + p_assassin) as f64 {
            let by = prose::assassin(w, p, &Pick::rolled(&rng));
            w.polities[p].stability = (w.polities[p].stability - 0.15).max(0.0);
            ruler_dies(w, p, prose::murdered_by(&by), 2);
        }
    }
}

// ---------------------------------------------------------------------------
// Unrest, kind changes, fragmentation, collapse
// ---------------------------------------------------------------------------

/// Carve a province off `p` into a new polity. Returns the new polity id.
pub fn split_off(
    w: &mut World,
    p: usize,
    leader: Option<usize>,
    prefer_culture: Option<usize>,
) -> Option<usize> {
    let rng = w.rng.clone();
    let cells = w.cells_of(p);
    if cells.len() < 8 {
        return None;
    }
    let capital = w.capital_cell(p)?;
    // Pick a seed: far from the capital, preferably foreign culture or a city.
    let mut best = None;
    let mut best_s = -1.0f32;
    for _ in 0..60 {
        let i = cells[rng.below(cells.len())];
        if i == capital {
            continue;
        }
        let d = w.terrain.dist(capital, i) as f32;
        let mut s = d;
        if let Some(pc) = prefer_culture {
            if w.cells[i].culture == Some(pc) {
                s *= 3.0;
            }
        } else if w.cells[i].culture != Some(w.polities[p].culture) {
            s *= 2.0;
        }
        if w.cells[i].city.is_some() {
            s *= 1.8;
        }
        if s > best_s {
            best_s = s;
            best = Some(i);
        }
    }
    let seed = best?;
    let seed_culture = w.cells[seed].culture.unwrap_or(w.polities[p].culture);
    let target = ((cells.len() as f32
        * rng.range32(w.tuning.split_min_share, w.tuning.split_max_share))
        as usize)
        .clamp(4, 90);
    // BFS from seed through cells of p.
    let mut region = vec![seed];
    let mut seen = std::collections::BTreeSet::new();
    seen.insert(seed);
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(seed);
    while let Some(i) = queue.pop_front() {
        if region.len() >= target {
            break;
        }
        for nb in w.terrain.neighbors8(i) {
            if seen.contains(&nb) || w.cells[nb].owner != Some(p) || nb == capital {
                continue;
            }
            // Regions prefer their own culture.
            if w.cells[nb].culture != Some(seed_culture) && rng.chance(0.4) {
                continue;
            }
            seen.insert(nb);
            region.push(nb);
            queue.push_back(nb);
            if region.len() >= target {
                break;
            }
        }
    }
    if region.len() < 4 {
        return None;
    }
    let kind = if region.len() >= 25 {
        PolityKind::Kingdom
    } else {
        PolityKind::Chiefdom
    };
    // Seat: a city within the region if any, else the seed.
    let seat = region
        .iter()
        .copied()
        .find(|&i| w.cells[i].city.is_some())
        .unwrap_or(seed);
    let leader = leader.map(|l| {
        w.persons[l].culture = seed_culture;
        l
    });
    let np = found_polity(w, seed_culture, seat, kind, Some(p), &region, leader);
    w.polities[p].last_revolt = w.year;
    Some(np)
}

pub fn fall(
    w: &mut World,
    p: usize,
    cause: String,
    absorbed_by: Option<usize>,
    importance_floor: u8,
) {
    if !w.polities[p].alive() {
        return;
    }
    super::stories::artifacts_on_fall(w, p, absorbed_by);
    let cells = w.cells_of(p);
    for &i in &cells {
        w.cells[i].owner = absorbed_by;
        w.cells[i].since = w.year;
    }
    let cities = w.polities[p].cities.clone();
    for c in cities {
        w.cities[c].polity = absorbed_by;
    }
    let wars = w.polities[p].wars.clone();
    for wid in wars {
        let result = prose::war_lapsed(w, p);
        w.end_war(wid, result, false);
    }
    if let Some(r) = w.polities[p].ruler {
        if w.persons[r].alive() {
            w.persons[r].role = Role::Ruler;
        }
    }
    let pol = &mut w.polities[p];
    pol.fell = Some(w.year);
    pol.fall_cause = cause.clone();
    let peak = pol.peak_cells;
    let imp = if peak > 150 {
        3
    } else if peak > 40 {
        2
    } else {
        1
    }
    .max(importance_floor);
    let cities_built = w
        .cities
        .iter()
        .filter(|c| c.founder == Some(p))
        .count()
        .max(w.polities[p].cities.len());
    let text = prose::realm_fell(w, p, &cause, cities_built);
    let mut refs = vec![Ref::Polity(p)];
    if let Some(a) = absorbed_by {
        refs.push(Ref::Polity(a));
        w.polities[a].conquered += 1;
    }
    w.log(imp, EventKind::Politics, &refs, w.capital_cell(p), text);
}

pub fn unrest(w: &mut World) {
    let rng = w.rng.clone();
    let tn = w.tuning;
    for p in w.living_polities() {
        let pol = &w.polities[p];
        if pol.cells == 0 && pol.founded < w.year {
            let cause = prose::faded_away(!pol.wars.is_empty()).to_string();
            fall(w, p, cause, None, 1);
            continue;
        }
        // Kind progression.
        kind_changes(w, p);
        let pol = &w.polities[p];
        let stability = pol.stability;
        let since_revolt = w.year - pol.last_revolt;
        // Revolts.
        if stability < tn.revolt_stability_threshold
            && since_revolt > tn.revolt_cooldown
            && pol.cells > tn.revolt_min_cells as usize
        {
            let mut p_rev =
                (tn.revolt_stability_threshold - stability) as f64 * tn.revolt_chance_factor;
            if pol.kind == PolityKind::Empire {
                p_rev *= tn.revolt_empire_mult;
            }
            if rng.chance(p_rev) {
                let cruelty = pol
                    .ruler
                    .map(|r| w.persons[r].traits.cruelty)
                    .unwrap_or(0.5);
                let leader = w.new_person(
                    pol.culture,
                    Role::Rebel,
                    None,
                    w.year - rng.int(22, 45),
                    None,
                );
                if let Some(rebel) = split_off(w, p, Some(leader), None) {
                    let rc = w.polities[rebel].culture;
                    let kind = if rc == w.polities[p].culture {
                        super::WarKind::CivilWar
                    } else {
                        super::WarKind::Rebellion
                    };
                    let cause = prose::revolt_cause(
                        w,
                        p,
                        rc,
                        kind == super::WarKind::CivilWar,
                        cruelty > 0.6,
                    );
                    w.wars_start(rebel, p, kind, cause.clone());
                    let seat = w.cities[w.polities[rebel].capital.unwrap()].name.clone();
                    let text = prose::revolt(w, p, rebel, leader, &seat, &cause);
                    w.log(
                        2,
                        EventKind::War,
                        &[Ref::Polity(rebel), Ref::Polity(p), Ref::Person(leader)],
                        w.capital_cell(rebel),
                        text,
                    );
                    w.polities[p].stability = (w.polities[p].stability + 0.1).min(1.0);
                    continue;
                }
            }
        }
        // Fragmentation of large, failing states.
        let pol = &w.polities[p];
        if pol.stability < tn.fragment_stability_threshold
            && pol.cells > tn.fragment_min_cells as usize
            && since_revolt > tn.fragment_cooldown
            && rng.chance(tn.fragment_chance)
        {
            fragment(w, p);
        }
    }
}

fn kind_changes(w: &mut World, p: usize) {
    let rng = w.rng.clone();
    let pol = &w.polities[p];
    if w.year - pol.last_kind_change < 5 {
        return;
    }
    let culture = pol.culture;
    let vals = w.cultures[culture].values;
    let old = pol.kind;
    let mut new = old;
    match old {
        PolityKind::Tribe => {
            if pol.cells >= 12 {
                new = PolityKind::Chiefdom;
            }
        }
        PolityKind::Chiefdom => {
            if (pol.cells >= 35 && (pol.dev > 0.2 || pol.cities.len() >= 2)) || pol.cells >= 70 {
                new = PolityKind::Kingdom;
                if vals.mercantilism > 0.65
                    && pol
                        .capital
                        .map(|c| w.terrain.coast[w.cities[c].cell])
                        .unwrap_or(false)
                    && rng.chance(0.5)
                {
                    new = PolityKind::Republic;
                }
            }
        }
        PolityKind::Horde => {
            if pol.cities.len() >= 4 && pol.dev > 0.5 && rng.chance(0.1) {
                new = PolityKind::Kingdom;
            }
        }
        PolityKind::Kingdom
        | PolityKind::Republic
        | PolityKind::Theocracy
        | PolityKind::Magocracy => {
            let empire_cells = (w.tuning.empire_min_cells as usize).max(w.stats.owned_cells / 8);
            if pol.cells >= empire_cells
                && (pol.conquered >= 2 || pol.cultures_within >= 3)
                && pol.stability > 0.4
            {
                new = PolityKind::Empire;
            } else if old == PolityKind::Kingdom {
                if let Some(s) = pol.school {
                    let infl = w.schools[s].influence.get(&p).copied().unwrap_or(0.0);
                    let piety = pol.ruler.map(|r| w.persons[r].traits.piety).unwrap_or(0.3);
                    if w.schools[s].kind == super::SchoolKind::Divine
                        && infl > 0.8
                        && piety > 0.75
                        && rng.chance(0.03)
                    {
                        new = PolityKind::Theocracy;
                    } else if w.schools[s].kind == super::SchoolKind::Arcane
                        && infl > 0.85
                        && vals.mysticism > 0.6
                        && rng.chance(0.03)
                    {
                        new = PolityKind::Magocracy;
                    }
                }
            }
        }
        PolityKind::Empire => {
            if pol.cells < 60 && pol.stability < 0.4 {
                new = PolityKind::Kingdom;
            }
        }
    }
    if new == old {
        return;
    }
    let short = w.polities[p].short.clone();
    let cap_name = w.polities[p].capital.map(|c| w.cities[c].name.clone());
    let name = make_name(w, new, &short, culture, cap_name.as_deref());
    let oldname = w.polities[p].name.clone();
    let ruler = w.ruler_short(p);
    let pol = &mut w.polities[p];
    pol.kind = new;
    pol.name = name.clone();
    pol.last_kind_change = w.year;
    if new.has_dynasty() && pol.dynasty.is_empty() {
        if let Some(r) = pol.ruler {
            let founder = w.persons[r].name.clone();
            let d = dynasty_name(w, culture, &founder);
            w.polities[p].dynasty = d;
        }
    }
    let capital = cap_name.unwrap_or_else(|| short.clone());
    let (imp, text) = prose::rank_changed(w, p, old, new, &oldname, &name, &ruler, &capital);
    w.log(
        imp,
        EventKind::Politics,
        &[Ref::Polity(p)],
        w.capital_cell(p),
        text,
    );
}

fn fragment(w: &mut World, p: usize) {
    let rng = w.rng.clone();
    let capital = match w.polities[p].capital {
        Some(c) => c,
        None => return,
    };
    let mut cities: Vec<usize> = w.polities[p]
        .cities
        .iter()
        .copied()
        .filter(|&c| c != capital)
        .collect();
    cities.sort_by(|&a, &b| w.cities[b].pop.partial_cmp(&w.cities[a].pop).unwrap());
    let k = (2 + rng.below(3)).min(cities.len());
    if k < 1 {
        // No cities to seed successors: the state simply collapses.
        fall(
            w,
            p,
            prose::collapsed_into_lawlessness().to_string(),
            None,
            2,
        );
        return;
    }
    let seeds: Vec<usize> = cities[..k].iter().map(|&c| w.cities[c].cell).collect();
    let capital_cell = w.cities[capital].cell;
    let cells = w.cells_of(p);
    let mut regions: Vec<Vec<usize>> = vec![Vec::new(); k];
    for &i in &cells {
        let dc = w.terrain.dist(i, capital_cell);
        let mut best = None;
        let mut best_d = dc;
        for (s, &sc) in seeds.iter().enumerate() {
            let d = w.terrain.dist(i, sc);
            if d < best_d {
                best_d = d;
                best = Some(s);
            }
        }
        if let Some(s) = best {
            regions[s].push(i);
        }
    }
    let oldname = w.polities[p].name.clone();
    let mut successors = Vec::new();
    for (s, region) in regions.iter().enumerate() {
        if region.len() < 4 {
            continue;
        }
        let seat = seeds[s];
        let culture = w.cells[seat].culture.unwrap_or(w.polities[p].culture);
        let kind = if region.len() >= 25 {
            PolityKind::Kingdom
        } else {
            PolityKind::Chiefdom
        };
        let np = found_polity(w, culture, seat, kind, Some(p), region, None);
        successors.push(np);
    }
    let pol = &mut w.polities[p];
    pol.stability = 0.45;
    pol.decadence *= 0.3;
    pol.last_revolt = w.year;
    if pol.kind == PolityKind::Empire {
        pol.kind = PolityKind::Kingdom;
        pol.last_kind_change = w.year;
        let short = pol.short.clone();
        let culture = pol.culture;
        let cap_name = w.cities[capital].name.clone();
        let name = make_name(w, PolityKind::Kingdom, &short, culture, Some(&cap_name));
        w.polities[p].name = name;
    }
    let names: Vec<String> = successors
        .iter()
        .map(|&s| w.polities[s].name.clone())
        .collect();
    let capital_name = w.cities[capital].name.clone();
    let text = prose::shattered(w, p, &oldname, &names, &capital_name);
    let mut refs = vec![Ref::Polity(p)];
    for &s in &successors {
        refs.push(Ref::Polity(s));
    }
    w.log(3, EventKind::Politics, &refs, Some(capital_cell), text);
}

pub fn traits_of(w: &World, p: usize) -> Traits {
    w.polities[p]
        .ruler
        .map(|r| w.persons[r].traits)
        .unwrap_or(Traits {
            ambition: 0.4,
            valor: 0.4,
            wisdom: 0.4,
            piety: 0.4,
            cruelty: 0.4,
            charisma: 0.4,
        })
}
