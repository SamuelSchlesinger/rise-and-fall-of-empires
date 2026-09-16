//! Creation of the world's races and first peoples, and the opening
//! lines of the chronicle.

use super::chronicle::{EventKind, Ref};
use super::prose::{self, genesis::FEATURE, genesis::STATURE};
use super::{Culture, Era, Race, Values, World, GROUP_COUNT};
use crate::geo::FeatureKind;
use crate::lang::Language;
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
                    if d < 6 || d > 18 {
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
