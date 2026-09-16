//! Deeper stories: prophecies that come true or fail, artifacts that
//! change hands across the centuries, tyrants, heroes and legends.

use super::chronicle::{EventKind, Ref};
use super::prose::{self, Pick};
use super::{Artifact, ArtifactKind, Holder, PolityKind, Prophecy, ProphecyKind, Role, World};

const ART_ADJ: &[&str] = &[
    "Sunless",
    "Weeping",
    "Ninefold",
    "Iron",
    "Ashen",
    "Starlit",
    "Serpent",
    "Hollow",
    "Bright",
    "Black",
    "Unsleeping",
    "Glass",
    "Thunder",
    "Moon",
    "Ember",
    "Silent",
];

impl ArtifactKind {
    pub fn word(self) -> &'static str {
        match self {
            ArtifactKind::Crown => "Crown",
            ArtifactKind::Blade => "Blade",
            ArtifactKind::Tome => "Tome",
            ArtifactKind::Gem => "Gem",
            ArtifactKind::Banner => "Banner",
            ArtifactKind::Staff => "Staff",
            ArtifactKind::Chalice => "Chalice",
            ArtifactKind::Horn => "Horn",
            ArtifactKind::Mirror => "Mirror",
            ArtifactKind::Shard => "Shard",
        }
    }
}

impl World {
    /// Artifacts currently held by a polity (in its treasury or by its ruler/capital).
    pub fn artifacts_of(&self, p: usize) -> Vec<usize> {
        self.artifacts
            .iter()
            .filter(|a| match a.holder {
                Holder::Polity(x) => x == p,
                Holder::Person(per) => {
                    self.persons[per].alive()
                        && self.persons[per].polity == Some(p)
                        && self.polities[p].ruler == Some(per)
                }
                Holder::City(c) => self.cities[c].polity == Some(p),
                Holder::Lost => false,
            })
            .map(|a| a.id)
            .collect()
    }

    pub fn artifact_holder_name(&self, a: usize) -> String {
        match self.artifacts[a].holder {
            Holder::Polity(p) => self.polities[p].name.clone(),
            Holder::Person(per) => self.persons[per].full_name(),
            Holder::City(c) => self.cities[c].name.clone(),
            Holder::Lost => "no one; it is lost".to_string(),
        }
    }

    pub fn artifact_loc(&self, a: usize) -> Option<usize> {
        match self.artifacts[a].holder {
            Holder::Polity(p) => self.capital_cell(p),
            Holder::Person(per) => self.persons[per].polity.and_then(|p| self.capital_cell(p)),
            Holder::City(c) => Some(self.cities[c].cell),
            Holder::Lost => self.artifacts[a].lost_at,
        }
    }
}

/// Create an artifact and log its making.
pub fn make_artifact(
    w: &mut World,
    kind: ArtifactKind,
    maker: Option<usize>,
    polity: Option<usize>,
    holder: Holder,
    occasion: &str,
) -> usize {
    let rng = w.rng.clone();
    let id = w.artifacts.len();
    let lang = polity
        .map(|p| w.polity_lang(p).clone())
        .or_else(|| maker.map(|m| w.cultures[w.persons[m].culture].lang.clone()))
        .unwrap_or_else(|| w.cultures[0].lang.clone());
    let adj = *rng.pick(ART_ADJ);
    let name = match rng.below(4) {
        0 => format!("the {} {}", adj, kind.word()),
        1 => format!("the {} of {}", kind.word(), lang.name(&rng)),
        2 => match maker {
            Some(m) => format!("the {} of {}", kind.word(), w.persons[m].name),
            None => format!("the {} {}", adj, kind.word()),
        },
        _ => format!(
            "the {} {} of {}",
            adj,
            kind.word(),
            polity
                .and_then(|p| w.polities[p].capital)
                .map(|c| w.cities[c].name.clone())
                .unwrap_or_else(|| lang.name(&rng))
        ),
    };
    let description = prose::artifact_description(kind);
    let lost_at = if let Holder::Lost = holder {
        polity.and_then(|p| w.capital_cell(p))
    } else {
        None
    };
    w.artifacts.push(Artifact {
        id,
        name: name.clone(),
        kind,
        made: w.year,
        maker,
        origin: polity,
        holder,
        power: rng.range32(0.3, 1.0),
        description: description.clone(),
        lost_at,
        hands: 0,
    });
    let text = prose::artifact_made(occasion, &name, &description);
    let mut refs = vec![Ref::Artifact(id)];
    if let Some(m) = maker {
        refs.push(Ref::Person(m));
    }
    if let Some(p) = polity {
        refs.push(Ref::Polity(p));
    }
    let loc = w.artifact_loc(id);
    w.log(1, EventKind::Wonder, &refs, loc, text);
    id
}

