//! Creation of the world's races and first peoples, and the opening
//! lines of the chronicle.

use super::chronicle::{EventKind, Ref};
use super::prose::{self, genesis::FEATURE, genesis::STATURE};
use super::{tech, Culture, Era, Race, Values, World, GROUP_COUNT};
use crate::geo::FeatureKind;
use crate::lang::Language;
use crate::rng::Rng;
use crate::term::Rgb;

pub fn culture_color(id: usize) -> Rgb {
    let hue = (id as f32 * 97.3 + 40.0) % 360.0;
    Rgb::from_hsv(hue, 0.5, 0.9)
}

pub fn populate(w: &mut World) {
    let rng = w.rng.clone();
    let n_races = 4 + rng.below(3);
    let mut used_stature: Vec<usize> = Vec::new();
    let mut used_feature: Vec<usize> = Vec::new();
    let mut homes: Vec<usize> = Vec::new();
    let min_sep = (w.terrain.w.min(w.terrain.h * 2) / (n_races + 1)).max(8);

    // World name via a primordial tongue.
    let primordial = Language::generate(&rng);
    let world_name = primordial.name(&rng);
    w.eras.push(Era {
        start: 0,
        name: "the Dawn Age".to_string(),
        description: prose::dawn_age(&world_name),
        // No figures: the world had not begun, so this age is not counted
        // when later centuries are judged against the usual run of them.
        wars: 0,
        schools: 0,
        born: 0,
        realms: 0,
        fell: 0,
        wonders: 0,
    });
    w.log(
        3,
        EventKind::Genesis,
        &[],
        None,
        prose::first_words(&world_name),
    );

    for ri in 0..n_races {
        let lang = Language::generate(&rng);
        let name = lang.name(&rng);
        let adj = lang.adjective(&name);
        let plural = lang.demonym(&name);
        let mut affinity = [0.5f32; GROUP_COUNT];
        let mut groups: Vec<usize> = (0..GROUP_COUNT).collect();
        rng.shuffle(&mut groups);
        affinity[groups[0]] = 1.0;
        affinity[groups[1]] = rng.range32(0.6, 0.9);
        affinity[groups[2]] = rng.range32(0.35, 0.6);
        affinity[groups[GROUP_COUNT - 1]] = rng.range32(0.05, 0.25);
        let lifespan = match rng.below(10) {
            0 => rng.range32(38.0, 55.0),
            1 => rng.range32(150.0, 320.0),
            2 | 3 => rng.range32(95.0, 140.0),
            _ => rng.range32(58.0, 90.0),
        };
        let mut race = Race {
            id: ri,
            name: name.clone(),
            adj,
            plural,
            lang,
            lifespan,
            affinity,
            coast_love: rng.range32(0.0, 1.0),
            martial: rng.trait_value(0.5, 0.2),
            mystic: rng.trait_value(0.5, 0.2),
            mercantile: rng.trait_value(0.5, 0.2),
            fecund: rng.trait_value(0.5, 0.15),
            seafaring: rng.trait_value(0.4, 0.25),
            description: String::new(),
            home: 0,
        };
        if race.coast_love > 0.7 {
            race.seafaring = (race.seafaring + 0.3).min(1.0);
        }
        let si = loop {
            let i = rng.below(STATURE.len());
            if !used_stature.contains(&i) || used_stature.len() >= STATURE.len() {
                break i;
            }
        };
        let fi = loop {
            let i = rng.below(FEATURE.len());
            if !used_feature.contains(&i) || used_feature.len() >= FEATURE.len() {
                break i;
            }
        };
        used_stature.push(si);
        used_feature.push(fi);
        race.description = prose::race_description(&race, STATURE[si], FEATURE[fi]);

        // Choose a homeland: fertile, favoured biome, far from other homes.
        let mut best = None;
        let mut best_score = -1.0f32;
        for _ in 0..600 {
            let i = rng.below(w.terrain.n());
            if !w.terrain.is_land(i) {
                continue;
            }
            let b = w.terrain.biome[i];
            let aff = b.group().map(|g| race.affinity[g as usize]).unwrap_or(0.3);
            let mut score = w.terrain.fertility[i]
                * (0.4 + aff)
                * (1.0 + race.coast_love * if w.terrain.coast[i] { 0.6 } else { 0.0 });
            // Neighbourhood fertility matters as much as the cell itself.
            let mut around = 0.0;
            for nb in w.terrain.neighbors8(i) {
                around += w.terrain.fertility[nb]
                    * w.terrain.biome[nb]
                        .group()
                        .map(|g| race.affinity[g as usize])
                        .unwrap_or(0.0);
            }
            score += around * 0.15;
            for &h in &homes {
                let d = w.terrain.dist(h, i);
                if d < min_sep {
                    score *= 0.05;
                } else if d < min_sep * 2 {
                    score *= 0.6;
                }
            }
            if score > best_score {
                best_score = score;
                best = Some(i);
            }
        }
        let home = best.unwrap_or(0);
        homes.push(home);
        race.home = home;
        w.races.push(race);

        // One or two founding cultures for the race.
        let n_cult = if rng.chance(0.4) { 2 } else { 1 };
        for ci in 0..n_cult {
            let mut home_c = home;
            if ci > 0 {
                // Second culture settles some distance away.
                let mut best = None;
                let mut best_score = -1.0f32;
                for _ in 0..300 {
                    let i = rng.below(w.terrain.n());
                    if !w.terrain.is_land(i) {
                        continue;
                    }
                    let d = w.terrain.dist(home, i);
                    if !(6..=18).contains(&d) {
                        continue;
                    }
                    let race = &w.races[ri];
                    let aff = w.terrain.biome[i]
                        .group()
                        .map(|g| race.affinity[g as usize])
                        .unwrap_or(0.3);
                    let score = w.terrain.fertility[i] * (0.4 + aff);
                    if score > best_score {
                        best_score = score;
                        best = Some(i);
                    }
                }
                match best {
                    Some(i) => home_c = i,
                    None => continue,
                }
            }
            let lang = if ci == 0 {
                w.races[ri].lang.mutate(&rng)
            } else {
                w.races[ri].lang.mutate(&rng).mutate(&rng)
            };
            found_culture(w, ri, lang, home_c, None);
        }
    }

    // Name each home continent in the tongue of its first people.
    for ci in 0..w.cultures.len() {
        let home = w.cultures[ci].home;
        let lm = w.terrain.landmass[home];
        if lm == 0 {
            continue;
        }
        let feat = w.terrain.features.iter().position(|f| {
            matches!(f.kind, FeatureKind::Continent | FeatureKind::Island)
                && f.cells.contains(&home)
        });
        if let Some(f) = feat {
            if w.terrain.features[f].name.is_none() {
                let name = crate::geo::name_feature(
                    w.terrain.features[f].kind,
                    &w.cultures[ci].lang,
                    &rng,
                );
                w.terrain.features[f].name = Some(name);
                w.terrain.features[f].named_by = Some(ci);
            }
        }
    }

    // Opening chronicle entries for each race.
    for ri in 0..w.races.len() {
        let home = w.races[ri].home;
        let place = w.place_phrase(home, None);
        let text = prose::race_awakes(&w.races[ri].plural, &w.races[ri].description, &place);
        w.log(2, EventKind::Genesis, &[Ref::Race(ri)], Some(home), text);
    }
}

