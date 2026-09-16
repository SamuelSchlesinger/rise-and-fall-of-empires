//! Diplomacy and war: tension between neighbours, declarations, yearly
//! battles shaped by terrain, sieges, sackings, and peace.

use super::chronicle::{EventKind, Ref};
use super::politics::{self, claim};
use super::{Role, SchoolKind, War, WarKind, World};
use std::collections::{BTreeMap, BTreeSet};

impl World {
    pub fn war_between(&self, p: usize, q: usize) -> Option<usize> {
        self.wars
            .iter()
            .find(|w| {
                w.alive()
                    && ((w.attacker == p && w.defender == q)
                        || (w.attacker == q && w.defender == p))
            })
            .map(|w| w.id)
    }

    pub fn wars_start(
        &mut self,
        attacker: usize,
        defender: usize,
        kind: WarKind,
        cause: String,
    ) -> usize {
        let id = self.wars.len();
        let name = self.war_name(attacker, defender, kind);
        self.wars.push(War {
            id,
            attacker,
            defender,
            started: self.year,
            ended: None,
            cause,
            name,
            score: 0.0,
            battles: 0,
            result: String::new(),
            cells_taken: 0,
            kind,
        });
        self.polities[attacker].wars.push(id);
        self.polities[defender].wars.push(id);
        self.polities[attacker].tension.remove(&defender);
        self.polities[defender].tension.remove(&attacker);
        self.century_wars += 1;
        id
    }

    fn war_name(&mut self, a: usize, d: usize, kind: WarKind) -> String {
        let rng = self.rng.clone();
        let count = self
            .wars
            .iter()
            .filter(|w| {
                (w.attacker == a && w.defender == d) || (w.attacker == d && w.defender == a)
            })
            .count();
        let ordinal = match count {
            0 => "",
            1 => "Second ",
            2 => "Third ",
            3 => "Fourth ",
            _ => "Latest ",
        };
        match kind {
            WarKind::Rebellion => {
                format!("the {} Rising", self.cultures[self.polities[a].culture].adj)
            }
            WarKind::CivilWar => format!("the {} Civil War", self.polities[d].adj),
            WarKind::Succession => format!("the War of the {} Succession", self.polities[d].adj),
            WarKind::Holy => match self.polities[a].school {
                Some(s) => format!("the {} Crusade", self.schools[s].short),
                None => format!("the {} Holy War", self.polities[a].adj),
            },
            _ => {
                // Try naming after a contested feature near the border.
                let mut feature = None;
                if let Some(cap) = self.capital_cell(d) {
                    let t = &self.terrain;
                    if let Some(f) = t.feature_at(cap) {
                        if !matches!(t.features[f].kind, crate::geo::FeatureKind::Continent) {
                            feature = Some(f);
                        }
                    }
                    if feature.is_none() {
                        feature = t.river_at(cap);
                    }
                }
                if let Some(f) = feature {
                    if rng.chance(0.5) {
                        let n = self.feature_name(f, Some(a));
                        return format!(
                            "the {}War of {}",
                            ordinal,
                            n.trim_start_matches("the ").to_string()
                        );
                    }
                }
                match rng.below(3) {
                    0 => format!(
                        "the {}{}-{} War",
                        ordinal, self.polities[a].adj, self.polities[d].adj
                    ),
                    1 => format!("the {}{} War", ordinal, self.polities[d].adj),
                    _ => format!("the {}War of {}", ordinal, self.polities[a].short),
                }
            }
        }
    }

    pub fn end_war(&mut self, wid: usize, result: String, log: bool) {
        if !self.wars[wid].alive() {
            return;
        }
        self.wars[wid].ended = Some(self.year);
        self.wars[wid].result = result.clone();
        let (a, d) = (self.wars[wid].attacker, self.wars[wid].defender);
        self.polities[a].wars.retain(|&x| x != wid);
        self.polities[d].wars.retain(|&x| x != wid);
        let until = self.year + 8 + self.rng.int(0, 12);
        self.polities[a].truce.insert(d, until);
        self.polities[d].truce.insert(a, until);
        self.polities[a].tension.insert(d, 0.0);
        self.polities[d].tension.insert(a, 0.0);
        if log {
            let text = format!(
                "{} {}",
                crate::lang::capitalize(&self.wars[wid].name),
                result
            );
            self.log(
                2,
                EventKind::Peace,
                &[Ref::War(wid), Ref::Polity(a), Ref::Polity(d)],
                None,
                text,
            );
        }
    }
}