/// Move an artifact to a new holder, logging it.
pub fn artifact_passes(w: &mut World, a: usize, to: Holder, how: &str) {
    w.artifacts[a].holder = to;
    w.artifacts[a].hands += 1;
    if let Holder::Lost = to {
        w.artifacts[a].lost_at = None;
    }
    let name = w.artifacts[a].name.clone();
    let mut refs = vec![Ref::Artifact(a)];
    match to {
        Holder::Polity(p) => refs.push(Ref::Polity(p)),
        Holder::Person(per) => refs.push(Ref::Person(per)),
        Holder::City(c) => refs.push(Ref::City(c)),
        Holder::Lost => {}
    }
    let loc = w.artifact_loc(a);
    let text = prose::artifact_passed(&name, how);
    w.log(1, EventKind::Wonder, &refs, loc, text);
}

/// Called when a polity captures a city: artifacts there are carried off.
pub fn artifacts_on_capture(w: &mut World, city: usize, winner: usize, loser: usize) -> String {
    let was_capital =
        w.polities[loser].capital == Some(city) || w.polities[loser].capital.is_none();
    let taken: Vec<usize> = w
        .artifacts
        .iter()
        .filter(|a| match a.holder {
            Holder::City(c) => c == city,
            Holder::Polity(p) => p == loser && was_capital,
            Holder::Person(per) => was_capital && w.polities[loser].ruler == Some(per),
            Holder::Lost => false,
        })
        .map(|a| a.id)
        .collect();
    let mut s = String::new();
    for a in taken {
        let name = w.artifacts[a].name.clone();
        if w.rng.chance(w.tuning.artifact_capture_lost_chance) {
            w.artifacts[a].lost_at = Some(w.cities[city].cell);
            let how = prose::relic_vanished_in_sack(&w.cities[city].name.clone());
            artifact_passes(w, a, Holder::Lost, &how);
            s.push_str(&prose::relic_lost_aside(&name));
        } else {
            let how = prose::relic_taken_as_spoils(w, winner);
            artifact_passes(w, a, Holder::Polity(winner), &how);
            s.push_str(&prose::relic_taken_aside(&name));
        }
    }
    s
}

/// Called when a polity falls: its artifacts pass to the conqueror or are lost.
pub fn artifacts_on_fall(w: &mut World, p: usize, absorbed_by: Option<usize>) {
    for a in w.artifacts_of(p) {
        match absorbed_by {
            Some(q) if w.rng.chance(w.tuning.artifact_fall_pass_chance) => {
                let how = prose::relic_passed_with_fall(w, q, p);
                artifact_passes(w, a, Holder::Polity(q), &how);
            }
            _ => {
                w.artifacts[a].lost_at = w.capital_cell(p);
                let how = prose::relic_lost_in_ruin(w, p);
                artifact_passes(w, a, Holder::Lost, &how);
            }
        }
    }
}

