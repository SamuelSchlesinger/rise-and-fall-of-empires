//! Diplomacy and war: tension between neighbours, declarations, yearly
//! battles shaped by terrain, sieges, sackings, and peace.

use super::chronicle::{EventKind, Ref};
use super::politics::{self, claim};
use super::prose::{self, Pick};
use super::{Role, SchoolKind, Stance, War, WarAim, WarKind, World};
use std::collections::{BTreeMap, BTreeSet};

/// Years a war must have run before it can be called hopeless.
const HOPELESS_MIN_YEARS: i32 = 2;
/// Field strength, as a share of the other side's, below which a war is
/// hopeless: one host is worth less than a seventh of the other.
const HOPELESS_RATIO: f32 = 0.15;
/// How hot a quarrel with a realm that is not a neighbour and is not the
/// power of the age has to be to stay on a realm's books at all.
const FAR_TENSION_FLOOR: f32 = 0.25;
/// How often a realm's expired truces are swept up, in years. Staggered by
/// realm id so the work is spread rather than spiking in one year.
const TRUCE_SWEEP: usize = 16;
/// Most realms a realm will keep a running quarrel with. Beyond this the
/// coldest are forgotten, which is both what a chancery would do and what
/// keeps the phase's cost flat as the world ages.
const TENSION_CAP: usize = 24;
/// What an assault across open water is worth against one over a border.
/// A landing is the hardest thing an army does, and the reason an island is
/// worth holding.
const LANDING_PENALTY: f32 = 0.6;
/// A world with less settled land than this has no hegemon, whatever share
/// of it one realm holds: being the largest of three chiefdoms on an empty
/// map is not the same fact as ruling a quarter of the world.
const HEGEMON_MIN_WORLD: usize = 300;
/// Nor is a realm too small to matter a hegemon, however empty the map.
const HEGEMON_MIN_CELLS: usize = 60;
/// Most allies one side of a war may gather. A coalition is two or three
/// realms acting together; beyond that it is every realm on the map, which
/// is neither interesting to read nor possible to balance.
const MAX_JOINERS_PER_SIDE: usize = 2;
/// An army this small cannot prosecute a war at all.
const NEGLIGIBLE_ARMY: f32 = 1.0;

impl World {
    /// The war being fought between `p` and `q`, if any. A polity's `wars`
    /// holds only its live ones, so this reads a handful of ids rather than
    /// the whole history of wars; the lowest id wins, as a scan would give.
    pub fn war_between(&self, p: usize, q: usize) -> Option<usize> {
        self.polities[p]
            .wars
            .iter()
            .copied()
            .filter(|&id| {
                let w = &self.wars[id];
                w.alive() && (w.attacker == q || w.defender == q)
            })
            .min()
    }

