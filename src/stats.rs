//! Balance metrics for headless runs: how the world behaves per century,
//! and the profiler that says where a year of simulation goes.

use crate::sim::{PolityKind, World, PHASES};

pub fn run(world: &mut World, years: i32) {
    println!(
        "{:>5} {:>6} {:>5} {:>4} {:>5} {:>4} {:>6} {:>5} {:>5} {:>5} {:>5}",
        "year", "pop_k", "realm", "emp", "wars", "sch", "cities", "cult", "relic", "proph", "top%"
    );
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
            let top = alive
                .iter()
                .map(|&p| world.polities[p].cells)
                .max()
                .unwrap_or(0) as f32
                / owned as f32
                * 100.0;
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
        }
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
    rows.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
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