pub fn diplomacy(w: &mut World) {
    let rng = w.rng.clone();
    let alive = w.living_polities();
    for &p in &alive {
        let neighbors = w.polities[p].neighbors.clone();
        let pol = &w.polities[p];
        let vals = w.cultures[pol.culture].values;
        let traits = politics::traits_of(w, p);
        let mut updates: Vec<(usize, f32)> = Vec::new();
        for &(q, border) in &neighbors {
            if !w.polities[q].alive() {
                continue;
            }
            let qol = &w.polities[q];
            let mut d = 0.008
                * (border as f32 / 8.0).min(2.0)
                * (0.5 + vals.militarism)
                * (0.5 + traits.ambition);
            if qol.culture != pol.culture {
                d += 0.02;
                if w.cultures[qol.culture].race != w.cultures[pol.culture].race {
                    d += 0.01;
                }
            }
            // Claims: they hold lands of our people.
            let ours_under_them = qol.culture_counts.get(&pol.culture).copied().unwrap_or(0);
            if ours_under_them > 5 {
                d += 0.02;
            }
            // Rival faiths.
            if let (Some(a), Some(b)) = (pol.school, qol.school) {
                if a != b {
                    let sa = &w.schools[a];
                    if sa.kind == SchoolKind::Divine || w.schools[b].kind == SchoolKind::Divine {
                        d += 0.025 * (sa.hostility + w.schools[b].hostility);
                    }
                }
            }
            // Trade calms things.
            if vals.mercantilism > 0.5 && w.cultures[qol.culture].values.mercantilism > 0.5 {
                d -= 0.02;
            }
            // Weak neighbours tempt the ambitious.
            if qol.stability < 0.3 && traits.ambition > 0.5 {
                d += 0.02;
            }
            // Same parent (recently split) keeps grudges.
            if qol.parent == Some(p) || pol.parent == Some(q) {
                d += 0.01;
            }
            d -= 0.015;
            let cur = pol.tension.get(&q).copied().unwrap_or(0.1);
            updates.push((q, (cur + d).clamp(0.0, 1.0)));
        }
        let pol = &mut w.polities[p];
        let neighbor_ids: BTreeSet<usize> = neighbors.iter().map(|&(q, _)| q).collect();
        pol.tension.retain(|q, v| {
            *v *= 0.97;
            neighbor_ids.contains(q) || *v > 0.05
        });
        for (q, v) in updates {
            pol.tension.insert(q, v);
        }
    }
    // Declarations.
    for &p in &alive {
        if !w.polities[p].alive() || w.polities[p].wars.len() >= 2 || w.polities[p].stability < 0.3
        {
            continue;
        }
        let traits = politics::traits_of(w, p);
        let targets: Vec<(usize, f32)> = w.polities[p]
            .tension
            .iter()
            .map(|(&q, &t)| (q, t))
            .collect();
        for (q, t) in targets {
            if !w.polities[q].alive() || w.war_between(p, q).is_some() {
                continue;
            }
            if w.polities[p]
                .truce
                .get(&q)
                .map(|&y| y > w.year)
                .unwrap_or(false)
            {
                continue;
            }
            if t < 0.65 {
                continue;
            }
            let my = w.polities[p].army;
            let their = w.polities[q].army * (1.0 + w.polities[q].wars.len() as f32 * 0.2);
            let bold = 0.7 + traits.valor * 0.5;
            if my < their * bold * rng.range32(0.7, 1.2) {
                continue;
            }
            if !rng.chance(0.08 + traits.ambition as f64 * 0.15) {
                continue;
            }
            // Cause.
            let pol = &w.polities[p];
            let qol = &w.polities[q];
            let holy = pol.school.is_some()
                && qol.school.is_some()
                && pol.school != qol.school
                && w.schools[pol.school.unwrap()].kind == SchoolKind::Divine
                && traits.piety > 0.6;
            let kind = if holy {
                WarKind::Holy
            } else if pol.kind == super::PolityKind::Horde {
                WarKind::Raid
            } else {
                WarKind::Conquest
            };
            let cause = if holy {
                format!("the heresies of {}", qol.short)
            } else {
                match rng.below(6) {
                    0 => format!("a dispute over the borderlands of {}", w.polities[q].short),
                    1 => "the insult of a refused marriage".to_string(),
                    2 => format!("the ambition of {}", w.ruler_short(p)),
                    3 => format!(
                        "the {} settlers living under {} rule",
                        w.cultures[pol.culture].adj, qol.adj
                    ),
                    4 => "a murdered envoy".to_string(),
                    _ => "old grievances no one could quite recall".to_string(),
                }
            };
            let wid = w.wars_start(p, q, kind, cause.clone());
            let text = match kind {
                WarKind::Holy => format!(
                    "{} declared holy war upon {}, citing {}. Thus began {}.",
                    w.ruler_title(p),
                    w.polities[q].name,
                    cause,
                    w.wars[wid].name
                ),
                WarKind::Raid => format!(
                    "The riders of {} fell upon {}. {} had begun.",
                    w.polities[p].short,
                    w.polities[q].name,
                    crate::lang::capitalize(&w.wars[wid].name)
                ),
                _ => format!(
                    "{} declared war upon {}, citing {}. Thus began {}.",
                    w.ruler_title(p),
                    w.polities[q].name,
                    cause,
                    w.wars[wid].name
                ),
            };
            w.log(
                2,
                EventKind::War,
                &[Ref::War(wid), Ref::Polity(p), Ref::Polity(q)],
                w.capital_cell(q),
                text,
            );
            break;
        }
    }
}

