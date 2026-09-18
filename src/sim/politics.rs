//! States: how tribes form, expand across terrain, found cities, keep or
//! lose stability, change rulers, and eventually fragment or fall.

use super::chronicle::{EventKind, Ref};
use super::prose::{self, Pick};
use super::{City, Inheritance, Polity, PolityKind, Role, Traits, World};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Naming
// ---------------------------------------------------------------------------

/// How many fresh coinings to try before qualifying a name that a living
/// realm already answers to.
const NAME_TRIES: usize = 6;

/// A realm's full name together with the short name it goes by.
///
/// The two must agree: "Consul Aro of Tessek" is nonsense if the realm is
/// "the Republic of Ilmen", so whenever the full name is built around the
/// capital's name, that becomes the short name too.
pub struct Naming {
    /// The full name: "the Republic of Ilmen".
    pub name: String,
    /// The one word the realm is known by afterwards: "Ilmen".
    pub short: String,
}

/// Coin a full name for a realm of this kind.
///
/// `capital_name` is passed only when the capital's name is free for the
/// realm to take as its own short name; otherwise the variants built
/// around a capital fall back to `short`, so no two living realms end up
/// sharing a name. Draws exactly once, as it always did.
pub fn make_name(
    w: &World,
    kind: PolityKind,
    short: &str,
    culture: usize,
    capital_name: Option<&str>,
) -> Naming {
    let lang = &w.cultures[culture].lang;
    let adj = lang.adjective(short);
    let rng = &w.rng;
    let cap = capital_name.unwrap_or(short);
    // Each variant carries the proper noun it is built around, which is the
    // short name the realm will go by.
    let variants: Vec<(String, &str)> = match kind {
        PolityKind::Tribe => vec![(format!("the {}", lang.demonym(short)), short)],
        PolityKind::Chiefdom => vec![
            (format!("the {} Chiefdom", adj), short),
            (format!("the {} Clans", adj), short),
            (format!("the Chiefdom of {}", cap), cap),
        ],
        PolityKind::Kingdom => vec![
            (format!("the Kingdom of {}", short), short),
            (format!("the {} Kingdom", adj), short),
            (format!("the Realm of {}", short), short),
            (format!("the Crown of {}", short), short),
            (format!("the Kingdom of {}", cap), cap),
        ],
        PolityKind::Empire => vec![
            (format!("the {} Empire", adj), short),
            (format!("the Empire of {}", short), short),
            (format!("the {} Dominion", adj), short),
            (format!("the {} Imperium", adj), short),
        ],
        PolityKind::Republic => vec![
            (format!("the Republic of {}", cap), cap),
            (format!("the {} League", adj), short),
            (format!("the Free Cities of {}", short), short),
            (format!("the {} Commonwealth", adj), short),
        ],
        PolityKind::Theocracy => vec![
            (format!("the Holy {} Realm", adj), short),
            (format!("the {} Theocracy", adj), short),
            (format!("the See of {}", cap), cap),
            (format!("the Blessed Land of {}", short), short),
        ],
        PolityKind::Magocracy => vec![
            (format!("the {} Conclave", adj), short),
            (format!("the Magisterium of {}", short), short),
            (format!("the {} Covenant", adj), short),
            (format!("the Towers of {}", cap), cap),
        ],
        PolityKind::Horde => vec![
            (format!("the {} Horde", adj), short),
            (format!("the Horde of {}", short), short),
            (format!("the {} Riders", adj), short),
        ],
    };
    let chosen = rng.pick(&variants);
    Naming {
        name: chosen.0.clone(),
        short: chosen.1.to_string(),
    }
}

/// Whether a living realm other than `except` already answers to `s`.
pub fn short_taken(w: &World, s: &str, except: Option<usize>) -> bool {
    let lower = s.to_lowercase();
    w.polities
        .iter()
        .any(|p| p.alive() && Some(p.id) != except && p.short.to_lowercase() == lower)
}

/// A coined short name no living realm is using. Tries fresh coinings
/// first, since a new word is better than a qualified one, and only then
/// marks the name to tell the two realms apart.
fn unique_short(w: &World, culture: usize, except: Option<usize>) -> String {
    let mut s = w.cultures[culture].lang.name(&w.rng);
    for _ in 0..NAME_TRIES {
        if !short_taken(w, &s, except) {
            return s;
        }
        s = w.cultures[culture].lang.name(&w.rng);
    }
    distinguish(w, &s, except)
}

/// `"Zhi"` when there is already a Zhi becomes `"New Zhi"`, then
/// `"Upper Zhi"`, and at the very last `"Zhi the Second"`.
fn distinguish(w: &World, base: &str, except: Option<usize>) -> String {
    const MARKS: [&str; 6] = ["New", "Upper", "Lower", "Greater", "Lesser", "Far"];
    if !short_taken(w, base, except) {
        return base.to_string();
    }
    for m in MARKS {
        let cand = format!("{} {}", m, base);
        if !short_taken(w, &cand, except) {
            return cand;
        }
    }
    for n in 2..=12 {
        let cand = format!("{} the {}", base, prose::cap(&prose::ordinal_word(n)));
        if !short_taken(w, &cand, except) {
            return cand;
        }
    }
    base.to_string()
}

