//! Population, migration and culture: how peoples spread over the land,
//! are absorbed by the states that rule them, and drift apart into new
//! peoples when geography keeps them separate for long enough.

use super::chronicle::{EventKind, Ref};
use super::{PolityKind, World};
use crate::geo::Biome;

impl World {
    /// How well a race fares in a biome, 0..1.
    pub fn affinity(&self, race: usize, i: usize) -> f32 {
        let b = self.terrain.biome[i];
        if b.is_water() {
            return 0.0;
        }
        let r = &self.races[race];
        let base = b.group().map(|g| r.affinity[g as usize]).unwrap_or(0.4);
        let coast = if self.terrain.coast[i] {
            r.coast_love * 0.3
        } else {
            0.0
        };
        (base + coast).min(1.2)
    }

    pub fn cell_capacity(&self, i: usize) -> f32 {
        let cs = &self.cells[i];
        let race = match cs.culture {
            Some(c) => self.cultures[c].race,
            None => return 0.0,
        };
        let dev = cs.owner.map(|p| self.polities[p].dev).unwrap_or(0.0);
        let aff = self.affinity(race, i);
        let mut cap = self.terrain.fertility[i] * 5.0 * (0.15 + aff) * (1.0 + dev * 0.7);
        if cs.city.is_some() {
            cap *= 1.5;
        }
        if cs.plague > 0 {
            cap *= 0.6;
        }
        cap
    }
}

pub fn grow_and_migrate(w: &mut World) {
    let n = w.cells.len();
    let rng = w.rng.clone();
    let mut delta = vec![0f32; n];
    let mut new_culture: Vec<(usize, usize)> = Vec::new();
    for i in 0..n {
        let cs = w.cells[i];
        let culture = match cs.culture {
            Some(c) => c,
            None => continue,
        };
        if cs.pop <= 0.0 {
            continue;
        }
        let race = w.cultures[culture].race;
        let cap = w.cell_capacity(i);
        let fecund = w.races[race].fecund;
        let r = 0.04 * (0.6 + fecund * 0.8);
        let growth = if cap <= 0.01 {
            -cs.pop * 0.08
        } else if cs.pop > cap {
            -(cs.pop - cap) * 0.15
        } else {
            cs.pop * r * (1.0 - cs.pop / cap)
        };
        delta[i] += growth;
        // Migration pressure.
        let pressure = if cap > 0.01 { cs.pop / cap } else { 2.0 };
        if pressure > 0.55 && cs.pop > 0.25 {
            let nbs: Vec<usize> = w.terrain.neighbors8(i).collect();
            let nb = nbs[rng.below(nbs.len())];
            let b = w.terrain.biome[nb];
            if b.is_water() {
                continue;
            }
            if b.move_cost().is_none() {
                continue;
            }
            let aff = w.affinity(race, nb);
            if aff < 0.12 {
                continue;
            }
            let target = &w.cells[nb];
            let tcap = if target.culture.is_some() {
                w.cell_capacity(nb)
            } else {
                w.terrain.fertility[nb] * 5.0 * (0.15 + aff)
            };
            if tcap < 0.15 {
                continue;
            }
            let tfill = target.pop / tcap;
            if tfill < pressure * 0.7 {
                let amount = cs.pop * 0.08 * (pressure - tfill).min(1.0);
                if amount > 0.01 {
                    delta[i] -= amount;
                    delta[nb] += amount;
                    if target.culture.is_none() || (target.pop < 0.05 && target.owner.is_none()) {
                        new_culture.push((nb, culture));
                    }
                }
            }
        }
    }
    for (i, c) in new_culture {
        if w.cells[i].culture.is_none() || w.cells[i].pop < 0.05 {
            w.cells[i].culture = Some(c);
        }
    }
    for i in 0..n {
        let cs = &mut w.cells[i];
        cs.pop = (cs.pop + delta[i]).max(0.0);
        if cs.pop < 0.01 && cs.owner.is_none() && cs.city.is_none() {
            cs.pop = 0.0;
        }
    }
    // Cities grow on their own account.
    for c in 0..w.cities.len() {
        if w.cities[c].destroyed.is_some() {
            continue;
        }
        let cell = w.cities[c].cell;
        let polity = w.cities[c].polity;
        let dev = polity.map(|p| w.polities[p].dev).unwrap_or(0.0);
        let stab = polity.map(|p| w.polities[p].stability).unwrap_or(0.5);
        let mut local = 0.0;
        for nb in w.terrain.neighbors8(cell) {
            local += w.terrain.fertility[nb];
        }
        local += w.terrain.fertility[cell] * 2.0;
        let city = &mut w.cities[c];
        let mut cap =
            local * 1.6 * (1.0 + dev * 1.2) * (0.6 + city.prosperity) * (0.7 + stab * 0.5);
        if w.terrain.coast[cell] {
            cap *= 1.25;
        }
        if w.terrain.river[cell] >= 2 {
            cap *= 1.2;
        }
        if w.cells[cell].plague > 0 {
            cap *= 0.5;
        }
        let r = 0.03;
        if city.pop < cap {
            city.pop += city.pop * r * (1.0 - city.pop / cap) + 0.02;
        } else {
            city.pop -= (city.pop - cap) * 0.1;
        }
        city.pop = city.pop.max(0.1);
        if city.pop > city.peak_pop {
            city.peak_pop = city.pop;
        }
    }
}