pub fn resolve_wars(w: &mut World) {
    let rng = w.rng.clone();
    let active: Vec<usize> = w.wars.iter().filter(|x| x.alive()).map(|x| x.id).collect();
    if active.is_empty() {
        return;
    }
    // Front lines: for each war, cells of each side adjacent to the other.
    let pairs: BTreeSet<(usize, usize)> = active
        .iter()
        .map(|&i| (w.wars[i].attacker, w.wars[i].defender))
        .collect();
    let mut front: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new(); // (owner, enemy) -> cells of owner adjacent to enemy
    let n = w.cells.len();
    for i in 0..n {
        let p = match w.cells[i].owner {
            Some(p) => p,
            None => continue,
        };
        for nb in w.terrain.neighbors8(i) {
            if let Some(q) = w.cells[nb].owner {
                if q != p && (pairs.contains(&(p, q)) || pairs.contains(&(q, p))) {
                    front.entry((p, q)).or_default().push(i);
                }
            }
        }
    }
    for v in front.values_mut() {
        v.sort_unstable();
        v.dedup();
    }
    for wid in active {
        if !w.wars[wid].alive() {
            continue;
        }
        let (a, d) = (w.wars[wid].attacker, w.wars[wid].defender);
        if !w.polities[a].alive() || !w.polities[d].alive() {
            w.end_war(wid, "ended.".to_string(), false);
            continue;
        }
        let years = w.year - w.wars[wid].started;
        let a_front = front.get(&(a, d)).cloned().unwrap_or_default();
        let d_front = front.get(&(d, a)).cloned().unwrap_or_default();
        let has_front = !a_front.is_empty() && !d_front.is_empty();
        if has_front {
            let rounds = if w.detail.level() >= 1 && rng.chance(0.4) {
                2
            } else {
                1
            };
            for round in 0..rounds {
                if !w.wars[wid].alive() || !w.polities[a].alive() || !w.polities[d].alive() {
                    break;
                }
                battle(w, wid, a, d, &a_front, &d_front, round > 0);
            }
        } else {
            w.wars[wid].score *= 0.9;
        }
        if !w.wars[wid].alive() {
            continue;
        }
        // Peace?
        let ex = (w.polities[a].exhaustion + w.polities[d].exhaustion) / 2.0;
        let score = w.wars[wid].score;
        let mut p_peace = if years < 3 {
            0.0
        } else {
            0.03 + ex as f64 * 0.15 + (score.abs() as f64 / 3.0).min(0.25)
        };
        if !has_front {
            p_peace += 0.2;
        }
        let kind = w.wars[wid].kind;
        if matches!(
            kind,
            WarKind::Rebellion | WarKind::CivilWar | WarKind::Succession
        ) && years > 15
        {
            p_peace += 0.3;
        }
        if rng.chance(p_peace) {
            make_peace(w, wid);
        }
    }
}