    /// Start a war with an explicit aim. Every declaration goes through
    /// here, so every war has something a peace can settle.
    pub fn wars_start_aim(
        &mut self,
        attacker: usize,
        defender: usize,
        kind: WarKind,
        aim: WarAim,
        cause: String,
    ) -> usize {
        let id = self.wars.len();
        let name = self.war_name(attacker, defender, kind, aim);
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
            allies_a: Vec::new(),
            allies_d: Vec::new(),
            aim,
            aim_met: false,
            kind,
        });
        self.polities[attacker].wars.push(id);
        self.polities[defender].wars.push(id);
        self.polities[attacker].tension.remove(&defender);
        self.polities[defender].tension.remove(&attacker);
        self.century_wars += 1;
        id
    }

    /// Start a war over the border, which is what most wars are about.
    pub fn wars_start(
        &mut self,
        attacker: usize,
        defender: usize,
        kind: WarKind,
        cause: String,
    ) -> usize {
        self.wars_start_aim(attacker, defender, kind, WarAim::Border, cause)
    }

    /// Bring `q` into war `wid` on `side_a`'s side.
    ///
    /// A joiner is listed on the war and gets the war on its own books, so
    /// `at_war` and the front-line sweep see it, but the war keeps its two
    /// principals: they are who named it and who will sign for it.
    pub fn war_join(&mut self, wid: usize, q: usize, side_a: bool) {
        if !self.wars[wid].alive() || !self.polities[q].alive() {
            return;
        }
        let (a, d) = (self.wars[wid].attacker, self.wars[wid].defender);
        if q == a || q == d {
            return;
        }
        if self.wars[wid].allies_a.contains(&q) || self.wars[wid].allies_d.contains(&q) {
            return;
        }
        if side_a {
            self.wars[wid].allies_a.push(q);
        } else {
            self.wars[wid].allies_d.push(q);
        }
        self.polities[q].wars.push(wid);
        // A joiner is at war with everyone on the other side, so the truce
        // that would otherwise hold them back has to go.
        let foes: Vec<usize> = if side_a {
            std::iter::once(d)
                .chain(self.wars[wid].allies_d.clone())
                .collect()
        } else {
            std::iter::once(a)
                .chain(self.wars[wid].allies_a.clone())
                .collect()
        };
        for f in foes {
            self.polities[q].truce.remove(&f);
            self.polities[f].truce.remove(&q);
        }
    }

    /// Everyone fighting on each side of a war: principals first.
    pub fn war_sides(&self, wid: usize) -> (Vec<usize>, Vec<usize>) {
        let war = &self.wars[wid];
        let live = |v: &[usize]| -> Vec<usize> {
            v.iter()
                .copied()
                .filter(|&x| self.polities[x].alive())
                .collect()
        };
        let mut a = vec![war.attacker];
        a.extend(live(&war.allies_a));
        let mut d = vec![war.defender];
        d.extend(live(&war.allies_d));
        (a, d)
    }

    fn war_name(&mut self, a: usize, d: usize, kind: WarKind, aim: WarAim) -> String {
        let rng = self.rng.clone();
        // The base name first, then the ordinal — because a feature name is
        // shared by every realm near it, and counting wars between *this
        // pair* left three different realms all fighting a "First War of the
        // Ngerrilm Plain". The ordinal has to count the name, not the pair.
        let base = match kind {
            WarKind::Rebellion | WarKind::CivilWar | WarKind::Succession | WarKind::Holy => {
                return self.war_name_ordinal(&prose::war_name_internal(self, a, d, kind))
            }
            _ => match aim {
                WarAim::City(c) if self.cities[c].destroyed.is_none() => {
                    let n = self.cities[c].name.clone();
                    prose::war_name_city(&n)
                }
                WarAim::Independence => {
                    let n = self.polities[a].adj.clone();
                    prose::war_name_independence(&n)
                }
                WarAim::Vassalage => {
                    let n = self.polities[d].short.clone();
                    prose::war_name_vassalage(&n)
                }
                WarAim::Claimant(r) => {
                    let n = self.persons[r].name.clone();
                    prose::war_name_claim(&n)
                }
                WarAim::Relic(x) => {
                    let n = self.artifacts[x].name.clone();
                    prose::war_name_relic(&n)
                }
                WarAim::Containment => {
                    let n = self.polities[d].adj.clone();
                    prose::war_name_containment(&n)
                }
                _ => {
                    // Name it after a contested feature near the border.
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
                    match feature {
                        Some(f) if rng.chance(0.4) => {
                            let n = self.feature_name(f, Some(a));
                            prose::war_name_feature(&n)
                        }
                        _ => prose::war_name_realms(self, a, d, &Pick::rolled(&rng)),
                    }
                }
            },
        };
        self.war_name_ordinal(&base)
    }

    /// Number a war after the ones that already carry the same name.
    fn war_name_ordinal(&self, base: &str) -> String {
        let n = self
            .wars
            .iter()
            .filter(|w| {
                w.name == base
                    || w.name
                        .ends_with(&format!(" {}", base.trim_start_matches("the ")))
            })
            .count();
        prose::war_name_numbered(base, n)
    }

    pub fn end_war(&mut self, wid: usize, result: &str, log: bool) {
        if !self.wars[wid].alive() {
            return;
        }
        self.wars[wid].ended = Some(self.year);
        self.wars[wid].result = result.to_string();
        let (a, d) = (self.wars[wid].attacker, self.wars[wid].defender);
        let side_a: Vec<usize> = std::iter::once(a)
            .chain(self.wars[wid].allies_a.iter().copied())
            .collect();
        let side_d: Vec<usize> = std::iter::once(d)
            .chain(self.wars[wid].allies_d.iter().copied())
            .collect();
        for &x in side_a.iter().chain(side_d.iter()) {
            self.polities[x].wars.retain(|&y| y != wid);
        }
        let until = self.year
            + self.tuning.truce_years_min
            + self.rng.int(0, self.tuning.truce_years_random);
        // Everybody who fought gets a truce with everybody they fought, or
        // the allies simply carry the war on by themselves next year.
        for &x in &side_a {
            for &y in &side_d {
                self.polities[x].truce.insert(y, until);
                self.polities[y].truce.insert(x, until);
                self.polities[x].tension.insert(y, 0.0);
                self.polities[y].tension.insert(x, 0.0);
            }
        }
        if log {
            let text = prose::war_ended(&self.wars[wid].name, result);
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

/// The realm holding more of the settled world than anyone should.
///
/// A hegemon is the single fact that makes the rest of the diplomatic model
/// worth having: without it every realm only ever weighs its immediate
/// neighbour, nobody combines, and the map settles into a permanent stand-off
/// in which no power can grow and none can be pulled down. With it, growing
/// large is its own punishment, which is the shape the title of this game
/// promises.
pub fn hegemon(w: &World) -> Option<usize> {
    // `owned_cells` is recomputed at the end of a tick, so in the opening
    // years of a world it is still counting an empty map while realms have
    // already begun to claim. Dominating a world of eleven settled cells is
    // not hegemony, and dividing by that count produced shares over 100%.
    let owned = w.stats.owned_cells;
    if owned < HEGEMON_MIN_WORLD {
        return None;
    }
    let mut best = None;
    let mut best_share = w.tuning.hegemon_share;
    for p in w.living_polities() {
        if w.polities[p].cells < HEGEMON_MIN_CELLS {
            continue;
        }
        let share = (w.polities[p].cells as f32 / owned as f32).min(1.0);
        if share > best_share {
            best_share = share;
            best = Some(p);
        }
    }
    best
}

pub fn diplomacy(w: &mut World) {
    let rng = w.rng.clone();
    let tn = w.tuning;
    let alive = w.living_polities();
    let top = hegemon(w);
    let top_share = top
        .map(|p| super::dynasty::world_share(w, p))
        .unwrap_or(0.0);
    // Who is close enough to the power of the age to do anything about it:
    // its own neighbours, and theirs.
    //
    // Fear of a hegemon used to be given to every realm in the world, which
    // was both wrong and expensive — wrong because a realm on the far side
    // of an ocean does not join a coalition it cannot march to, and
    // expensive because it put a fresh entry in every realm's books every
    // year, which is what made this the costliest phase in the tick.
    let mut contained: BTreeSet<usize> = BTreeSet::new();
    if let Some(h) = top {
        for &(q, _) in &w.polities[h].neighbors {
            contained.insert(q);
            for &(r, _) in &w.polities[q].neighbors {
                contained.insert(r);
            }
        }
        contained.remove(&h);
    }
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
            let mut d = tn.tension_base_growth
                * (border as f32 / 8.0).min(2.0)
                * (0.5 + vals.militarism)
                * (0.5 + traits.ambition);
            if qol.culture != pol.culture {
                d += tn.tension_foreign_culture;
                if w.cultures[qol.culture].race != w.cultures[pol.culture].race {
                    d += tn.tension_foreign_race;
                }
            }
            // Claims: they hold lands of our people.
            let ours_under_them = qol.culture_counts.get(&pol.culture).copied().unwrap_or(0);
            if ours_under_them > 5 {
                d += tn.tension_claims;
            }
            // Rival faiths.
            if let (Some(a), Some(b)) = (pol.school, qol.school) {
                if a != b {
                    let sa = &w.schools[a];
                    if sa.kind == SchoolKind::Divine || w.schools[b].kind == SchoolKind::Divine {
                        d += tn.tension_faith_weight * (sa.hostility + w.schools[b].hostility);
                    }
                }
            }
            // Trade calms things.
            if vals.mercantilism > 0.5 && w.cultures[qol.culture].values.mercantilism > 0.5 {
                d -= tn.tension_trade_relief;
            }
            // Weak neighbours tempt the ambitious.
            if qol.stability < 0.3 && traits.ambition > 0.5 {
                d += tn.tension_weak_neighbor;
            }
            // Same parent (recently split) keeps grudges.
            if qol.parent == Some(p) || pol.parent == Some(q) {
                d += 0.01;
            }
            // Standing arrangements. An alliance or a marriage is what lets
            // a border simply stop being a problem, which is the thing the
            // old model had no way to express.
            match pol.stance.get(&q).copied().unwrap_or(Stance::Neutral) {
                Stance::Allied => d -= tn.tension_decay * 3.0,
                Stance::Married => d -= tn.tension_decay * 2.0,
                Stance::Rival => d += tn.tension_base_growth,
                Stance::Neutral => {}
            }
            // An overlord and its tributary do not build tension: the
            // question between them is settled until somebody unsettles it.
            if pol.overlord == Some(q) || qol.overlord == Some(p) {
                d -= tn.tension_decay * 4.0;
            }
            d -= tn.tension_decay;
            let cur = pol.tension.get(&q).copied().unwrap_or(0.1);
            updates.push((q, (cur + d).clamp(0.0, 1.0)));
        }
        let pol = &mut w.polities[p];
        let neighbor_ids: BTreeSet<usize> = neighbors.iter().map(|&(q, _)| q).collect();
        // Non-neighbours have to clear a much higher bar to stay on the
        // books, and the list is capped whatever happens.
        //
        // Both guards are here because this map is the one thing in
        // `diplomacy` that can grow without bound: containment gives every
        // realm in the world an entry for the hegemon, hegemons change, and
        // an entry decaying at `tension_decay_mult` takes a century and a
        // half to fall under a bar of 0.05. Left alone, a realm ended up
        // weighing every power that had ever frightened its ancestors and
        // the phase's cost grew with the age of the world.
        pol.tension.retain(|q, v| {
            *v *= tn.tension_decay_mult;
            if neighbor_ids.contains(q) || Some(*q) == top {
                true
            } else {
                *v > FAR_TENSION_FLOOR
            }
        });
        for (q, v) in updates {
            pol.tension.insert(q, v);
        }
        if pol.tension.len() > TENSION_CAP {
            // Keep the ones that matter: neighbours, the hegemon, and the
            // hottest of the rest.
            let mut far: Vec<(usize, f32)> = pol
                .tension
                .iter()
                .filter(|&(q, _)| !neighbor_ids.contains(q) && Some(*q) != top)
                .map(|(&q, &v)| (q, v))
                .collect();
            far.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            let keep = TENSION_CAP.saturating_sub(neighbor_ids.len() + 1);
            for (q, _) in far.into_iter().skip(keep) {
                pol.tension.remove(&q);
            }
        }
        // Truces expire, and an expired truce is a fact about the past: they
        // were never dropped, so a long-lived realm carried one entry for
        // every realm it had ever fought. Swept on a rolling schedule rather
        // than every year — a stale entry is harmless, since every reader of
        // the map compares it against the year anyway, and rebuilding the
        // map annually for every realm costs more than the entries do.
        let year = w.year;
        if (year as usize).wrapping_add(p) % TRUCE_SWEEP == 0 {
            w.polities[p].truce.retain(|_, &mut until| until > year);
        }
        // Everyone fears the biggest realm in the world, whether or not they
        // share a border with it. This is deliberately not limited to
        // neighbours: a distant power that has swallowed a quarter of the map
        // is everybody's problem, and it is what gets a coalition started.
        if let Some(h) = top {
            if h != p && contained.contains(&p) && w.polities[p].overlord != Some(h) {
                let fear = tn.containment_tension * (top_share / tn.hegemon_share.max(0.01));
                let cur = w.polities[p].tension.get(&h).copied().unwrap_or(0.1);
                w.polities[p]
                    .tension
                    .insert(h, (cur + fear).clamp(0.0, 1.0));
            }
        }
    }
    // The world notices, once, when one realm has become the question of
    // the age. This is the line that tells a reader the shape of the
    // century they are about to read.
    if let Some(h) = top {
        if w.polities[h].hegemon_since.is_none() {
            w.polities[h].hegemon_since = Some(w.year);
            let text = prose::hegemon_recognised(w, h, top_share, &Pick::rolled(&rng));
            w.log(
                3,
                EventKind::Politics,
                &[Ref::Polity(h)],
                w.capital_cell(h),
                text,
            );
        }
    }
    // Declarations.
    for &p in &alive {
        if !w.polities[p].alive() || w.polities[p].wars.len() >= 2 || w.polities[p].stability < 0.3
        {
            continue;
        }

        let traits = politics::traits_of(w, p);
        // Only quarrels hot enough to become a war are worth examining.
        // The cheap numeric bar comes first and thins the list by an order
        // of magnitude; the map lookups that follow are what this loop
        // spends its time on, and there is no sense doing them for a border
        // nobody is angry about. (This ordering is the difference between
        // diplomacy costing a third of the tick on a large map and costing
        // a twentieth of it.)
        let mut targets: Vec<(usize, f32)> = w.polities[p]
            .tension
            .iter()
            .filter(|&(_, &t)| t >= tn.war_declare_threshold)
            .map(|(&q, &t)| (q, t))
            .collect();
        if targets.is_empty() {
            continue;
        }
        // Hottest first, so a realm picks its worst quarrel rather than
        // whichever id happens to sort lowest.
        targets.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        for (q, _tension) in targets {
            if !w.polities[q].alive() || w.war_between(p, q).is_some() {
                continue;
            }
            // You do not declare war on your own ally, your own tributary or
            // the house you have just married into.
            match super::dynasty::stance_of(w, p, q) {
                Stance::Allied | Stance::Married => continue,
                _ => {}
            }
            // You do not declare on your own client, nor on your overlord
            // by ordinary means — shaking off an overlord is a revolt, and
            // `tribute` is where that is decided.
            if w.polities[q].overlord == Some(p) || w.polities[p].overlord == Some(q) {
                continue;
            }
            // Nor on somebody who answers to the same overlord you do.
            if w.polities[p].overlord.is_some() && w.polities[p].overlord == w.polities[q].overlord
            {
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
            // A realm weighs its own army against the target's — except
            // when the target is the power everybody fears, in which case it
            // weighs the army it can expect to have beside it. Without this
            // no single realm was ever bold enough to move against a
            // hegemon, so the coalitions formed and then did nothing.
            let against_hegemon = Some(q) == top;
            let my = if against_hegemon {
                w.polities[p].army
                    + w.polities[p]
                        .stance
                        .iter()
                        .filter(|&(&x, &st)| {
                            st == Stance::Allied
                                && w.polities[x].alive()
                                && x != q
                                && w.polities[x].overlord != Some(q)
                        })
                        .take(MAX_JOINERS_PER_SIDE)
                        .map(|(&x, _)| w.polities[x].army * 0.7)
                        .sum::<f32>()
            } else {
                w.polities[p].army
            };
            let their = w.polities[q].army * (1.0 + w.polities[q].wars.len() as f32 * 0.2);
            let bold = (tn.war_boldness_base + traits.valor * 0.5)
                * if against_hegemon { 0.7 } else { 1.0 };
            if my < their * bold * rng.range32(0.7, 1.2) {
                continue;
            }
            if !rng.chance(
                tn.war_declare_chance + traits.ambition as f64 * tn.war_declare_ambition_weight,
            ) {
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
            let aim = choose_aim(w, p, q, holy);
            let cause = prose::war_cause(w, p, q, holy, &Pick::rolled(&rng));
            let wid = w.wars_start_aim(p, q, kind, aim, cause.clone());
            let war_name = w.wars[wid].name.clone();
            let text = prose::war_declared_aim(w, p, q, kind, &cause, &war_name, aim);
            w.log(
                2,
                EventKind::War,
                &[Ref::War(wid), Ref::Polity(p), Ref::Polity(q)],
                w.capital_cell(q),
                text,
            );
            call_allies(w, wid);
            break;
        }
    }
}

/// Alliances form and lapse.
///
/// The rule is the oldest one in diplomacy: two realms that fear the same
/// third realm more than they fear each other will come to an arrangement.
/// A hegemon therefore manufactures its own opposition just by existing.
pub fn alliances(w: &mut World) {
    let top = hegemon(w);
    let rng = w.rng.clone();
    let tn = w.tuning;
    for p in w.living_polities() {
        // Old arrangements lapse when the reason for them has gone.
        let held: Vec<(usize, Stance)> = w.polities[p]
            .stance
            .iter()
            .map(|(&q, &st)| (q, st))
            .collect();
        for (q, st) in held {
            if !w.polities[q].alive() {
                super::dynasty::set_stance(w, p, q, Stance::Neutral);
                continue;
            }
            if st == Stance::Allied {
                let age = w.year
                    - w.polities[p]
                        .stance_since
                        .get(&q)
                        .copied()
                        .unwrap_or(w.year);
                let tension = w.polities[p].tension.get(&q).copied().unwrap_or(0.0);
                if age > 40 && tension > 0.5 && rng.chance(0.2) {
                    super::dynasty::set_stance(w, p, q, Stance::Neutral);
                    let text = prose::alliance_lapsed(w, p, q);
                    w.log(
                        1,
                        EventKind::Politics,
                        &[Ref::Polity(p), Ref::Polity(q)],
                        w.capital_cell(p),
                        text,
                    );
                }
            }
        }
        if w.polities[p]
            .stance
            .values()
            .filter(|&&s| s == Stance::Allied)
            .count()
            >= 3
        {
            continue;
        }
        if !rng.chance(tn.alliance_chance) {
            continue;
        }
        // Who is worth an alliance: somebody whose worst enemy is our worst
        // enemy, and who is not currently our problem.
        let my_worst = w.polities[p]
            .tension
            .iter()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(&q, _)| q)
            .or(top);
        let threat = match my_worst {
            Some(t) if t != p => t,
            _ => continue,
        };
        let candidates: Vec<usize> = w.polities[p]
            .neighbors
            .iter()
            .map(|&(q, _)| q)
            .chain(w.polities[threat].neighbors.iter().map(|&(q, _)| q))
            .filter(|&q| {
                q != p
                    && q != threat
                    && w.polities[q].alive()
                    && w.war_between(p, q).is_none()
                    && w.polities[p].tension.get(&q).copied().unwrap_or(0.0) < 0.35
                    && super::dynasty::stance_of(w, p, q) == Stance::Neutral
                    && w.polities[q].tension.get(&threat).copied().unwrap_or(0.0) > 0.3
            })
            .collect();
        let mut candidates = candidates;
        candidates.sort_unstable();
        candidates.dedup();
        if candidates.is_empty() {
            continue;
        }
        let q = candidates[rng.below(candidates.len())];
        super::dynasty::set_stance(w, p, q, Stance::Allied);
        let against_hegemon = Some(threat) == top;
        let text = prose::alliance_made(w, p, q, threat, against_hegemon);
        w.log(
            if against_hegemon { 2 } else { 1 },
            EventKind::Politics,
            &[Ref::Polity(p), Ref::Polity(q), Ref::Polity(threat)],
            w.capital_cell(p),
            text,
        );
    }
}

/// Tributaries pay, and sometimes stop paying.
///
/// Tribute is what lets a great power be great on the map without being
/// wrecked by its own administrative limits: a tributary is counted as
/// somebody else's problem to govern and the overlord takes the money. It
/// is also a standing invitation to rebellion the moment the overlord looks
/// weak, which is where a hegemon's unravelling usually starts.
pub fn tribute(w: &mut World) {
    let rng = w.rng.clone();
    let tn = w.tuning;
    for p in w.living_polities() {
        let over = match w.polities[p].overlord {
            Some(o) if w.polities[o].alive() => o,
            Some(_) => {
                // The overlord is gone: the obligation dies with it.
                release_tributary(w, p);
                continue;
            }
            None => continue,
        };
        let due = (w.polities[p].treasury * tn.tribute_share).max(0.0);
        w.polities[p].treasury -= due;
        w.polities[over].treasury = (w.polities[over].treasury + due).min(600.0);
        w.polities[over].prestige += 0.05;
        // A tributary tests the grip when the overlord is weak, stretched or
        // already fighting somebody else.
        let overlord_weak = w.polities[over].stability < 0.45
            || w.polities[over].at_war()
            || w.polities[over].overextension(&tn) > 1.2;
        let mine = strength(w, p, &[]);
        let theirs = strength(w, over, &[]);
        let chance = tn.tributary_revolt_chance
            * if overlord_weak { 4.0 } else { 1.0 }
            * (mine / theirs.max(0.01)).clamp(0.2, 3.0) as f64;
        if !rng.chance(chance) {
            continue;
        }
        let cause = prose::tribute_refused(w, p, over);
        release_tributary(w, p);
        let wid = w.wars_start_aim(
            p,
            over,
            WarKind::Rebellion,
            WarAim::Independence,
            cause.clone(),
        );
        let name = w.wars[wid].name.clone();
        let text = prose::tribute_revolt(w, p, over, &name);
        w.log(
            2,
            EventKind::War,
            &[Ref::War(wid), Ref::Polity(p), Ref::Polity(over)],
            w.capital_cell(p),
            text,
        );
        call_allies(w, wid);
    }
}

/// Make `p` a tributary of `over`, both sides in step.
pub fn make_tributary(w: &mut World, p: usize, over: usize) {
    release_tributary(w, p);
    w.polities[p].overlord = Some(over);
    if !w.polities[over].tributaries.contains(&p) {
        w.polities[over].tributaries.push(p);
    }
}

/// End `p`'s obligation to whoever held it.
pub fn release_tributary(w: &mut World, p: usize) {
    if let Some(o) = w.polities[p].overlord.take() {
        w.polities[o].tributaries.retain(|&x| x != p);
    }
}

/// What `p` says its war on `q` is for.
///
/// Drawn once at the declaration and never changed, so the peace has
/// something definite to settle and the chronicle has something definite to
/// promise. Exactly one draw from the world RNG, whichever arm is taken.
fn choose_aim(w: &World, p: usize, q: usize, holy: bool) -> WarAim {
    let rng = &w.rng;
    let roll = rng.f64();
    // Throwing off an overlord is not a choice; nor is being the one realm
    // everybody has decided to pull down.
    if w.polities[p].overlord == Some(q) {
        return WarAim::Independence;
    }
    if holy {
        return WarAim::Faith;
    }
    if w.polities[p].kind == super::PolityKind::Horde {
        return if roll < 0.7 {
            WarAim::Plunder
        } else {
            WarAim::Border
        };
    }
    let share = super::dynasty::world_share(w, q);
    if share > w.tuning.hegemon_share && roll < 0.8 {
        return WarAim::Containment;
    }
    let mine = strength(w, p, &[]);
    let theirs = strength(w, q, &[]);
    // A realm that plainly outmatches its neighbour would rather have it
    // paying tribute than have it swallowed: cheaper to hold, and it keeps
    // the administrative burden on somebody else's books.
    if mine > theirs * 2.2 && w.polities[q].cells >= 12 && roll < 0.45 {
        return WarAim::Vassalage;
    }
    // A consort or a child with blood in the other house is a claim, and a
    // claim is the most specific war aim there is.
    if let Some(r) = w.polities[p].ruler {
        let claimant = w.persons[r]
            .children
            .iter()
            .chain(w.persons[r].spouse.iter())
            .copied()
            .find(|&c| {
                w.persons[c].alive()
                    && w.persons[c].house.is_some()
                    && w.persons[c].house == w.polities[q].house
            });
        if let Some(c) = claimant {
            if roll < 0.6 {
                return WarAim::Claimant(c);
            }
        }
    }
    // A named city on the border is worth a war of its own.
    if roll < 0.35 {
        let mut best: Option<(usize, f32)> = None;
        for &c in &w.polities[q].cities {
            if w.cities[c].destroyed.is_some() {
                continue;
            }
            let cell = w.cities[c].cell;
            let near = w
                .terrain
                .neighbors8(cell)
                .into_iter()
                .any(|nb| w.cells[nb].owner == Some(p));
            if !near {
                continue;
            }
            let worth = w.cities[c].pop * w.cities[c].prosperity;
            if best.map(|(_, b)| worth > b).unwrap_or(true) {
                best = Some((c, worth));
            }
        }
        if let Some((c, _)) = best {
            return WarAim::City(c);
        }
    }
    // A relic in the other realm's treasury.
    if roll < 0.42 {
        if let Some(&a) = w.artifacts_of(q).first() {
            return WarAim::Relic(a);
        }
    }
    WarAim::Border
}

/// Allies and rivals decide whether to come in.
///
/// Called the year a war is declared. An ally of a principal may answer the
/// call; a rival of a principal may take the chance to join the other side.
/// This is the whole of what makes a great war great.
pub fn call_allies(w: &mut World, wid: usize) {
    let rng = w.rng.clone();
    let tn = w.tuning;
    let (a, d) = (w.wars[wid].attacker, w.wars[wid].defender);
    let aim = w.wars[wid].aim;
    let containment = aim == WarAim::Containment;
    // Not every war is worth an ally's blood. A border raid or a grab at a
    // relic is the principals' own business; the calls go out when a realm
    // is to be made a client, a throne is to be handed to a foreigner, or
    // somebody has decided to pull the great power down. Without this gate
    // every quarrel became a world war and the largest army won all of them.
    let worth_a_call = containment
        || matches!(
            aim,
            WarAim::Vassalage | WarAim::Claimant(_) | WarAim::Independence
        )
        || super::dynasty::world_share(w, d) > tn.hegemon_share * 0.6
        || super::dynasty::world_share(w, a) > tn.hegemon_share * 0.6;
    if !worth_a_call {
        return;
    }
    for (side, principal, foe) in [(true, a, d), (false, d, a)] {
        let mut friends: Vec<(usize, bool)> = w.polities[principal]
            .stance
            .iter()
            .filter(|&(_, &st)| st == Stance::Allied || st == Stance::Married)
            .map(|(&q, &st)| (q, st == Stance::Allied))
            .collect();
        friends.sort_unstable();
        let mut joined = 0;
        for (q, sworn) in friends {
            // Two is a coalition; five is a mob, and it made the chronicle
            // unreadable as well as unbalanced.
            if joined >= MAX_JOINERS_PER_SIDE {
                break;
            }
            if !w.polities[q].alive() || q == foe || w.polities[q].wars.len() >= 2 {
                continue;
            }
            if w.war_between(q, foe).is_some() {
                continue;
            }
            if w.polities[q].overlord == Some(foe) {
                continue;
            }
            // Honouring a call is not automatic: a realm already stretched,
            // or one bound only by a marriage rather than an oath, stays
            // at home more often than not.
            let loyal = tn.ally_joins_chance
                * if sworn { 1.0 } else { 0.3 }
                * if w.polities[q].stability < 0.4 {
                    0.4
                } else {
                    1.0
                }
                * if containment { 1.6 } else { 1.0 };
            if !rng.chance(loyal.min(0.95)) {
                continue;
            }
            joined += 1;
            w.war_join(wid, q, side);
            let text = prose::ally_joined(w, q, principal, foe, &w.wars[wid].name.clone());
            w.log(
                2,
                EventKind::War,
                &[Ref::War(wid), Ref::Polity(q), Ref::Polity(principal)],
                w.capital_cell(q),
                text,
            );
        }
    }
}

pub fn resolve_wars(w: &mut World) {
    let rng = w.rng.clone();
    let tn = w.tuning;
    let active: Vec<usize> = w.wars.iter().filter(|x| x.alive()).map(|x| x.id).collect();
    if active.is_empty() {
        return;
    }
    // Front lines: for each war, cells of each side adjacent to the other.
    // A war with allies has a front wherever *any* member of one side meets
    // any member of the other, which is what makes a coalition a coalition
    // rather than several separate quarrels.
    let mut pairs: BTreeSet<(usize, usize)> = BTreeSet::new();
    for &i in &active {
        let (side_a, side_d) = w.war_sides(i);
        for &x in &side_a {
            for &y in &side_d {
                pairs.insert((x, y));
            }
        }
    }
    // (owner, enemy) -> cells of owner adjacent to enemy. Only a belligerent's
    // own land can hold a front, so walk that rather than the whole map; the
    // lists are sorted below, so the order the realms come in does not matter.
    let mut front: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    let belligerents: BTreeSet<usize> = pairs.iter().flat_map(|&(a, d)| [a, d]).collect();
    for p in belligerents {
        for &i in w.cells_of_ref(p) {
            for nb in w.terrain.neighbors8(i) {
                if let Some(q) = w.cells[nb].owner {
                    if q != p && (pairs.contains(&(p, q)) || pairs.contains(&(q, p))) {
                        front.entry((p, q)).or_default().push(i);
                    }
                }
            }
        }
    }
    // Amphibious contact. A crossing whose two shores are held by opposite
    // sides of a war puts each side's coast on the other's front, so the
    // war can actually be fought — before this, a war across water had no
    // front at all, its score decayed, and peace came within two years
    // whatever either side wanted.
    let reaches: Vec<u16> = (0..w.polities.len()).map(|p| w.sea_reach(p)).collect();
    for c in &w.terrain.crossings {
        let (a, b) = (c.from as usize, c.to as usize);
        let (Some(p), Some(q)) = (w.cells[a].owner, w.cells[b].owner) else {
            continue;
        };
        if p == q {
            continue;
        }
        if !pairs.contains(&(p, q)) && !pairs.contains(&(q, p)) {
            continue;
        }
        // Somebody has to be able to make the crossing. If only one side
        // can, only that side can attack over it — but both shores are a
        // front, because the one that cannot cross still has to defend.
        if c.width > reaches[p].max(reaches[q]) {
            continue;
        }
        front.entry((p, q)).or_default().push(a);
        front.entry((q, p)).or_default().push(b);
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
            w.end_war(wid, "ended.", false);
            continue;
        }
        let years = w.year - w.wars[wid].started;
        let (side_a, side_d) = w.war_sides(wid);
        let gather = |own: &[usize], foe: &[usize]| -> Vec<usize> {
            let mut v: Vec<usize> = Vec::new();
            for &x in own {
                for &y in foe {
                    if let Some(f) = front.get(&(x, y)) {
                        v.extend_from_slice(f);
                    }
                }
            }
            v.sort_unstable();
            v.dedup();
            v
        };
        let a_front = gather(&side_a, &side_d);
        let d_front = gather(&side_d, &side_a);
        let has_front = !a_front.is_empty() && !d_front.is_empty();
        if has_front {
            let rounds = if rng.chance(0.4) { 2 } else { 1 };
            for round in 0..rounds {
                if !w.wars[wid].alive() || !w.polities[a].alive() || !w.polities[d].alive() {
                    break;
                }
                battle(w, wid, &side_a, &side_d, &a_front, &d_front, round > 0);
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
        let mut p_peace = if years < tn.peace_min_years {
            0.0
        } else {
            tn.peace_base_chance
                + ex as f64 * tn.peace_exhaustion_weight
                + (score.abs() as f64 / 3.0).min(0.25)
        };
        if !has_front {
            p_peace += 0.2;
        }
        // A war nobody can win is a war somebody stops fighting. Once one
        // side has been hopelessly outmatched in the field for a couple of
        // years, or the attacker has no army worth the name, peace comes
        // quickly — otherwise a realm with two soldiers prosecutes a
        // seven-year war against sixty-seven.
        if years >= HOPELESS_MIN_YEARS {
            let (sa, sd) = (
                side_strength(w, &side_a, &[]),
                side_strength(w, &side_d, &[]),
            );
            if sa.min(sd) < sa.max(sd) * HOPELESS_RATIO {
                p_peace += 0.5;
            }
            if side_a.iter().all(|&x| w.polities[x].army < NEGLIGIBLE_ARMY) {
                p_peace += 0.35;
            }
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

/// The field strength of a whole side.
///
/// Allies do not merely appear in the chronicle: their armies are added to
/// the scale that decides battles. Without this a five-realm coalition
/// fought with one realm's army, which is why nothing could ever be done
/// about a realm that had grown too large.
fn side_strength(w: &World, side: &[usize], defending_cells: &[usize]) -> f32 {
    let total: f32 = side.iter().map(|&p| strength(w, p, defending_cells)).sum();
    // Coalitions are worth less than the sum of their parts: separate
    // commands, separate aims, and nobody willing to spend their own men
    // first. A single realm of the same size beats an alliance of four.
    let n = side.len().max(1) as f32;
    total * (1.0 - (n - 1.0) * 0.07).max(0.6)
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

/// One year's fighting on one front of one war.
///
/// Sides, not realms: `side_a` and `side_d` are whole coalitions, the
/// strength that decides the field is the sum of each side, and the losses
/// fall on everybody who was there. The two principals still supply the
/// names, because a war is remembered by who started it.
#[allow(clippy::too_many_arguments)]
fn battle(
    w: &mut World,
    wid: usize,
    side_a: &[usize],
    side_d: &[usize],
    a_front: &[usize],
    d_front: &[usize],
    quiet: bool,
) {
    let rng = w.rng.clone();
    let tn = w.tuning;
    let a = side_a[0];
    // Who takes the offensive this year?
    let sa = side_strength(w, side_a, &[]);
    let sd = side_strength(w, side_d, &[]);
    let attacker_offense = rng.chance(if sa >= sd { 0.7 } else { 0.35 });
    let (off_side, def_side, def_front) = if attacker_offense {
        (side_a, side_d, d_front)
    } else {
        (side_d, side_a, a_front)
    };
    let (off, def) = (off_side[0], def_side[0]);
    // The ground is chosen before the fighting is weighed, because where it
    // is decides how hard it is to get to.
    let site = def_front[rng.below(def_front.len())];
    // An assault nobody could have walked to is a landing, and a landing is
    // a far harder thing than a march: the defenders meet it at the water's
    // edge and the attackers arrive in the order their ships allow.
    let amphibious = !w
        .terrain
        .neighbors8(site)
        .any(|nb| matches!(w.cells[nb].owner, Some(o) if off_side.contains(&o)));
    let s_off = side_strength(w, off_side, &[]) * if amphibious { LANDING_PENALTY } else { 1.0 };
    let s_def = side_strength(w, def_side, def_front);
    let p_win = s_off / (s_off + s_def);
    let win = rng.chance((p_win as f64 + rng.normal() * 0.08).clamp(0.05, 0.95));
    let margin = ((s_off - s_def) / (s_off + s_def)).abs();
    let def_culture = w.cells[site].culture.unwrap_or(w.polities[def].culture);
    let place = battlefield_name(w, site, def_culture);
    w.wars[wid].battles += 1;
    let (winner_side, loser_side) = if win {
        (off_side, def_side)
    } else {
        (def_side, off_side)
    };
    let (winner, loser) = (winner_side[0], loser_side[0]);
    // Casualties fall on everyone who was on the field.
    for &x in loser_side {
        w.polities[x].army *= 1.0 - (tn.battle_loser_casualties + margin * 0.15);
        w.polities[x].exhaustion += 0.04;
    }
    for &x in winner_side {
        w.polities[x].army *= 1.0 - tn.battle_winner_casualties;
        w.polities[x].exhaustion += 0.02;
    }
    let swing = tn.battle_swing_base + margin * tn.battle_swing_margin_weight;
    if winner_side[0] == a {
        w.wars[wid].score += swing;
    } else {
        w.wars[wid].score -= swing;
    }
    for &x in winner_side {
        for &g in w.polities[x].generals.clone().iter() {
            if w.persons[g].alive() {
                w.persons[g].renown += 1.0;
                w.persons[g].battles_won += 1;
            }
        }
    }
    let mut importance = if winner_side.len() + loser_side.len() > 2 {
        2
    } else {
        1
    };
    let mut text = prose::battle_opening(w, off, def, &place, win);
    if amphibious {
        text.push_str(&prose::came_by_sea(w, off, win));
    }
    if winner_side.len() > 1 || loser_side.len() > 1 {
        text.push_str(&prose::battle_allies(w, winner_side, loser_side));
    }
    let mut refs = vec![Ref::War(wid), Ref::Polity(winner), Ref::Polity(loser)];
    // Territory changes: the winner takes cells from whichever realm on the
    // losing side actually holds them.
    let loser_front: Vec<usize> = if std::ptr::eq(loser_side, def_side) {
        def_front.to_vec()
    } else if std::ptr::eq(loser_side, side_a) {
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
            .filter(|&c| matches!(w.cells[c].owner, Some(o) if loser_side.contains(&o)))
            .collect();
        // Nearest to the battle site first.
        cells.sort_by_key(|&c| w.terrain.dist(c, site));
        for c in cells {
            if taken >= k {
                break;
            }
            let holder = match w.cells[c].owner {
                Some(o) => o,
                None => continue,
            };
            if let Some(city) = w.cells[c].city {
                // Siege.
                let walls = w.cities[city].walls;
                // What the walls are worth depends on what both sides know.
                // For as long as nobody has siege engines a walled city is
                // very nearly untakeable; the century they appear is the one
                // in which every realm that trusted its walls finds out it
                // was wrong.
                let siege = w.siege_advantage(winner, holder);
                let ratio = s_off.max(s_def) * siege / strength(w, holder, &[]).max(0.05);
                if ratio > 1.0 + walls * 0.6 && rng.chance(tn.siege_capture_chance) {
                    let s = capture_city(w, city, winner, holder, wid);
                    text.push_str(&s);
                    refs.push(Ref::City(city));
                    importance = 2;
                    taken += 1;
                } else if rng.chance(0.3) {
                    text.push_str(&prose::siege_withstood(w, city));
                    w.cities[city].walls = (walls - 0.05).max(0.0);
                }
                continue;
            }
            if !win && taken == 0 {
                text.push_str(&prose::pressed_advantage(w, winner));
            }
            claim(w, winner, c);
            w.credit_conquest(winner, 1);
            w.credit_conquest(holder, -1);
            if winner_side[0] == a {
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
            let p_die = if side == loser {
                tn.ruler_battle_death_loser
            } else {
                tn.ruler_battle_death_winner
            };
            if leads && rng.chance(p_die) {
                let name = w.persons[r].name.clone();
                politics::ruler_dies(w, side, &prose::fell_at(&place), 2);
                text.push_str(&prose::ruler_fell_in_battle(&name));
                importance = 2;
            }
        }
    }
    // Generals may die too.
    for &side in &[winner, loser] {
        for g in w.polities[side].generals.clone() {
            if w.persons[g].alive() && rng.chance(if side == loser { 0.06 } else { 0.015 }) {
                w.persons[g].died = Some(w.year);
                w.persons[g].death = prose::fell_at(&place);
                text.push_str(&prose::general_slain(w, g, side));
                refs.push(Ref::Person(g));
            }
        }
    }
    // The draws happen whatever the detail; only whether the line is
    // written depends on it (see `Detail`).
    if importance == 1 && rng.chance(0.25) {
        let flourish = prose::battle_flourish(w, winner, loser, &Pick::rolled(&rng));
        if w.high_detail() {
            text.push_str(&flourish);
        }
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
    w.credit_conquest(winner, 1);
    w.credit_conquest(loser, -1);
    w.credit_city_taken(winner);
    if winner == w.wars[wid].attacker {
        w.wars[wid].score += 0.3;
    } else {
        w.wars[wid].score -= 0.3;
    }
    let sack = rng
        .chance(w.tuning.city_sack_chance + t.cruelty as f64 * w.tuning.city_sack_cruelty_weight);
    let mut s = if sack {
        w.cities[city].pop *= 0.5;
        w.cities[city].times_sacked += 1;
        w.cities[city].prosperity *= 0.6;
        let lost = if !w.cities[city].wonders.is_empty() && rng.chance(0.5) {
            Some(w.cities[city].wonders.remove(0))
        } else {
            None
        };
        w.polities[winner].treasury += w.cities[city].pop * 2.0;
        prose::city_sacked(w, &name, winner, lost.as_deref())
    } else {
        prose::city_surrendered(w, &name, winner)
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
            .max_by(|&&x, &&y| w.cities[x].pop.total_cmp(&w.cities[y].pop))
        {
            w.polities[loser].capital = Some(nc);
            let to = w.cities[nc].name.clone();
            s.push_str(&prose::court_flees(w, loser, &to));
        } else {
            let cause = prose::conquered_by(w, loser, winner, &name);
            s.push_str(&prose::capital_lost(w, loser));
            politics::fall(w, loser, &cause, Some(winner), 2);
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
                let cause = prose::rebellion_crushed(w, a, d, kind, years);
                let leader = w.polities[a].ruler;
                politics::fall(w, a, &cause, Some(d), 2);
                if let Some(l) = leader {
                    if w.persons[l].alive() {
                        w.persons[l].died = Some(w.year);
                        w.persons[l].death = prose::rebel_executed().to_string();
                    }
                }
                w.credit_war_won(d);
                return;
            } else if score > 0.8
                && kind != WarKind::Rebellion
                && w.polities[a].cells > w.polities[d].cells
            {
                // Rebels take over the old realm.
                let cause = prose::rebellion_triumphant(w, d, a);
                politics::fall(w, d, &cause, Some(a), 2);
                return;
            } else {
                prose::independence_recognised(w, a, d)
            }
        }
        _ => {
            if score > 0.5 {
                w.credit_war_won(a);
                w.polities[a].prestige += 5.0;
                w.polities[d].prestige -= 3.0;
                let tribute = (w.polities[d].treasury * 0.3).max(0.0);
                w.polities[d].treasury -= tribute;
                w.polities[a].treasury += tribute;
                settle_aim(w, wid, a, d)
            } else if score < -0.5 {
                w.credit_war_won(d);
                w.polities[d].prestige += 5.0;
                w.polities[a].prestige -= 3.0;
                prose::peace_defender_won_aim(w, a, d, years, w.wars[wid].aim)
            } else {
                // Even a draw now says what was not achieved, which is a
                // verdict where "neither side was master" was only a shrug.
                prose::peace_stalemate_aim(w, a, d, years, w.wars[wid].aim)
            }
        }
    };
    w.end_war(wid, &result, true);
}

/// Carry out what the attacker said the war was for.
///
/// This is the half of the war model that was missing: a victory used to
/// mean a treasury transfer and a sentence about tribute that transferred
/// nothing. Now each aim has a consequence on the map or in the diplomatic
/// record, and the peace line reports whether the promise was kept.
fn settle_aim(w: &mut World, wid: usize, a: usize, d: usize) -> String {
    let aim = w.wars[wid].aim;
    let years = w.year - w.wars[wid].started;
    let mut met = true;
    match aim {
        WarAim::Vassalage => {
            make_tributary(w, d, a);
            w.polities[a].prestige += 8.0;
        }
        WarAim::Independence => {
            release_tributary(w, a);
        }
        WarAim::Faith => {
            if let Some(s) = w.polities[a].school {
                let e = w.schools[s].influence.entry(d).or_insert(0.0);
                *e = (*e + 0.45).min(1.0);
                if w.polities[d].school.is_none() {
                    w.polities[d].school = Some(s);
                    w.schools[s].state_of.push(d);
                }
            } else {
                met = false;
            }
        }
        WarAim::Claimant(r) => {
            // Installing a claimant is the one victory that changes who a
            // realm *is* without changing where its borders run.
            if w.persons[r].alive() {
                if let Some(old) = w.polities[d].ruler {
                    w.persons[old].died = Some(w.year);
                    w.persons[old].death = prose::deposed().to_string();
                }
                let old_house = w.polities[d].house;
                politics::install_ruler(w, d, r);
                if let Some(h) = w.persons[r].house {
                    w.seat_house(d, h);
                }
                if let Some(oh) = old_house {
                    w.close_house_if_spent(oh);
                }
                w.polities[d].stability = (w.polities[d].stability - 0.15).max(0.0);
                super::dynasty::set_stance(w, a, d, Stance::Married);
            } else {
                met = false;
            }
        }
        WarAim::Relic(x) => {
            if matches!(w.artifacts[x].holder, super::Holder::Polity(h) if h == d) {
                let how = prose::relic_taken_as_spoils(w, a);
                super::stories::artifact_passes(w, x, super::Holder::Polity(a), &how);
            } else {
                met = false;
            }
        }
        WarAim::Plunder => {
            let loot = (w.polities[d].treasury * 0.4).max(0.0) + w.polities[d].cells as f32 * 0.2;
            w.polities[d].treasury -= loot * 0.5;
            w.polities[a].treasury = (w.polities[a].treasury + loot).min(600.0);
        }
        WarAim::Containment => {
            // The point of a coalition is not to take the land but to break
            // the grip: the hegemon loses its tributaries and its standing.
            let freed: Vec<usize> = w.polities[d].tributaries.clone();
            for t in freed {
                release_tributary(w, t);
            }
            w.polities[d].prestige *= 0.5;
            w.polities[d].stability = (w.polities[d].stability - 0.2).max(0.0);
        }
        WarAim::City(c) => {
            met = w.cities[c].polity == Some(a);
        }
        WarAim::Border => {
            met = w.wars[wid].cells_taken > 0;
        }
    }
    w.wars[wid].aim_met = met;
    prose::peace_attacker_won_aim(w, a, d, years, aim, met)
}

pub fn role_general(w: &mut World, p: usize, g: usize) {
    w.persons[g].role = Role::General;
    w.persons[g].polity = Some(p);
    w.polities[p].generals.push(g);
    w.polities[p].generals.retain(|&x| w.persons[x].alive());
}