/// Create a culture and seed its people around `home`.
pub fn found_culture(
    w: &mut World,
    race: usize,
    lang: Language,
    home: usize,
    parent: Option<usize>,
) -> usize {
    let rng = w.rng.clone();
    let id = w.cultures.len();
    let name = lang.name(&rng);
    let adj = lang.adjective(&name);
    let plural = lang.demonym(&name);
    let r = &w.races[race];
    let base = match parent {
        Some(p) => w.cultures[p].values,
        None => Values {
            militarism: r.martial,
            mysticism: r.mystic,
            mercantilism: r.mercantile,
            tradition: 0.5,
            openness: 0.5,
        },
    };
    let jitter = |v: f32| rng.trait_value(v as f64, 0.15);
    let values = Values {
        militarism: jitter(base.militarism),
        mysticism: jitter(base.mysticism),
        mercantilism: jitter(base.mercantilism),
        tradition: jitter(base.tradition),
        openness: jitter(base.openness),
    };
    // What this people takes to. Drawn from what it values, so a martial
    // people is good at warcraft and a trading one at accounts and ships,
    // and inherited with drift by its daughters — which is how a region
    // keeps a recognisable bent across the rise and fall of its realms.
    let learning = learning_from(w, parent, &values, r, &rng);
    w.cultures.push(Culture {
        id,
        name: name.clone(),
        adj,
        plural,
        race,
        lang,
        parent,
        founded: w.year,
        values,
        learning,
        home,
        color: culture_color(id),
        extinct: None,
        cells: 0,
        pop: 0.0,
        last_seen: w.year,
    });
    if parent.is_none() {
        // Seed population.
        let (hx, hy) = w.terrain.xy(home);
        for dy in -2i32..=2 {
            for dx in -3i32..=3 {
                let x = hx as i32 + dx;
                let y = hy as i32 + dy;
                if x < 0 || y < 0 || x >= w.terrain.w as i32 || y >= w.terrain.h as i32 {
                    continue;
                }
                let i = w.terrain.idx(x as usize, y as usize);
                if !w.terrain.is_land(i) || w.cells[i].culture.is_some() {
                    continue;
                }
                let f = w.terrain.fertility[i];
                if f <= 0.05 {
                    continue;
                }
                w.cells[i].culture = Some(id);
                w.cells[i].pop = (0.6 + f * 1.5) * rng.range32(0.7, 1.3);
            }
        }
    }
    id
}

