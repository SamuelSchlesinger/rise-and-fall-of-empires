//! Terminal user interface: a scrollable world map with overlays, a
//! sidebar, a live event log, browsable lists, detail pages and the full
//! chronicle. Keys follow vim conventions: counts, gg/G, zz, ZZ, `:`
//! commands and `/` search, with mouse and modified arrows as well.

mod commands;
pub(crate) mod detail;
mod export;
mod input;
mod learn;
pub mod query;
mod recap;
mod render;
mod words;

use crate::config::Config;
use crate::geo::Biome;
use crate::sim::chronicle::{EventKind, Ref};
use crate::sim::World;
use crate::term::{self, Input, Key, Rect, Rgb, Screen, Style, BOLD, DIM, REVERSE};
use crate::theme::Theme;
use std::time::{Duration, Instant};

pub const SPEEDS: [f64; 8] = [0.5, 1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0];

/// The storyteller: what is worth watching right now.
pub(crate) fn stories(w: &World) -> Vec<(String, Ref)> {
    let mut out: Vec<(f32, String, Ref)> = Vec::new();
    // This runs on every frame, so it reads the living indexes rather than
    // the vectors behind them. Those hold everything that has ever been: by
    // the eightieth century of a large map, forty thousand finished wars and
    // four hundred thousand dead, against some thirty wars and a thousand
    // people who are actually going on. Walking the whole of history sixty
    // times a second is what made a long game feel slow.
    for x in w.alive_wars.iter().map(|&i| &w.wars[i]) {
        let (a, d) = (x.attacker, x.defender);
        let size = (w.polities[a].cells + w.polities[d].cells) as f32;
        let lean = if x.score > 0.3 {
            format!("{} gaining", w.polities[a].short)
        } else if x.score < -0.3 {
            format!("{} holding", w.polities[d].short)
        } else {
            "in the balance".to_string()
        };
        out.push((
            size * 0.02 + x.battles as f32 * 0.5,
            format!(
                "{}: {} v {}, {}, {}",
                x.name,
                w.polities[a].short,
                w.polities[d].short,
                crate::sim::prose::years((w.year - x.started).max(1) as i64),
                lean
            ),
            Ref::War(x.id),
        ));
    }
    for p in w.living_polities() {
        let pol = &w.polities[p];
        if pol.cells > 40 && pol.stability < 0.25 {
            out.push((
                pol.cells as f32 * 0.05 + (0.25 - pol.stability) * 40.0,
                // The vocabulary from `words::stability`, not a word of
                // its own: the sidebar, the lists and the pages all have
                // to call 19% the same thing. The number is on the
                // realm's own page; this is a headline.
                format!(
                    "{} is {}{}",
                    pol.name,
                    words::stability(pol.stability),
                    if pol.at_war() { ", and at war" } else { "" }
                ),
                Ref::Polity(p),
            ));
        }
        if pol.kind == crate::sim::PolityKind::Empire && w.year - pol.last_kind_change < 60 {
            out.push((
                6.0 + pol.cells as f32 * 0.01,
                format!("A new empire: {} under {}", pol.name, w.ruler_short(p)),
                Ref::Polity(p),
            ));
        }
        if pol.reign_gained > 40 {
            out.push((
                pol.reign_gained as f32 * 0.1,
                format!(
                    "{} has won {} lands for {}",
                    w.ruler_short(p),
                    pol.reign_gained,
                    pol.short
                ),
                Ref::Polity(p),
            ));
        }
    }
    // The figures of the age, weighted above almost everything else.
    // A reader who looks up once a century should find the name that
    // century will be remembered by without going hunting for it, and
    // the deed is carried with the name so it means something the first
    // time they see it.
    for r in w.alive_persons.iter().copied() {
        let per = &w.persons[r];
        if !per.alive() || !per.is_acclaimed() {
            continue;
        }
        let Some(p) = per.polity.filter(|&p| w.polities[p].alive()) else {
            continue;
        };
        let deed = crate::sim::dynasty::standing(w, r)
            .first()
            .map(|c| c.what.clone())
            .unwrap_or_else(|| "is spoken of everywhere".into());
        // Weighted above the ordinary run of wars and unrest: a living
        // figure is the most interesting thing in a century, and the
        // line is kept short so it fits a sidebar row without wrapping.
        out.push((
            30.0 + per.greatness * 0.02,
            format!("{} of {}: {}", per.full_name(), w.polities[p].short, deed),
            Ref::Person(r),
        ));
    }
    // The power of the age, if there is one.
    if let Some(h) = crate::sim::war::hegemon(w) {
        out.push((
            26.0,
            format!(
                "{} holds {:.0}% of the world",
                w.polities[h].short,
                crate::sim::dynasty::world_share(w, h) * 100.0
            ),
            Ref::Polity(h),
        ));
    }
    for pl in &w.plagues {
        let names: Vec<&str> = pl
            .polities
            .iter()
            .filter(|&&p| w.polities[p].alive())
            .map(|&p| w.polities[p].short.as_str())
            .take(3)
            .collect();
        if let Some(&p) = pl.polities.first() {
            out.push((
                5.0 + pl.deaths as f32 * 0.01,
                format!("{} ravages {}", pl.name, names.join(", ")),
                Ref::Polity(p),
            ));
        }
    }
    for pr in w.prophecies.iter().filter(|pr| pr.outcome.is_none()) {
        let left = pr.deadline - w.year;
        if left < 30 {
            out.push((
                4.0 + (30 - left) as f32 * 0.1,
                format!(
                    "{} years left for the prophecy that {}",
                    left.max(0),
                    pr.what
                ),
                Ref::Person(pr.seer),
            ));
        }
    }
    out.sort_by(|a, b| b.0.total_cmp(&a.0));
    out.truncate(3);
    out.into_iter().map(|(_, t, r)| (t, r)).collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Map,
    List,
    Detail,
    Chronicle,
    Help,
    Guide,
    Fate,
    Recap,
}

/// The kind of question a layer answers. Tab steps between these.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LayerFamily {
    /// Who holds what.
    Power,
    /// Who the people are and what they believe.
    People,
    /// What the ground is.
    Land,
    /// How people live on it: how many, on what, and how well.
    Living,
    /// What is changing, rather than what is: see [`crate::sim::flows`].
    Motion,
}