/// Called when a ruler dies: crowns pass to the realm, great rulers leave relics.
pub fn artifacts_on_ruler_death(w: &mut World, p: usize, r: usize) {
    let rng = w.rng.clone();
    for a in w.artifacts_of(p) {
        if let Holder::Person(per) = w.artifacts[a].holder {
            if per == r {
                w.artifacts[a].holder = Holder::Polity(p);
            }
        }
    }
    let epithet = w.persons[r].epithet.clone().unwrap_or_default();
    let great = matches!(
        epithet.as_str(),
        "the Great"
            | "the Conqueror"
            | "the Wise"
            | "the Lawgiver"
            | "the Builder"
            | "the Victorious"
            | "the Lion"
    );
    if great
        && rng.chance(w.tuning.artifact_ruler_chance)
        && w.artifacts_of(p).len() < w.tuning.artifact_per_polity_cap
        && w.artifacts.len()
            < w.tuning.artifact_world_cap_base
                + w.living_polities().len() / w.tuning.artifact_world_cap_divisor
    {
        let kind = *rng.pick(&[
            ArtifactKind::Crown,
            ArtifactKind::Blade,
            ArtifactKind::Banner,
            ArtifactKind::Horn,
        ]);
        let name = w.persons[r].full_name();
        let capital = w.polities[p]
            .capital
            .map(|c| w.cities[c].name.clone())
            .unwrap_or_else(|| w.polities[p].short.clone());
        make_artifact(
            w,
            kind,
            Some(r),
            Some(p),
            Holder::Polity(p),
            &prose::occasion_ruler_memorial(&name, &capital),
        );
    }
}

pub fn artifacts_on_wonder(w: &mut World, p: usize, cap: usize) {
    if w.rng.chance(w.tuning.artifact_wonder_chance)
        && w.artifacts_of(p).len() < w.tuning.artifact_per_polity_cap
        && w.artifacts.len()
            < w.tuning.artifact_world_cap_base
                + w.living_polities().len() / w.tuning.artifact_world_cap_divisor
    {
        let kind = *w.rng.pick(&[
            ArtifactKind::Gem,
            ArtifactKind::Chalice,
            ArtifactKind::Mirror,
            ArtifactKind::Crown,
        ]);
        let city = w.cities[cap].name.clone();
        make_artifact(
            w,
            kind,
            w.polities[p].ruler,
            Some(p),
            Holder::City(cap),
            &prose::occasion_wonder(&city),
        );
    }
}

pub fn artifacts_on_school(w: &mut World, s: usize) {
    let founder = w.schools[s].founder;
    let city = w.schools[s].home_city;
    let polity = w.cities[city].polity;
    if w.rng.chance(w.tuning.artifact_school_chance) {
        let kind = if w.schools[s].kind == super::SchoolKind::Arcane {
            *w.rng.pick(&[
                ArtifactKind::Staff,
                ArtifactKind::Tome,
                ArtifactKind::Mirror,
            ])
        } else {
            *w.rng.pick(&[
                ArtifactKind::Tome,
                ArtifactKind::Chalice,
                ArtifactKind::Horn,
            ])
        };
        let sname = w.schools[s].short.clone();
        make_artifact(
            w,
            kind,
            Some(founder),
            polity,
            Holder::City(city),
            &prose::occasion_school(w.schools[s].kind.follower(), &sname),
        );
    }
}

pub fn artifacts_on_catastrophe(w: &mut World, cell: usize, s: usize) {
    let founder = w.schools[s].founder;
    let id = make_artifact(
        w,
        ArtifactKind::Shard,
        Some(founder),
        None,
        Holder::Lost,
        prose::occasion_catastrophe(),
    );
    w.artifacts[id].lost_at = Some(cell);
}

// ---------------------------------------------------------------------------
// Yearly: lost relics are found, relics are stolen, holders gain a little.
// ---------------------------------------------------------------------------