pub fn culture_drift(w: &mut World) {
    let rng = w.rng.clone();
    let n = w.cells.len();
    // Assimilation toward the ruling polity's culture.
    for i in 0..n {
        let cs = w.cells[i];
        let (p, c) = match (cs.owner, cs.culture) {
            (Some(p), Some(c)) => (p, c),
            _ => continue,
        };
        let pc = w.polities[p].culture;
        if pc == c {
            continue;
        }
        let tenure = (w.year - cs.since).max(0) as f32;
        let same_race = w.cultures[pc].race == w.cultures[c].race;
        let kind_mult = match w.polities[p].kind {
            PolityKind::Empire => 1.3,
            PolityKind::Tribe | PolityKind::Horde => 0.5,
            _ => 1.0,
        };
        let tenure_mult = (0.3 + tenure / 60.0).min(1.6);
        let mut p_assim = 0.004 * tenure_mult * kind_mult * if same_race { 1.6 } else { 0.5 };
        if cs.city.is_some() {
            p_assim *= 1.8;
        }
        p_assim *= 0.6 + w.cultures[pc].values.openness * 0.8;
        if rng.chance(p_assim as f64) {
            w.cells[i].culture = Some(pc);
            if let Some(city) = cs.city {
                if w.cities[city].destroyed.is_none() {
                    let cname = w.cities[city].name.clone();
                    let pl = w.cultures[pc].plural.clone();
                    let old = w.cultures[c].name.clone();
                    w.cities[city].culture = pc;
                    w.log(
                        0,
                        EventKind::Culture,
                        &[Ref::City(city), Ref::Culture(pc), Ref::Culture(c)],
                        Some(i),
                        format!("The people of {} now count themselves among the {}; the {} tongue is heard there no more.", cname, pl, old),
                    );
                }
            }
        }
    }

    // Polities whose territory has become mostly foreign shift their own culture.
    for p in w.living_polities() {
        let pol = &w.polities[p];
        if pol.foreign_share > 0.7 && pol.cells > 20 && rng.chance(0.03) {
            // Find the dominant culture in the territory.
            let mut counts: std::collections::BTreeMap<usize, u32> =
                std::collections::BTreeMap::new();
            for i in 0..n {
                if w.cells[i].owner == Some(p) {
                    if let Some(c) = w.cells[i].culture {
                        *counts.entry(c).or_insert(0) += 1;
                    }
                }
            }
            if let Some((&c, _)) = counts.iter().max_by_key(|(_, &v)| v) {
                if c != pol.culture {
                    let old = pol.culture;
                    let text = format!(
                        "The rulers of {} had long spoken {} at court, but their subjects were {}; the realm now counts itself {}.",
                        pol.name, w.cultures[old].name, w.cultures[c].plural, w.cultures[c].adj
                    );
                    w.polities[p].culture = c;
                    w.log(
                        1,
                        EventKind::Culture,
                        &[Ref::Polity(p), Ref::Culture(c), Ref::Culture(old)],
                        None,
                        text,
                    );
                }
            }
        }
    }

    // Divergence: isolated communities become new peoples.
    if w.year % 20 == 0 && w.year > 0 {
        divergence(w);
    }
    // Extinction.
    if w.year % 10 == 0 {
        for c in 0..w.cultures.len() {
            let cu = &w.cultures[c];
            if cu.extinct.is_some() {
                continue;
            }
            let has_polity = w.polities.iter().any(|p| p.alive() && p.culture == c);
            let has_city = w
                .cities
                .iter()
                .any(|ci| ci.destroyed.is_none() && ci.culture == c);
            if cu.cells == 0 && !has_polity && !has_city && w.year - cu.last_seen > 30 {
                w.cultures[c].extinct = Some(w.year);
                let text = format!(
                    "The last speakers of {} died, and with them the {} passed out of the world.",
                    w.cultures[c].lang.name, w.cultures[c].plural
                );
                w.log(2, EventKind::Culture, &[Ref::Culture(c)], None, text);
            }
        }
    }
}