fn strength(w: &World, p: usize, defending_cells: &[usize]) -> f32 {
    let pol = &w.polities[p];
    let t = politics::traits_of(w, p);
    let mut s = pol.army * (0.7 + t.valor * 0.6) * (0.55 + pol.stability * 0.5);
    let generals = pol
        .generals
        .iter()
        .filter(|&&g| w.persons[g].alive())
        .count()
        .min(2) as f32;
    s *= 1.0 + generals * 0.15;
    if !defending_cells.is_empty() {
        let mut def = 0.0;
        for &c in defending_cells.iter().take(40) {
            def += w.terrain.biome[c].defense();
        }
        s *= def / defending_cells.len().min(40) as f32;
    }
    s.max(0.05)
}

fn battlefield_name(w: &mut World, cell: usize, culture: usize) -> String {
    // Nearest city within 3 cells.
    for c in &w.cities {
        if c.destroyed.is_none() && w.terrain.dist(c.cell, cell) <= 3 {
            return c.name.clone();
        }
    }
    if let Some(n) = w.battlefield_names.get(&cell) {
        return n.clone();
    }
    let (x, y) = w.terrain.xy(cell);
    let key = w.terrain.idx(x / 3 * 3, y / 3 * 3);
    if let Some(n) = w.battlefield_names.get(&key) {
        return n.clone();
    }
    let name = w.cultures[culture].lang.place(&w.rng);
    w.battlefield_names.insert(key, name.clone());
    name
}