/// What a people takes to, by field.
///
/// A daughter culture inherits its parent's bent with drift, so a region
/// stays recognisable across centuries even as its realms come and go. A
/// fresh people draws from its race and its values: this is the whole of the
/// link between what a people *believes* and what it *knows how to do*.
fn learning_from(
    w: &World,
    parent: Option<usize>,
    values: &Values,
    race: &Race,
    rng: &Rng,
) -> [f32; tech::FIELDS.len()] {
    use tech::Field;
    let mut out = [0.0f32; tech::FIELDS.len()];
    for f in tech::FIELDS {
        let base = match parent {
            // Inherited, and drifting: a daughter people is recognisably of
            // its mother for a long time, and not for ever.
            Some(p) => w.cultures[p].learning[f as usize],
            None => {
                // A first people's bent comes from what it is and what it
                // holds dear.
                let v = match f {
                    Field::Warcraft => values.militarism,
                    Field::Leycraft => values.mysticism * 0.7 + race.mystic * 0.3,
                    Field::Statecraft => values.mercantilism * 0.6 + values.tradition * 0.2,
                    Field::Seafaring => race.seafaring * 0.7 + race.coast_love * 0.3,
                    Field::Metalcraft => values.militarism * 0.4 + 0.3,
                    Field::Husbandry => 0.45 + race.fecund * 0.25,
                    Field::Building => values.tradition * 0.4 + 0.3,
                    Field::Letters => values.openness * 0.5 + 0.2,
                    Field::Reckoning => values.openness * 0.35 + values.mysticism * 0.25 + 0.15,
                    Field::Physic => 0.35 + values.openness * 0.2,
                };
                v.clamp(0.05, 0.95)
            }
        };
        out[f as usize] = rng.trait_value(base as f64, 0.12).clamp(0.05, 0.98);
    }
    // A people is not equally interested in everything. Left as a gentle
    // spread around the middle, every culture ends up above the threshold in
    // every field, adopts whatever reaches it, and the world converges on
    // one body of knowledge — which is the homogenising this exists to
    // prevent. So each people is given real strengths and real blind spots:
    // two fields it takes to, and two it never much cared for.
    //
    // A daughter culture keeps its mother's shape, drifting, so a region's
    // character outlasts the realms that ruled it.
    if parent.is_none() {
        let mut order: Vec<usize> = (0..tech::FIELDS.len()).collect();
        // Shuffled by a draw per position, so the choice is deterministic
        // and the stream advances the same way every time.
        for k in (1..order.len()).rev() {
            order.swap(k, rng.below(k + 1));
        }
        for (rank, &f) in order.iter().enumerate() {
            match rank {
                0 | 1 => out[f] = (out[f] + 0.35).min(0.97),
                2 => out[f] = (out[f] + 0.15).min(0.97),
                7 => out[f] = (out[f] - 0.22).max(0.03),
                8 | 9 => out[f] = (out[f] - 0.34).max(0.02),
                _ => {}
            }
        }
    }
    out
}