fn divergence(w: &mut World) {
    let rng = w.rng.clone();
    let n = w.cells.len();
    let ncult = w.cultures.len();
    for c in 0..ncult {
        if w.cultures[c].extinct.is_some() || w.cultures[c].cells < 50 {
            continue;
        }
        // Connected components of this culture's cells.
        let mut seen = vec![false; n];
        let home = w.cultures[c].home;
        let mut comps: Vec<Vec<usize>> = Vec::new();
        for s in 0..n {
            if seen[s] || w.cells[s].culture != Some(c) {
                continue;
            }
            let mut comp = Vec::new();
            let mut stack = vec![s];
            seen[s] = true;
            while let Some(i) = stack.pop() {
                comp.push(i);
                for nb in w.terrain.neighbors8(i) {
                    if !seen[nb] && w.cells[nb].culture == Some(c) {
                        seen[nb] = true;
                        stack.push(nb);
                    }
                }
            }
            comps.push(comp);
        }
        if comps.len() < 2 {
            continue;
        }
        let home_comp = comps.iter().position(|cp| cp.contains(&home)).unwrap_or(0);
        for (k, comp) in comps.iter().enumerate() {
            if k == home_comp || comp.len() < 25 {
                continue;
            }
            let center = comp[comp.len() / 2];
            let d = w.terrain.dist(center, home);
            let home_owner = w.cells[home].owner;
            let other_owner = w.cells[center].owner;
            let mut p = 0.15 + (d as f64 / 60.0).min(0.35);
            if home_owner != other_owner {
                p += 0.2;
            }
            if !rng.chance(p) {
                continue;
            }
            let race = w.cultures[c].race;
            let lang = w.cultures[c].lang.mutate(&rng);
            let nc = super::genesis::found_culture(w, race, lang, center, Some(c));
            for &i in comp {
                w.cells[i].culture = Some(nc);
            }
            // Cities and polities within the region follow.
            let mut polities_changed = Vec::new();
            for city in 0..w.cities.len() {
                if w.cities[city].destroyed.is_none()
                    && comp.contains(&w.cities[city].cell)
                    && w.cities[city].culture == c
                {
                    w.cities[city].culture = nc;
                }
            }
            for pi in w.living_polities() {
                if w.polities[pi].culture == c {
                    if let Some(cap) = w.capital_cell(pi) {
                        if comp.contains(&cap) {
                            w.polities[pi].culture = nc;
                            polities_changed.push(pi);
                        }
                    }
                }
            }
            let place = w.place_phrase(center, None);
            let (name, plural, oldname) = (
                w.cultures[nc].name.clone(),
                w.cultures[nc].plural.clone(),
                w.cultures[c].name.clone(),
            );
            let mut text = format!(
                "Cut off from their kin, the {} folk living {} drifted in speech and custom until they were a people apart: the {}, who call themselves {}.",
                oldname, place, plural, name
            );
            if let Some(&pi) = polities_changed.first() {
                text.push_str(&format!(
                    " {} is now a {} realm.",
                    w.polities[pi].name, w.cultures[nc].adj
                ));
            }
            let mut refs = vec![Ref::Culture(nc), Ref::Culture(c)];
            for pi in polities_changed {
                refs.push(Ref::Polity(pi));
            }
            w.log(2, EventKind::Culture, &refs, Some(center), text);
        }
    }
    let _ = Biome::Ocean;
}