/// Name a realm: a full name and a short name that agree with each other
/// and that no other living realm is already using. `short` must itself be
/// free, which is what [`unique_short`] and the realm's existing name are.
fn name_realm(
    w: &World,
    p: usize,
    kind: PolityKind,
    short: &str,
    culture: usize,
    capital_name: Option<&str>,
) -> Naming {
    // Only offer the capital's name if the realm could take it as its own.
    let cap = capital_name.filter(|c| !short_taken(w, c, Some(p)));
    make_name(w, kind, short, culture, cap)
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
    w.own_cell(cell, p);
    w.cells[cell].since = w.year;
    if w.cells[cell].culture.is_none() {
        w.cells[cell].culture = Some(culture);
        w.cells[cell].pop = w.cells[cell].pop.max(0.05);
    }
    if let Some(c) = w.cells[cell].city {
        w.cities[c].polity = Some(p);
    }
    // Discover and name features. Naming coins a word, so it happens at
    // every detail level or the three would not share a history.
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
    let short = unique_short(w, culture, None);
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
        peak_cities: 0,
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
        stance: BTreeMap::new(),
        stance_since: BTreeMap::new(),
        overlord: None,
        tributaries: Vec::new(),
        inheritance: super::dynasty::inheritance_for(w, culture, kind),
        house: None,
        peak_share: 0.0,
        hegemon_since: None,
        heirs: Vec::new(),
        sprawl: 0.0,
        tech_admin: 0.0,
    });
    // A realm founded as a kingdom is founded with a house. Building the
    // dynasty name here but not the house left the realm with a family in
    // name only until its next promotion, and its founder off the tree.
    if w.persons[ruler].crowned.is_none() {
        w.persons[ruler].crowned = Some(w.year);
    }
    if kind.has_dynasty() {
        match w.persons[ruler].house {
            Some(h) => w.seat_house(id, h),
            None => {
                let name = w.polities[id].dynasty.clone();
                let h = w.new_house(name, culture, Some(ruler));
                w.seat_house(id, h);
                super::dynasty::seed_house_kin(w, id, h, ruler);
            }
        }
        w.house_accession(ruler);
    }
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
    let naming = name_realm(w, id, kind, &short, culture, Some(&cap_name));
    let adj = w.cultures[culture].lang.adjective(&naming.short);
    let pol = &mut w.polities[id];
    pol.name = naming.name;
    pol.short = naming.short;
    pol.adj = adj;
    w.alive_polities.push(id);
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
            &[Ref::Polity(p), Ref::Culture(culture), Ref::City(capital)],
            Some(i),
            text,
        );
    }
}

/// What a point of trade adds to a city's prosperity.
/// Above this, each further advantage a realm has is worth less than the
/// last. Below it nothing is changed.
pub const STABILITY_KNEE: f32 = 0.7;

pub const TRADE_TO_PROSPERITY: f32 = 0.035;
/// The prosperity a city can approach but not reach.
pub const PROSPERITY_MAX: f32 = 2.5;
/// Below this a city's prosperity is whatever its advantages add up to;
/// above it, each further advantage is worth less than the last.
pub const PROSPERITY_KNEE: f32 = 1.5;
/// The most prosperity a city can owe to the traffic through it, approached
/// but never reached.
pub const TRADE_PROSPERITY_MAX: f32 = 0.9;
/// The amount of that traffic worth half of it.
pub const TRADE_PROSPERITY_HALF: f32 = 0.35;

// ---------------------------------------------------------------------------
// Expansion
// ---------------------------------------------------------------------------

/// What putting to sea at all adds to the price of a cell.
const SEA_CROSSING_COST: f32 = 3.0;
/// What each cell of open water adds on top of that.
const SEA_WIDTH_COST: f32 = 0.12;
/// How much less attractive a cell across water is than the same cell
/// reachable on foot, all else equal. Realms fill their own shore first.
const SEA_SCORE_PENALTY: f32 = 0.35;