fn battle(
    w: &mut World,
    wid: usize,
    a: usize,
    d: usize,
    a_front: &[usize],
    d_front: &[usize],
    quiet: bool,
) {
    let rng = w.rng.clone();
    // Who takes the offensive this year?
    let sa = strength(w, a, &[]);
    let sd = strength(w, d, &[]);
    let attacker_offense = rng.chance(if sa >= sd { 0.7 } else { 0.35 });
    let (off, def, def_front) = if attacker_offense {
        (a, d, d_front)
    } else {
        (d, a, a_front)
    };
    let s_off = strength(w, off, &[]);
    let s_def = strength(w, def, def_front);
    let p_win = s_off / (s_off + s_def);
    let win = rng.chance((p_win as f64 + rng.normal() * 0.08).clamp(0.05, 0.95));
    let margin = ((s_off - s_def) / (s_off + s_def)).abs();
    let site = def_front[rng.below(def_front.len())];
    let def_culture = w.cells[site].culture.unwrap_or(w.polities[def].culture);
    let place = battlefield_name(w, site, def_culture);
    w.wars[wid].battles += 1;
    let (winner, loser) = if win { (off, def) } else { (def, off) };
    // Casualties.
    w.polities[loser].army *= 1.0 - (0.08 + margin * 0.15);
    w.polities[winner].army *= 1.0 - 0.04;
    w.polities[winner].exhaustion += 0.02;
    w.polities[loser].exhaustion += 0.04;
    let swing = 0.07 + margin * 0.2;
    if winner == a {
        w.wars[wid].score += swing;
    } else {
        w.wars[wid].score -= swing;
    }
    for &g in w.polities[winner].generals.clone().iter() {
        if w.persons[g].alive() {
            w.persons[g].renown += 1.0;
            w.persons[g].battles_won += 1;
        }
    }
    let mut importance = 1;
    let mut text = if win {
        format!(
            "The armies of {} defeated {} at the Battle of {}.",
            w.polities[off].short, w.polities[def].short, place
        )
    } else {
        format!(
            "The {} attack on {} was thrown back at the Battle of {}.",
            w.polities[off].adj, w.polities[def].short, place
        )
    };
    let mut refs = vec![Ref::War(wid), Ref::Polity(winner), Ref::Polity(loser)];
    // Territory changes: the winner takes cells from the loser along the front.
    let loser_front: Vec<usize> = if loser == def {
        def_front.to_vec()
    } else if loser == a {
        a_front.to_vec()
    } else {
        d_front.to_vec()
    };
    if win || rng.chance(0.3) {
        let k = 1 + (margin * 6.0 + rng.f32() * 2.0) as usize;
        let mut taken = 0;
        let mut cells: Vec<usize> = loser_front
            .iter()
            .copied()
            .filter(|&c| w.cells[c].owner == Some(loser))
            .collect();
        // Nearest to the battle site first.
        cells.sort_by_key(|&c| w.terrain.dist(c, site));
        for c in cells {
            if taken >= k {
                break;
            }
            if let Some(city) = w.cells[c].city {
                // Siege.
                let walls = w.cities[city].walls;
                let ratio = strength(w, winner, &[]) / strength(w, loser, &[]).max(0.05);
                if ratio > 1.0 + walls * 0.6 && rng.chance(0.5) {
                    let s = capture_city(w, city, winner, loser, wid);
                    text.push_str(&s);
                    refs.push(Ref::City(city));
                    importance = 2;
                    taken += 1;
                } else if w.detail.level() >= 1 && rng.chance(0.3) {
                    text.push_str(&format!(" {} withstood the siege.", w.cities[city].name));
                    w.cities[city].walls = (walls - 0.05).max(0.0);
                }
                continue;
            }
            if !win && taken == 0 {
                text.push_str(&format!(
                    " The {} pressed their advantage.",
                    w.polities[winner].adj
                ));
            }
            claim(w, winner, c);
            w.polities[winner].reign_gained += 1;
            w.polities[loser].reign_gained -= 1;
            if winner == a {
                w.wars[wid].cells_taken += 1;
            } else {
                w.wars[wid].cells_taken -= 1;
            }
            taken += 1;
        }
    }
    // A leading ruler may fall.
    for &side in &[winner, loser] {
        if let Some(r) = w.polities[side].ruler {
            let t = w.persons[r].traits;
            let leads = t.valor > 0.55;
            let p_die = if side == loser { 0.035 } else { 0.008 };
            if leads && rng.chance(p_die) {
                let name = w.persons[r].name.clone();
                politics::ruler_dies(w, side, format!("fell at the Battle of {}.", place), 2);
                text.push_str(&format!(" {} fell in the fighting.", name));
                importance = 2;
            }
        }
    }
    // Generals may die too.
    for &side in &[winner, loser] {
        for g in w.polities[side].generals.clone() {
            if w.persons[g].alive() && rng.chance(if side == loser { 0.06 } else { 0.015 }) {
                w.persons[g].died = Some(w.year);
                w.persons[g].death = format!("fell at the Battle of {}.", place);
                text.push_str(
                    &format!(
                        " {} {}, the {} general, was slain.",
                        w.persons[g].name,
                        w.persons[g].epithet.clone().unwrap_or_default(),
                        w.polities[side].adj
                    )
                    .replace("  ", " "),
                );
                refs.push(Ref::Person(g));
            }
        }
    }
    if w.high_detail() && importance == 1 && rng.chance(0.25) {
        let fl = match rng.below(5) {
            0 => " The river ran red for a day.".to_string(),
            1 => format!(" The {} held the high ground.", w.polities[winner].adj),
            2 => " Rain turned the field to mud and the wounded drowned in it.".to_string(),
            3 => format!(
                " The {} fled at dusk, leaving their baggage.",
                w.polities[loser].adj
            ),
            _ => " Both sides claimed the victory; the ravens did not care.".to_string(),
        };
        text.push_str(&fl);
    }
    if quiet && importance < 2 {
        return;
    }
    w.log(importance, EventKind::Battle, &refs, Some(site), text);
}

