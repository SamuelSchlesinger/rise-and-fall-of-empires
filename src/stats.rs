//! Balance metrics for headless runs: how the world behaves per century,
//! and the profiler that says where a year of simulation goes.

use crate::sim::{PolityKind, World, PHASES};

pub fn run(world: &mut World, years: i32) {
    println!(
        "{:>5} {:>6} {:>5} {:>4} {:>5} {:>4} {:>6} {:>5} {:>5} {:>5} {:>5}",
        "year", "pop_k", "realm", "emp", "wars", "sch", "cities", "cult", "relic", "proph", "top%"
    );
    println!("      (top realm: stab / sprawl / over / foreign / tributaries; then what the world knows)");
    // The largest realm's own numbers, printed beside the world's, because
    // every balance question about this simulation is really a question
    // about whether the biggest realm is being held in check and by what.

    let mut wars_at_last = 0usize;
    for _ in 0..years {
        world.tick();
        if world.year % 100 == 0 {
            let alive = world.living_polities();
            let empires = alive
                .iter()
                .filter(|&&p| world.polities[p].kind == PolityKind::Empire)
                .count();
            let wars_total = world.wars.len();
            let wars_century = wars_total - wars_at_last;
            wars_at_last = wars_total;
            let owned = world.stats.owned_cells.max(1);
            let top_p = alive
                .iter()
                .copied()
                .max_by_key(|&p| world.polities[p].cells);
            let top =
                top_p.map(|p| world.polities[p].cells).unwrap_or(0) as f32 / owned as f32 * 100.0;
            let relics = world
                .artifacts
                .iter()
                .filter(|a| !matches!(a.holder, crate::sim::Holder::Lost))
                .count();
            let proph = world
                .prophecies
                .iter()
                .filter(|p| p.outcome.is_none())
                .count();
            println!(
                "{:>5} {:>6.0} {:>5} {:>4} {:>5} {:>4} {:>6} {:>5} {:>5} {:>5} {:>5.0}",
                world.year,
                world.stats.pop,
                alive.len(),
                empires,
                wars_century,
                world.stats.schools_alive,
                world.stats.cities_alive,
                world.stats.cultures_alive,
                relics,
                proph,
                top
            );
            if let Some(p) = top_p {
                let pol = &world.polities[p];
                println!(
                    "      {:<18} stab {:.2}  sprawl {:.2}  over {:.2}  foreign {:.2}  trib {}",
                    pol.short,
                    pol.stability,
                    pol.sprawl,
                    pol.overextension(&world.tuning),
                    pol.foreign_share,
                    pol.tributaries.len()
                );
                // What is holding it together, in the simulation's own
                // terms. This is `explain`, so a disagreement between these
                // numbers and the realm's actual stability is a bug in one
                // of the two and shows up here first.
                let mut f = crate::sim::explain::stability_factors(world, p);
                f.sort_by(|a, b| b.weight.abs().total_cmp(&a.weight.abs()));
                let target: f32 = crate::sim::explain::stability_base(world)
                    + f.iter().map(|x| x.weight).sum::<f32>();
                let parts: Vec<String> = f
                    .iter()
                    .take(4)
                    .map(|x| format!("{:+.2} {}", x.weight, short_reason(&x.text)))
                    .collect();
                println!("      target {:.2} = {}", target, parts.join("  "));
            }
            // What the world knows, and how widely. The question the ratchet
            // exists to answer is whether year 1500 differs in kind from year
            // 300 — so: how many innovations exist at all, how many the
            // leading realm has, and what share of settled land knows the
            // median one.
            {
                let seen = world.tech_seen.iter().filter(|&&b| b).count();
                let best = world
                    .living_polities()
                    .iter()
                    .map(|&p| world.tech_count(p))
                    .max()
                    .unwrap_or(0);
                let owned = world.stats.owned_cells.max(1);
                let mut spread: Vec<f32> = (0..world.techs.len())
                    .filter(|&t| world.tech_seen[t])
                    .map(|t| {
                        let n = (0..world.cells.len())
                            .filter(|&i| world.cells[i].owner.is_some() && world.cell_knows(i, t))
                            .count();
                        n as f32 / owned as f32
                    })
                    .collect();
                spread.sort_by(f32::total_cmp);
                let median = spread.get(spread.len() / 2).copied().unwrap_or(0.0);
                // How far apart the realms are: the gap between the best
                // and the median realm is what "local differentiation"
                // means in a number. A world where everyone knows the same
                // things has a gap of nothing.
                let mut counts: Vec<u32> = world
                    .living_polities()
                    .iter()
                    .map(|&p| world.tech_count(p))
                    .collect();
                counts.sort_unstable();
                let mid = counts.get(counts.len() / 2).copied().unwrap_or(0);
                let low = counts.first().copied().unwrap_or(0);
                println!(
                    "      known: {} of {} exist | realms hold {}/{}/{} (least/median/best) | \
                     median innovation reaches {:.0}% of settled land",
                    seen,
                    world.techs.len(),
                    low,
                    mid,
                    best,
                    median * 100.0
                );
            }
        }
    }
    // How many figures a world actually produced, and how long any of them
    // were alive to be watched. A world that acclaims nobody has no
    // characters in it; one that acclaims somebody every decade has no
    // characters either, only a list.
    let acclaimed: Vec<&crate::sim::Person> =
        world.persons.iter().filter(|p| p.is_acclaimed()).collect();
    let living = acclaimed.iter().filter(|p| p.alive()).count();
    let watched: i32 = acclaimed
        .iter()
        .map(|p| (p.died.unwrap_or(world.year) - p.acclaimed.unwrap_or(0)).max(0))
        .sum();
    // Coverage is the number that actually matters: the share of the world's
    // years in which there was somebody alive for the reader to follow. A
    // count per century says nothing if every figure is acclaimed on their
    // deathbed.
    let mut covered = vec![false; (years + 1).max(1) as usize];
    for p in &acclaimed {
        let from = p.acclaimed.unwrap_or(0).max(0);
        let to = p.died.unwrap_or(world.year).min(world.year);
        for y in from..=to {
            if let Some(slot) = covered.get_mut(y as usize) {
                *slot = true;
            }
        }
    }
    let coverage = covered.iter().filter(|&&c| c).count() as f32 / covered.len() as f32;
    println!(
        "figures: {} acclaimed in {} years ({:.1} per century), {} alive now; \
         {:.0}% of years had a living figure, {} years on average from acclaim to death",
        acclaimed.len(),
        years,
        acclaimed.len() as f32 / (years as f32 / 100.0),
        living,
        coverage * 100.0,
        if acclaimed.is_empty() {
            0
        } else {
            watched / acclaimed.len() as i32
        }
    );

    // The single most important question about a world called "the rise and
    // fall of empires": did anything ever actually rise? A world whose
    // largest realm never passes a tenth of the map has no empires in it,
    // and one whose largest realm passes three quarters has no history left
    // after the first one wins.
    let mut biggest: Vec<(f32, usize)> = world
        .polities
        .iter()
        .map(|p| (p.peak_share, p.id))
        .collect();
    biggest.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("the great realms, by the share of the world they held at their height:");
    for &(share, id) in biggest.iter().take(5) {
        if share <= 0.0 {
            break;
        }
        let p = &world.polities[id];
        let end = p.fell.unwrap_or(world.year);
        println!(
            "  {:>3.0}% in {:>4}  {:<22} {}-{} ({}y), {} lands now{}",
            share * 100.0,
            p.peak_year,
            p.short,
            p.founded,
            end,
            end - p.founded,
            p.cells,
            match p.fell {
                Some(_) => format!(" — {}", p.fall_cause.trim_end_matches('.')),
                None => String::new(),
            }
        );
    }

    // The great houses. A dynasty is the longest thread in the world and
    // the one a reader is most likely to follow, so a balance readout that
    // never mentions one is missing the thing it should be checking.
    let mut hs: Vec<usize> = (0..world.houses.len()).collect();
    hs.sort_by_key(|&h| std::cmp::Reverse(world.houses[h].seniors.len()));
    println!("the great houses, by how many of them ruled:");
    for &h in hs.iter().take(5) {
        let ho = &world.houses[h];
        if ho.seniors.len() < 2 {
            break;
        }
        println!(
            "  {:>3} rulers over {:>4}y  {:<30} {} at its height{}",
            ho.seniors.len(),
            ho.span(world.year).max(0),
            ho.name,
            crate::sim::prose::count(ho.peak_realms.max(1) as i64, "throne"),
            match ho.ended {
                Some(y) => format!(", died out in {}", y),
                None => String::new(),
            }
        );
    }

    // Lifespans.
    let mut realm_spans = Vec::new();
    let mut empire_spans = Vec::new();
    for p in &world.polities {
        let end = p.fell.unwrap_or(world.year);
        let span = end - p.founded;
        if p.peak_cells >= 20 {
            realm_spans.push(span);
        }
        if p.peak_cells >= 150 {
            empire_spans.push(span);
        }
    }
    let avg = |v: &[i32]| {
        if v.is_empty() {
            0.0
        } else {
            v.iter().sum::<i32>() as f64 / v.len() as f64
        }
    };
    let median = |v: &mut Vec<i32>| {
        v.sort();
        if v.is_empty() {
            0
        } else {
            v[v.len() / 2]
        }
    };
    let mut rs = realm_spans.clone();
    let mut es = empire_spans.clone();
    println!(
        "realms >=20 lands: {} (avg life {:.0}y, median {}y)",
        realm_spans.len(),
        avg(&realm_spans),
        median(&mut rs)
    );
    println!(
        "great powers >=150 lands: {} (avg life {:.0}y, median {}y)",
        empire_spans.len(),
        avg(&empire_spans),
        median(&mut es)
    );
    let fulfilled = world
        .prophecies
        .iter()
        .filter(|p| p.outcome == Some(true))
        .count();
    let failed = world
        .prophecies
        .iter()
        .filter(|p| p.outcome == Some(false))
        .count();
    println!(
        "prophecies: {} uttered, {} came true, {} failed",
        world.prophecies.len(),
        fulfilled,
        failed
    );
    let wars: Vec<i32> = world
        .wars
        .iter()
        .filter_map(|w| w.ended.map(|e| e - w.started))
        .collect();
    println!(
        "wars: {} (avg length {:.1}y); events {}; {:.2} ms/year",
        world.wars.len(),
        avg(&wars),
        world.chronicle.len(),
        world.ticks_ms
    );
}