pub fn tick_artifacts(w: &mut World) {
    let rng = w.rng.clone();
    for a in 0..w.artifacts.len() {
        match w.artifacts[a].holder {
            Holder::Lost => {
                if !rng.chance(w.tuning.artifact_found_chance) {
                    continue;
                }
                // Found by whoever now rules the ground it was lost on, or a nearby realm.
                let at = match w.artifacts[a].lost_at {
                    Some(c) => c,
                    None => continue,
                };
                let finder = w.cells[at]
                    .owner
                    .filter(|&p| w.polities[p].alive())
                    .or_else(|| {
                        let mut best = None;
                        let mut best_d = 30;
                        for p in w.living_polities() {
                            if let Some(c) = w.capital_cell(p) {
                                let d = w.terrain.dist(c, at);
                                if d < best_d {
                                    best_d = d;
                                    best = Some(p);
                                }
                            }
                        }
                        best
                    });
                if let Some(p) = finder {
                    let finder_desc = prose::finders(&Pick::rolled(&rng));
                    let place = w.place_phrase(at, Some(p));
                    let buried = w.year - w.artifacts[a].made;
                    let how = prose::relic_found(w, p, &finder_desc, &place, buried);
                    artifact_passes(w, a, Holder::Polity(p), &how);
                }
            }
            Holder::Polity(_) | Holder::City(_) if rng.chance(w.tuning.artifact_lost_chance) => {
                let p = match w.artifacts[a].holder {
                    Holder::Polity(p) => p,
                    Holder::City(c) => match w.cities[c].polity {
                        Some(p) => p,
                        None => continue,
                    },
                    _ => continue,
                };
                if !w.polities[p].alive() {
                    continue;
                }
                let at = w.artifact_loc(a);
                w.artifacts[a].lost_at = at;
                let how = prose::relic_stolen(w, p, &Pick::rolled(&rng));
                artifact_passes(w, a, Holder::Lost, &how);
            }
            Holder::Person(per) if !w.persons[per].alive() => {
                // A dead holder outside the succession path: the relic is lost.
                w.artifacts[a].lost_at = w.persons[per].polity.and_then(|p| w.capital_cell(p));
                let how = prose::relic_buried_with(&w.persons[per].full_name());
                artifact_passes(w, a, Holder::Lost, &how);
            }
            _ => {}
        }
    }
    // Holders gain a little from their relics.
    for p in w.living_polities() {
        let held = w.artifacts_of(p);
        if held.is_empty() {
            continue;
        }
        let mut stab = 0.0;
        let mut prestige = 0.0;
        for &a in &held {
            let pw = w.artifacts[a].power;
            prestige += pw * 0.4;
            match w.artifacts[a].kind {
                ArtifactKind::Crown | ArtifactKind::Banner | ArtifactKind::Horn => {
                    stab += 0.01 * pw
                }
                ArtifactKind::Chalice | ArtifactKind::Mirror => stab += 0.005 * pw,
                _ => {}
            }
        }
        let pol = &mut w.polities[p];
        pol.stability = (pol.stability + stab).min(1.0);
        pol.prestige += prestige;
    }
}

/// Army multiplier from relics.
impl World {
    pub fn artifact_army_mult(&self, p: usize) -> f32 {
        let mut m = 1.0;
        for a in self.artifacts_of(p) {
            if matches!(
                self.artifacts[a].kind,
                ArtifactKind::Blade | ArtifactKind::Banner
            ) {
                m += 0.06 * self.artifacts[a].power;
            }
        }
        m
    }
}

// ---------------------------------------------------------------------------
// Prophecies
// ---------------------------------------------------------------------------

