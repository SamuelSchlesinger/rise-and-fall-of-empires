//! Rise and Fall of Empires: a lo-fi passive world simulator.
//!
//! This binary is the whole game. [`geo`] raises a world out of [`noise`],
//! [`sim`] runs the centuries on it and writes them down, [`ui`] draws the
//! result over a hand-rolled terminal layer in [`term`], and [`ser`] puts a
//! world on disk and takes it off again. Everything is seeded from [`rng`],
//! so a seed names one world and one history for good.
//!
//! This module is only the front door: command-line arguments, the headless
//! and snapshot modes, and the hand-off to [`ui::run`].

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(
    clippy::doc_markdown,
    clippy::needless_pass_by_value,
    clippy::redundant_closure_for_method_calls,
    clippy::semicolon_if_nothing_returned
)]

mod config;
mod geo;
mod lang;
mod noise;
mod rng;
mod ser;
#[cfg(test)]
mod ser_tests;
mod sim;
mod stats;
mod term;
#[cfg(test)]
mod tests;
mod theme;
mod ui;

use sim::{Detail, World};

struct Args {
    seed: u64,
    width: usize,
    height: usize,
    detail: Detail,
    headless: Option<i32>,
    stats: bool,
    bench: bool,
    load: Option<String>,
    save: Option<String>,
    min_importance: u8,
    ascii: bool,
    mouse: bool,
    snapshot: Option<String>,
    layer: String,
    cols: usize,
    rows: usize,
    tour: bool,
    /// A people to be bound to from the first year: a name, or "auto" for
    /// whichever is largest. Lets a headless run, a snapshot or a script
    /// start with a covenant instead of the thin trickle of no covenant.
    bind: Option<String>,
    /// Where a snapshot looks, how closely, and whether it keeps the
    /// interface furniture around the map.
    shot: ui::Shot,
}

/// Complain on stderr and stop. Bad arguments are the user's to fix, so the
/// message says what was wrong and points at `--help` rather than carrying on
/// with a value nobody asked for.
fn fail(msg: &str) -> ! {
    eprintln!("empires: {}", msg);
    eprintln!("Try 'empires --help' for the full list of options.");
    std::process::exit(2);
}

/// The text `--help` prints. Also the answer to a mistyped flag, in spirit:
/// every option the binary accepts is listed here, short forms included.
const USAGE: &str = "\
empires — rise and fall of empires

  -s, --seed N          world seed (default: the current time)
  -w, --width W         map width, 40-600 (default 288)
      --height H        map height, 20-300 (default 144)
  -d, --detail L        how much of the history gets written down:
                        low | medium | high (default medium). It is a
                        verbosity setting only — all three produce exactly
                        the same world from the same seed, and differ in
                        the smallest events they keep (importance 2, 1 and
                        0) and in the extra line of colour high adds to a
                        battle or a ruler's death.
      --headless N      run N years without a UI and print the chronicle
      --stats           with --headless N (default 1500): balance metrics per
                        century instead of the chronicle
      --bench           with --headless N (default 300): ms/year and per-phase
                        timings
  -l, --load FILE       continue a saved world
  -o, --save FILE       save to FILE (autosaves every 100 years and on quit;
                        in headless mode, once at the end)
  -i, --min-importance  hide headless events below this importance, 0-3
                        (default 1; above 3 prints the summary line alone)
      --ascii           use plain ASCII glyphs
      --no-mouse        do not capture the mouse (keeps the terminal's own
                        text selection)
      --tour            show the introductory card again
      --bind PEOPLE     bind yourself to a people from the first year, by
                        name or `auto` for whichever is largest
      --mkconfig        write a commented config template to
                        ~/.config/empires/config
      --snapshot PATH   render one frame after --headless N years (default 300)
                        to PATH.txt and PATH.html
      --layer L         layer for the snapshot: political, terrain, culture,
                        mana, population, biomes
      --at PLACE        centre the snapshot on a named realm, city, region
                        or people rather than the largest realm's capital
      --zoom N          snapshot map zoom, 1-4
      --plain           snapshot the map alone, without the sidebar, the
                        event log or the key rows
      --cols C          terminal width for the snapshot, 20-1000 (default 160)
      --rows R          terminal height for the snapshot, 5-1000 (default 45)
  -V, --version         print the version and exit
  -h, --help            print this message and exit";