/// Run `years` years with the per-phase profiler on and print where the time
/// went. Rough resident memory is read from /proc where it exists.
pub fn bench(world: &mut World, years: i32) {
    world.prof.on = true;
    let t0 = std::time::Instant::now();
    for _ in 0..years {
        world.tick();
    }
    let elapsed = t0.elapsed().as_secs_f64() * 1000.0;
    let years_f = years.max(1) as f64;
    println!(
        "{}x{} = {} cells | {} years | {:.3} ms/year total",
        world.terrain.w,
        world.terrain.h,
        world.cells.len(),
        years,
        elapsed / years_f
    );
    let mut rows: Vec<(f64, &str)> = PHASES
        .iter()
        .enumerate()
        .map(|(i, name)| (world.prof.ms[i], *name))
        .collect();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    let accounted: f64 = rows.iter().map(|r| r.0).sum();
    for (ms, name) in &rows {
        println!(
            "  {:<18} {:>8.4} ms/year  {:>5.1}%",
            name,
            ms / years_f,
            ms / elapsed * 100.0
        );
    }
    println!(
        "  {:<18} {:>8.4} ms/year  {:>5.1}%",
        "(unaccounted)",
        (elapsed - accounted) / years_f,
        (elapsed - accounted) / elapsed * 100.0
    );
    println!(
        "events {} (dropped {}) | polities {} | persons {} | cities {} | rss {}",
        world.chronicle.len(),
        world.chronicle.dropped,
        world.polities.len(),
        world.persons.len(),
        world.cities.len(),
        rss()
    );
}

/// Peak resident set size as the kernel reports it, or "?" elsewhere.
fn rss() -> String {
    match std::fs::read_to_string("/proc/self/status") {
        Ok(s) => s
            .lines()
            .find(|l| l.starts_with("VmHWM:"))
            .map(|l| l["VmHWM:".len()..].trim().to_string())
            .unwrap_or_else(|| "?".to_string()),
        Err(_) => "?".to_string(),
    }
}

/// The first few words of an explanation, for a one-line balance readout.
fn short_reason(text: &str) -> String {
    let mut out: String = text
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ");
    if out.len() > 26 {
        out.truncate(26);
    }
    out
}