impl LayerFamily {
    pub fn name(self) -> &'static str {
        match self {
            LayerFamily::Power => "power",
            LayerFamily::People => "peoples",
            LayerFamily::Land => "land",
            LayerFamily::Living => "living",
            LayerFamily::Motion => "motion",
        }
    }
    pub fn all() -> [LayerFamily; 5] {
        [
            LayerFamily::Power,
            LayerFamily::People,
            LayerFamily::Land,
            LayerFamily::Living,
            LayerFamily::Motion,
        ]
    }
    /// The layers of this family, in the order they step.
    pub fn layers(self) -> Vec<Layer> {
        Layer::all()
            .into_iter()
            .filter(|l| l.family() == self)
            .collect()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layer {
    Political,
    Terrain,
    Culture,
    Magic,
    Population,
    Biomes,
    /// What the ground yields, per cell.
    Goods,
    /// What trade is worth, and the roads it moves along.
    Trade,
    /// What the land can feed, which is fertility, weather and knowledge
    /// multiplied together.
    Harvest,
    /// What the ground remembers: the local technology multiplier.
    Knowledge,
    /// Where people are arriving and where they are leaving.
    Settling,
    /// Where the fighting actually is.
    Fighting,
    /// Where the map is being redrawn.
    Frontier,
    /// Where the weather has turned since living memory.
    Drift,
    /// Where in the world history actually happened.
    Memory,
    /// How long each stretch of country has been lived in.
    Settled,
    /// Who is bound to whom, and who is angry with whom.
    ///
    /// The political layer says who owns what and nothing about who is
    /// sworn to whom, which left the whole of diplomacy — tension,
    /// stances, tributaries, hegemony — readable one realm page at a time
    /// and nowhere else, though it is among the largest systems here.
    Relations,
}

impl Layer {
    pub fn name(self) -> &'static str {
        match self {
            Layer::Political => "political",
            Layer::Terrain => "terrain",
            Layer::Culture => "culture",
            Layer::Magic => "mana",
            Layer::Population => "population",
            Layer::Biomes => "biomes",
            Layer::Goods => "goods",
            Layer::Trade => "trade",
            Layer::Harvest => "harvest",
            Layer::Knowledge => "knowledge",
            Layer::Settling => "settling",
            Layer::Fighting => "fighting",
            Layer::Frontier => "frontier",
            Layer::Drift => "drift",
            Layer::Relations => "relations",
            Layer::Memory => "memory",
            Layer::Settled => "settled",
        }
    }
    pub fn all() -> [Layer; 17] {
        [
            Layer::Political,
            Layer::Terrain,
            Layer::Culture,
            Layer::Magic,
            Layer::Population,
            Layer::Biomes,
            Layer::Goods,
            Layer::Trade,
            Layer::Harvest,
            Layer::Knowledge,
            Layer::Settling,
            Layer::Fighting,
            Layer::Frontier,
            Layer::Drift,
            Layer::Relations,
            Layer::Memory,
            Layer::Settled,
        ]
    }

    /// Which family a layer belongs to, for stepping between kinds of
    /// question rather than between individual layers.
    ///
    /// Ten layers is too many to cycle one at a time — a reader looking for
    /// the trade map should not have to pass through biomes and mana to
    /// reach it. Tab moves between families, and within a family the layers
    /// are variations on one question.
    pub fn family(self) -> LayerFamily {
        match self {
            Layer::Political | Layer::Relations => LayerFamily::Power,
            Layer::Culture | Layer::Magic => LayerFamily::People,
            Layer::Terrain | Layer::Biomes => LayerFamily::Land,
            Layer::Population | Layer::Goods | Layer::Trade | Layer::Harvest | Layer::Knowledge => {
                LayerFamily::Living
            }
            Layer::Settling | Layer::Fighting | Layer::Frontier | Layer::Drift => {
                LayerFamily::Motion
            }
            Layer::Memory | Layer::Settled => LayerFamily::Land,
        }
    }
    /// The next layer within the same family, wrapping.
    ///
    /// Stepping *within* a family rather than across the whole list is what
    /// keeps ten layers navigable: Tab moves between kinds of question and
    /// this moves between the variations on one.
    pub fn next(self) -> Layer {
        let within = self.family().layers();
        let i = within.iter().position(|&l| l == self).unwrap_or(0);
        within[(i + 1) % within.len()]
    }
    pub fn prev(self) -> Layer {
        let within = self.family().layers();
        let i = within.iter().position(|&l| l == self).unwrap_or(0);
        within[(i + within.len() - 1) % within.len()]
    }
    /// The first layer of the next family, wrapping.
    pub fn next_family(self) -> Layer {
        Layer::step_family(self, 1)
    }
    pub fn prev_family(self) -> Layer {
        Layer::step_family(self, -1)
    }
    fn step_family(from: Layer, by: isize) -> Layer {
        let fams = LayerFamily::all();
        let here = from.family();
        let i = fams.iter().position(|&f| f == here).unwrap_or(0) as isize;
        let n = fams.len() as isize;
        let next = fams[((i + by).rem_euclid(n)) as usize];
        next.layers().first().copied().unwrap_or(Layer::Political)
    }
    pub fn from_name(s: &str) -> Option<Layer> {
        let s = s.to_lowercase();
        // Matching is by prefix, and every name starts with the empty
        // string: without this, a bare `:layer` would silently switch to
        // political instead of printing the usage line.
        if s.is_empty() {
            return None;
        }
        Layer::all().into_iter().find(|l| {
            l.name().starts_with(&s)
                || (s == "magic" && *l == Layer::Magic)
                || (s == "pop" && *l == Layer::Population)
                || (s == "mana" && *l == Layer::Magic)
                // What people call them when they are looking for them.
                || (s == "economy" && *l == Layer::Trade)
                || (s == "roads" && *l == Layer::Trade)
                || (s == "food" && *l == Layer::Harvest)
                || (s == "tech" && *l == Layer::Knowledge)
                || (s == "prosperity" && *l == Layer::Trade)
                || (s == "migration" && *l == Layer::Settling)
                || (s == "war" && *l == Layer::Fighting)
                || (s == "battles" && *l == Layer::Fighting)
                || (s == "climate" && *l == Layer::Drift)
                || (s == "weather" && *l == Layer::Drift)
                || (s == "diplomacy" && *l == Layer::Relations)
                || (s == "alliances" && *l == Layer::Relations)
                || (s == "tension" && *l == Layer::Relations)
                || (s == "history" && *l == Layer::Memory)
                || (s == "events" && *l == Layer::Memory)
                || (s == "age" && *l == Layer::Settled)
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Prompt {
    None,
    Command,
    Search,
}

pub struct Ui {
    pub world: World,
    screen: Screen,
    input: Input,
    pub mode: Mode,
    prev_mode: Mode,
    pub layer: Layer,
    pub cursor: (usize, usize),
    view: (usize, usize),
    speed_idx: usize,
    pub paused: bool,
    acc: f64,
    last: Instant,
    pub selected: Option<Ref>,
    pub list_tab: usize,
    pub list_idx: usize,
    pub list_scroll: usize,
    pub detail_scroll: usize,
    pub chron_scroll: usize,
    /// How far down the help page the reader has scrolled. It is longer than
    /// a small terminal, so it has to be able to move.
    pub help_scroll: usize,
    guide_scroll: usize,
    tutorial: Option<learn::Tutorial>,
    pub chron_min: u8,
    pub log_min: u8,
    pub ascii: bool,
    msg: String,
    msg_until: Instant,
    pub follow: bool,
    pub back: Vec<Ref>,
    fps_last: Instant,
    frames: u32,
    fps: u32,
    // vim-style input state
    mouse: bool,
    count: Option<usize>,
    pending: Option<char>,
    prompt: Prompt,
    prompt_text: String,
    search_results: Vec<Ref>,
    search_idx: usize,
    list_filter: String,
    chron_filter: String,
    run_until: Option<i32>,
    quit_armed: Option<Instant>,
    // click targets recorded while rendering
    log_rows: Vec<(usize, usize)>,
    power_rows: Vec<(usize, usize)>,
    list_y0: usize,
    chron_rows: Vec<(usize, usize)>,
    fate_rect: (usize, usize, usize, usize),
    last_click: Option<(usize, usize, Instant)>,
    pub save_path: Option<std::path::PathBuf>,
    pub autosave: i32,
    last_autosave: i32,
    pub theme: Theme,
    pub keymap: Vec<(Key, Key)>,
    pub zoom: usize,
    pub muted: Vec<EventKind>,
    story_rows: Vec<(usize, Ref)>,
    /// The one-line key under the map (`:legend` toggles it).
    pub show_legend: bool,
    /// The first-run card, offering a tutorial, guide or immediate play.
    tour: bool,
    recap_years: i32,
    recap_scroll: usize,
    recap_scope: Option<usize>,
    /// The chronicle length when the viewer last left the map, and the
    /// note built from whatever happened while they were away.
    /// The year the viewer left the map, for the "while you were away"
    /// note. A year and not an index into the chronicle: compaction removes
    /// events, so a stored index slides backwards through history and the
    /// note then reports things from long before they left.
    away_year: Option<i32>,
    since_note: Option<String>,
    last_mode: Mode,
}

/// Where saves and the "tour has been seen" marker live.
pub fn data_dir() -> std::path::PathBuf {
    if let Ok(x) = std::env::var("XDG_DATA_HOME") {
        if !x.is_empty() {
            return std::path::PathBuf::from(x).join("empires");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::PathBuf::from(home).join(".local/share/empires")
}

/// True the very first time the game is started here: no config file, no
/// data directory, and no marker saying the card has already been read.
fn first_run() -> bool {
    let dir = data_dir();
    !dir.join(".tour-seen").exists() && !dir.exists() && !crate::config::config_path().exists()
}

fn mark_tour_seen() {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(".tour-seen"), b"seen\n");
}

/// The card a first-time viewer sees, with optional help before watching.
pub const TOUR: &[&str] = &[
    "A world that rises and falls on its own.",
    "",
    "Watch history unfold, or pause and explore.",
    "The map legend explains its symbols; the sidebar describes each place.",
    "",
    "t  Start a short, skippable interface tutorial",
    "p  Read the player guide",
    "Space pauses time. Arrows move. Enter inspects. ? opens help.",
    "e browses realms and people; r recaps history; c opens the chronicle.",
    "The bottom rows show the keys for each screen. Esc takes you back.",
    "Time starts at 2 years/sec. The map stays put unless you enable f.",
    "",
    "Any other key: watch the world.",
];

pub fn run(
    world: World,
    ascii: bool,
    mouse: bool,
    save_path: Option<String>,
    cfg: &Config,
    tour: bool,
) {
    if !term::enter(mouse) {
        eprintln!("empires: stdin/stdout is not a terminal. Use --headless N to print a chronicle instead.");
        return;
    }
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        term::leave();
        default_hook(info);
    }));
    let (w, h) = term::size();
    let mut ui = Ui::new(world, ascii, mouse, w, h);
    ui.save_path = save_path.map(std::path::PathBuf::from);
    ui.apply_config(cfg);
    ui.tour = tour || first_run();
    if cfg.errors.is_empty() {
        ui.say("? for keys   : for commands   / to search   :w to save");
    } else {
        ui.say(&format!("config: {}", cfg.errors.join("; ")));
    }
    ui.msg_until = Instant::now() + Duration::from_secs(8);
    ui.main_loop();
    if ui.save_path.is_some() {
        let _ = ui.save(None);
    }
    term::leave();
}

/// Render one frame offscreen after simulating `years`, writing a plain
/// text dump and a coloured HTML page. Used for testing and screenshots.
/// Build a `Ui` over a world and render `frames` frames, returning the
/// average milliseconds a frame took.
///
/// Only for measurement: the interactive loop renders every frame, so what
/// a frame costs at year five thousand is the number that decides whether
/// the game still feels responsive in a long world.
/// A paused interface over a given world, for the tests that exercise what
/// the interface can produce rather than how it is driven.
#[cfg(test)]
pub(crate) fn for_test(world: World, cols: usize, rows: usize) -> Ui {
    let mut ui = Ui::new(world, false, false, cols, rows);
    ui.paused = true;
    ui
}

/// A paused interface looking at a given layer with the cursor on a given
/// cell, for the tests that check what reaches the screen.
#[cfg(test)]
pub(crate) fn for_test_at(world: World, cols: usize, rows: usize, layer: Layer, cell: usize) -> Ui {
    let mut ui = for_test(world, cols, rows);
    ui.layer = layer;
    let (x, y) = ui.world.terrain.xy(cell);
    ui.cursor = (x, y);
    ui.center_view();
    ui
}

#[cfg(test)]
pub(crate) fn time_frames(world: World, cols: usize, rows: usize, frames: u32) -> f64 {
    let mut ui = Ui::new(world, false, false, cols, rows);
    ui.paused = true;
    let t = std::time::Instant::now();
    for _ in 0..frames {
        ui.render();
    }
    t.elapsed().as_secs_f64() * 1000.0 / f64::from(frames)
}

pub fn snapshot(
    world: World,
    ascii: bool,
    cols: usize,
    rows: usize,
    years: i32,
    layer: &str,
    path: &str,
) {
    let mut ui = Ui::new(world, ascii, false, cols, rows);
    ui.layer = Layer::from_name(layer).unwrap_or(Layer::Political);
    for _ in 0..years {
        ui.world.tick();
    }
    // Look at the largest realm's capital.
    let mut ps = ui.world.living_polities();
    ps.sort_by_key(|&p| std::cmp::Reverse(ui.world.polities[p].cells));
    if let Some(&p) = ps.first() {
        if let Some(c) = ui.world.capital_cell(p) {
            ui.goto_cell(c);
        }
        ui.selected = Some(Ref::Polity(p));
    }
    ui.paused = true;
    match layer {
        "detail" => ui.mode = Mode::Detail,
        "city" => {
            let mut cs: Vec<usize> = (0..ui.world.cities.len())
                .filter(|&c| ui.world.cities[c].destroyed.is_none())
                .collect();
            cs.sort_by(|&a, &b| ui.world.cities[b].pop.total_cmp(&ui.world.cities[a].pop));
            if let Some(&c) = cs.first() {
                ui.selected = Some(Ref::City(c));
                ui.mode = Mode::Detail;
            }
        }
        "school" => {
            if let Some(s) = ui
                .world
                .schools
                .iter()
                .filter(|s| s.alive())
                .max_by(|a, b| a.total_influence().total_cmp(&b.total_influence()))
            {
                ui.selected = Some(Ref::School(s.id));
                ui.mode = Mode::Detail;
            }
        }
        "war" => {
            let mut ws: Vec<usize> = (0..ui.world.wars.len()).collect();
            ws.sort_by_key(|&x| {
                (
                    ui.world.wars[x].ended.is_some(),
                    std::cmp::Reverse(ui.world.wars[x].battles),
                )
            });
            if let Some(&x) = ws.first() {
                ui.selected = Some(Ref::War(x));
                ui.mode = Mode::Detail;
            }
        }
        "person" => {
            let mut ps: Vec<usize> = (0..ui.world.persons.len()).collect();
            ps.sort_by(|&a, &b| {
                ui.world.persons[b]
                    .renown
                    .total_cmp(&ui.world.persons[a].renown)
            });
            if let Some(&i) = ps.first() {
                ui.selected = Some(Ref::Person(i));
                ui.mode = Mode::Detail;
            }
        }
        // As if the viewer had spent the last stretch of history on another
        // screen and just come back to the map.
        "missed" => {
            // Far enough back that a good stretch of history counts as
            // missed, in years rather than entries.
            ui.away_year = Some(ui.world.year - 40);
            ui.last_mode = Mode::List;
        }
        "recap" => {
            ui.selected = None;
            ui.open_recap(Some(50));
        }
        "recap-realm" => ui.open_recap(Some(100)),
        "tour" => ui.tour = true,
        "list" => ui.mode = Mode::List,
        // Any list page by name, so a snapshot can depict one. `:list
        // wealth` reaches the same page in play.
        name if detail::LIST_TABS
            .iter()
            .any(|t| t.eq_ignore_ascii_case(name)) =>
        {
            ui.list_tab = detail::LIST_TABS
                .iter()
                .position(|t| t.eq_ignore_ascii_case(name))
                .unwrap_or(0);
            ui.mode = Mode::List;
        }
        "chronicle" => {
            ui.mode = Mode::Chronicle;
            ui.chron_min = 2;
        }
        "help" => ui.mode = Mode::Help,
        "guide" => ui.mode = Mode::Guide,
        "tutorial" => ui.start_tutorial(),
        "fate" => ui.mode = Mode::Fate,
        "zoom2" => ui.set_zoom(2),
        "zoom3" => ui.set_zoom(3),
        "paper" => ui.theme = Theme::Paper,
        "phosphor" => ui.theme = Theme::Phosphor,
        "search" => {
            ui.prompt = Prompt::Search;
            ui.prompt_text = "king".into();
        }
        _ => {}
    }
    ui.compose();
    let mut txt = String::new();
    let mut html = String::from("<!doctype html><meta charset=utf-8><style>body{background:#000;margin:0}pre{font:13px/1.15 monospace;color:#ccc;padding:8px}</style><pre>");
    let mut last: Option<(Rgb, Rgb, u8)> = None;
    for y in 0..ui.screen.h {
        for x in 0..ui.screen.w {
            let c = ui.screen.cell(x, y);
            txt.push(c.ch);
            let st = (c.fg, c.bg, c.attr);
            if last != Some(st) {
                if last.is_some() {
                    html.push_str("</span>");
                }
                html.push_str(&format!(
                    "<span style=\"color:rgb({},{},{});background:rgb({},{},{}){}{}\">",
                    c.fg.0,
                    c.fg.1,
                    c.fg.2,
                    c.bg.0,
                    c.bg.1,
                    c.bg.2,
                    if c.attr & BOLD != 0 {
                        ";font-weight:bold"
                    } else {
                        ""
                    },
                    if c.attr & REVERSE != 0 {
                        ";outline:2px solid #fff"
                    } else {
                        ""
                    }
                ));
                last = Some(st);
            }
            let ch = match c.ch {
                '<' => "&lt;".to_string(),
                '&' => "&amp;".to_string(),
                ch => ch.to_string(),
            };
            html.push_str(&ch);
        }
        txt.push('\n');
        html.push('\n');
    }
    html.push_str("</span></pre>");
    let _ = std::fs::write(format!("{}.txt", path), txt);
    let _ = std::fs::write(format!("{}.html", path), html);
}

impl Ui {
    fn new(world: World, ascii: bool, mouse: bool, w: usize, h: usize) -> Ui {
        let mut ui = Ui {
            world,
            screen: Screen::new(w, h),
            input: Input::new(),
            mode: Mode::Map,
            prev_mode: Mode::Map,
            layer: Layer::Political,
            cursor: (0, 0),
            view: (0, 0),
            speed_idx: 2,
            paused: false,
            acc: 0.0,
            last: Instant::now(),
            selected: None,
            list_tab: 0,
            list_idx: 0,
            list_scroll: 0,
            detail_scroll: 0,
            chron_scroll: 0,
            help_scroll: 0,
            guide_scroll: 0,
            tutorial: None,
            chron_min: 1,
            // Level 2, the notable: about a line a year, which is a feed a
            // reader can follow at two years a second. Level 1 is roughly
            // seven lines a year, which at that speed is a blur.
            log_min: 2,
            ascii,
            msg: String::new(),
            msg_until: Instant::now(),
            follow: false,
            back: Vec::new(),
            fps_last: Instant::now(),
            frames: 0,
            fps: 0,
            mouse,
            count: None,
            pending: None,
            prompt: Prompt::None,
            prompt_text: String::new(),
            search_results: Vec::new(),
            search_idx: 0,
            list_filter: String::new(),
            chron_filter: String::new(),
            run_until: None,
            quit_armed: None,
            log_rows: Vec::new(),
            power_rows: Vec::new(),
            list_y0: 2,
            chron_rows: Vec::new(),
            fate_rect: (0, 0, 0, 0),
            last_click: None,
            save_path: None,
            autosave: 100,
            last_autosave: 0,
            theme: Theme::Default,
            keymap: Vec::new(),
            zoom: 1,
            muted: Vec::new(),
            story_rows: Vec::new(),
            show_legend: true,
            tour: false,
            recap_years: 50,
            recap_scroll: 0,
            recap_scope: None,
            away_year: None,
            since_note: None,
            last_mode: Mode::Map,
        };
        // Start looking at the first race's homeland.
        let home = ui.world.races.first().map(|r| r.home).unwrap_or(0);
        ui.cursor = ui.world.terrain.xy(home);
        ui.center_view();
        ui
    }

    pub fn apply_config(&mut self, cfg: &Config) {
        if let Some(d) = cfg.detail {
            self.world.detail = d;
        }
        if let Some(v) = cfg.speed {
            let (i, _) = SPEEDS
                .iter()
                .enumerate()
                .min_by(|a, b| (a.1 - v).abs().total_cmp(&(b.1 - v).abs()))
                .unwrap();
            self.speed_idx = i;
        }
        if let Some(t) = &cfg.theme {
            if let Some(t) = Theme::from_name(t) {
                self.theme = t;
            }
        }
        if let Some(m) = cfg.mouse {
            if m != self.mouse {
                self.mouse = m;
                term::set_mouse(m);
            }
        }
        if let Some(a) = cfg.ascii {
            self.ascii = a;
        }
        if let Some(a) = cfg.autosave {
            self.autosave = a.max(0);
        }
        if let Some(l) = cfg.log {
            self.log_min = l.clamp(1, 3);
        }
        if let Some(f) = cfg.follow {
            self.follow = f;
        }
        if let Some(z) = cfg.zoom {
            self.zoom = z.clamp(1, 4);
        }
        for (a, b) in &cfg.maps {
            self.keymap.retain(|(x, _)| x != a);
            self.keymap.push((a.clone(), b.clone()));
        }
    }

    fn main_loop(&mut self) {
        loop {
            let (w, h) = term::size();
            self.screen.resize(w, h);
            let mut timeout = if self.paused { 120 } else { 16 };
            let mut quit = false;
            while let Some(k) = self.input.poll_key(timeout) {
                timeout = 0;
                if !self.handle_key(k) {
                    quit = true;
                    break;
                }
            }
            if quit {
                break;
            }
            let now = Instant::now();
            let dt = now.duration_since(self.last).as_secs_f64().min(0.25);
            self.last = now;
            if !self.paused {
                self.acc += dt * SPEEDS[self.speed_idx];
                let n = self.acc.floor() as usize;
                if n > 0 {
                    self.acc -= n as f64;
                    let max_per_frame = (SPEEDS[self.speed_idx] / 12.0).ceil().max(1.0) as usize;
                    for _ in 0..n.min(max_per_frame) {
                        self.world.tick();
                        if let Some(y) = self.run_until {
                            if self.world.year >= y {
                                self.paused = true;
                                self.run_until = None;
                                self.say(&format!("reached year {}", y));
                                break;
                            }
                        }
                    }
                    self.follow_events();
                    if self.autosave > 0
                        && self.save_path.is_some()
                        && self.world.year - self.last_autosave >= self.autosave
                    {
                        let _ = self.save(None);
                    }
                }
            }
            self.render();
            self.frames += 1;
            if self.fps_last.elapsed() >= Duration::from_secs(1) {
                self.fps = self.frames;
                self.frames = 0;
                self.fps_last = Instant::now();
            }
        }
    }

    /// When following, jump the cursor to the latest major event.
    /// Jump the cursor to this year's most recent notable event.
    ///
    /// Asks the year rather than remembering an index. It used to keep a
    /// watermark into `chronicle.events` and scan everything above it —
    /// but `Chronicle::compact` *removes* events, so the watermark pointed
    /// at a different event afterwards and the scan was skipped entirely
    /// for that tick. An index into a vector that shrinks is not a name for
    /// anything, which is the rule the rest of this tree already keeps.
    ///
    /// This runs once a tick, so "logged this year" is exactly "logged
    /// since I last looked", and a year is a fact about an event that
    /// compaction cannot touch.
    fn follow_events(&mut self) {
        if !self.follow || self.mode != Mode::Map {
            return;
        }
        let year = self.world.year;
        let target = self
            .world
            .chronicle
            .events
            .iter()
            .rev()
            .take_while(|e| e.year >= year)
            .find(|e| e.importance >= 2 && e.loc.is_some())
            .and_then(|e| e.loc);
        if let Some(l) = target {
            self.goto_cell(l);
        }
    }

    // -- the view: which part of the world is on screen -----------------------

    /// Wrap whole key/action pairs; never strand a key on the previous line.
    fn navigation_lines(&self) -> Vec<String> {
        if self.screen.h < 16 || self.tutorial.is_some() {
            return Vec::new();
        }
        let width = self.screen.w.saturating_sub(2);
        let mut lines = Vec::new();
        for group in words::navigation(self.mode, self.paused) {
            let mut row = String::new();
            for action in group.split("  ") {
                if !row.is_empty() && row.chars().count() + 2 + action.chars().count() > width {
                    lines.push(std::mem::take(&mut row));
                }
                if !row.is_empty() {
                    row.push_str("  ");
                }
                row.push_str(action);
            }
            if !row.is_empty() {
                lines.push(row);
            }
        }
        lines.truncate(self.screen.h.saturating_sub(8).min(5));
        lines
    }

    /// Keep the active category visible; the mouse uses these same positions.
    fn list_tab_layout(&self) -> Vec<(usize, usize)> {
        let width = self.screen.w.saturating_sub(2);
        let mut start = self.list_tab;
        let mut used = detail::LIST_TABS[start].chars().count() + 2;
        while start > 0 {
            let prev = detail::LIST_TABS[start - 1].chars().count() + 3;
            if used + prev > width {
                break;
            }
            start -= 1;
            used += prev;
        }
        let mut x = 1;
        let mut tabs = Vec::new();
        for (i, name) in detail::LIST_TABS.iter().enumerate().skip(start) {
            let len = name.chars().count() + 2;
            if x + len > self.screen.w.saturating_sub(1) {
                break;
            }
            tabs.push((i, x));
            x += len + 1;
        }
        tabs
    }

    fn content_height(&self) -> usize {
        self.screen
            .h
            .saturating_sub(self.navigation_lines().len() + 1)
    }

    fn map_rect(&self) -> (usize, usize, usize, usize) {
        let sw = self.screen.w;
        let sh = self.screen.h;
        let sidebar = if sw >= 100 { 36 } else { 0 };
        let log_h = if sh >= 34 {
            8
        } else if sh >= 24 {
            5
        } else {
            3
        };
        let mw = sw.saturating_sub(sidebar);
        let mh = self.content_height().saturating_sub(log_h);
        (0, 0, mw.max(1), mh.max(1))
    }

    /// Visible world size in cells at the current zoom.
    fn view_dims(&self) -> (usize, usize) {
        let (_, _, mw, mh) = self.map_rect();
        (mw * self.zoom, mh * self.zoom)
    }

    /// Blank characters left of and above the world in the map pane.
    ///
    /// Zoomed out far enough the whole world is smaller than the pane. It
    /// then sits in the middle of it rather than in the top-left corner,
    /// and every mapping between screen and world — the renderer, the map
    /// labels and the mouse — goes through this.
    pub(crate) fn view_pad(&self) -> (usize, usize) {
        let (_, _, mw, mh) = self.map_rect();
        (
            pane_pad(mw, self.world.terrain.w, self.zoom),
            pane_pad(mh, self.world.terrain.h, self.zoom),
        )
    }

    fn center_view(&mut self) {
        let (vw, vh) = self.view_dims();
        let tw = self.world.terrain.w;
        let th = self.world.terrain.h;
        let ox = self
            .cursor
            .0
            .saturating_sub(vw / 2)
            .min(tw.saturating_sub(vw));
        let oy = self
            .cursor
            .1
            .saturating_sub(vh / 2)
            .min(th.saturating_sub(vh));
        self.view = (ox, oy);
    }

    fn clamp_view(&mut self) {
        let (vw, vh) = self.view_dims();
        let z = self.zoom;
        let tw = self.world.terrain.w;
        let th = self.world.terrain.h;
        let (cx, cy) = self.cursor;
        let (mut ox, mut oy) = self.view;
        if cx < ox + 2 * z {
            ox = cx.saturating_sub(2 * z);
        }
        if cx + 3 * z > ox + vw {
            ox = (cx + 3 * z).saturating_sub(vw);
        }
        if cy < oy + z {
            oy = cy.saturating_sub(z);
        }
        if cy + 2 * z > oy + vh {
            oy = (cy + 2 * z).saturating_sub(vh);
        }
        ox = ox.min(tw.saturating_sub(vw));
        oy = oy.min(th.saturating_sub(vh));
        self.view = (ox, oy);
    }

    fn set_zoom(&mut self, z: usize) {
        self.zoom = z.clamp(1, 4);
        self.center_view();
        self.say(&format!(
            "zoom: {} cell{} per character",
            self.zoom,
            if self.zoom == 1 { "" } else { "s" }
        ));
    }

    // -- messages, jumping and the selection ----------------------------------

    pub fn say(&mut self, s: &str) {
        self.msg = s.to_string();
        self.msg_until = Instant::now() + Duration::from_millis(2500);
    }

    pub fn goto_cell(&mut self, cell: usize) {
        self.cursor = self.world.terrain.xy(cell);
        self.center_view();
    }

    fn goto_ref(&mut self, r: Ref) {
        self.selected = Some(r);
        if let Some(cell) = detail::entity_loc(&self.world, r) {
            self.goto_cell(cell);
        }
    }

    pub fn open_detail(&mut self, r: Ref) {
        if let Some(cur) = self.selected {
            if self.mode == Mode::Detail && cur != r {
                self.back.push(cur);
                if self.back.len() > 32 {
                    self.back.remove(0);
                }
            }
        }
        self.selected = Some(r);
        self.detail_scroll = 0;
        if self.mode != Mode::Detail {
            self.prev_mode = self.mode;
        }
        self.mode = Mode::Detail;
    }

    fn jump_to_selected(&mut self) {
        match self.selected {
            Some(r) => self.goto_ref(r),
            None => {
                let mut ps = self.world.living_polities();
                ps.sort_by_key(|&p| std::cmp::Reverse(self.world.polities[p].cells));
                if let Some(&p) = ps.first() {
                    self.goto_ref(Ref::Polity(p));
                }
            }
        }
    }

    /// Open whatever chronicle entry `e` is, if it is still there.
    ///
    /// `log_rows` and `chron_rows` hold raw indices from the frame that drew
    /// them, and compaction can remove events between a frame and the click
    /// that follows it. Today the loop polls input before ticking so the
    /// window is closed, but that is an ordering accident rather than a
    /// guarantee, and the cost of not relying on it is one `get`.
    fn jump_to_event(&mut self, e: usize) {
        let Some((loc, first, year)) = self
            .world
            .chronicle
            .events
            .get(e)
            .map(|ev| (ev.loc, ev.refs.first().copied(), ev.year))
        else {
            return;
        };
        if let Some(r) = first {
            self.selected = Some(r);
        }
        match loc {
            Some(l) => self.goto_cell(l),
            None => {
                if let Some(r) = first {
                    self.goto_ref(r);
                }
            }
        }
        self.say(&format!("year {}", year));
    }

    pub fn entity_at_cursor(&self) -> Option<Ref> {
        let i = self.world.terrain.idx(self.cursor.0, self.cursor.1);
        let cs = &self.world.cells[i];
        if let Some(c) = cs.city {
            if self.world.cities[c].destroyed.is_none() {
                return Some(Ref::City(c));
            }
        }
        match self.layer {
            Layer::Culture => {
                if let Some(c) = cs.culture {
                    return Some(Ref::Culture(c));
                }
            }
            Layer::Magic => {
                if let Some(p) = cs.owner {
                    if let Some(s) = self.world.polities[p].school {
                        return Some(Ref::School(s));
                    }
                }
            }
            _ => {}
        }
        if let Some(p) = cs.owner {
            return Some(Ref::Polity(p));
        }
        if let Some(c) = cs.culture {
            return Some(Ref::Culture(c));
        }
        if let Some(f) = self
            .world
            .terrain
            .river_at(i)
            .or_else(|| self.world.terrain.feature_at(i))
        {
            return Some(Ref::Feature(f));
        }
        None
    }

    /// The rows of the open list page, and how many the page is a window on.
    ///
    /// The second number is the world's count before the page was trimmed,
    /// so the footer can say when there is more history than fits.
    /// Compose one frame and say whether `mark` reached the screen.
    ///
    /// The screen buffer is the interface's own business, so a test asks a
    /// question about it rather than being handed it.
    #[cfg(test)]
    pub(crate) fn frame_shows(&mut self, mark: char) -> bool {
        self.compose();
        (0..self.screen.h).any(|y| (0..self.screen.w).any(|x| self.screen.cell(x, y).ch == mark))
    }

    fn list_rows(&self) -> (Vec<(String, Ref)>, usize) {
        let (rows, total) = detail::list_rows(&self.world, self.list_tab);
        if self.list_filter.is_empty() {
            return (rows, total);
        }
        // A filter term is a word to look for or a comparison against a
        // named quantity — `lands>200`, `income<0` — so a list can be asked
        // what things *are* and not only what they are called. See
        // [`query`].
        let terms = query::parse(&self.list_filter);
        (
            rows.into_iter()
                .filter(|(text, r)| query::matches(&self.world, *r, text, &terms))
                .collect(),
            total,
        )
    }

    fn cycle_realm(&mut self, delta: i32) {
        let mut ps = self.world.living_polities();
        if ps.is_empty() {
            return;
        }
        ps.sort_by_key(|&p| std::cmp::Reverse(self.world.polities[p].cells));
        let cur = match self.selected {
            Some(Ref::Polity(p)) => ps.iter().position(|&x| x == p).map(|i| i as i32),
            _ => None,
        };
        let len = ps.len() as i32;
        let next = match cur {
            Some(i) => (i + delta).rem_euclid(len),
            None => {
                if delta > 0 {
                    0
                } else {
                    len - 1
                }
            }
        };
        let p = ps[next as usize];
        self.goto_ref(Ref::Polity(p));
        if self.mode == Mode::Detail {
            self.open_detail(Ref::Polity(p));
        }
        self.say(&format!(
            "realm {} of {}: {}",
            next + 1,
            len,
            self.world.polities[p].name
        ));
    }

    fn cycle_city(&mut self, delta: i32) {
        let mut cs: Vec<usize> = (0..self.world.cities.len())
            .filter(|&c| self.world.cities[c].destroyed.is_none())
            .collect();
        if cs.is_empty() {
            return;
        }
        cs.sort_by(|&a, &b| {
            self.world.cities[b]
                .pop
                .total_cmp(&self.world.cities[a].pop)
        });
        let cur = match self.selected {
            Some(Ref::City(c)) => cs.iter().position(|&x| x == c).map(|i| i as i32),
            _ => None,
        };
        let len = cs.len() as i32;
        let next = match cur {
            Some(i) => (i + delta).rem_euclid(len),
            None => {
                if delta > 0 {
                    0
                } else {
                    len - 1
                }
            }
        };
        let c = cs[next as usize];
        self.goto_ref(Ref::City(c));
        if self.mode == Mode::Detail {
            self.open_detail(Ref::City(c));
        }
        self.say(&format!(
            "city {} of {}: {} ({:.0}k)",
            next + 1,
            len,
            self.world.cities[c].name,
            self.world.cities[c].pop
        ));
    }

    // -- searching, the storyteller and the Hand of Fate ----------------------

    /// Rank every named thing in the world against a query.
    pub fn search(&self, q: &str) -> Vec<Ref> {
        let q = q.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        let w = &self.world;
        let mut hits: Vec<(i32, Ref)> = Vec::new();
        let mut consider = |names: &[&str], alive: bool, weight: i32, r: Ref| {
            let mut best = None;
            for n in names {
                let n = n.to_lowercase();
                let score = if n == q {
                    100
                } else if n.starts_with(&q) {
                    60
                } else if n
                    .split(|c: char| !c.is_alphanumeric())
                    .any(|word| word.starts_with(&q))
                {
                    40
                } else if n.contains(&q) {
                    20
                } else {
                    continue;
                };
                best = Some(best.unwrap_or(0).max(score));
            }
            if let Some(s) = best {
                hits.push((s + weight + if alive { 10 } else { 0 }, r));
            }
        };
        for p in &w.polities {
            consider(&[&p.name, &p.short], p.alive(), 5, Ref::Polity(p.id));
        }
        for c in &w.cities {
            consider(&[&c.name], c.destroyed.is_none(), 4, Ref::City(c.id));
        }
        for c in &w.cultures {
            consider(
                &[&c.name, &c.plural, &c.adj],
                c.extinct.is_none(),
                3,
                Ref::Culture(c.id),
            );
        }
        for s in &w.schools {
            consider(&[&s.name, &s.short], s.alive(), 3, Ref::School(s.id));
        }
        for (i, per) in w.persons.iter().enumerate() {
            consider(&[&per.full_name()], per.alive(), 1, Ref::Person(i));
        }
        for x in &w.wars {
            consider(&[&x.name], x.alive(), 2, Ref::War(x.id));
        }
        for (i, f) in w.terrain.features.iter().enumerate() {
            if let Some(n) = &f.name {
                consider(&[n], true, 2, Ref::Feature(i));
            }
        }
        for a in &w.artifacts {
            consider(
                &[&a.name],
                !matches!(a.holder, crate::sim::Holder::Lost),
                3,
                Ref::Artifact(a.id),
            );
        }
        hits.sort_by_key(|h| std::cmp::Reverse(h.0));
        hits.truncate(60);
        hits.into_iter().map(|(_, r)| r).collect()
    }

    /// Open the digest, for the whole world or for the realm in hand.
    pub(super) fn open_recap(&mut self, years: Option<i32>) {
        if let Some(y) = years {
            self.recap_years = y.clamp(1, 100_000);
        }
        self.recap_scope = match self.selected {
            Some(Ref::Polity(p)) => Some(p),
            Some(Ref::City(c)) => self.world.cities[c].polity,
            _ => None,
        };
        if self.mode != Mode::Recap {
            self.prev_mode = self.mode;
        }
        self.mode = Mode::Recap;
        self.recap_scroll = 0;
    }

    fn open_fate(&mut self) {
        if self.selected.is_none() {
            self.selected = self.entity_at_cursor();
        }
        // A realm, a town, a person or a stretch of country. It used to
        // reach realms alone, so the only way to touch a city was to find
        // whichever realm happened to hold it and act on all of that
        // instead — and a city is the unit most of this world's history
        // actually happens to.
        //
        // Failing all of those, the region under the cursor, so that there
        // is always something to act on even where nobody lives.
        let target = self
            .selected
            .filter(|&r| detail::fate_reaches(&self.world, r))
            .or_else(|| {
                let i = self.world.terrain.idx(self.cursor.0, self.cursor.1);
                self.world.terrain.feature_at(i).map(Ref::Feature)
            });
        match target {
            Some(r) => {
                self.selected = Some(r);
                self.prev_mode = self.mode;
                self.mode = Mode::Fate;
            }
            None => self.say("nothing here the hand of fate can reach"),
        }
    }

    fn choose_fate(&mut self, r: Ref, choice: usize) {
        let msg = detail::hand_of_fate_on(&mut self.world, r, choice as u8);
        self.say(&msg);
        self.mode = self.prev_mode;
        // What was done may have changed what the derived indexes say — a
        // blighted region, an emptied one, a town that has doubled — and the
        // next frame reads them.
        self.world.recompute();
    }
}

pub fn biome_style(b: Biome, ascii: bool) -> (Rgb, char) {
    let (c, g, a) = match b {
        Biome::DeepOcean => (Rgb(8, 24, 58), '~', '~'),
        Biome::Ocean => (Rgb(14, 40, 88), '~', '~'),
        Biome::Shallows => (Rgb(30, 76, 124), '≈', '~'),
        Biome::Lake => (Rgb(40, 90, 140), '≈', '~'),
        Biome::Ice => (Rgb(215, 225, 235), '*', '*'),
        Biome::Tundra => (Rgb(140, 140, 120), '.', '.'),
        Biome::Taiga => (Rgb(40, 82, 62), '♠', 't'),
        Biome::Steppe => (Rgb(150, 140, 85), ',', ','),
        Biome::Grassland => (Rgb(88, 138, 58), '"', '"'),
        Biome::Forest => (Rgb(34, 96, 46), '♣', 'T'),
        Biome::Jungle => (Rgb(20, 78, 30), '♣', '%'),
        Biome::Savanna => (Rgb(165, 145, 70), ',', ','),
        Biome::Desert => (Rgb(196, 170, 108), ':', ':'),
        Biome::Swamp => (Rgb(58, 88, 72), '"', '"'),
        Biome::Hills => (Rgb(112, 102, 72), '∩', 'n'),
        Biome::Mountain => (Rgb(120, 112, 104), '▲', '^'),
        Biome::Peak => (Rgb(190, 190, 195), '▲', '^'),
        Biome::Wastes => (Rgb(70, 50, 76), '¤', '%'),
    };
    (c, if ascii { a } else { g })
}

/// A trade good's colour and glyph, for the Goods layer and its key.
///
/// The same bargain as [`biome_style`]: the map and the legend both call
/// this, so the key cannot drift from what is drawn. Hues are grouped by
/// what a good *is* — greens grow, browns are dug up, blues come out of the
/// water — so that a glance at the map separates farmland from mining
/// country without reading the key at all.
pub fn good_style(g: crate::sim::trade::Good, ascii: bool) -> (Rgb, char) {
    use crate::sim::trade::Good;
    let (c, uni, a) = match g {
        Good::Grain => (Rgb(214, 190, 90), '⁂', 'g'),
        Good::Livestock => (Rgb(168, 150, 96), 'ᴥ', 'c'),
        Good::Fish => (Rgb(90, 170, 200), '≈', 'f'),
        Good::Timber => (Rgb(46, 124, 60), '♣', 'T'),
        Good::Stone => (Rgb(150, 150, 156), '▣', 's'),
        Good::Metal => (Rgb(196, 132, 72), '◆', 'o'),
        Good::Salt => (Rgb(236, 236, 226), '▫', 'x'),
        Good::Furs => (Rgb(140, 100, 74), '❖', 'u'),
        Good::Spice => (Rgb(214, 96, 52), '✳', 'p'),
        Good::Wine => (Rgb(160, 60, 110), '❉', 'w'),
        Good::Horses => (Rgb(190, 164, 120), 'Ω', 'h'),
        Good::Leystone => (Rgb(170, 60, 230), '✦', 'l'),
    };
    (c, if ascii { a } else { uni })
}

/// The cells a straight line from `a` to `b` passes through, ends included.
///
/// Bresenham, because a trade route is stored as a pair of cities and has to
/// be drawn as a path. Deliberately ignorant of terrain: the line a reader
/// wants is the one joining the two towns, not the road a caravan would
/// actually pick through the hills, and a sea route has no road at all.
pub(crate) fn cells_between(t: &crate::geo::Terrain, a: usize, b: usize) -> Vec<usize> {
    let (x0, y0) = t.xy(a);
    let (x1, y1) = t.xy(b);
    let (mut x, mut y) = (x0 as i64, y0 as i64);
    let (x1, y1) = (x1 as i64, y1 as i64);
    let (dx, dy) = ((x1 - x).abs(), -(y1 - y).abs());
    let (sx, sy) = (if x < x1 { 1 } else { -1 }, if y < y1 { 1 } else { -1 });
    let mut err = dx + dy;
    // A line can be no longer than the map's diagonal; the bound is a guard
    // against a malformed save rather than an expected case.
    let mut out = Vec::with_capacity((dx.max(-dy) as usize) + 1);
    let limit = t.w + t.h + 2;
    for _ in 0..limit {
        if x >= 0 && y >= 0 && (x as usize) < t.w && (y as usize) < t.h {
            out.push(t.idx(x as usize, y as usize));
        }
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    out
}

/// Where a world `world` cells across starts in a pane `pane` characters
/// across, at `zoom` cells to the character: half the slack, so a world too
/// small to fill the pane is centred in it instead of pinned to a corner.
pub(crate) fn pane_pad(pane: usize, world: usize, zoom: usize) -> usize {
    let zoom = zoom.max(1);
    let need = (world + zoom - 1) / zoom;
    pane.saturating_sub(need) / 2
}

pub fn event_style(kind: EventKind, importance: u8) -> (Rgb, u8) {
    let c = match kind {
        EventKind::Genesis => Rgb(230, 200, 120),
        EventKind::Founding => Rgb(150, 220, 150),
        EventKind::Politics => Rgb(220, 200, 150),
        EventKind::Death => Rgb(180, 160, 160),
        EventKind::War => Rgb(240, 110, 100),
        EventKind::Battle => Rgb(220, 140, 120),
        EventKind::Peace => Rgb(150, 200, 230),
        EventKind::Disaster => Rgb(200, 220, 100),
        EventKind::Magic => Rgb(200, 140, 240),
        EventKind::Culture => Rgb(120, 210, 200),
        EventKind::Era => Rgb(255, 230, 140),
        EventKind::Wonder => Rgb(255, 210, 100),
        EventKind::Discovery => Rgb(140, 190, 240),
        EventKind::Person => Rgb(200, 200, 210),
    };
    let attr = match importance {
        3 => BOLD,
        0 => DIM,
        _ => 0,
    };
    (c, attr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::Detail;

    fn world() -> World {
        let mut w = World::new(7, 60, 30, Detail::Medium);
        for _ in 0..150 {
            w.tick();
        }
        w
    }

    /// The sidebar says what the Hand of Fate would act on.
    ///
    /// `x` reaches a realm, a town, a person or a stretch of country, and
    /// offers a different six for each — so pressing it without knowing
    /// which is selected is a guess. The footer cannot say: it is a fixed
    /// pair of strings. The block that describes the selection can.
    #[test]
    fn the_sidebar_says_what_the_hand_of_fate_would_touch() {
        let w = world();
        let city = (0..w.cities.len())
            .find(|&c| w.cities[c].destroyed.is_none())
            .expect("a 150 year world has a city");
        let realm = *w
            .alive_polities
            .first()
            .expect("a 150 year world has a realm");
        for (sel, want) in [(Ref::City(city), "x town"), (Ref::Polity(realm), "x realm")] {
            let mut ui = Ui::new(world(), false, false, 160, 45);
            ui.paused = true;
            ui.selected = Some(sel);
            ui.compose();
            let out = text(&ui);
            assert!(
                out.contains(want),
                "with {:?} selected the sidebar does not say {:?}",
                sel,
                want
            );
        }
    }

    /// Leaving the map and coming back must report only what happened
    /// while the viewer was gone.
    ///
    /// The mark used to be an index into `chronicle.events`, and
    /// `Chronicle::compact` removes events — so the index slid backwards
    /// through history and the note reported things from long before the
    /// viewer left. The guard against it only caught the rarer half: an
    /// index past the end is refused, but the common case is an index that
    /// still lands inside the vector, several thousand entries too early.
    /// The note then read "a thousand notable things over nine hundred
    /// years" for a two-minute absence.
    ///
    /// The mark is a year now, which is a fact about an event that
    /// compaction cannot shift. So: be away a known number of years across
    /// a compaction, and demand the note does not claim more.
    #[test]
    fn coming_back_reports_only_what_was_missed() {
        let mut w = World::new(23, 60, 30, Detail::Medium);
        // Small enough that compaction runs, and runs often.
        w.tuning.chronicle_cap = 300;
        for _ in 0..400 {
            w.tick();
        }
        let mut ui = Ui::new(w, false, false, 100, 40);
        ui.paused = true;
        // On the map, then away.
        ui.mode = Mode::Map;
        ui.compose();
        ui.mode = Mode::List;
        ui.compose();
        let left_in = ui.world.year;

        // Time passes, and the chronicle is compacted while it does.
        let before = ui.world.chronicle.dropped;
        for _ in 0..30 {
            ui.world.tick();
        }
        assert!(
            ui.world.chronicle.dropped > before,
            "no compaction happened, so this proves nothing"
        );
        let away = ui.world.year - left_in;

        ui.mode = Mode::Map;
        ui.compose();
        // There must *be* a note. The index-based mark produced none at all
        // once compaction had shrunk the chronicle below it — the report
        // silently stopped working — so a test that only checked the note
        // when one appeared passed happily against the bug.
        let note = ui
            .since_note
            .clone()
            .expect("thirty years of a busy world went unreported");
        // And the span it names must fit inside the time actually away.
        let rest = note
            .split(" over the ")
            .nth(1)
            .expect("a note about many things names a span");
        let years: i32 = rest
            .split_whitespace()
            .next()
            .and_then(|n| n.parse().ok())
            .expect("the span is a number");
        assert!(
            years <= away,
            "away for {} years, but the note claims {}: {:?}",
            away,
            years,
            note
        );
    }

    fn frame(ascii: bool, mode: Mode, cols: usize, rows: usize) -> Ui {
        let mut ui = Ui::new(world(), ascii, false, cols, rows);
        ui.paused = true;
        ui.selected = ui.world.living_polities().first().copied().map(Ref::Polity);
        ui.mode = mode;
        ui.compose();
        ui
    }

    fn text(ui: &Ui) -> String {
        let mut s = String::new();
        for y in 0..ui.screen.h {
            for x in 0..ui.screen.w {
                s.push(ui.screen.cell(x, y).ch);
            }
            s.push('\n');
        }
        s
    }

    const MODES: [Mode; 7] = [
        Mode::Map,
        Mode::List,
        Mode::Detail,
        Mode::Chronicle,
        Mode::Help,
        Mode::Guide,
        Mode::Recap,
    ];

    /// `--ascii` is a promise about the whole frame, not about the glyphs one
    /// widget happens to remember to convert.
    #[test]
    fn ascii_frames_contain_no_unicode() {
        for mode in MODES {
            for (cols, rows) in [(160, 45), (80, 24), (60, 20)] {
                let ui = frame(true, mode, cols, rows);
                let out = text(&ui);
                if let Some(c) = out.chars().find(|c| !c.is_ascii() && *c != '\n') {
                    panic!(
                        "{:?} at {}x{} drew {:?} ({:#x}) in --ascii mode",
                        mode, cols, rows, c, c as u32
                    );
                }
            }
        }
    }

    /// The complaint this answers: the political layer was the terrain layer
    /// in other colours, so a black-and-white screenshot of one was the same
    /// picture as the other. Borders and a neutral field must make them
    /// different frames, glyph for glyph, with no colour at all.
    #[test]
    fn ownership_is_visible_without_colour() {
        for ascii in [false, true] {
            let mut ui = Ui::new(world(), ascii, false, 120, 40);
            ui.paused = true;
            let of = |ui: &mut Ui, layer: Layer| {
                ui.layer = layer;
                ui.compose();
                text(ui)
            };
            let political = of(&mut ui, Layer::Political);
            let terrain = of(&mut ui, Layer::Terrain);
            let culture = of(&mut ui, Layer::Culture);
            assert_ne!(political, terrain, "ascii = {}", ascii);
            assert_ne!(culture, terrain, "ascii = {}", ascii);
            assert_ne!(political, culture, "ascii = {}", ascii);
            // And the difference is frontiers, not a stray glyph or two.
            let rules = if ascii {
                ['|', '-', '+']
            } else {
                ['│', '─', '┼']
            };
            for (name, frame) in [("political", &political), ("culture", &culture)] {
                let n = frame.chars().filter(|c| rules.contains(c)).count();
                assert!(n > 40, "{} layer drew {} border marks", name, n);
            }
        }
    }

    /// The key to the colours has to be on screen: with fixed reserves for
    /// the blocks above it, it was down to a single row at 44 rows.
    #[test]
    fn the_sidebar_names_the_realms_on_screen() {
        for (cols, rows, want) in [(150usize, 44usize, 4usize), (120, 40, 4), (100, 30, 2)] {
            let ui = frame(false, Mode::Map, cols, rows);
            assert!(
                ui.power_rows.len() >= want,
                "{}x{} named {} realms, wanted {}",
                cols,
                rows,
                ui.power_rows.len(),
                want
            );
        }
    }

    /// A world smaller than the pane sits in the middle of it, and every
    /// mapping between screen and world agrees about where that is.
    #[test]
    fn a_small_world_is_centred() {
        assert_eq!(pane_pad(100, 160, 2), 10);
        assert_eq!(pane_pad(100, 160, 1), 0);
        assert_eq!(pane_pad(100, 60, 1), 20);
        assert_eq!(pane_pad(9, 5, 2), 3);
        let mut ui = Ui::new(world(), false, false, 120, 40);
        ui.set_zoom(2);
        ui.paused = true;
        ui.compose();
        let (_, _, mw, mh) = ui.map_rect();
        let (padx, pady) = ui.view_pad();
        assert!(padx > 0 && pady > 0, "{} {}", padx, pady);
        // Blank on both sides, not just the right: the same number of empty
        // columns before the world as after it.
        for sy in pady..mh - pady {
            for sx in 0..padx {
                assert_eq!(ui.screen.cell(sx, sy).ch, ' ');
                assert_eq!(ui.screen.cell(mw - 1 - sx, sy).ch, ' ');
            }
        }
    }

    /// A click lands on the cell the eye is pointing at, margins and all:
    /// the renderer, the labels and the mouse all read the same padding.
    #[test]
    fn a_click_lands_where_the_world_is_drawn() {
        use crate::term::{Mouse, MouseKind};
        for zoom in [1usize, 2, 3] {
            let mut ui = Ui::new(world(), false, true, 120, 40);
            ui.paused = true;
            ui.set_zoom(zoom);
            ui.compose();
            let (padx, pady) = ui.view_pad();
            let (ox, oy) = ui.view;
            for (dx, dy) in [(0usize, 0usize), (3, 2), (7, 5)] {
                ui.handle_key(Key::Mouse(Mouse {
                    kind: MouseKind::Press(0),
                    x: padx + dx,
                    y: pady + dy,
                }));
                assert_eq!(ui.view, (ox, oy), "clicking must not pan the map");
                assert_eq!(
                    ui.cursor,
                    (ox + dx * zoom + zoom / 2, oy + dy * zoom + zoom / 2),
                    "zoom {} at +{},+{}",
                    zoom,
                    dx,
                    dy
                );
            }
        }
    }

    /// However small the terminal, the status line has to say how to get out.
    #[test]
    fn every_frame_says_how_to_leave() {
        for mode in MODES {
            for (cols, rows) in [(160, 45), (80, 24), (60, 20), (40, 10), (20, 5)] {
                let ui = frame(false, mode, cols, rows);
                let last: String = (0..ui.screen.w)
                    .map(|x| ui.screen.cell(x, ui.screen.h - 1).ch)
                    .collect();
                assert!(
                    last.contains(":q") || last.contains("returns") || last.contains("back"),
                    "{:?} at {}x{} offers no way out: {:?}",
                    mode,
                    cols,
                    rows,
                    last.trim_end()
                );
            }
        }
    }

    #[test]
    fn navigation_survives_narrow_windows_and_feedback() {
        for (cols, rows) in [(160, 45), (120, 30), (80, 24), (60, 20), (40, 20)] {
            let mut ui = frame(false, Mode::Map, cols, rows);
            ui.say("Saved the world successfully");
            ui.compose();
            let out = text(&ui);
            for hint in [
                "Enter inspect",
                "e browse",
                "r recap",
                "c history",
                "t story",
                "x intervene",
            ] {
                assert!(out.contains(hint), "{}x{} lost {}", cols, rows, hint);
            }
            let top = ui.content_height();
            assert!(ui.log_rows.iter().all(|(y, _)| *y < top));
        }
        for mode in [Mode::List, Mode::Detail, Mode::Chronicle, Mode::Recap] {
            let ui = frame(false, mode, 80, 24);
            let out = text(&ui);
            assert!(out.contains("Esc back"));
            assert!(out.contains("Arrows"));
        }
    }

    #[test]
    fn prompts_remain_visible_on_short_terminals() {
        for prompt in [Prompt::Command, Prompt::Search] {
            let mut ui = frame(false, Mode::Map, 40, 10);
            ui.prompt = prompt;
            ui.prompt_text = "test".into();
            ui.say("An earlier notification");
            ui.compose();
            let out = text(&ui);
            assert!(out.contains("test"));
            assert!(out.contains("Esc cancel"));
        }
    }

    #[test]
    fn every_category_stays_visible_and_clickable() {
        use crate::term::{Mouse, MouseKind};
        for cols in [20, 40, 60, 80, 120] {
            let mut ui = frame(false, Mode::List, cols, 24);
            for tab in 0..detail::LIST_TABS.len() {
                ui.list_tab = tab;
                ui.list_filter = "a".into();
                ui.compose();
                let title: String = (0..cols).map(|x| ui.screen.cell(x, 0).ch).collect();
                assert!(
                    title.contains(detail::LIST_TABS[tab]),
                    "{}: {}",
                    cols,
                    title
                );
                let tabs = ui.list_tab_layout();
                let (target, x) = tabs[0];
                ui.handle_key(Key::Mouse(Mouse {
                    kind: MouseKind::Press(0),
                    x,
                    y: 0,
                }));
                assert_eq!(ui.list_tab, target);
                assert!(ui.list_filter.is_empty());
            }
        }
    }

    /// Small terminals must render at all — this is the size sweep the pty
    /// harness cannot reach, since it cannot make a window twenty columns wide.
    #[test]
    fn odd_terminal_sizes_render() {
        for mode in MODES {
            for (cols, rows) in [(20, 5), (24, 6), (31, 9), (40, 10), (79, 23), (200, 60)] {
                let ui = frame(false, mode, cols, rows);
                assert_eq!(ui.screen.w, cols);
                assert_eq!(ui.screen.h, rows);
            }
        }
    }

    /// The chronicle is drawn from the bottom up, and the row it stops at
    /// must be the start of an entry: a panel opening on the tail of a
    /// wrapped sentence reads as a fragment with no year and no subject.
    #[test]
    fn the_chronicle_never_opens_mid_sentence() {
        for (cols, rows) in [(160, 45), (80, 24), (60, 20), (40, 12)] {
            for mode in [Mode::Map, Mode::Chronicle] {
                let ui = frame(false, mode, cols, rows);
                let rows_seen = if mode == Mode::Map {
                    &ui.log_rows
                } else {
                    &ui.chron_rows
                };
                let Some(&(top, _)) = rows_seen.last() else {
                    continue;
                };
                let line: String = (0..ui.screen.w)
                    .map(|x| ui.screen.cell(x, top).ch)
                    .collect();
                assert!(
                    line.trim_start().starts_with(|c: char| c.is_ascii_digit()),
                    "{:?} at {}x{} opens on a continuation line: {:?}",
                    mode,
                    cols,
                    rows,
                    line.trim_end()
                );
            }
        }
    }
}
