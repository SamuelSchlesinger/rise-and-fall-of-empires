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
}

fn parse_args(cfg: &config::Config) -> Args {
    let mut a = Args {
        seed: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(1),
        width: cfg.width.unwrap_or(160),
        height: cfg.height.unwrap_or(64),
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
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let next = |i: &mut usize| -> Option<String> {
            *i += 1;
            args.get(*i).cloned()
        };
        match args[i].as_str() {
            "--seed" | "-s" => a.seed = next(&mut i).and_then(|v| v.parse().ok()).unwrap_or(a.seed),
            "--width" | "-w" => {
                a.width = next(&mut i).and_then(|v| v.parse().ok()).unwrap_or(a.width);
            }
            "--height" | "-h" => {
                a.height = next(&mut i)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(a.height);
            }
            "--detail" | "-d" => {
                a.detail = match next(&mut i).as_deref() {
                    Some("low") => Detail::Low,
                    Some("high") => Detail::High,
                    _ => Detail::Medium,
                }
            }
            "--headless" | "--years" => {
                a.headless = Some(next(&mut i).and_then(|v| v.parse().ok()).unwrap_or(500));
            }
            "--min-importance" | "-i" => {
                a.min_importance = next(&mut i).and_then(|v| v.parse().ok()).unwrap_or(1);
            }
            "--ascii" => a.ascii = true,
            "--tour" => a.tour = true,
            "--stats" => a.stats = true,
            "--bench" => a.bench = true,
            "--load" | "-l" => a.load = next(&mut i),
            "--save" | "-o" => a.save = next(&mut i),
            "--no-mouse" => a.mouse = false,
            "--snapshot" => a.snapshot = next(&mut i),
            "--layer" => a.layer = next(&mut i).unwrap_or_else(|| "political".into()),
            "--cols" => a.cols = next(&mut i).and_then(|v| v.parse().ok()).unwrap_or(160),
            "--rows" => a.rows = next(&mut i).and_then(|v| v.parse().ok()).unwrap_or(45),
            "--mkconfig" => {
                match config::write_template() {
                    Ok(p) => println!("wrote {}", p.display()),
                    Err(e) => println!("{}", e),
                }
                std::process::exit(0);
            }
            "--version" | "-V" => {
                println!("empires {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--help" => {
                println!(
                    "empires — rise and fall of empires\n\n  --seed N          world seed (default: time)\n  --width W         map width (default 160)\n  --height H        map height (default 64)\n  --detail L        low | medium | high (default medium)\n  --headless N      run N years without a UI and print the chronicle\n  --stats           with --headless N: print balance metrics per century instead of the chronicle\n  --bench           with --headless N (default 300): print ms/year and per-phase timings\n  --load FILE       continue a saved world\n  --save FILE       save to FILE (autosaves every 100 years and on quit; in headless mode, at the end)\n  --min-importance  0-3, filter for headless output (default 1)\n  --ascii           use plain ASCII glyphs\n  --no-mouse        do not capture the mouse (keeps the terminal's own text selection)\n  --tour            show the introductory card again\n  --mkconfig        write a commented config template to ~/.config/empires/config\n  --snapshot PATH   render one frame after --headless N years to PATH.txt and PATH.html\n  --layer L         layer for the snapshot (political, terrain, culture, mana, population, biomes)\n  --cols C --rows R terminal size for the snapshot\n  -V, --version     print the version and exit\n  --help            print this message and exit"
                );
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }
    a.width = a.width.clamp(40, 600);
    a.height = a.height.clamp(20, 300);
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
    if args.load.is_some() && args.headless.is_some() {
        // A loaded world keeps its own detail unless one was given explicitly.
        if std::env::args().any(|a| a == "--detail" || a == "-d") {
            world.detail = args.detail;
        }
    }
    if let Some(path) = args.snapshot {
        ui::snapshot(
            world,
            args.ascii,
            args.cols,
            args.rows,
            args.headless.unwrap_or(300),
            &args.layer,
            &path,
        );
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
        eprintln!(
            "seed {} | {} years in {:.2}s ({:.2} ms/year) | {} events | pop {:.0}k | {} realms alive of {} | {} cities | {} cultures | {} schools",
            world.seed,
            years,
            elapsed.as_secs_f64(),
            elapsed.as_secs_f64() * 1000.0 / years.max(1) as f64,
            world.chronicle.len(),
            world.stats.pop,
            world.stats.polities_alive,
            world.polities.len(),
            world.stats.cities_alive,
            world.stats.cultures_alive,
            world.stats.schools_alive
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