fn capture_city(w: &mut World, city: usize, winner: usize, loser: usize, wid: usize) -> String {
    let rng = w.rng.clone();
    let name = w.cities[city].name.clone();
    let cell = w.cities[city].cell;
    let t = politics::traits_of(w, winner);
    let was_capital = w.polities[loser].capital == Some(city);
    claim(w, winner, cell);
    w.cities[city].polity = Some(winner);
    w.polities[winner].reign_gained += 1;
    w.polities[loser].reign_gained -= 1;
    if winner == w.wars[wid].attacker {
        w.wars[wid].score += 0.3;
    } else {
        w.wars[wid].score -= 0.3;
    }
    let sack = rng.chance(0.3 + t.cruelty as f64 * 0.5);
    let mut s = if sack {
        w.cities[city].pop *= 0.5;
        w.cities[city].times_sacked += 1;
        w.cities[city].prosperity *= 0.6;
        let lost = if !w.cities[city].wonders.is_empty() && rng.chance(0.5) {
            let wn = w.cities[city].wonders.remove(0);
            format!(" {} was cast down.", wn)
        } else {
            String::new()
        };
        w.polities[winner].treasury += w.cities[city].pop * 2.0;
        format!(
            " {} was taken and sacked by the {}.{}",
            name, w.polities[winner].adj, lost
        )
    } else {
        format!(
            " {} opened its gates to the {}.",
            name, w.polities[winner].adj
        )
    };
    s.push_str(&super::stories::artifacts_on_capture(
        w, city, winner, loser,
    ));
    if was_capital {
        w.polities[loser].stability = (w.polities[loser].stability - 0.3).max(0.0);
        // Move the capital.
        let remaining: Vec<usize> = w.polities[loser]
            .cities
            .iter()
            .copied()
            .filter(|&c| {
                c != city && w.cities[c].destroyed.is_none() && w.cities[c].polity == Some(loser)
            })
            .collect();
        if let Some(&nc) = remaining
            .iter()
            .max_by(|&&x, &&y| w.cities[x].pop.partial_cmp(&w.cities[y].pop).unwrap())
        {
            w.polities[loser].capital = Some(nc);
            s.push_str(&format!(
                " The court of {} fled to {}.",
                w.polities[loser].short, w.cities[nc].name
            ));
        } else {
            let cause = format!(
                "was conquered by {} with the fall of {}.",
                w.polities[winner].name, name
            );
            s.push_str(&format!(
                " With its capital lost, {} was no more.",
                w.polities[loser].name
            ));
            politics::fall(w, loser, cause, Some(winner), 2);
        }
    }
    s
}

fn make_peace(w: &mut World, wid: usize) {
    let (a, d) = (w.wars[wid].attacker, w.wars[wid].defender);
    let score = w.wars[wid].score;
    let kind = w.wars[wid].kind;
    let years = w.year - w.wars[wid].started;
    let result = match kind {
        WarKind::Rebellion | WarKind::CivilWar | WarKind::Succession => {
            if score < -0.4 {
                // Rebels crushed.
                let cause = format!(
                    "was crushed by {} after {} years of fighting.",
                    w.polities[d].name,
                    years.max(1)
                );
                let leader = w.polities[a].ruler;
                politics::fall(w, a, cause, Some(d), 2);
                if let Some(l) = leader {
                    if w.persons[l].alive() {
                        w.persons[l].died = Some(w.year);
                        w.persons[l].death =
                            "was executed after the failure of the rising.".to_string();
                    }
                }
                w.polities[d].reign_wars_won += 1;
                return;
            } else if score > 0.8
                && kind != WarKind::Rebellion
                && w.polities[a].cells > w.polities[d].cells
            {
                // Rebels take over the old realm.
                let cause = format!(
                    "was overthrown; {} took the old capital and ruled in its place.",
                    w.polities[a].name
                );
                politics::fall(w, d, cause, Some(a), 2);
                return;
            } else {
                w.polities[d].reign_wars_won += 0;
                format!(
                    "ended with {} recognising the independence of {}.",
                    w.polities[d].short, w.polities[a].name
                )
            }
        }
        _ => {
            if score > 0.5 {
                w.polities[a].reign_wars_won += 1;
                w.polities[a].prestige += 5.0;
                w.polities[d].prestige -= 3.0;
                let tribute = (w.polities[d].treasury * 0.3).max(0.0);
                w.polities[d].treasury -= tribute;
                w.polities[a].treasury += tribute;
                if kind == WarKind::Holy {
                    if let Some(s) = w.polities[a].school {
                        let e = w.schools[s].influence.entry(d).or_insert(0.0);
                        *e = (*e + 0.3).min(1.0);
                    }
                }
                format!("ended in victory for {} after {} years; {} ceded the lands it had lost and paid tribute.", w.polities[a].short, years.max(1), w.polities[d].short)
            } else if score < -0.5 {
                w.polities[d].reign_wars_won += 1;
                w.polities[d].prestige += 5.0;
                w.polities[a].prestige -= 3.0;
                format!(
                    "ended after {} years with {} thrown back and humbled.",
                    years.max(1),
                    w.polities[a].short
                )
            } else {
                format!("ended after {} years with neither side the master; the exhausted realms made peace.", years.max(1))
            }
        }
    };
    w.end_war(wid, result, true);
}

pub fn role_general(w: &mut World, p: usize, g: usize) {
    w.persons[g].role = Role::General;
    w.persons[g].polity = Some(p);
    w.polities[p].generals.push(g);
    w.polities[p].generals.retain(|&x| w.persons[x].alive());
}