fn parse_args(cfg: &config::Config) -> Args {
    let mut a = Args {
        seed: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(1),
        // Twice as wide as tall, which is what draws square: at zoom 1
        // a world cell is one terminal character, and a character is about
        // twice as tall as it is wide.
        width: cfg.width.unwrap_or(288),
        height: cfg.height.unwrap_or(144),
        detail: cfg.detail.unwrap_or(Detail::Medium),
        headless: None,
        stats: false,
        bench: false,
        load: None,
        save: None,
        min_importance: 1,
        ascii: cfg.ascii.unwrap_or(false),
        mouse: cfg.mouse.unwrap_or(true),
        snapshot: None,
        layer: "political".into(),
        cols: 160,
        rows: 45,
        tour: false,
        bind: None,
        shot: ui::Shot::default(),
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].clone();
        // The next argument, or a complaint naming the flag that wanted one.
        let value = |i: &mut usize| -> String {
            *i += 1;
            match args.get(*i) {
                Some(v) => v.clone(),
                None => fail(&format!("{} needs a value", flag)),
            }
        };
        // The next argument parsed as `T`, or a complaint saying what it
        // should have looked like. Silently falling back to a default here is
        // how `--seed abc` used to become a different world every run.
        macro_rules! number {
            ($i:expr, $t:ty, $what:expr) => {{
                let raw = value($i);
                match raw.parse::<$t>() {
                    Ok(v) => v,
                    Err(_) => fail(&format!("{} {}: expected {}", args[*$i - 1], raw, $what)),
                }
            }};
        }
        match args[i].as_str() {
            "--seed" | "-s" => a.seed = number!(&mut i, u64, "a whole number"),
            "--width" | "-w" => a.width = number!(&mut i, usize, "a whole number"),
            "--height" => a.height = number!(&mut i, usize, "a whole number"),
            "--detail" | "-d" => {
                let v = value(&mut i);
                a.detail = match v.as_str() {
                    "low" => Detail::Low,
                    "medium" => Detail::Medium,
                    "high" => Detail::High,
                    _ => fail(&format!("--detail {}: expected low, medium or high", v)),
                };
            }
            "--headless" | "--years" => a.headless = Some(number!(&mut i, i32, "a year count")),
            // Not range-checked: importance runs 0 to 3, so anything above
            // that is a legitimate way of asking for the summary line alone.
            "--min-importance" | "-i" => a.min_importance = number!(&mut i, u8, "a whole number"),
            "--ascii" => a.ascii = true,
            "--tour" => a.tour = true,
            "--bind" => a.bind = Some(value(&mut i)),
            "--at" => a.shot.at = value(&mut i),
            "--zoom" => a.shot.zoom = number!(&mut i, usize, "a zoom level, 1-4"),
            "--plain" => a.shot.plain = true,
            "--stats" => a.stats = true,
            "--bench" => a.bench = true,
            "--load" | "-l" => a.load = Some(value(&mut i)),
            "--save" | "-o" => a.save = Some(value(&mut i)),
            "--no-mouse" => a.mouse = false,
            "--snapshot" => a.snapshot = Some(value(&mut i)),
            "--layer" => a.layer = value(&mut i),
            "--cols" => a.cols = number!(&mut i, usize, "a column count"),
            "--rows" => a.rows = number!(&mut i, usize, "a row count"),
            "--mkconfig" => {
                match config::write_template() {
                    Ok(p) => println!("wrote {}", p.display()),
                    Err(e) => {
                        eprintln!("empires: {}", e);
                        std::process::exit(1);
                    }
                }
                std::process::exit(0);
            }
            "--version" | "-V" => {
                println!("empires {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--help" | "-h" => {
                println!("{}", USAGE);
                std::process::exit(0);
            }
            other => fail(&format!("unrecognised option '{}'", other)),
        }
        i += 1;
    }
    // Checked after the whole line has been read, because these four can also
    // come from the config file, and a bad value there deserves the same
    // complaint rather than a silent clamp.
    let range = |name: &str, v: usize, lo: usize, hi: usize| -> usize {
        if v < lo || v > hi {
            fail(&format!(
                "{} {}: must be between {} and {}",
                name, v, lo, hi
            ));
        }
        v
    };
    a.width = range("map width", a.width, 40, 600);
    a.height = range("map height", a.height, 20, 300);
    // A snapshot's terminal is ours to invent, but a zero-column screen has
    // nothing to draw on and a huge one is a request to allocate the machine.
    a.cols = range("snapshot columns", a.cols, 20, 1000);
    a.rows = range("snapshot rows", a.rows, 5, 1000);
    if a.headless.is_some_and(|y| y < 0) {
        fail("--headless must not be negative");
    }
    a
}

fn main() {
    let cfg = config::load();
    let args = parse_args(&cfg);
    let mut world = match &args.load {
        Some(path) => match World::load_from(std::path::Path::new(path)) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("empires: {}", e);
                std::process::exit(1);
            }
        },
        None => World::new(args.seed, args.width, args.height, args.detail),
    };
    for (field, value) in &cfg.tunes {
        if let Err(e) = world.tuning.set(field, *value) {
            eprintln!("empires: {}", e);
        }
    }
    // Config complaints reach the interface through its status line, which
    // a headless or snapshot run does not have. The config template
    // promises that a name this build does not know is reported when the
    // game starts, so say it here too rather than swallowing it.
    if !cfg.errors.is_empty() && (args.headless.is_some() || args.snapshot.is_some()) {
        for e in &cfg.errors {
            eprintln!("empires: config: {}", e);
        }
    }
    // Before anything reads the world, so that a headless run's whole
    // history is lived under the covenant rather than the last part of it.
    if let Some(want) = &args.bind {
        let lower = want.to_lowercase();
        let pick = if lower == "auto" {
            (0..world.cultures.len())
                .filter(|&c| world.cultures[c].extinct.is_none())
                .max_by(|&a, &b| world.cultures[a].pop.total_cmp(&world.cultures[b].pop))
        } else {
            (0..world.cultures.len()).find(|&c| {
                world.cultures[c].extinct.is_none()
                    && (world.cultures[c].plural.to_lowercase().starts_with(&lower)
                        || world.cultures[c].name.to_lowercase().starts_with(&lower)
                        || world.cultures[c].adj.to_lowercase().starts_with(&lower))
            })
        };
        match pick {
            Some(c) => sim::fate::bind(&mut world, c),
            None => {
                let living: Vec<&str> = world
                    .cultures
                    .iter()
                    .filter(|c| c.extinct.is_none())
                    .map(|c| c.plural.as_str())
                    .collect();
                eprintln!(
                    "empires: --bind {}: no living people by that name. Try one of: {}",
                    want,
                    living.join(", ")
                );
                std::process::exit(1);
            }
        }
    }
    if args.load.is_some() && args.headless.is_some() {
        // A loaded world keeps its own detail unless one was given explicitly.
        if std::env::args().any(|a| a == "--detail" || a == "-d") {
            world.detail = args.detail;
        }
    }
    if let Some(path) = args.snapshot {
        let mut shot = args.shot.clone();
        shot.ascii = args.ascii;
        shot.cols = args.cols;
        shot.rows = args.rows;
        shot.layer = args.layer.clone();
        shot.years = args.headless.unwrap_or(300);
        ui::snapshot(world, &path, &shot);
        return;
    }
    if args.bench {
        stats::bench(&mut world, args.headless.unwrap_or(300));
        return;
    }
    if args.stats {
        let years = args.headless.unwrap_or(1500);
        stats::run(&mut world, years);
        return;
    }
    if let Some(years) = args.headless {
        let t0 = std::time::Instant::now();
        for _ in 0..years {
            world.tick();
        }
        let elapsed = t0.elapsed();
        {
            use std::io::Write;
            let stdout = std::io::stdout();
            let mut out = stdout.lock();
            for e in &world.chronicle.events {
                if e.importance >= args.min_importance
                    && writeln!(
                        out,
                        "[{:>5}] {}{}",
                        e.year,
                        "*".repeat(e.importance as usize),
                        e.text
                    )
                    .is_err()
                {
                    return; // downstream closed the pipe (e.g. `| head`)
                }
            }
        }
        if let Some(path) = &args.save {
            match world.save_to(std::path::Path::new(path)) {
                Ok(n) => eprintln!("saved {} ({} bytes)", path, n),
                Err(e) => eprintln!("empires: {}", e),
            }
        }
        // "1 school", not "1 schools": the summary is the only thing a
        // headless run always prints, so it may as well read properly.
        let n = |count: usize, one: &str, many: &str| {
            format!("{} {}", count, if count == 1 { one } else { many })
        };
        eprintln!(
            "seed {} | {} years in {:.2}s ({:.2} ms/year) | {} | pop {:.0}k | {} alive of {} | {} | {} | {}",
            world.seed,
            years,
            elapsed.as_secs_f64(),
            elapsed.as_secs_f64() * 1000.0 / years.max(1) as f64,
            n(world.chronicle.len(), "event", "events"),
            world.stats.pop,
            n(world.stats.polities_alive, "realm", "realms"),
            world.polities.len(),
            n(world.stats.cities_alive, "city", "cities"),
            n(world.stats.cultures_alive, "culture", "cultures"),
            n(world.stats.schools_alive, "school", "schools"),
        );
        return;
    }
    ui::run(
        world,
        args.ascii,
        args.mouse,
        args.save.or(args.load),
        &cfg,
        args.tour,
    );
}