pub fn utter_prophecy(w: &mut World, seer: usize, p: usize) -> Option<usize> {
    let rng = w.rng.clone();
    if !w.polities[p].alive() {
        return None;
    }
    let mut kinds: Vec<(f64, ProphecyKind)> = Vec::new();
    let stab = w.polities[p].stability as f64;
    kinds.push((0.4 + (1.0 - stab) * 1.6, ProphecyKind::RealmFalls(p)));
    if w.polities[p].kind != PolityKind::Empire && w.polities[p].cells > 40 {
        kinds.push((1.0, ProphecyKind::CrownOfEmpire(p)));
    }
    if let Some(cap) = w.polities[p].capital {
        kinds.push((
            if w.polities[p].at_war() { 1.2 } else { 0.4 },
            ProphecyKind::CityBurns(cap, w.cities[cap].times_sacked),
        ));
    }
    kinds.push((0.6, ProphecyKind::RulerMurdered(p)));
    if let Some(s) = w.polities[p].school {
        let n = w.schools[s]
            .influence
            .values()
            .filter(|v| **v > 0.3)
            .count();
        kinds.push((0.7, ProphecyKind::FaithSpreads(s, n + 2 + rng.below(3))));
    }
    let lost: Vec<usize> = w
        .artifacts
        .iter()
        .filter(|a| matches!(a.holder, Holder::Lost) && a.origin == Some(p))
        .map(|a| a.id)
        .collect();
    if let Some(&a) = lost.first() {
        kinds.push((1.5, ProphecyKind::RelicReturns(a, p)));
    }
    // Neighbours make good villains.
    if let Some(&(q, _)) = w.polities[p].neighbors.first() {
        if w.polities[q].alive() {
            kinds.push((
                0.3 + (1.0 - w.polities[q].stability as f64) * 1.2,
                ProphecyKind::RealmFalls(q),
            ));
        }
    }
    let weights: Vec<f64> = kinds.iter().map(|k| k.0).collect();
    let kind = kinds[rng.weighted(&weights)].1;
    let years = w.tuning.prophecy_deadline_min
        + rng.below(w.tuning.prophecy_deadline_range as usize) as i32;
    let deadline = w.year + years;
    let seer_name = w.persons[seer].name.clone();
    let where_ = w.persons[seer]
        .city
        .map(|c| w.cities[c].name.clone())
        .unwrap_or_else(|| w.polities[p].short.clone());
    let what = prose::prophecy_what(w, kind);
    let opening = prose::prophecy_opening(&Pick::rolled(&rng));
    let text = prose::prophecy_uttered(&seer_name, &where_, &opening, &what, years);
    let id = w.prophecies.len();
    w.prophecies.push(Prophecy {
        id,
        seer,
        year: w.year,
        deadline,
        kind,
        what: what.clone(),
        outcome: None,
        resolved: None,
    });
    let mut refs = vec![Ref::Person(seer), Ref::Polity(p)];
    match kind {
        ProphecyKind::RealmFalls(q)
        | ProphecyKind::CrownOfEmpire(q)
        | ProphecyKind::RulerMurdered(q) => {
            if q != p {
                refs.push(Ref::Polity(q));
            }
        }
        ProphecyKind::CityBurns(c, _) => refs.push(Ref::City(c)),
        ProphecyKind::FaithSpreads(s, _) => refs.push(Ref::School(s)),
        ProphecyKind::RelicReturns(a, _) => refs.push(Ref::Artifact(a)),
    }
    w.log(1, EventKind::Magic, &refs, w.capital_cell(p), text);
    w.persons[seer].renown += 1.0;
    Some(id)
}

fn prophecy_met(w: &World, pr: &Prophecy) -> bool {
    match pr.kind {
        ProphecyKind::RealmFalls(q) => w.polities[q].fell.map(|y| y >= pr.year).unwrap_or(false),
        ProphecyKind::CrownOfEmpire(q) => w.polities[q].kind == PolityKind::Empire,
        ProphecyKind::CityBurns(c, sacked) => {
            w.cities[c].times_sacked > sacked
                || w.cities[c].destroyed.map(|y| y >= pr.year).unwrap_or(false)
        }
        ProphecyKind::RulerMurdered(q) => w.polities[q].rulers.iter().any(|&r| {
            w.persons[r].died.map(|y| y >= pr.year).unwrap_or(false)
                && (w.persons[r].death.contains("murdered")
                    || w.persons[r].death.contains("deposed"))
        }),
        ProphecyKind::FaithSpreads(s, n) => {
            w.schools[s]
                .influence
                .iter()
                .filter(|(p, v)| **v > 0.3 && w.polities[**p].alive())
                .count()
                >= n
        }
        ProphecyKind::RelicReturns(a, q) => w.polities[q].alive() && w.artifacts_of(q).contains(&a),
    }
}

pub fn tick_prophecies(w: &mut World) {
    for i in 0..w.prophecies.len() {
        if w.prophecies[i].outcome.is_some() {
            continue;
        }
        let met = prophecy_met(w, &w.prophecies[i]);
        let seer = w.prophecies[i].seer;
        let seer_name = w.persons[seer].full_name();
        let what = w.prophecies[i].what.clone();
        if met {
            w.prophecies[i].outcome = Some(true);
            w.prophecies[i].resolved = Some(w.year);
            let age = w.year - w.prophecies[i].year;
            let text = prose::prophecy_fulfilled(&seer_name, &what, age);
            w.persons[seer].renown += 3.0;
            if let Some(s) = w.persons[seer].school {
                if w.schools[s].alive() {
                    for v in w.schools[s].influence.values_mut() {
                        *v = (*v + 0.05).min(1.0);
                    }
                }
            }
            let refs = prophecy_refs(w, i);
            w.log(2, EventKind::Magic, &refs, None, text);
        } else if w.year >= w.prophecies[i].deadline {
            w.prophecies[i].outcome = Some(false);
            w.prophecies[i].resolved = Some(w.year);
            let rng = w.rng.clone();
            let text = prose::prophecy_failed(&seer_name, &what, &Pick::rolled(&rng));
            let refs = prophecy_refs(w, i);
            w.log(1, EventKind::Magic, &refs, None, text);
        }
    }
}