pub fn expand(w: &mut World) {
    let rng = w.rng.clone();
    let tn = w.tuning;
    let n = w.cells.len();
    let np = w.polities.len();
    let mut cand: Vec<Vec<(f32, usize, f32)>> = vec![Vec::new(); np];
    let capitals: Vec<Option<usize>> = (0..np).map(|p| w.capital_cell(p)).collect();
    let alive: Vec<bool> = w.polities.iter().map(super::Polity::alive).collect();
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
    }
    // Overseas colonisation, along the map's own sea routes.
    //
    // This used to scan a fixed box of nine by five cells around every
    // coastal cell, which meant a realm could cross about four cells of
    // water and no more, whatever it knew about ships — and over fifteen
    // centuries nought to two realms in an entire world ever held land on
    // two landmasses. Now the crossing has to be one the map actually
    // affords and one the realm's reach can manage, and a mature naval
    // power can cross an ocean.
    for p in w.living_polities() {
        let reach = w.sea_reach(p);
        if reach == 0 {
            continue;
        }
        for c in w.sea_sorties(p) {
            let far = c.to as usize;
            if w.cells[far].owner.is_some() || !w.terrain.is_land(far) {
                continue;
            }
            let base_cost = match w.terrain.biome[far].move_cost() {
                Some(x) => x,
                None => continue,
            };
            // Open water is dear, and dearer the wider it is. A strait is
            // barely more than a land border; an ocean is a generation's
            // undertaking.
            let sea_cost = base_cost + SEA_CROSSING_COST + c.width as f32 * SEA_WIDTH_COST;
            if let Some((score, cost)) = score_cell(w, p, far, capitals[p], sea_cost, &rng) {
                cand[p].push((score - SEA_SCORE_PENALTY, far, cost));
            }
        }
    }
    for (p, list) in cand.iter_mut().enumerate() {
        if list.is_empty() {
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
        list.sort_by(|a, b| b.0.total_cmp(&a.0));
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
                w.credit_land(p, 1);
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
        + w.tech_reach_bonus(p)
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
    let np = w.polities.len();
    // Ground already too near a standing town, painted once instead of being
    // re-tested against every city for every candidate site.
    let mut crowded = vec![false; w.cells.len()];
    for c in w.cities.iter().filter(|c| c.destroyed.is_none()) {
        let (cx, cy) = w.terrain.xy(c.cell);
        let y0 = (cy as i32 - tn.city_spacing_y).max(0) as usize;
        let y1 = ((cy as i32 + tn.city_spacing_y) as usize).min(w.terrain.h - 1);
        let x0 = (cx as i32 - tn.city_spacing_x).max(0) as usize;
        let x1 = ((cx as i32 + tn.city_spacing_x) as usize).min(w.terrain.w - 1);
        for y in y0..=y1 {
            for x in x0..=x1 {
                crowded[y * w.terrain.w + x] = true;
            }
        }
    }
    for p in 0..np {
        if !w.polities[p].alive() || w.cells_of_ref(p).is_empty() {
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
        let cells = w.cells_of_ref(p);
        let samples = cells.len().min(80);
        for _ in 0..samples {
            let i = cells[rng.below(cells.len())];
            if w.cells[i].city.is_some() || !w.terrain.is_land(i) {
                continue;
            }
            if crowded[i] {
                continue;
            }
            let t = &w.terrain;
            // Crowded country recommends itself: a dense interior is as good
            // a place for a town as an empty coast, which is what keeps new
            // towns coming once the shorelines are full.
            let s = t.fertility[i]
                + t.river[i] as f32 * 0.3
                + if t.coast[i] { 0.5 } else { 0.0 }
                + w.cells[i].pop * tn.city_site_pop_weight
                + t.minerals[i] * 0.3;
            if s > best_s {
                best_s = s;
                best = Some(i);
            }
        }
        if let Some(i) = best {
            let culture = w.cells[i].culture.unwrap_or(w.polities[p].culture);
            let c = found_city(w, Some(p), i, culture);
            w.credit_city_founded(p);
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
    for (p, &nb_dev) in neighbor_dev.iter().enumerate() {
        if !w.polities[p].alive() {
            continue;
        }
        let (army_mult, stab_bonus, dev_bonus, prosp_bonus) = w.school_effects(p);
        let art_mult = w.artifact_army_mult(p);
        let art_stab = w.artifact_stability(p);
        let tech_army = w.tech_army_mult(p);
        let trade_income = w.realm_trade(p);
        // Coin, roads and a tax roll are the difference between owning a
        // province and being able to tax it.
        let tech_income = w.tech_income_mult(p);
        let tech_prosp = w.tech_prosperity_bonus(p);
        let tech_order = w.tech_order_bonus(p);
        let world_share = super::dynasty::world_share(w, p);
        let culture = w.polities[p].culture;
        let vals = w.cultures[culture].values;
        let at_war = w.polities[p].at_war();
        let peace_neighbors = w.polities[p]
            .neighbors
            .iter()
            .filter(|&&(q, _)| w.war_between(p, q).is_none())
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
                    + prosp_bonus
                    + tech_prosp;
            if is_capital == Some(c) {
                target += 0.1;
            }
            if at_war {
                target -= 0.15;
            }
            target += w.cities[c].wonders.len() as f32 * 0.08;
            // What passes through. This is the whole of why a city on a
            // strait is worth more than a city in a bog, and why a realm
            // astride the roads is worth attacking.
            // Saturating rather than clipped. A hard ceiling here meant
            // that every city past a modest amount of traffic drew exactly
            // the same benefit from it, so the difference between a good
            // position on the roads and a commanding one disappeared — the
            // same mistake the treasury's ceiling made, in the place where
            // it matters most, since what passes through is the whole reason
            // a city on a strait is worth more than a city in a bog.
            let passing = w.city_trade(c) * TRADE_TO_PROSPERITY;
            target += TRADE_PROSPERITY_MAX * passing / (passing + TRADE_PROSPERITY_HALF);
            let city = &mut w.cities[c];
            // Taxed on the year's opening prosperity, before this year's
            // adjustment. A treasury is filled from what was actually
            // produced, and prosperity is a slow-moving stock rather than a
            // flow — but the real reason the order matters is that `explain`
            // has to be able to state where the money came from, and it can
            // only read the world as it stands, not as this loop is about to
            // leave it. An explanation that is a percent out is worse than
            // none, because a reader checks the parts against the whole.
            income += city.pop * city.prosperity * tn.city_income_factor;
            // The target is bent towards its ceiling rather than the value
            // being clipped against it. Everything that makes a city rich —
            // development, wonders, what passes through, what its realm
            // knows — was added up and the sum then clamped, so by the
            // twenty-first century the *median* city in the world sat at
            // exactly the maximum and more than half of them had their
            // income decided by population alone. Prosperity had stopped
            // distinguishing anything, which also made the saturating trade
            // term pointless for the cities it mattered most to: their
            // target was already over the cap.
            let target = super::soft_ceiling(target, PROSPERITY_KNEE, PROSPERITY_MAX);
            city.prosperity += (target - city.prosperity) * tn.prosperity_adjust_rate;
            city.prosperity = city.prosperity.clamp(0.05, PROSPERITY_MAX);
            city.walls += (dev * 0.8 - city.walls) * 0.03;
            prosp_sum += city.prosperity;
        }
        let avg_prosp = if cities.is_empty() {
            0.3
        } else {
            prosp_sum / cities.len() as f32
        };
        let pol = &mut w.polities[p];
        income += pol.cells as f32 * tn.cell_income_factor * (1.0 + pol.dev);
        income += trade_income * tn.trade_toll_factor;
        income *= tech_income;
        // What never reaches the treasury. A rotten court and a long road to
        // the capital both take their cut of the tax roll before the crown
        // sees it, which is what gives economic decline a cause of its own
        // rather than leaving it a consequence of losing wars.
        let skim = super::corruption_share(&tn, pol.decadence, pol.sprawl);
        income *= 1.0 - skim;
        let upkeep = pol.army * tn.army_upkeep_factor + pol.cells as f32 * tn.cell_upkeep_factor;
        // And what the court spends, which is the drain that makes a ceiling
        // unnecessary: everything above the war chest is fair game, so the
        // richer a crown is the more it burns, and the burning rots it.
        let spend = super::court_spending(&tn, pol.treasury);
        pol.treasury = (pol.treasury + income - upkeep - spend).max(super::TREASURY_FLOOR);
        if spend > 0.0 {
            // Gold spent on a court is not wasted — it is how a dynasty is
            // remembered — but it is how a dynasty rots, too.
            let scale = spend / tn.court_reserve.max(1.0);
            pol.prestige += scale * tn.court_prestige_rate;
            pol.decadence = (pol.decadence + scale * tn.court_decadence_rate).clamp(0.0, 1.0);
        }
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
            * art_mult
            * tech_army
            // Gold buys soldiers, and the soldiers then cost upkeep for as
            // long as they stand: the third drain, and the one that turns a
            // hoard into something the rest of the world has to reckon with.
            * (1.0 + super::wealth_reach(&tn, pol.treasury) * tn.army_gold_weight);
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
        if nb_dev > pol.dev {
            ddev += (nb_dev - pol.dev) * 0.004 * (0.5 + vals.openness);
        }
        // Past the soft cap a realm still improves, only slowly, so the
        // late centuries mean denser land rather than a world that stops.
        if pol.dev > tn.dev_soft_cap && ddev > 0.0 {
            ddev *= tn.dev_overflow_rate;
        }
        pol.dev = (pol.dev + ddev).clamp(0.0, tn.dev_hard_cap);
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
            - (pol.sprawl - 1.0).max(0.0) * tn.stability_sprawl_weight
            - (world_share - tn.hegemony_free_share).max(0.0) * tn.hegemony_weight
            - pol.foreign_share * tn.stability_foreign_weight
            - pol.exhaustion * tn.stability_exhaustion_weight
            - pol.decadence * tn.stability_decadence_weight
            + art_stab
            + tech_order
            + (avg_prosp - 0.4) * 0.15
            + vals.tradition * 0.05
            + super::treasury_confidence(&tn, pol.treasury) * 0.2;
        // Bent towards the ceiling rather than clipped against it, the
        // fifth and last place this simulation summed every advantage a
        // thing had and then clamped the sum. A settled, rich, well-ruled
        // kingdom at peace clears 1.0 on the thirteen terms above without
        // difficulty, so the top tenth of realms all sat at exactly 1.0 and
        // read identically: the same drift word, the same "pulls towards
        // 100%", for a realm at 1.02 and one at 1.6.
        let target = super::soft_ceiling(target, STABILITY_KNEE, 1.0);
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

pub fn ruler_dies(w: &mut World, p: usize, cause: &str, importance: u8) {
    let r = match w.polities[p].ruler {
        Some(r) => r,
        None => return,
    };
    let epithet = epithet_for(w, p, r);
    w.persons[r].epithet = epithet;
    w.persons[r].died = Some(w.year);
    w.persons[r].death = cause.to_string();
    let length = w.year - w.polities[p].reign_start;
    let mut text = prose::ruler_died(w, p, r, cause, length);
    // The draw happens whatever the detail: only whether the extra line is
    // written depends on it (see `Detail`).
    let flourish = prose::ruler_death_flourish(w, p, r, length, &Pick::rolled(&w.rng.clone()));
    if w.high_detail() {
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

/// Hand a throne on after its holder has died.
///
/// The old version of this conjured an heir at the graveside: a brand-new
/// `Person` with no history, whom the reader had never met. Now it draws on
/// the realm's actual [`Polity::heirs`] — children who were born, named and
/// grown up in the chronicle — and the realm's own
/// [`Polity::inheritance`] custom decides what happens to them:
///
/// * **Primogeniture** — the eldest takes the whole, and a younger sibling
///   may refuse.
/// * **Partition** — every adult child takes a share, which is how a great
///   reign is undone by its own law rather than by an enemy.
/// * **Tanistry** — the most capable kinsman takes it, and the passed-over
///   do not always accept the judgement.
/// * **Elective** — blood counts for nothing and somebody is chosen.
///
/// Only if there is nobody at all does it fall back to inventing a usurper,
/// which is now the exception rather than the rule.
fn succession(w: &mut World, p: usize, old: usize) {
    let rng = w.rng.clone();
    let custom = w.polities[p].inheritance;
    let heirs = super::dynasty::adult_heirs(w, p);
    let minors: Vec<usize> = w.polities[p]
        .heirs
        .iter()
        .copied()
        .filter(|&h| w.persons[h].alive() && w.persons[h].age(w.year) < super::dynasty::MAJORITY)
        .collect();

    // The Diadochi. A conqueror the world acclaimed, no grown child to hold
    // what they took, and generals who each command a share of the army:
    // the realm does not pass, it is divided among the men who won it.
    if w.persons[old].is_acclaimed() && heirs.is_empty() {
        let generals: Vec<usize> = w.polities[p]
            .generals
            .iter()
            .copied()
            .filter(|&g| w.persons[g].alive())
            .collect();
        if generals.len() >= w.tuning.diadochi_min_generals
            && w.polities[p].cells >= w.tuning.diadochi_min_cells
        {
            diadochi(w, p, old, &generals);
            return;
        }
    }

    if !custom.has_heirs() {
        let heir = elect(w, p, old);
        let text = prose::elected(w, p, w.polities[p].kind, heir, old);
        w.log(
            1,
            EventKind::Politics,
            &[Ref::Person(heir), Ref::Polity(p)],
            w.capital_cell(p),
            text,
        );
        return;
    }

    // No child of the body. Before the throne goes to a stranger it goes
    // sideways: a brother, a nephew, an uncle, a cousin. This is how a
    // house survives a ruler who dies young, and without it almost every
    // succession in the world was a usurpation.
    if heirs.is_empty() {
        let kin = super::dynasty::kin_heirs(w, p, old);
        // Under tanistry the crown is the house's, not the dead man's, so a
        // grown kinsman is preferred to an infant son. Everywhere else the
        // child inherits and somebody rules in their name.
        let child_first = custom != Inheritance::Tanistry;
        if child_first {
            if let Some(&child) = minors.first() {
                // The uncle who does not wait. A very young heir, a grown
                // kinsman with ambition and a throne that is not sitting
                // steady: the regency that should have followed never
                // happens, and everybody knows why.
                let infant = w.persons[child].age(w.year) <= 9;
                let grasping = kin.first().map(|&(m, _)| {
                    w.persons[m].traits.ambition * 0.7 + (0.6 - w.polities[p].stability).max(0.0)
                        > 0.55
                });
                if !(infant && grasping == Some(true) && rng.chance(0.6)) {
                    regency(w, p, child, kin.first().map(|&(m, _)| m));
                    return;
                }
                let (usurper, tie) = kin[0];
                install_ruler(w, p, usurper);
                w.polities[p].stability = (w.polities[p].stability - 0.18).max(0.0);
                let house = w.polities[p].dynasty.clone();
                let text = prose::uncle_takes_throne(
                    w,
                    p,
                    usurper,
                    child,
                    tie,
                    &house,
                    &Pick::rolled(&rng),
                );
                w.log(
                    3,
                    EventKind::Politics,
                    &[Ref::Person(usurper), Ref::Person(child), Ref::Polity(p)],
                    w.capital_cell(p),
                    text,
                );
                personal_union(w, p, usurper);
                return;
            }
        }
        if let Some(&(kinsman, tie)) = kin.first() {
            install_ruler(w, p, kinsman);
            // A crown that moves sideways is never quite as firmly held as
            // one that moves down, and the further the blood the worse it
            // sits.
            let doubt = 0.04 * (tie.steps() as f32 - 1.0);
            w.polities[p].stability = (w.polities[p].stability - doubt).max(0.0);
            let house = w.polities[p].dynasty.clone();
            let text = prose::kin_succeeds(w, p, kinsman, old, tie, &house, &Pick::rolled(&rng));
            w.log(
                1,
                EventKind::Politics,
                &[Ref::Person(kinsman), Ref::Polity(p)],
                w.capital_cell(p),
                text,
            );
            personal_union(w, p, kinsman);
            return;
        }
        if let Some(&child) = minors.first() {
            regency(w, p, child, None);
            return;
        }
        usurpation(w, p, old);
        return;
    }

    // Who gets the seat.
    let chosen = match custom {
        Inheritance::Tanistry => *heirs
            .iter()
            .max_by(|&&a, &&b| {
                let key = |x: usize| {
                    let t = w.persons[x].traits;
                    t.ambition * 0.5 + t.valor * 0.3 + t.charisma * 0.2
                };
                key(a).total_cmp(&key(b)).then(b.cmp(&a))
            })
            .unwrap(),
        _ => heirs[0],
    };
    let rest: Vec<usize> = heirs.iter().copied().filter(|&h| h != chosen).collect();

    // Partition: the realm is divided, and a conqueror's work with it.
    if custom == Inheritance::Partition && !rest.is_empty() && w.polities[p].cells >= 30 {
        partition(w, p, old, chosen, &rest);
        return;
    }

    install_ruler(w, p, chosen);
    let house = w.polities[p].dynasty.clone();
    let text = prose::heir_succeeds(w, p, chosen, old, &house);
    w.log(
        1,
        EventKind::Politics,
        &[Ref::Person(chosen), Ref::Polity(p)],
        w.capital_cell(p),
        text,
    );

    // A passed-over sibling with ambition and a weak throne takes the
    // provinces with them. This is the succession war, but now it is fought
    // between two people with names, ages and traits the reader has seen.
    if let Some(&loser) = rest.first() {
        let t = w.persons[loser].traits;
        let anger = t.ambition * 0.6 + (0.6 - w.polities[p].stability).max(0.0);
        let bar = if custom == Inheritance::Tanistry {
            0.35
        } else {
            0.5
        };
        if anger > bar && rng.chance(0.55) {
            w.persons[loser].role = Role::Rebel;
            if let Some(rebel) = split_off(w, p, Some(loser), None) {
                let claim = prose::succession_claim(w, p, &w.persons[loser].name.clone());
                w.wars_start_aim(
                    rebel,
                    p,
                    super::WarKind::Succession,
                    super::WarAim::Claimant(loser),
                    claim,
                );
                let text = prose::sibling_war(w, p, rebel, chosen, loser);
                w.log(
                    3,
                    EventKind::Politics,
                    &[
                        Ref::Polity(p),
                        Ref::Polity(rebel),
                        Ref::Person(chosen),
                        Ref::Person(loser),
                    ],
                    w.capital_cell(p),
                    text,
                );
            }
        }
    }

    // A crown can arrive by marriage as easily as by war: if the new
    // ruler's spouse is the nearest heir of a realm with none of its own,
    // the two come under one head.
    personal_union(w, p, chosen);
}

/// A realm with no dynasty chooses somebody.
fn elect(w: &mut World, p: usize, old: usize) -> usize {
    let rng = w.rng.clone();
    let culture = w.polities[p].culture;
    // Prefer somebody already in the realm's service: a general or a
    // grown child of the last holder. An election the reader can follow
    // beats a stranger with a new name.
    let mut pool: Vec<usize> = w.polities[p]
        .generals
        .iter()
        .copied()
        .filter(|&g| w.persons[g].alive() && w.persons[g].age(w.year) >= 25)
        .collect();
    pool.extend(super::dynasty::adult_heirs(w, p));
    pool.retain(|&x| x != old);
    pool.sort_unstable();
    pool.dedup();
    let heir = match pool.first() {
        Some(_) => {
            let best = *pool
                .iter()
                .max_by(|&&a, &&b| {
                    let key = |x: usize| {
                        let t = w.persons[x].traits;
                        t.charisma * 0.5 + t.wisdom * 0.4 + w.persons[x].renown * 0.02
                    };
                    key(a).total_cmp(&key(b)).then(b.cmp(&a))
                })
                .unwrap();
            best
        }
        None => w.new_person(
            culture,
            Role::Ruler,
            Some(p),
            w.year - rng.int(25, 50),
            None,
        ),
    };
    install_ruler(w, p, heir);
    heir
}

/// Nobody had a claim: somebody takes it anyway.
/// A child takes the throne and somebody else does the governing.
///
/// The regent is named when the house has a grown kinsman to supply one,
/// because "the great houses ruled in his name" is a much smaller sentence
/// than "his uncle did, and everyone knew what that meant".
fn regency(w: &mut World, p: usize, child: usize, regent: Option<usize>) {
    install_ruler(w, p, child);
    w.polities[p].stability = (w.polities[p].stability - 0.25).max(0.0);
    let text = match regent {
        Some(g) => prose::succession_regent(w, p, child, g, &Pick::rolled(&w.rng.clone())),
        None => prose::succession_regency(w, p, child),
    };
    let mut refs = vec![Ref::Person(child), Ref::Polity(p)];
    if let Some(g) = regent {
        refs.push(Ref::Person(g));
    }
    w.log(2, EventKind::Politics, &refs, w.capital_cell(p), text);
}

fn usurpation(w: &mut World, p: usize, old: usize) {
    let rng = w.rng.clone();
    let culture = w.polities[p].culture;
    let dynasty = w.polities[p].dynasty.clone();
    // A living general who served the dead ruler is a far better usurper
    // than a stranger, because the reader already knows what they did.
    let general = w.polities[p]
        .generals
        .iter()
        .copied()
        .find(|&g| w.persons[g].alive() && w.persons[g].age(w.year) >= 20);
    let usurper = match general {
        Some(g) => g,
        None => w.new_person(
            culture,
            Role::Ruler,
            Some(p),
            w.year - rng.int(25, 50),
            None,
        ),
    };
    w.persons[usurper].traits.ambition = (w.persons[usurper].traits.ambition + 0.3).min(1.0);
    w.persons[usurper].served = Some(old);
    let dyn_name = dynasty_name(w, culture, &w.persons[usurper].name.clone());
    let old_house = w.polities[p].house;
    let house = w.new_house(dyn_name.clone(), culture, Some(usurper));
    w.seat_house(p, house);
    super::dynasty::seed_house_kin(w, p, house, usurper);
    install_ruler(w, p, usurper);
    if let Some(oh) = old_house {
        w.close_house_if_spent(oh);
    }
    w.polities[p].stability = (w.polities[p].stability - 0.2).max(0.0);
    let text = prose::succession_usurped(w, p, usurper, &dyn_name, &dynasty);
    w.log(
        2,
        EventKind::Politics,
        &[Ref::Person(usurper), Ref::Polity(p)],
        w.capital_cell(p),
        text,
    );
}

/// Divide a realm between the heirs, by its own law.
fn partition(w: &mut World, p: usize, old: usize, eldest: usize, rest: &[usize]) {
    install_ruler(w, p, eldest);
    let mut shares: Vec<(usize, String)> = Vec::new();
    for &h in rest.iter().take(3) {
        w.persons[h].role = Role::Ruler;
        if let Some(np) = split_off(w, p, Some(h), None) {
            // A partition is a settlement, not a revolt: the new realms
            // start out at peace with the senior line and inherit its law.
            w.polities[np].inheritance = w.polities[p].inheritance;
            if let Some(house) = w.polities[p].house {
                w.seat_house(np, house);
            }
            let until = w.year + 20;
            w.polities[np].truce.insert(p, until);
            w.polities[p].truce.insert(np, until);
            super::dynasty::set_stance(w, p, np, super::Stance::Married);
            let name = w.polities[np].name.clone();
            shares.push((h, name));
        }
    }
    if shares.is_empty() {
        let house = w.polities[p].dynasty.clone();
        let text = prose::heir_succeeds(w, p, eldest, old, &house);
        w.log(
            1,
            EventKind::Politics,
            &[Ref::Person(eldest), Ref::Polity(p)],
            w.capital_cell(p),
            text,
        );
        return;
    }
    let text = prose::partitioned(w, p, old, eldest, &shares);
    let mut refs = vec![Ref::Polity(p), Ref::Person(old), Ref::Person(eldest)];
    for (h, _) in &shares {
        refs.push(Ref::Person(*h));
    }
    w.log(3, EventKind::Politics, &refs, w.capital_cell(p), text);
}

/// The generals of a dead conqueror divide what he took between them.
fn diadochi(w: &mut World, p: usize, old: usize, generals: &[usize]) {
    let mut successors = Vec::new();
    let mut names = Vec::new();
    for &g in generals.iter().take(4) {
        w.persons[g].role = Role::Ruler;
        w.persons[g].served = Some(old);
        if let Some(np) = split_off(w, p, Some(g), None) {
            w.polities[np].inheritance = w.polities[p].inheritance;
            successors.push(np);
            names.push(w.polities[np].name.clone());
        }
    }
    if successors.is_empty() {
        usurpation(w, p, old);
        return;
    }
    // Whoever is left holding the old capital is just another successor
    // now, so the senior line gets a general too.
    let remaining: Vec<usize> = generals
        .iter()
        .copied()
        .filter(|&g| w.persons[g].alive())
        .collect();
    if let Some(&keeper) = remaining
        .iter()
        .find(|&&g| !successors.iter().any(|&s| w.polities[s].ruler == Some(g)))
    {
        install_ruler(w, p, keeper);
        w.persons[keeper].served = Some(old);
    } else {
        usurpation(w, p, old);
    }
    w.polities[p].stability = (w.polities[p].stability - 0.25).max(0.0);
    let text = prose::generals_divide(w, p, old, &names, &successors);
    let mut refs = vec![Ref::Polity(p), Ref::Person(old)];
    for &s in &successors {
        refs.push(Ref::Polity(s));
    }
    w.log(3, EventKind::Politics, &refs, w.capital_cell(p), text);
}

/// A crown that arrives by marriage rather than by war.
///
/// If the new ruler's consort is the nearest surviving heir of a realm that
/// has run out of its own, the two realms come under one head. This is the
/// quietest way a great power is ever assembled, and the chronicle says so.
fn personal_union(w: &mut World, p: usize, ruler: usize) {
    let spouse = match w.persons[ruler].spouse {
        Some(s) if w.persons[s].alive() => s,
        _ => return,
    };
    let q = match w.persons[spouse].polity {
        Some(q) if q != p && w.polities[q].alive() => q,
        _ => return,
    };
    if w.polities[q].ruler.is_some() || !super::dynasty::adult_heirs(w, q).is_empty() {
        return;
    }
    if w.war_between(p, q).is_some() || w.polities[q].cells == 0 {
        return;
    }
    // The junior crown is absorbed: one realm, one ruler, and a chronicle
    // line for the realm that ended without a battle.
    let cause = prose::united_by_marriage(w, p, q);
    let text = prose::personal_union(w, ruler, p, q);
    let refs = vec![Ref::Person(ruler), Ref::Polity(p), Ref::Polity(q)];
    let loc = w.capital_cell(q);
    fall(w, q, &cause, Some(p), 3);
    w.log(3, EventKind::Politics, &refs, loc, text);
}

pub fn install_ruler(w: &mut World, p: usize, r: usize) {
    // Whether the throne changes families, which has to be read before the
    // accession writes over it.
    let swept = match w.polities[p].rulers.last().copied() {
        Some(q) => {
            let old = w.persons[q].house;
            old.is_some() && old != w.persons[r].house
        }
        None => false,
    };
    w.persons[r].polity = Some(p);
    w.persons[r].role = Role::Ruler;
    // A house is joined by birth, by marriage, or by founding it — never
    // by being handed a crown. Enrolling every new ruler in whatever house
    // the realm last had turned an elective chiefdom into a four-hundred
    // year "dynasty" of unrelated strangers, and made every line of
    // succession read "kinsman".
    //
    // What an accession *can* do is seat a house that already exists: a
    // ruler who belongs to one brings it to the throne with them.
    if let Some(h) = w.persons[r].house {
        if w.polities[p].kind.has_dynasty() && w.polities[p].house != Some(h) {
            w.seat_house(p, h);
        }
    }
    if w.persons[r].crowned.is_none() {
        w.persons[r].crowned = Some(w.year);
    }
    // Read before the accession is written down, since it asks who has held
    // this throne up to now.
    let returned = super::dynasty::restoration(w, p, r);
    w.house_accession(r);
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
    // A new house on an old throne is the one thing that clears a rotted
    // court, and this is the whole of the dynastic cycle.
    //
    // Rot used to be permanent. Decadence climbed to its ceiling in a
    // century and a half and stayed there, so every realm older than that
    // carried the same crushing stability penalty for the rest of its life
    // — which is why the top realm in the world held seventeen percent of
    // it in the third century and never more than four percent again. The
    // world was not finding an equilibrium; it was accumulating an
    // unpayable debt. A dynasty that ends is a court that is swept, the
    // creditors are killed, and the new house gets the century its
    // predecessor wasted.
    if swept {
        let pol = &mut w.polities[p];
        pol.decadence *= 0.3;
        pol.exhaustion *= 0.7;
    }
    // A line come back to a throne it lost. The oldest story there is, and
    // until now this world had no way of telling it.
    if let Some((last, gap)) = returned {
        w.polities[p].prestige += 20.0;
        w.polities[p].stability = (w.polities[p].stability + 0.1).min(1.0);
        let house = w.polities[p].dynasty.clone();
        let text = prose::house_restored(w, p, r, last, gap, &house, &Pick::rolled(&w.rng.clone()));
        w.log(
            3,
            EventKind::Politics,
            &[Ref::Person(r), Ref::Polity(p), Ref::Person(last)],
            w.capital_cell(p),
            text,
        );
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
        let lifespan = (w.races[per.race].lifespan * per.vigour).max(1.0);
        let rel = age / lifespan;
        let p_nat = tn.ruler_death_base + tn.ruler_death_age_weight * rel.powi(6);
        let stability = w.polities[p].stability;
        let p_assassin = tn.ruler_assassination_base
            * (1.0 + per.traits.cruelty * tn.ruler_assassination_cruelty_weight)
            + (0.5 - stability).max(0.0) * tn.ruler_assassination_unrest_weight;
        let roll = rng.f64();
        if roll < p_nat as f64 {
            let cx = prose::DeathContext {
                at_war: w.polities[p].at_war(),
                plague: w
                    .plagues
                    .iter()
                    .any(|pl| pl.years_left > 0 && pl.polities.contains(&p)),
                reign_years: w.persons[r].reign_years,
                cruel: w.persons[r].traits.cruelty > 0.65,
            };
            let cause = prose::natural_death_in(age as i32, cx, &Pick::rolled(&rng));
            ruler_dies(w, p, &cause, 1);
        } else if roll < (p_nat + p_assassin) as f64 {
            let by = prose::assassin(w, p, &Pick::rolled(&rng));
            w.polities[p].stability = (w.polities[p].stability - 0.15).max(0.0);
            ruler_dies(w, p, &prose::murdered_by(&by), 2);
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
    let cells = w.cells_of_ref(p);
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
    cause: &str,
    absorbed_by: Option<usize>,
    importance_floor: u8,
) {
    if !w.polities[p].alive() {
        return;
    }
    w.century_polities_fell += 1;
    super::stories::artifacts_on_fall(w, p, absorbed_by);
    let cells = w.transfer_cells(p, absorbed_by);
    for &i in &cells {
        w.cells[i].since = w.year;
    }
    let cities = w.polities[p].cities.clone();
    for c in cities {
        w.cities[c].polity = absorbed_by;
    }
    let wars = w.polities[p].wars.clone();
    for wid in wars {
        let result = prose::war_lapsed(w, p);
        w.end_war(wid, &result, false);
    }
    if let Some(r) = w.polities[p].ruler {
        if w.persons[r].alive() {
            w.persons[r].role = Role::Ruler;
        }
    }
    let pol = &mut w.polities[p];
    pol.fell = Some(w.year);
    pol.fall_cause = cause.to_string();
    w.alive_polities.retain(|&x| x != p);
    let peak = pol.peak_cells;
    let imp = if peak > 150 {
        3
    } else if peak > 40 {
        2
    } else {
        1
    }
    .max(importance_floor);
    // The epitaph pairs the peak land with the peak number of cities: a
    // realm that lost everything before it died still ruled what it ruled.
    let cities_built = w
        .cities
        .iter()
        .filter(|c| c.founder == Some(p))
        .count()
        .max(w.polities[p].peak_cities);
    let text = prose::realm_fell(w, p, cause, cities_built);
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
            let cause = prose::faded_away(w, p, !pol.wars.is_empty());
            fall(w, p, &cause, None, 1);
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
            // The share of the world a realm must hold caps the bar rather
            // than raising it: `empire_min_cells` is the ambition, and on a
            // map too small or too crowded for it the bar comes down to
            // whoever is genuinely dominant. Without the cap the constant
            // was dead by mid-game and nothing ever became an empire.
            let share = w.stats.owned_cells / w.tuning.empire_share_divisor.max(1) as usize;
            let floor = (w.tuning.empire_min_cells / 3).max(1) as usize;
            let empire_cells = (w.tuning.empire_min_cells as usize).min(share.max(floor));
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
    apply_kind_change(w, p, new);
}

/// Crown a realm an empire outright, for the one path to the purple that
/// does not run through conquest (see `dynasty::coronations`).
pub fn promote_to_empire(w: &mut World, p: usize) {
    if w.polities[p].kind != PolityKind::Empire {
        apply_kind_change(w, p, PolityKind::Empire);
    }
}

/// Rename and restyle a realm that has changed what kind of thing it is.
fn apply_kind_change(w: &mut World, p: usize, new: PolityKind) {
    let old = w.polities[p].kind;
    let culture = w.polities[p].culture;
    let short = w.polities[p].short.clone();
    let cap_name = w.polities[p].capital.map(|c| w.cities[c].name.clone());
    let naming = name_realm(w, p, new, &short, culture, cap_name.as_deref());
    let name = naming.name.clone();
    let new_short = naming.short;
    let new_adj = w.cultures[culture].lang.adjective(&new_short);
    let oldname = w.polities[p].name.clone();
    let ruler = w.ruler_short(p);
    let pol = &mut w.polities[p];
    pol.kind = new;
    pol.name = name.clone();
    // A name built around the capital renames the realm itself, so that
    // "Consul Aro of Ilmen" and "the Republic of Ilmen" are the same place.
    pol.short = new_short;
    pol.adj = new_adj;
    pol.last_kind_change = w.year;
    if new.has_dynasty() && pol.house.is_none() {
        if let Some(r) = pol.ruler {
            let founder = w.persons[r].name.clone();
            let d = dynasty_name(w, culture, &founder);
            let house = w.new_house(d, culture, Some(r));
            w.seat_house(p, house);
            super::dynasty::seed_house_kin(w, p, house, r);
            w.house_accession(r);
        }
    }
    let capital = cap_name.unwrap_or_else(|| short.clone());
    let (imp, text) = prose::rank_changed(
        w,
        p,
        &prose::RankChange {
            old,
            new,
            old_name: &oldname,
            new_name: &name,
            ruler: &ruler,
            capital: &capital,
        },
    );
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
    cities.sort_by(|&a, &b| w.cities[b].pop.total_cmp(&w.cities[a].pop));
    let k = (2 + rng.below(3)).min(cities.len());
    if k < 1 {
        // No cities to seed successors: the state simply collapses.
        fall(w, p, &prose::collapsed_into_lawlessness(w, p), None, 2);
        return;
    }
    let seeds: Vec<usize> = cities[..k].iter().map(|&c| w.cities[c].cell).collect();
    let capital_cell = w.cities[capital].cell;
    let cells = w.cells_of_ref(p);
    let mut regions: Vec<Vec<usize>> = vec![Vec::new(); k];
    for &i in cells {
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
    // Somebody crowns themselves in each fragment, and the chronicle would
    // rather name them. Living generals first, then grown heirs of the
    // house, because "the Kingdom of Xatath under Shass, who had commanded
    // the northern army" is a sentence and "a kingdom appeared" is not.
    let mut claimants: Vec<usize> = w.polities[p]
        .generals
        .iter()
        .copied()
        .filter(|&g| w.persons[g].alive() && w.persons[g].age(w.year) >= 18)
        .collect();
    claimants.extend(
        super::dynasty::adult_heirs(w, p)
            .into_iter()
            .filter(|&h| Some(h) != w.polities[p].ruler),
    );
    claimants.sort_unstable();
    claimants.dedup();
    let mut next_claimant = claimants.into_iter();
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
        let leader = next_claimant.next().map(|l| {
            w.persons[l].role = Role::Ruler;
            l
        });
        let np = found_polity(w, culture, seat, kind, Some(p), region, leader);
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
        let naming = name_realm(w, p, PolityKind::Kingdom, &short, culture, Some(&cap_name));
        let adj = w.cultures[culture].lang.adjective(&naming.short);
        let pol = &mut w.polities[p];
        pol.name = naming.name;
        pol.short = naming.short;
        pol.adj = adj;
    }
    let capital_name = w.cities[capital].name.clone();
    let text = prose::shattered(w, p, &oldname, &successors, &capital_name);
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