fn prophecy_refs(w: &World, i: usize) -> Vec<Ref> {
    let pr = &w.prophecies[i];
    let mut refs = vec![Ref::Person(pr.seer)];
    match pr.kind {
        ProphecyKind::RealmFalls(q)
        | ProphecyKind::CrownOfEmpire(q)
        | ProphecyKind::RulerMurdered(q) => refs.push(Ref::Polity(q)),
        ProphecyKind::CityBurns(c, _) => refs.push(Ref::City(c)),
        ProphecyKind::FaithSpreads(s, _) => refs.push(Ref::School(s)),
        ProphecyKind::RelicReturns(a, q) => {
            refs.push(Ref::Artifact(a));
            refs.push(Ref::Polity(q));
        }
    }
    let _ = w;
    refs
}

// ---------------------------------------------------------------------------
// Tyrants and legends
// ---------------------------------------------------------------------------

pub fn tick_legends(w: &mut World) {
    let rng = w.rng.clone();
    // Tyrants.
    for p in w.living_polities() {
        let r = match w.polities[p].ruler {
            Some(r) => r,
            None => continue,
        };
        let per = &w.persons[r];
        let reign = w.year - w.polities[p].reign_start;
        if per.traits.cruelty > 0.8
            && reign >= 8
            && per.epithet.is_none()
            && rng.chance(w.tuning.tyrant_chance)
        {
            w.persons[r].epithet = Some(
                rng.pick(&[
                    "the Tyrant",
                    "the Cruel",
                    "the Butcher",
                    "Bloodhand",
                    "the Merciless",
                ])
                .to_string(),
            );
            w.polities[p].stability = (w.polities[p].stability - 0.06).max(0.0);
            let n = 3 + rng.below(30);
            let text = prose::tyrant(w, p, r, n);
            w.log(
                1,
                EventKind::Politics,
                &[Ref::Person(r), Ref::Polity(p)],
                w.capital_cell(p),
                text,
            );
        }
    }
    // Legends: generals and notables of great renown are remembered at death.
    for i in 0..w.persons.len() {
        let per = &w.persons[i];
        if per.died != Some(w.year)
            || per.role == Role::Ruler
            || per.renown < 5.0
            || per.epithet.is_some()
        {
            continue;
        }
        let role = per.role;
        let epithet = match role {
            Role::General => *rng.pick(&[
                "the Unbroken",
                "Shieldbreaker",
                "the Hammer",
                "Stormcrow",
                "of the Hundred Battles",
            ]),
            Role::Mage => *rng.pick(&["the Undying", "Starbinder", "the Grey", "Nine-Tongued"]),
            Role::Prophet => *rng.pick(&["the Voice", "the Blessed", "of the Burning Eyes"]),
            Role::Poet => *rng.pick(&["Goldentongue", "the Nightingale", "the Mournful"]),
            Role::Explorer => *rng.pick(&["Far-Sailor", "of the Ends of the World", "Wavefinder"]),
            _ => *rng.pick(&["the Remembered", "the Great"]),
        };
        w.persons[i].epithet = Some(epithet.to_string());
        let culture = w.cultures[w.persons[i].culture].plural.clone();
        let deeds = w.chronicle.for_ref(Ref::Person(i)).len();
        let _ = deeds;
        let text = prose::legend_remembered(w, i, role.name(), &culture);
        w.log(2, EventKind::Death, &[Ref::Person(i)], None, text);
    }
}

// ---------------------------------------------------------------------------
// Family
// ---------------------------------------------------------------------------

impl World {
    pub fn children_of(&self, r: usize) -> Vec<usize> {
        self.persons
            .iter()
            .filter(|p| p.parent == Some(r))
            .map(|p| p.id)
            .collect()
    }

    /// "child of X, grandchild of Y"
    pub fn lineage(&self, r: usize) -> String {
        prose::lineage(self, r)
    }
}
