//! Terminal user interface: a scrollable world map with overlays, a
//! sidebar, a live event log, browsable lists, detail pages and the full
//! chronicle. Keys follow vim conventions: counts, gg/G, zz, ZZ, `:`
//! commands and `/` search, with mouse and modified arrows as well.

mod detail;

use crate::config::Config;
use crate::geo::Biome;
use crate::sim::chronicle::{EventKind, Ref};
use crate::sim::{Detail, World};
use crate::term::{self, Input, Key, Mouse, MouseKind, Rgb, Screen, BOLD, DIM, REVERSE};
use crate::theme::Theme;
use std::time::{Duration, Instant};

pub const SPEEDS: [f64; 8] = [0.5, 1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Map,
    List,
    Detail,
    Chronicle,
    Help,
    Fate,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layer {
    Political,
    Terrain,
    Culture,
    Magic,
    Population,
    Biomes,
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
        }
    }
    pub fn all() -> [Layer; 6] {
        [
            Layer::Political,
            Layer::Terrain,
            Layer::Culture,
            Layer::Magic,
            Layer::Population,
            Layer::Biomes,
        ]
    }
    pub fn next(self) -> Layer {
        let all = Layer::all();
        let i = all.iter().position(|&l| l == self).unwrap_or(0);
        all[(i + 1) % all.len()]
    }
    pub fn prev(self) -> Layer {
        let all = Layer::all();
        let i = all.iter().position(|&l| l == self).unwrap_or(0);
        all[(i + all.len() - 1) % all.len()]
    }
    pub fn from_name(s: &str) -> Option<Layer> {
        let s = s.to_lowercase();
        Layer::all().into_iter().find(|l| {
            l.name().starts_with(&s)
                || (s == "magic" && *l == Layer::Magic)
                || (s == "pop" && *l == Layer::Population)
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
    last_follow_event: usize,
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
}

pub fn run(world: World, ascii: bool, mouse: bool, save_path: Option<String>, cfg: Config) {
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
    ui.apply_config(&cfg);
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
            cs.sort_by(|&a, &b| {
                ui.world.cities[b]
                    .pop
                    .partial_cmp(&ui.world.cities[a].pop)
                    .unwrap()
            });
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
                .max_by(|a, b| {
                    a.total_influence()
                        .partial_cmp(&b.total_influence())
                        .unwrap()
                })
            {
                ui.selected = Some(Ref::School(s.id));
                ui.mode = Mode::Detail;
            }
        }
        "list" => ui.mode = Mode::List,
        "chronicle" => {
            ui.mode = Mode::Chronicle;
            ui.chron_min = 2;
        }
        "help" => ui.mode = Mode::Help,
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
            speed_idx: 3,
            paused: false,
            acc: 0.0,
            last: Instant::now(),
            selected: None,
            list_tab: 0,
            list_idx: 0,
            list_scroll: 0,
            detail_scroll: 0,
            chron_scroll: 0,
            chron_min: 1,
            log_min: 1,
            ascii,
            msg: String::new(),
            msg_until: Instant::now(),
            follow: true,
            back: Vec::new(),
            fps_last: Instant::now(),
            frames: 0,
            fps: 0,
            last_follow_event: 0,
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
                .min_by(|a, b| (a.1 - v).abs().partial_cmp(&(b.1 - v).abs()).unwrap())
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
            self.log_min = l.min(3);
        }
        if let Some(f) = cfg.follow {
            self.follow = f;
        }
        if let Some(z) = cfg.zoom {
            self.zoom = z.clamp(1, 4);
        }
        for (a, b) in &cfg.maps {
            self.keymap.retain(|(x, _)| x != a);
            self.keymap.push((*a, *b));
        }
    }

    fn remap(&self, k: Key) -> Key {
        self.keymap
            .iter()
            .find(|(a, _)| *a == k)
            .map(|(_, b)| *b)
            .unwrap_or(k)
    }

    /// The storyteller: what is worth watching right now.
    fn stories(&self) -> Vec<(String, Ref)> {
        let w = &self.world;
        let mut out: Vec<(f32, String, Ref)> = Vec::new();
        for x in w.wars.iter().filter(|x| x.alive()) {
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
                    "{}: {} v {}, {} years, {}",
                    x.name,
                    w.polities[a].short,
                    w.polities[d].short,
                    w.year - x.started,
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
                    format!(
                        "{} teeters: stability {:.0}%, {} at war",
                        pol.name,
                        pol.stability * 100.0,
                        if pol.at_war() { "and" } else { "not" }
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
        out.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        out.truncate(3);
        out.into_iter().map(|(_, t, r)| (t, r)).collect()
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
        let mh = sh.saturating_sub(log_h + 1);
        (0, 0, mw.max(1), mh.max(1))
    }

    /// Visible world size in cells at the current zoom.
    fn view_dims(&self) -> (usize, usize) {
        let (_, _, mw, mh) = self.map_rect();
        (mw * self.zoom, mh * self.zoom)
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
    fn follow_events(&mut self) {
        if !self.follow || self.mode != Mode::Map {
            return;
        }
        let n = self.world.chronicle.len();
        let mut target = None;
        let mut i = n;
        while i > self.last_follow_event && i > 0 {
            i -= 1;
            let e = &self.world.chronicle.events[i];
            if e.importance >= 2 {
                if let Some(l) = e.loc {
                    target = Some(l);
                    break;
                }
            }
        }
        self.last_follow_event = n;
        if let Some(l) = target {
            self.goto_cell(l);
        }
    }

    // -----------------------------------------------------------------------
    // Input
    // -----------------------------------------------------------------------

    fn take_count(&mut self) -> usize {
        self.count.take().unwrap_or(1).max(1)
    }

    fn handle_key(&mut self, k: Key) -> bool {
        let k = if self.prompt == Prompt::None {
            self.remap(k)
        } else {
            k
        };
        if let Key::Mouse(m) = k {
            self.handle_mouse(m);
            return true;
        }
        if k == Key::Ctrl('c') {
            return false;
        }
        if self.prompt != Prompt::None {
            return self.key_prompt(k);
        }
        if self.mode == Mode::Help {
            self.mode = self.prev_mode;
            return true;
        }
        if self.mode == Mode::Fate {
            self.key_fate(k);
            return true;
        }
        // Second key of a two-key command.
        if let Some(p) = self.pending.take() {
            let n = self.take_count();
            match (p, k) {
                ('Z', Key::Char('Z')) | ('Z', Key::Char('Q')) => return false,
                ('g', Key::Char('g')) => self.go_top(n),
                ('z', Key::Char('z')) | ('z', Key::Char('.')) => self.center_view(),
                ('z', Key::Char('t')) => {
                    self.view.1 = self.cursor.1;
                    self.clamp_view();
                }
                ('z', Key::Char('i')) | ('z', Key::Char('+')) => {
                    self.set_zoom(self.zoom.saturating_sub(n.max(1)))
                }
                ('z', Key::Char('o')) | ('z', Key::Char('-')) => {
                    self.set_zoom(self.zoom + n.max(1))
                }
                _ => {}
            }
            return true;
        }
        // Counts.
        if let Key::Char(c) = k {
            if c.is_ascii_digit() && !(c == '0' && self.count.is_none()) {
                let d = c as usize - '0' as usize;
                self.count = Some((self.count.unwrap_or(0) * 10 + d).min(9999));
                return true;
            }
            if matches!(c, 'g' | 'z' | 'Z') {
                self.pending = Some(c);
                return true;
            }
        }
        // Keys that work everywhere.
        let n = self.count.unwrap_or(1).max(1);
        match k {
            Key::Char(' ') => {
                self.paused = !self.paused;
                self.run_until = None;
                self.count = None;
                return true;
            }
            Key::Char('+') | Key::Char('=') | Key::Char('>') => {
                self.speed_idx = (self.speed_idx + n).min(SPEEDS.len() - 1);
                self.count = None;
                self.say(&format!("speed: {} years/sec", SPEEDS[self.speed_idx]));
                return true;
            }
            Key::Char('-') | Key::Char('_') | Key::Char('<') => {
                self.speed_idx = self.speed_idx.saturating_sub(n);
                self.count = None;
                self.say(&format!("speed: {} years/sec", SPEEDS[self.speed_idx]));
                return true;
            }
            Key::Char('.') => {
                self.count = None;
                for _ in 0..n.min(1000) {
                    self.world.tick();
                }
                self.follow_events();
                return true;
            }
            Key::Char('D') => {
                self.world.detail = self.world.detail.next();
                self.count = None;
                self.say(&format!("simulation detail: {}", self.world.detail.name()));
                return true;
            }
            Key::Char('?') | Key::F(1) => {
                self.prev_mode = self.mode;
                self.mode = Mode::Help;
                self.count = None;
                return true;
            }
            Key::Char(':') => {
                self.prompt = Prompt::Command;
                self.prompt_text.clear();
                self.count = None;
                return true;
            }
            Key::Char('/') => {
                self.prompt = Prompt::Search;
                self.prompt_text.clear();
                self.count = None;
                return true;
            }
            Key::Char('n') | Key::Char('N')
                if !matches!(self.mode, Mode::List | Mode::Chronicle) =>
            {
                self.count = None;
                self.search_step(if k == Key::Char('n') {
                    n as i32
                } else {
                    -(n as i32)
                });
                return true;
            }
            Key::Char(']') | Key::Char('[') if self.mode != Mode::List => {
                self.count = None;
                self.cycle_realm(if k == Key::Char(']') {
                    n as i32
                } else {
                    -(n as i32)
                });
                return true;
            }
            Key::Char('}') | Key::Char('{') if self.mode != Mode::List => {
                self.count = None;
                self.cycle_city(if k == Key::Char('}') {
                    n as i32
                } else {
                    -(n as i32)
                });
                return true;
            }
            Key::Char('G') => {
                self.count = None;
                self.go_bottom();
                return true;
            }
            _ => {}
        }
        match self.mode {
            Mode::Map => {
                if !self.key_map(k) {
                    return false;
                }
            }
            Mode::List => self.key_list(k),
            Mode::Detail => self.key_detail(k),
            Mode::Chronicle => self.key_chronicle(k),
            Mode::Help | Mode::Fate => {}
        }
        self.count = None;
        true
    }

    fn go_top(&mut self, _n: usize) {
        match self.mode {
            Mode::Map => self.jump_to_selected(),
            Mode::List => self.list_idx = 0,
            Mode::Detail => self.detail_scroll = 0,
            Mode::Chronicle => self.chron_scroll = usize::MAX / 2,
            _ => {}
        }
    }

    fn go_bottom(&mut self) {
        match self.mode {
            Mode::Map => self.jump_to_selected(),
            Mode::List => self.list_idx = usize::MAX / 2,
            Mode::Detail => self.detail_scroll = usize::MAX / 2,
            Mode::Chronicle => self.chron_scroll = 0,
            _ => {}
        }
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

    fn move_cursor(&mut self, dx: i32, dy: i32) {
        let tw = self.world.terrain.w as i32;
        let th = self.world.terrain.h as i32;
        let x = (self.cursor.0 as i32 + dx).clamp(0, tw - 1);
        let y = (self.cursor.1 as i32 + dy).clamp(0, th - 1);
        self.cursor = (x as usize, y as usize);
        self.clamp_view();
    }

    /// Returns false to quit.
    fn key_map(&mut self, k: Key) -> bool {
        let n = self.count.unwrap_or(1).max(1) as i32;
        let (_, _, mw, mh) = self.map_rect();
        let tw = self.world.terrain.w;
        let th = self.world.terrain.h;
        if k != Key::Char('q') {
            self.quit_armed = None;
        }
        match k {
            Key::Left | Key::Char('h') => self.move_cursor(-n, 0),
            Key::Right | Key::Char('l') => self.move_cursor(n, 0),
            Key::Up | Key::Char('k') => self.move_cursor(0, -n),
            Key::Down | Key::Char('j') => self.move_cursor(0, n),
            Key::Char('H') | Key::ShiftLeft => self.move_cursor(-8 * n, 0),
            Key::Char('L') | Key::ShiftRight => self.move_cursor(8 * n, 0),
            Key::Char('K') | Key::ShiftUp => self.move_cursor(0, -4 * n),
            Key::Char('J') | Key::ShiftDown => self.move_cursor(0, 4 * n),
            Key::CtrlLeft => self.move_cursor(-20 * n, 0),
            Key::CtrlRight => self.move_cursor(20 * n, 0),
            Key::CtrlUp => self.move_cursor(0, -10 * n),
            Key::CtrlDown => self.move_cursor(0, 10 * n),
            Key::PageUp | Key::Ctrl('u') => self.move_cursor(0, -(mh as i32 / 2) * n),
            Key::PageDown | Key::Ctrl('d') => self.move_cursor(0, (mh as i32 / 2) * n),
            Key::Ctrl('b') => self.move_cursor(0, -(mh as i32) * n),
            Key::Ctrl('f') => self.move_cursor(0, (mh as i32) * n),
            Key::Char('0') => self.move_cursor(-(tw as i32), 0),
            Key::Char('$') => self.move_cursor(tw as i32, 0),
            Key::Home => {
                self.cursor = (tw / 2, th / 2);
                self.center_view();
            }
            Key::Tab => self.layer = self.layer.next(),
            Key::BackTab => self.layer = self.layer.prev(),
            Key::Enter => {
                if let Some(r) = self.entity_at_cursor() {
                    self.open_detail(r);
                } else {
                    self.say("nothing here but the land");
                }
            }
            Key::Char('s') => self.selected = self.entity_at_cursor(),
            Key::Esc => {
                self.selected = None;
                self.search_results.clear();
            }
            Key::Char('e') => {
                self.prev_mode = Mode::Map;
                self.mode = Mode::List;
            }
            Key::Char('c') | Key::Char('C') => {
                self.prev_mode = Mode::Map;
                self.mode = Mode::Chronicle;
                self.chron_scroll = 0;
            }
            Key::Char('f') | Key::Char('F') => {
                self.follow = !self.follow;
                let m = if self.follow {
                    "following major events"
                } else {
                    "cursor is free"
                };
                self.say(m);
            }
            Key::Char('v') => {
                self.log_min = (self.log_min + 1) % 3;
                self.say(&format!("event log shows importance >= {}", self.log_min));
            }
            Key::Char('x') | Key::Char('X') => self.open_fate(),
            Key::Char('t') | Key::Char('T') => {
                let st = self.stories();
                match st.first() {
                    Some((text, r)) => {
                        let (text, r) = (text.clone(), *r);
                        self.goto_ref(r);
                        self.say(&text);
                    }
                    None => self.say("a quiet moment; nothing much is happening"),
                }
            }
            Key::Char('q') => {
                if self
                    .quit_armed
                    .map(|t| t.elapsed() < Duration::from_secs(3))
                    .unwrap_or(false)
                {
                    return false;
                }
                self.quit_armed = Some(Instant::now());
                self.say("press q again to quit (or :q, ZZ)");
            }
            _ => {}
        }
        let _ = mw;
        true
    }

    fn open_fate(&mut self) {
        if self.selected.is_none() {
            self.selected = self.entity_at_cursor();
        }
        if let Some(Ref::Polity(_)) = self.selected {
            self.prev_mode = self.mode;
            self.mode = Mode::Fate;
        } else {
            self.say("select a realm first (Enter or s over its lands)");
        }
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

    fn list_rows(&self) -> Vec<(String, Ref)> {
        let rows = detail::list_rows(&self.world, self.list_tab);
        if self.list_filter.is_empty() {
            return rows;
        }
        let f = self.list_filter.to_lowercase();
        rows.into_iter()
            .filter(|(s, _)| s.to_lowercase().contains(&f))
            .collect()
    }

    fn key_list(&mut self, k: Key) {
        let n = self.count.unwrap_or(1).max(1);
        let rows = self.list_rows();
        let len = rows.len();
        let page = self.screen.h.saturating_sub(4).max(1);
        match k {
            Key::Esc | Key::Char('q') => {
                if !self.list_filter.is_empty() {
                    self.list_filter.clear();
                } else {
                    self.mode = Mode::Map;
                }
            }
            Key::Tab | Key::Right | Key::Char('l') | Key::Char(']') => {
                self.list_tab = (self.list_tab + n) % detail::LIST_TABS.len();
                self.list_idx = 0;
                self.list_scroll = 0;
                self.list_filter.clear();
            }
            Key::BackTab | Key::Left | Key::Char('h') | Key::Char('[') => {
                self.list_tab = (self.list_tab + detail::LIST_TABS.len() * n
                    - n % detail::LIST_TABS.len())
                    % detail::LIST_TABS.len();
                self.list_idx = 0;
                self.list_scroll = 0;
                self.list_filter.clear();
            }
            Key::Up | Key::Char('k') | Key::Char('N') => {
                self.list_idx = self.list_idx.saturating_sub(n)
            }
            Key::Down | Key::Char('j') | Key::Char('n') => self.list_idx += n,
            Key::PageUp | Key::Ctrl('b') | Key::ShiftUp => {
                self.list_idx = self.list_idx.saturating_sub(page * n)
            }
            Key::PageDown | Key::Ctrl('f') | Key::ShiftDown => self.list_idx += page * n,
            Key::Ctrl('u') => self.list_idx = self.list_idx.saturating_sub(page / 2 * n),
            Key::Ctrl('d') => self.list_idx += page / 2 * n,
            Key::Home => self.list_idx = 0,
            Key::End => self.list_idx = usize::MAX / 2,
            Key::Enter => {
                if let Some((_, r)) = rows.get(self.list_idx) {
                    let r = *r;
                    self.open_detail(r);
                }
            }
            Key::Char('m') | Key::Char('s') => {
                if let Some((_, r)) = rows.get(self.list_idx) {
                    self.goto_ref(*r);
                    self.mode = Mode::Map;
                }
            }
            Key::Char('c') => self.mode = Mode::Chronicle,
            Key::Char('x') => {
                if let Some((_, r)) = rows.get(self.list_idx) {
                    self.selected = Some(*r);
                    self.open_fate();
                }
            }
            _ => {}
        }
        self.list_idx = self.list_idx.min(len.saturating_sub(1));
    }

    fn key_detail(&mut self, k: Key) {
        let n = self.count.unwrap_or(1).max(1);
        let r = match self.selected {
            Some(r) => r,
            None => {
                self.mode = Mode::Map;
                return;
            }
        };
        let page = self.screen.h.saturating_sub(2).max(1);
        match k {
            Key::Esc | Key::Char('q') => {
                self.mode = self.prev_mode;
                if self.mode == Mode::Detail {
                    self.mode = Mode::Map;
                }
            }
            Key::Backspace => {
                if let Some(prev) = self.back.pop() {
                    self.selected = Some(prev);
                    self.detail_scroll = 0;
                } else {
                    self.mode = self.prev_mode;
                }
            }
            Key::Up | Key::Char('k') => self.detail_scroll = self.detail_scroll.saturating_sub(n),
            Key::Down | Key::Char('j') => self.detail_scroll += n,
            Key::PageUp | Key::Ctrl('b') | Key::ShiftUp => {
                self.detail_scroll = self.detail_scroll.saturating_sub(page * n)
            }
            Key::PageDown | Key::Ctrl('f') | Key::ShiftDown => self.detail_scroll += page * n,
            Key::Ctrl('u') => self.detail_scroll = self.detail_scroll.saturating_sub(page / 2 * n),
            Key::Ctrl('d') => self.detail_scroll += page / 2 * n,
            Key::Home => self.detail_scroll = 0,
            Key::End => self.detail_scroll = usize::MAX / 2,
            Key::Char('m') => {
                self.goto_ref(r);
                self.mode = Mode::Map;
            }
            Key::Char('e') => self.mode = Mode::List,
            Key::Char('x') => {
                if let Ref::Polity(_) = r {
                    self.prev_mode = Mode::Detail;
                    self.mode = Mode::Fate;
                }
            }
            Key::Enter => {
                self.goto_ref(r);
                self.mode = Mode::Map;
            }
            Key::Char(c) => {
                if let Some(link) = detail::follow_link(&self.world, r, c) {
                    self.open_detail(link);
                }
            }
            _ => {}
        }
    }

    fn key_chronicle(&mut self, k: Key) {
        let n = self.count.unwrap_or(1).max(1);
        let page = self.screen.h.saturating_sub(2).max(1);
        match k {
            Key::Esc | Key::Char('q') => {
                if !self.chron_filter.is_empty() {
                    self.chron_filter.clear();
                } else {
                    self.mode = self.prev_mode;
                }
            }
            Key::Up | Key::Char('k') => self.chron_scroll += n,
            Key::Down | Key::Char('j') => self.chron_scroll = self.chron_scroll.saturating_sub(n),
            Key::PageUp | Key::Ctrl('b') | Key::ShiftUp => self.chron_scroll += page * n,
            Key::PageDown | Key::Ctrl('f') | Key::ShiftDown => {
                self.chron_scroll = self.chron_scroll.saturating_sub(page * n)
            }
            Key::Ctrl('u') => self.chron_scroll += page / 2 * n,
            Key::Ctrl('d') => self.chron_scroll = self.chron_scroll.saturating_sub(page / 2 * n),
            Key::End => self.chron_scroll = 0,
            Key::Home => self.chron_scroll = usize::MAX / 2,
            Key::Char('f') | Key::Char('v') => self.chron_min = (self.chron_min + 1) % 4,
            Key::Char('e') => self.mode = Mode::List,
            _ => {}
        }
    }

    fn key_fate(&mut self, k: Key) {
        let p = match self.selected {
            Some(Ref::Polity(p)) => p,
            _ => {
                self.mode = self.prev_mode;
                return;
            }
        };
        match k {
            Key::Esc | Key::Char('x') | Key::Char('q') => self.mode = self.prev_mode,
            Key::Char(c) if ('1'..='6').contains(&c) => {
                self.choose_fate(p, c as usize - '1' as usize)
            }
            _ => {}
        }
    }

    fn choose_fate(&mut self, p: usize, choice: usize) {
        let msg = detail::hand_of_fate(&mut self.world, p, choice as u8);
        self.say(&msg);
        self.mode = self.prev_mode;
    }

    // -- prompts: `:` commands and `/` search ------------------------------

    fn key_prompt(&mut self, k: Key) -> bool {
        match k {
            Key::Esc => {
                if self.prompt == Prompt::Search {
                    match self.mode {
                        Mode::List => self.list_filter.clear(),
                        Mode::Chronicle => self.chron_filter.clear(),
                        _ => {}
                    }
                }
                self.prompt = Prompt::None;
            }
            Key::Enter => {
                let text = std::mem::take(&mut self.prompt_text);
                let kind = self.prompt;
                self.prompt = Prompt::None;
                match kind {
                    Prompt::Command => return self.run_command(text.trim()),
                    Prompt::Search => self.submit_search(&text),
                    Prompt::None => {}
                }
            }
            Key::Backspace => {
                if self.prompt_text.pop().is_none() {
                    self.prompt = Prompt::None;
                }
                self.live_filter();
            }
            Key::Ctrl('u') => {
                self.prompt_text.clear();
                self.live_filter();
            }
            Key::Char(c) => {
                self.prompt_text.push(c);
                self.live_filter();
            }
            _ => {}
        }
        true
    }

    /// In lists and the chronicle, a search filters rows as you type.
    fn live_filter(&mut self) {
        if self.prompt != Prompt::Search {
            return;
        }
        match self.mode {
            Mode::List => {
                self.list_filter = self.prompt_text.clone();
                self.list_idx = 0;
            }
            Mode::Chronicle => {
                self.chron_filter = self.prompt_text.clone();
                self.chron_scroll = 0;
            }
            _ => {}
        }
    }

    fn submit_search(&mut self, text: &str) {
        match self.mode {
            Mode::List => self.list_filter = text.to_string(),
            Mode::Chronicle => self.chron_filter = text.to_string(),
            _ => {
                if text.trim().is_empty() {
                    return;
                }
                self.search_results = self.search(text);
                self.search_idx = 0;
                if self.search_results.is_empty() {
                    self.say(&format!("no match for \"{}\"", text));
                } else {
                    let r = self.search_results[0];
                    self.goto_ref(r);
                    if self.mode == Mode::Detail {
                        self.open_detail(r);
                    }
                    self.say(&format!(
                        "{} of {} matches: {}",
                        1,
                        self.search_results.len(),
                        detail::entity_name(&self.world, r)
                    ));
                }
            }
        }
    }

    fn search_step(&mut self, delta: i32) {
        if self.search_results.is_empty() {
            self.say("no search yet (press / to search)");
            return;
        }
        let len = self.search_results.len() as i32;
        self.search_idx = ((self.search_idx as i32 + delta).rem_euclid(len)) as usize;
        let r = self.search_results[self.search_idx];
        self.goto_ref(r);
        if self.mode == Mode::Detail {
            self.open_detail(r);
        }
        self.say(&format!(
            "{} of {} matches: {}",
            self.search_idx + 1,
            len,
            detail::entity_name(&self.world, r)
        ));
    }

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
        hits.sort_by(|a, b| b.0.cmp(&a.0));
        hits.truncate(60);
        hits.into_iter().map(|(_, r)| r).collect()
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
                .partial_cmp(&self.world.cities[a].pop)
                .unwrap()
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

    fn default_save_dir() -> std::path::PathBuf {
        if let Ok(x) = std::env::var("XDG_DATA_HOME") {
            if !x.is_empty() {
                return std::path::PathBuf::from(x).join("empires");
            }
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        std::path::PathBuf::from(home).join(".local/share/empires")
    }

    fn resolve_save_path(&self, arg: &str) -> std::path::PathBuf {
        if arg.is_empty() {
            return self.save_path.clone().unwrap_or_else(|| {
                Self::default_save_dir().join(format!("world-{}.rfe", self.world.seed))
            });
        }
        let p = std::path::PathBuf::from(arg);
        if p.components().count() > 1 || arg.ends_with(".rfe") {
            p
        } else {
            Self::default_save_dir().join(format!("{}.rfe", arg))
        }
    }

    /// Save the world to `arg` (or the current save path / default location).
    pub fn save(&mut self, arg: Option<&str>) -> Result<(), String> {
        let path = self.resolve_save_path(arg.unwrap_or(""));
        match self.world.save_to(&path) {
            Ok(n) => {
                self.save_path = Some(path.clone());
                self.last_autosave = self.world.year;
                self.say(&format!("saved {} ({} KB)", path.display(), n / 1024));
                Ok(())
            }
            Err(e) => {
                self.say(&e);
                Err(e)
            }
        }
    }

    fn load(&mut self, arg: &str) {
        if arg.is_empty() && self.save_path.is_none() {
            self.say("usage: :e NAME|PATH  (saves live in ~/.local/share/empires)");
            return;
        }
        let path = self.resolve_save_path(arg);
        match World::load_from(&path) {
            Ok(w) => {
                self.world = w;
                self.save_path = Some(path.clone());
                self.last_autosave = self.world.year;
                self.selected = None;
                self.back.clear();
                self.search_results.clear();
                self.last_follow_event = self.world.chronicle.len();
                self.mode = Mode::Map;
                self.paused = true;
                self.jump_to_selected();
                self.say(&format!(
                    "loaded {} · year {} · paused",
                    path.display(),
                    self.world.year
                ));
            }
            Err(e) => self.say(&e),
        }
    }

    /// Returns false to quit.
    fn run_command(&mut self, line: &str) -> bool {
        let mut parts = line.splitn(2, ' ');
        let cmd = parts.next().unwrap_or("").trim().to_lowercase();
        let arg = parts.next().unwrap_or("").trim().to_string();
        match cmd.as_str() {
            "" => {}
            "q" | "quit" | "exit" => return false,
            "q!" => {
                self.save_path = None;
                return false;
            }
            "wq" | "x" => {
                if self
                    .save(if arg.is_empty() { None } else { Some(&arg) })
                    .is_ok()
                {
                    return false;
                }
            }
            "w" | "write" | "save" => {
                let _ = self.save(if arg.is_empty() { None } else { Some(&arg) });
            }
            "saveas" => {
                if arg.is_empty() {
                    self.say("usage: :saveas NAME|PATH");
                } else {
                    let _ = self.save(Some(&arg));
                }
            }
            "e" | "edit" | "load" | "open" => self.load(&arg),
            "autosave" => match arg.parse::<i32>() {
                Ok(n) => {
                    self.autosave = n.max(0);
                    self.say(if n > 0 { "autosave on" } else { "autosave off" });
                }
                Err(_) => self.say(&format!(
                    "autosave every {} years (0 = off); usage: :autosave N",
                    self.autosave
                )),
            },
            "saves" | "ls" => {
                let dir = Self::default_save_dir();
                let mut names: Vec<String> = std::fs::read_dir(&dir)
                    .map(|rd| {
                        rd.filter_map(|e| e.ok())
                            .map(|e| e.file_name().to_string_lossy().into_owned())
                            .filter(|n| n.ends_with(".rfe"))
                            .collect()
                    })
                    .unwrap_or_default();
                names.sort();
                if names.is_empty() {
                    self.say(&format!("no saves in {}", dir.display()));
                } else {
                    self.say(&format!("{}: {}", dir.display(), names.join("  ")));
                }
            }
            "p" | "pause" => {
                self.paused = !self.paused;
                self.run_until = None;
            }
            "run" | "go" | "resume" if arg.is_empty() => self.paused = false,
            "speed" | "s" => match arg.parse::<f64>() {
                Ok(v) => {
                    let (i, _) = SPEEDS
                        .iter()
                        .enumerate()
                        .min_by(|a, b| (a.1 - v).abs().partial_cmp(&(b.1 - v).abs()).unwrap())
                        .unwrap();
                    self.speed_idx = i;
                    self.paused = false;
                    self.say(&format!("speed: {} years/sec", SPEEDS[i]));
                }
                Err(_) => self.say("usage: :speed N  (0.5 1 2 5 10 25 50 100)"),
            },
            "detail" | "d" => {
                let d = match arg.to_lowercase().as_str() {
                    "low" | "l" | "0" => Some(Detail::Low),
                    "medium" | "med" | "m" | "1" => Some(Detail::Medium),
                    "high" | "h" | "2" => Some(Detail::High),
                    _ => None,
                };
                match d {
                    Some(d) => {
                        self.world.detail = d;
                        self.say(&format!("simulation detail: {}", d.name()));
                    }
                    None => self.say("usage: :detail low|medium|high"),
                }
            }
            "layer" | "l" => match Layer::from_name(&arg) {
                Some(l) => self.layer = l,
                None => self.say("layers: political terrain culture mana population biomes"),
            },
            "find" | "go" | "goto" | "jump" | "f" => {
                let saved = self.mode;
                if matches!(saved, Mode::List | Mode::Chronicle) {
                    self.mode = Mode::Map;
                }
                self.submit_search(&arg);
                if self.search_results.is_empty() {
                    self.mode = saved;
                }
            }
            "new" | "world" => {
                let seed = arg.parse::<u64>().unwrap_or_else(|_| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(1)
                });
                let (w, h, d) = (
                    self.world.terrain.w,
                    self.world.terrain.h,
                    self.world.detail,
                );
                self.world = World::new(seed, w, h, d);
                self.save_path = None;
                self.selected = None;
                self.back.clear();
                self.search_results.clear();
                self.last_follow_event = 0;
                self.mode = Mode::Map;
                let home = self.world.races.first().map(|r| r.home).unwrap_or(0);
                self.goto_cell(home);
                self.say(&format!("a new world, seed {}", seed));
            }
            "seed" => self.say(&format!(
                "seed {} · {}x{} · year {}",
                self.world.seed, self.world.terrain.w, self.world.terrain.h, self.world.year
            )),
            "follow" => {
                self.follow = match arg.to_lowercase().as_str() {
                    "on" | "1" | "yes" => true,
                    "off" | "0" | "no" => false,
                    _ => !self.follow,
                };
                self.say(if self.follow {
                    "following major events"
                } else {
                    "cursor is free"
                });
            }
            "log" => match arg.parse::<u8>() {
                Ok(v) if v <= 3 => self.log_min = v,
                _ => self.say("usage: :log 0-3"),
            },
            "filter" => match arg.parse::<u8>() {
                Ok(v) if v <= 3 => self.chron_min = v,
                _ => self.say("usage: :filter 0-3"),
            },
            "until" | "year" | "to" => match arg.parse::<i32>() {
                Ok(y) if y > self.world.year => {
                    self.run_until = Some(y);
                    self.paused = false;
                    self.say(&format!("running until year {}", y));
                }
                _ => self.say("usage: :until YEAR (in the future)"),
            },
            "step" => {
                let n = arg.parse::<usize>().unwrap_or(1).min(5000);
                for _ in 0..n {
                    self.world.tick();
                }
                self.follow_events();
                self.say(&format!("advanced {} years", n));
            }
            "fate" => match (self.selected, arg.parse::<usize>()) {
                (Some(Ref::Polity(p)), Ok(c)) if (1..=6).contains(&c) => self.choose_fate(p, c - 1),
                (Some(Ref::Polity(_)), _) => self.open_fate(),
                _ => self.say("select a realm first"),
            },
            "help" | "h" | "?" => {
                self.prev_mode = self.mode;
                self.mode = Mode::Help;
            }
            "mouse" => {
                self.mouse = !self.mouse;
                term::set_mouse(self.mouse);
                self.say(if self.mouse {
                    "mouse captured"
                } else {
                    "mouse released to the terminal"
                });
            }
            "ascii" => {
                self.ascii = !self.ascii;
            }
            "center" | "centre" | "zz" => self.center_view(),
            "list" | "lists" => {
                self.prev_mode = self.mode;
                self.mode = Mode::List;
                if let Some(i) = detail::LIST_TABS.iter().position(|t| {
                    t.to_lowercase().starts_with(&arg.to_lowercase()) && !arg.is_empty()
                }) {
                    self.list_tab = i;
                    self.list_idx = 0;
                }
            }
            "chronicle" | "c" | "history" => {
                self.prev_mode = self.mode;
                self.mode = Mode::Chronicle;
                self.chron_filter = arg;
                self.chron_scroll = 0;
            }
            "map" if arg.is_empty() => self.mode = Mode::Map,
            "map" | "noremap" => {
                let mut it = arg.split_whitespace();
                match (
                    it.next().and_then(crate::config::parse_key),
                    it.next().and_then(crate::config::parse_key),
                ) {
                    (Some(a), Some(b)) => {
                        self.keymap.retain(|(x, _)| *x != a);
                        self.keymap.push((a, b));
                        self.say(&format!(
                            "mapped {} to {}",
                            crate::config::key_name(a),
                            crate::config::key_name(b)
                        ));
                    }
                    _ => self.say("usage: :map <from> <to>   e.g. :map w k   :map <S-Up> K"),
                }
            }
            "unmap" => match crate::config::parse_key(&arg) {
                Some(a) => {
                    self.keymap.retain(|(x, _)| *x != a);
                    self.say(&format!("unmapped {}", crate::config::key_name(a)));
                }
                None => self.say("usage: :unmap <key>"),
            },
            "maps" => {
                if self.keymap.is_empty() {
                    self.say("no key maps");
                } else {
                    let list: Vec<String> = self
                        .keymap
                        .iter()
                        .map(|(a, b)| {
                            format!(
                                "{}→{}",
                                crate::config::key_name(*a),
                                crate::config::key_name(*b)
                            )
                        })
                        .collect();
                    self.say(&list.join("  "));
                }
            }
            "set" => {
                let mut it = arg.splitn(2, |c: char| c == '=' || c == ' ');
                let k = it.next().unwrap_or("").trim().to_lowercase();
                let v = it.next().unwrap_or("").trim().to_string();
                if k.is_empty() {
                    self.say(&format!("theme {}  speed {}  detail {}  zoom {}  log {}  autosave {}  mouse {}  ascii {}  follow {}", self.theme.name(), SPEEDS[self.speed_idx], self.world.detail.name(), self.zoom, self.log_min, self.autosave, self.mouse, self.ascii, self.follow));
                } else {
                    let mut c = Config::default();
                    match c.set(&k, &v) {
                        Ok(()) => {
                            self.apply_config(&c);
                            self.say(&format!("{} = {}", k, v));
                        }
                        Err(e) => self.say(&e),
                    }
                }
            }
            "theme" => {
                self.theme = if arg.is_empty() {
                    self.theme.next()
                } else {
                    Theme::from_name(&arg).unwrap_or(self.theme)
                };
                self.say(&format!(
                    "theme: {} (default phosphor amber paper dusk)",
                    self.theme.name()
                ));
            }
            "zoom" => match arg.parse::<usize>() {
                Ok(z) => self.set_zoom(z),
                Err(_) => self.set_zoom(if self.zoom >= 4 { 1 } else { self.zoom + 1 }),
            },
            "mute" | "unmute" => {
                if arg.is_empty() {
                    let names: Vec<&str> = self.muted.iter().map(|k| k.name()).collect();
                    self.say(&if names.is_empty() { "nothing muted; :mute battle|death|founding|person|discovery|magic|politics|disaster|culture|wonder".to_string() } else { format!("muted: {}", names.join(" ")) });
                } else {
                    match EventKind::from_name(&arg) {
                        Some(k) => {
                            if self.muted.contains(&k) {
                                self.muted.retain(|x| *x != k);
                                self.say(&format!("{} events shown again", k.name()));
                            } else {
                                self.muted.push(k);
                                self.say(&format!("{} events muted", k.name()));
                            }
                        }
                        None => self.say("unknown event kind"),
                    }
                }
            }
            "story" | "t" | "now" => {
                let st = self.stories();
                match st.first() {
                    Some((text, r)) => {
                        let (text, r) = (text.clone(), *r);
                        self.goto_ref(r);
                        self.mode = Mode::Map;
                        self.say(&text);
                    }
                    None => self.say("a quiet moment; nothing much is happening"),
                }
            }
            "mkconfig" => match crate::config::write_template() {
                Ok(p) => self.say(&format!("wrote {}", p.display())),
                Err(e) => self.say(&e),
            },
            "config" => self.say(&format!("{}", crate::config::config_path().display())),
            _ => self.say(&format!("unknown command :{}  (try :help)", cmd)),
        }
        true
    }

    // -- mouse ---------------------------------------------------------------

    fn handle_mouse(&mut self, m: Mouse) {
        let double = matches!(self.last_click, Some((x, y, t)) if x == m.x && y == m.y && t.elapsed() < Duration::from_millis(500));
        if let MouseKind::Press(_) = m.kind {
            self.last_click = Some((m.x, m.y, Instant::now()));
        }
        if self.prompt != Prompt::None {
            return;
        }
        match self.mode {
            Mode::Help => {
                if let MouseKind::Press(_) = m.kind {
                    self.mode = self.prev_mode;
                }
            }
            Mode::Fate => {
                if let MouseKind::Press(_) = m.kind {
                    let (fx, fy, fw, fh) = self.fate_rect;
                    if m.x >= fx && m.x < fx + fw && m.y >= fy && m.y < fy + fh {
                        let row = m.y.saturating_sub(fy + 1);
                        if (2..8).contains(&row) {
                            if let Some(Ref::Polity(p)) = self.selected {
                                self.choose_fate(p, row - 2);
                            }
                        }
                    } else {
                        self.mode = self.prev_mode;
                    }
                }
            }
            Mode::Map => {
                let (_, _, mw, mh) = self.map_rect();
                match m.kind {
                    MouseKind::WheelUp => self.move_cursor(0, -3 * self.zoom as i32),
                    MouseKind::WheelDown => self.move_cursor(0, 3 * self.zoom as i32),
                    MouseKind::Press(button) => {
                        if m.x < mw && m.y < mh {
                            let z = self.zoom;
                            let cx = self.view.0 + m.x * z + z / 2;
                            let cy = self.view.1 + m.y * z + z / 2;
                            if cx < self.world.terrain.w && cy < self.world.terrain.h {
                                let same = self.cursor == (cx, cy);
                                self.cursor = (cx, cy);
                                self.clamp_view();
                                if button == 2 || (same && double) {
                                    if let Some(r) = self.entity_at_cursor() {
                                        self.open_detail(r);
                                    }
                                } else {
                                    self.selected = self.entity_at_cursor();
                                }
                            }
                        } else if let Some(&(_, p)) = self
                            .power_rows
                            .iter()
                            .find(|&&(y, _)| y == m.y)
                            .filter(|_| m.x >= mw)
                        {
                            self.goto_ref(Ref::Polity(p));
                            if double || button == 2 {
                                self.open_detail(Ref::Polity(p));
                            }
                        } else if let Some(&(_, r)) = self
                            .story_rows
                            .iter()
                            .find(|&&(y, _)| y == m.y)
                            .filter(|_| m.x >= mw)
                        {
                            self.goto_ref(r);
                            if double || button == 2 {
                                self.open_detail(r);
                            }
                        } else if let Some(&(_, e)) = self.log_rows.iter().find(|&&(y, _)| y == m.y)
                        {
                            self.jump_to_event(e);
                        }
                    }
                    _ => {}
                }
            }
            Mode::List => match m.kind {
                MouseKind::WheelUp => self.list_idx = self.list_idx.saturating_sub(3),
                MouseKind::WheelDown => self.list_idx += 3,
                MouseKind::Press(button) => {
                    if m.y == 0 {
                        // Tab bar: pick the tab whose label spans this column.
                        let mut x = 1;
                        for (i, t) in detail::LIST_TABS.iter().enumerate() {
                            let w = t.chars().count() + 2;
                            if m.x >= x && m.x < x + w {
                                self.list_tab = i;
                                self.list_idx = 0;
                                self.list_filter.clear();
                            }
                            x += w + 1;
                        }
                    } else if m.y >= self.list_y0 {
                        let row = m.y - self.list_y0 + self.list_scroll;
                        let rows = self.list_rows();
                        if row < rows.len() {
                            let r = rows[row].1;
                            if row == self.list_idx && (double || button == 2) {
                                self.open_detail(r);
                            } else if button == 2 {
                                self.open_detail(r);
                            }
                            self.list_idx = row;
                        }
                    }
                }
                _ => {}
            },
            Mode::Detail => match m.kind {
                MouseKind::WheelUp => self.detail_scroll = self.detail_scroll.saturating_sub(3),
                MouseKind::WheelDown => self.detail_scroll += 3,
                MouseKind::Press(2) => {
                    if let Some(prev) = self.back.pop() {
                        self.selected = Some(prev);
                        self.detail_scroll = 0;
                    } else {
                        self.mode = self.prev_mode;
                    }
                }
                _ => {}
            },
            Mode::Chronicle => match m.kind {
                MouseKind::WheelUp => self.chron_scroll += 3,
                MouseKind::WheelDown => self.chron_scroll = self.chron_scroll.saturating_sub(3),
                MouseKind::Press(_) => {
                    if let Some(&(_, e)) = self.chron_rows.iter().find(|&&(y, _)| y == m.y) {
                        self.jump_to_event(e);
                        self.mode = Mode::Map;
                    }
                }
                _ => {}
            },
        }
    }

    fn jump_to_event(&mut self, e: usize) {
        let (loc, first, year) = {
            let ev = &self.world.chronicle.events[e];
            (ev.loc, ev.refs.first().copied(), ev.year)
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

    // -----------------------------------------------------------------------
    // Rendering
    // -----------------------------------------------------------------------

    fn render(&mut self) {
        self.compose();
        self.screen.flush();
    }

    fn compose(&mut self) {
        let bg = Rgb(12, 12, 16);
        self.screen.clear(bg);
        match self.mode {
            Mode::Map | Mode::Fate => {
                self.render_map();
                self.render_sidebar();
                self.render_log();
                if self.mode == Mode::Fate {
                    self.render_fate();
                }
            }
            Mode::List => self.render_list(),
            Mode::Detail => self.render_detail(),
            Mode::Chronicle => self.render_chronicle(),
            Mode::Help => self.render_help(),
        }
        self.render_status();
        if !self.theme.is_identity() {
            let t = self.theme;
            for c in self.screen.cells_mut() {
                c.fg = t.map(c.fg, false);
                c.bg = t.map(c.bg, true);
            }
        }
    }

    fn render_log(&mut self) {
        let (_, _, _, mh) = self.map_rect();
        let sw = self.screen.w;
        let sh = self.screen.h;
        let y0 = mh;
        let h = sh.saturating_sub(mh + 1);
        self.log_rows.clear();
        if h == 0 {
            return;
        }
        let bg = Rgb(10, 10, 14);
        self.screen.fill(0, y0, sw, h, ' ', Rgb(200, 200, 200), bg);
        self.screen.hline(0, y0, sw, Rgb(60, 60, 70), bg);
        let title = format!(" Chronicle (importance ≥{}) ", self.log_min);
        let title = if self.ascii {
            title.replace('≥', ">=")
        } else {
            title
        };
        self.screen
            .text_attr(2, y0, &title, Rgb(230, 200, 120), bg, BOLD);
        let avail = h - 1;
        let width = sw.saturating_sub(10);
        let mut lines: Vec<(String, Rgb, u8, usize)> = Vec::new();
        let w = &self.world;
        for (idx, e) in w.chronicle.events.iter().enumerate().rev() {
            if e.importance < self.log_min || self.muted.contains(&e.kind) {
                continue;
            }
            let (color, attr) = event_style(e.kind, e.importance);
            let wrapped = term::wrap(&e.text, width);
            for (k, l) in wrapped.iter().enumerate().rev() {
                let prefix = if k == 0 {
                    format!("{:>5}  ", e.year)
                } else {
                    "       ".to_string()
                };
                lines.push((format!("{}{}", prefix, l), color, attr, idx));
            }
            if lines.len() >= avail {
                break;
            }
        }
        lines.truncate(avail);
        let mut y = y0 + h - 1;
        for (l, c, a, idx) in lines {
            self.screen.text_attr(1, y, &l, c, bg, a);
            self.log_rows.push((y, idx));
            if y == y0 + 1 {
                break;
            }
            y -= 1;
        }
    }

    fn render_status(&mut self) {
        let sw = self.screen.w;
        let y = self.screen.h - 1;
        let bg = Rgb(40, 40, 52);
        let fg = Rgb(220, 220, 225);
        let key = Rgb(255, 220, 120);
        self.screen.fill(0, y, sw, 1, ' ', fg, bg);
        // A prompt takes over the whole line, as in vim.
        if self.prompt != Prompt::None {
            let lead = if self.prompt == Prompt::Command {
                ":"
            } else {
                "/"
            };
            let x = self.screen.text_attr(
                1,
                y,
                &format!("{}{}", lead, self.prompt_text),
                Rgb(255, 255, 255),
                bg,
                BOLD,
            );
            self.screen.put_attr(x, y, ' ', fg, bg, REVERSE);
            let hint = match (self.prompt, self.mode) {
                (Prompt::Search, Mode::List) | (Prompt::Search, Mode::Chronicle) => "filtering as you type · Enter keep · Esc clear".to_string(),
                (Prompt::Search, _) => {
                    let hits = self.search(&self.prompt_text);
                    match hits.first() {
                        Some(r) => format!("{} matches · Enter jumps to {}", hits.len(), detail::entity_name(&self.world, *r)),
                        None if self.prompt_text.is_empty() => "type a name; Enter jumps to the best match".to_string(),
                        None => "no match".to_string(),
                    }
                }
                (Prompt::Command, _) => "w e speed layer detail find until step new theme set map zoom mute story fate q".to_string(),
                _ => String::new(),
            };
            let hx = sw.saturating_sub(hint.chars().count() + 1);
            if hx > x + 2 {
                self.screen.text(hx, y, &hint, Rgb(150, 150, 165), bg);
            }
            return;
        }
        let state = if self.paused {
            "PAUSED".to_string()
        } else {
            format!("{}y/s", SPEEDS[self.speed_idx])
        };
        let left = format!(
            " {} | {} | detail:{} | seed {} | {:.1}ms/y {}fps",
            state,
            self.layer.name(),
            self.world.detail.name(),
            self.world.seed,
            self.world.ticks_ms,
            self.fps
        );
        let mut x = self.screen.text_attr(0, y, &left, key, bg, BOLD);
        if Instant::now() < self.msg_until {
            x = self
                .screen
                .text(x + 2, y, &self.msg, Rgb(160, 255, 160), bg);
        }
        // showcmd: pending count and prefix, like vim.
        let mut showcmd = String::new();
        if let Some(c) = self.count {
            showcmd.push_str(&c.to_string());
        }
        if let Some(p) = self.pending {
            showcmd.push(p);
        }
        if !self.list_filter.is_empty() && self.mode == Mode::List {
            showcmd = format!("/{}", self.list_filter);
        }
        if !self.chron_filter.is_empty() && self.mode == Mode::Chronicle {
            showcmd = format!("/{}", self.chron_filter);
        }
        if !showcmd.is_empty() {
            x = self
                .screen
                .text_attr(x + 2, y, &showcmd, Rgb(255, 255, 255), bg, BOLD);
        }
        let hints = match self.mode {
            Mode::Map => "Space pause  +/- speed  Tab layer  Enter open  ] [ realms  / search  : cmd  e lists  c chronicle  x fate  ? help",
            Mode::List => "Tab tabs  j/k  Enter open  m map  / filter  x fate  Esc back",
            Mode::Detail => "j/k scroll  letters follow links  m map  Backspace back  ] [ realms  Esc back",
            Mode::Chronicle => "j/k scroll  f importance  / filter  click a line to jump  Esc back",
            Mode::Help => "any key to return",
            Mode::Fate => "1-6 choose  Esc cancel",
        };
        let mut hx = sw.saturating_sub(hints.chars().count() + 1);
        let mut hints = hints;
        if hx <= x + 2 {
            hints = "? help  :q quit";
            hx = sw.saturating_sub(hints.chars().count() + 1);
        }
        if hx > x + 2 {
            self.screen.text(hx, y, hints, Rgb(170, 170, 180), bg);
        }
    }

    fn render_list(&mut self) {
        let sw = self.screen.w;
        let sh = self.screen.h.saturating_sub(1);
        let bg = Rgb(14, 14, 20);
        let fg = Rgb(200, 200, 205);
        let accent = Rgb(230, 200, 120);
        self.screen.fill(0, 0, sw, sh, ' ', fg, bg);
        // Tabs.
        let mut x = 1;
        for (i, t) in detail::LIST_TABS.iter().enumerate() {
            let sel = i == self.list_tab;
            let (f, b, a) = if sel {
                (Rgb(10, 10, 10), accent, BOLD)
            } else {
                (fg, Rgb(30, 30, 40), 0)
            };
            x = self.screen.text_attr(x, 0, &format!(" {} ", t), f, b, a) + 1;
        }
        let rows = self.list_rows();
        let header = detail::list_header(self.list_tab);
        self.screen
            .text_attr(1, 1, header, Rgb(150, 150, 160), bg, BOLD);
        if !self.list_filter.is_empty() {
            let f = format!(" {} rows match \"{}\" ", rows.len(), self.list_filter);
            self.screen.text_attr(
                sw.saturating_sub(f.chars().count() + 1),
                0,
                &f,
                Rgb(255, 255, 255),
                Rgb(60, 60, 80),
                0,
            );
        }
        self.list_y0 = 2;
        let avail = sh.saturating_sub(3);
        self.list_idx = self.list_idx.min(rows.len().saturating_sub(1));
        if self.list_idx < self.list_scroll {
            self.list_scroll = self.list_idx;
        }
        if self.list_idx >= self.list_scroll + avail {
            self.list_scroll = self.list_idx + 1 - avail;
        }
        for (k, (row, r)) in rows.iter().enumerate().skip(self.list_scroll).take(avail) {
            let y = 2 + k - self.list_scroll;
            let sel = k == self.list_idx;
            let color = detail::ref_color(&self.world, *r);
            let (f, b, a) = if sel {
                (Rgb(255, 255, 255), Rgb(50, 50, 70), BOLD)
            } else {
                (fg, bg, 0)
            };
            if sel {
                self.screen.fill(0, y, sw, 1, ' ', f, b);
            }
            self.screen
                .put(1, y, if self.ascii { '#' } else { '■' }, color, b);
            self.screen.text_clip(3, y, row, sw - 4, f, b, a);
        }
        if rows.is_empty() {
            self.screen.text(
                2,
                3,
                if self.list_filter.is_empty() {
                    "nothing yet"
                } else {
                    "no rows match"
                },
                Rgb(120, 120, 130),
                bg,
            );
        }
    }

    fn terrain_style(&self, i: usize) -> (char, Rgb, Rgb) {
        let t = &self.world.terrain;
        let b = t.biome[i];
        let hn = t.height_norm(i);
        let (mut bg, glyph) = biome_style(b, self.ascii);
        match b {
            Biome::DeepOcean | Biome::Ocean | Biome::Shallows => {
                let depth = ((t.sea - t.elev[i]) / t.sea).clamp(0.0, 1.0);
                bg = bg.scale(1.15 - depth * 0.6);
            }
            Biome::Mountain | Biome::Peak | Biome::Hills => {
                bg = bg.mix(Rgb(235, 235, 235), (hn - 0.35).max(0.0) * 0.8);
            }
            _ => {
                bg = bg.scale(0.85 + hn * 0.6);
            }
        }
        let mut fg = bg.scale(1.35);
        let mut ch = glyph;
        if t.river[i] >= 1 && !b.is_water() {
            ch = if self.ascii { '~' } else { '≈' };
            fg = Rgb(70, 130, 220);
            if t.river[i] >= 3 {
                fg = Rgb(90, 160, 250);
            }
        }
        (ch, fg, bg)
    }

    fn render_map(&mut self) {
        let (mx, my, mw, mh) = self.map_rect();
        let (ox, oy) = self.view;
        let tw = self.world.terrain.w;
        let th = self.world.terrain.h;
        let sel_polity = match self.selected {
            Some(Ref::Polity(p)) => Some(p),
            _ => None,
        };
        let sel_culture = match self.selected {
            Some(Ref::Culture(c)) => Some(c),
            _ => None,
        };
        let sel_school = match self.selected {
            Some(Ref::School(s)) => Some(s),
            _ => None,
        };
        // Recent event markers.
        let mut marks: Vec<(usize, u8)> = Vec::new();
        let year = self.world.year;
        for e in self.world.chronicle.events.iter().rev() {
            if e.year < year - 1 {
                break;
            }
            if e.importance >= 2 {
                if let Some(l) = e.loc {
                    marks.push((l, e.importance));
                }
            }
        }
        let max_pop = 8.0f32;
        let z = self.zoom;
        for sy in 0..mh {
            let y0 = oy + sy * z;
            if y0 >= th {
                break;
            }
            for sx in 0..mw {
                let x0 = ox + sx * z;
                if x0 >= tw {
                    break;
                }
                // Representative cell of the z×z block: a city if there is one, else the centre.
                let mut i = (y0 + (z / 2).min(th - 1 - y0)) * tw + (x0 + (z / 2).min(tw - 1 - x0));
                let mut has_cursor = false;
                let mut mark: Option<u8> = None;
                if z > 1 {
                    let mut best_city = false;
                    for yy in y0..(y0 + z).min(th) {
                        for xx in x0..(x0 + z).min(tw) {
                            let j = yy * tw + xx;
                            if (xx, yy) == self.cursor {
                                has_cursor = true;
                            }
                            if let Some(c) = self.world.cells[j].city {
                                if self.world.cities[c].destroyed.is_none() && !best_city {
                                    i = j;
                                    best_city = true;
                                }
                            }
                            for &(l, imp) in &marks {
                                if l == j {
                                    mark = Some(mark.unwrap_or(0).max(imp));
                                }
                            }
                        }
                    }
                } else {
                    has_cursor = (x0, y0) == self.cursor;
                    for &(l, imp) in &marks {
                        if l == i {
                            mark = Some(mark.unwrap_or(0).max(imp));
                        }
                    }
                }
                let (mut ch, mut fg, mut bg) = self.terrain_style(i);
                let cs = &self.world.cells[i];
                let water = self.world.terrain.biome[i].is_water();
                let mut attr = 0u8;
                match self.layer {
                    Layer::Political => {
                        if let Some(p) = cs.owner {
                            let pc = self.world.polities[p].color;
                            let border = self
                                .world
                                .terrain
                                .neighbors4(i)
                                .any(|nb| self.world.cells[nb].owner != Some(p));
                            let mut k = if border { 0.8 } else { 0.42 };
                            if let Some(sp) = sel_polity {
                                if sp == p {
                                    k += 0.15;
                                } else {
                                    k *= 0.5;
                                }
                            }
                            bg = bg.mix(pc, k);
                            fg = bg.scale(1.3);
                            if self.world.terrain.river[i] >= 1 {
                                fg = Rgb(120, 170, 240);
                            }
                        }
                    }
                    Layer::Culture => {
                        if let Some(c) = cs.culture {
                            let cc = self.world.cultures[c].color;
                            let mut k = (0.25 + cs.pop / max_pop * 0.5).min(0.75);
                            if let Some(sc) = sel_culture {
                                if sc == c {
                                    k = 0.85;
                                } else {
                                    k *= 0.4;
                                }
                            }
                            bg = bg.mix(cc, k);
                            fg = bg.scale(1.3);
                        }
                    }
                    Layer::Magic => {
                        let m = self.world.terrain.mana[i];
                        let purple = Rgb(170, 60, 230);
                        if !water {
                            bg = bg.mix(purple, (m * m) * 0.9);
                        }
                        if let Some(p) = cs.owner {
                            if let Some(s) = self.world.polities[p].school {
                                let sc = self.world.schools[s].color;
                                let infl = self.world.schools[s]
                                    .influence
                                    .get(&p)
                                    .copied()
                                    .unwrap_or(0.0);
                                let mut k = 0.15 + infl * 0.35;
                                if sel_school == Some(s) {
                                    k += 0.3;
                                }
                                bg = bg.mix(sc, k);
                            }
                        }
                        fg = bg.scale(1.3);
                    }
                    Layer::Population => {
                        if !water {
                            let p = (cs.pop / max_pop).clamp(0.0, 1.0);
                            let heat = Rgb(40, 40, 50)
                                .mix(Rgb(255, 210, 60), p.sqrt())
                                .mix(Rgb(255, 60, 40), (p - 0.6).max(0.0) * 2.0);
                            bg = bg.mix(heat, 0.85);
                            fg = bg.scale(1.3);
                        }
                    }
                    Layer::Biomes => {
                        let (flat, g) = biome_style(self.world.terrain.biome[i], self.ascii);
                        bg = flat;
                        fg = flat.scale(1.4);
                        ch = g;
                    }
                    Layer::Terrain => {}
                }
                if cs.plague > 0 && self.layer != Layer::Biomes {
                    bg = bg.mix(Rgb(120, 200, 60), 0.35);
                }
                // Cities.
                if let Some(c) = cs.city {
                    let city = &self.world.cities[c];
                    if city.destroyed.is_none() {
                        let is_cap = city
                            .polity
                            .map(|p| self.world.polities[p].capital == Some(c))
                            .unwrap_or(false);
                        ch = if is_cap { '@' } else { '#' };
                        fg = if bg.luma() > 0.5 {
                            Rgb(10, 10, 10)
                        } else {
                            Rgb(255, 255, 255)
                        };
                        attr = BOLD;
                        if self.layer == Layer::Magic {
                            let is_home = self
                                .world
                                .schools
                                .iter()
                                .any(|s| s.alive() && s.home_city == c);
                            if is_home {
                                ch = '*';
                                fg = Rgb(255, 240, 120);
                            }
                        }
                    } else {
                        ch = if self.ascii { 'x' } else { '×' };
                        fg = Rgb(120, 110, 100);
                    }
                }
                if self.world.terrain.biome[i] == Biome::Wastes {
                    fg = Rgb(200, 120, 220);
                }
                // Event markers.
                if let Some(imp) = mark {
                    ch = '!';
                    fg = if imp >= 3 {
                        Rgb(255, 80, 80)
                    } else {
                        Rgb(255, 220, 80)
                    };
                    attr = BOLD;
                }
                if has_cursor {
                    attr |= REVERSE;
                }
                self.screen.put_attr(mx + sx, my + sy, ch, fg, bg, attr);
            }
        }
        // Scroll hints.
        let hint = Rgb(180, 180, 180);
        let dark = Rgb(20, 20, 24);
        if ox > 0 {
            for sy in (0..mh).step_by(4) {
                self.screen.put(mx, my + sy, '<', hint, dark);
            }
        }
        if ox + mw * z < tw {
            for sy in (0..mh).step_by(4) {
                self.screen.put(mx + mw - 1, my + sy, '>', hint, dark);
            }
        }
        if oy > 0 {
            for sx in (0..mw).step_by(8) {
                self.screen.put(mx + sx, my, '^', hint, dark);
            }
        }
        if oy + mh * z < th {
            for sx in (0..mw).step_by(8) {
                self.screen.put(mx + sx, my + mh - 1, 'v', hint, dark);
            }
        }
    }

    fn render_sidebar(&mut self) {
        let (_, _, mw, mh) = self.map_rect();
        let sw = self.screen.w;
        if sw <= mw {
            return;
        }
        let x0 = mw;
        let width = sw - mw;
        let bg = Rgb(18, 18, 24);
        let fg = Rgb(200, 200, 205);
        let dim = Rgb(120, 120, 130);
        let accent = Rgb(230, 200, 120);
        self.screen.fill(x0, 0, width, mh, ' ', fg, bg);
        self.screen.vline(x0, 0, mh, Rgb(60, 60, 70), bg);
        let x = x0 + 2;
        let tx = width - 3;
        let mut y = 0;
        let w = &self.world;
        let (era, era_desc) = w
            .eras
            .last()
            .map(|e| (e.name.clone(), e.description.clone()))
            .unwrap_or_default();
        self.screen
            .text_attr(x, y, &format!("Year {}", w.year), accent, bg, BOLD);
        y += 1;
        self.screen.text_clip(x, y, &era, tx, dim, bg, 0);
        y += 1;
        for l in term::wrap(&era_desc, tx).into_iter().take(2) {
            self.screen.text_clip(x, y, &l, tx, Rgb(95, 95, 105), bg, 0);
            y += 1;
        }
        y += 1;
        let st = &w.stats;
        let lines = [
            format!("people   {:>7.0}k", st.pop),
            format!("realms   {:>7}", st.polities_alive),
            format!("cities   {:>7}", st.cities_alive),
            format!("peoples  {:>7}", st.cultures_alive),
            format!("schools  {:>7}", st.schools_alive),
            format!("wars     {:>7}", st.wars_active),
        ];
        for l in lines.iter() {
            self.screen.text(x, y, l, fg, bg);
            y += 1;
        }
        y += 1;
        // The storyteller.
        self.story_rows.clear();
        let stories = self.stories();
        if !stories.is_empty() && y + 4 < mh {
            self.screen.hline(x0 + 1, y, width - 1, Rgb(60, 60, 70), bg);
            y += 1;
            self.screen.text_attr(x, y, "Now", accent, bg, BOLD);
            y += 1;
            for (text, r) in stories {
                let color = detail::ref_color(&self.world, r);
                let lines = term::wrap(&text, tx.saturating_sub(2));
                for (k, l) in lines.into_iter().take(2).enumerate() {
                    if y >= mh.saturating_sub(6) {
                        break;
                    }
                    if k == 0 {
                        self.screen
                            .put(x, y, if self.ascii { '*' } else { '•' }, color, bg);
                    }
                    self.screen.text_clip(x + 2, y, &l, tx - 2, fg, bg, 0);
                    self.story_rows.push((y, r));
                    y += 1;
                }
            }
            y += 1;
        }
        // Under cursor.
        let i = w.terrain.idx(self.cursor.0, self.cursor.1);
        self.screen.hline(x0 + 1, y, width - 1, Rgb(60, 60, 70), bg);
        y += 1;
        self.screen.text_attr(x, y, "Here", accent, bg, BOLD);
        y += 1;
        let cs = &w.cells[i];
        let mut here: Vec<(String, Rgb)> = Vec::new();
        here.push((w.terrain.describe_cell(i), fg));
        if let Some(f) = w.terrain.river_at(i) {
            here.push((w.terrain.features[f].display(), Rgb(120, 170, 240)));
        }
        if let Some(f) = w.terrain.feature_at(i) {
            here.push((w.terrain.features[f].display(), dim));
        }
        if let Some(c) = cs.city {
            let city = &w.cities[c];
            if city.destroyed.is_none() {
                here.push((
                    format!("{} ({:.0}k)", city.name, city.pop),
                    Rgb(255, 255, 255),
                ));
            } else {
                here.push((format!("ruins of {}", city.name), dim));
            }
        }
        if let Some(p) = cs.owner {
            let pol = &w.polities[p];
            here.push((pol.name.clone(), pol.color));
            here.push((w.ruler_short(p), fg));
        }
        if let Some(c) = cs.culture {
            here.push((
                format!("{} folk, {:.1}k", w.cultures[c].adj, cs.pop),
                w.cultures[c].color,
            ));
        }
        here.push((
            format!(
                "mana {:.0}%  fertility {:.0}%",
                w.terrain.mana[i] * 100.0,
                w.terrain.fertility[i] * 100.0
            ),
            dim,
        ));
        for (s, c) in here {
            for l in term::wrap(&s, tx) {
                if y >= mh.saturating_sub(2) {
                    break;
                }
                self.screen.text_clip(x, y, &l, tx, c, bg, 0);
                y += 1;
            }
        }
        y += 1;
        // Selected.
        if let Some(r) = self.selected {
            self.screen.hline(x0 + 1, y, width - 1, Rgb(60, 60, 70), bg);
            y += 1;
            self.screen.text_attr(x, y, "Selected", accent, bg, BOLD);
            y += 1;
            for (s, c) in detail::summary(w, r) {
                for l in term::wrap(&s, tx) {
                    if y >= mh.saturating_sub(1) {
                        break;
                    }
                    self.screen.text_clip(x, y, &l, tx, c, bg, 0);
                    y += 1;
                }
            }
            y += 1;
        }
        // Great powers.
        if y + 3 < mh {
            self.screen.hline(x0 + 1, y, width - 1, Rgb(60, 60, 70), bg);
            y += 1;
            self.screen
                .text_attr(x, y, "Great powers", accent, bg, BOLD);
            y += 1;
            let mut ps: Vec<usize> = w.living_polities();
            ps.sort_by_key(|&p| std::cmp::Reverse(w.polities[p].cells));
            self.power_rows.clear();
            for &p in ps.iter().take(mh.saturating_sub(y + 1)) {
                let pol = &w.polities[p];
                self.power_rows.push((y, p));
                self.screen
                    .put(x, y, if self.ascii { '#' } else { '■' }, pol.color, bg);
                let war = if pol.at_war() { "!" } else { " " };
                let namew = tx.saturating_sub(9);
                let name: String = if pol.name.chars().count() > namew {
                    pol.name
                        .chars()
                        .take(namew.saturating_sub(1))
                        .chain(std::iter::once('…'))
                        .collect()
                } else {
                    pol.name.clone()
                };
                let s = format!("{} {:<w$} {:>4}", war, name, pol.cells, w = namew);
                self.screen.text_clip(x + 2, y, &s, tx - 2, fg, bg, 0);
                y += 1;
            }
        }
    }

    fn render_detail(&mut self) {
        let r = match self.selected {
            Some(r) => r,
            None => return,
        };
        let sw = self.screen.w;
        let sh = self.screen.h.saturating_sub(1);
        let bg = Rgb(14, 14, 20);
        let fg = Rgb(200, 200, 205);
        self.screen.fill(0, 0, sw, sh, ' ', fg, bg);
        let width = sw.saturating_sub(4).max(20);
        let lines = detail::detail_lines(&self.world, r, width, self.ascii);
        let max_scroll = lines.len().saturating_sub(sh.saturating_sub(1));
        if self.detail_scroll > max_scroll {
            self.detail_scroll = max_scroll;
        }
        for (k, line) in lines.iter().skip(self.detail_scroll).take(sh).enumerate() {
            self.screen
                .text_clip(2, k, &line.text, width, line.fg, bg, line.attr);
        }
        if lines.len() > sh {
            let s = format!(
                " {}/{} ",
                self.detail_scroll + sh.min(lines.len()),
                lines.len()
            );
            self.screen.text(
                sw.saturating_sub(s.len() + 1),
                0,
                &s,
                Rgb(120, 120, 130),
                bg,
            );
        }
    }

    fn render_chronicle(&mut self) {
        let sw = self.screen.w;
        let sh = self.screen.h.saturating_sub(1);
        let bg = Rgb(12, 12, 16);
        let fg = Rgb(200, 200, 205);
        self.screen.fill(0, 0, sw, sh, ' ', fg, bg);
        self.chron_rows.clear();
        let filter = self.chron_filter.to_lowercase();
        let title = if filter.is_empty() {
            format!(
                " The Chronicle of the World — importance ≥{} ({} entries) ",
                self.chron_min,
                self.world.chronicle.len()
            )
        } else {
            format!(
                " The Chronicle of the World — importance ≥{} — matching \"{}\" ",
                self.chron_min, self.chron_filter
            )
        };
        let title = if self.ascii {
            title.replace('≥', ">=").replace('—', "-")
        } else {
            title
        };
        self.screen
            .text_attr(1, 0, &title, Rgb(230, 200, 120), bg, BOLD);
        let width = sw.saturating_sub(10);
        let avail = sh.saturating_sub(1);
        // Build lines from newest, skipping chron_scroll lines from the bottom.
        let mut lines: Vec<(String, Rgb, u8, usize)> = Vec::new();
        let need = avail.saturating_add(self.chron_scroll.min(1_000_000));
        let w = &self.world;
        for (idx, e) in w.chronicle.events.iter().enumerate().rev() {
            if e.importance < self.chron_min || self.muted.contains(&e.kind) {
                continue;
            }
            if !filter.is_empty() && !e.text.to_lowercase().contains(&filter) {
                continue;
            }
            let (color, attr) = event_style(e.kind, e.importance);
            let wrapped = term::wrap(&e.text, width);
            for (k, l) in wrapped.iter().enumerate().rev() {
                let prefix = if k == 0 {
                    format!("{:>5}  ", e.year)
                } else {
                    "       ".to_string()
                };
                lines.push((format!("{}{}", prefix, l), color, attr, idx));
            }
            if lines.len() >= need {
                break;
            }
        }
        if self.chron_scroll > lines.len().saturating_sub(avail) {
            self.chron_scroll = lines.len().saturating_sub(avail);
        }
        if lines.is_empty() {
            self.screen
                .text(2, 2, "nothing matches", Rgb(120, 120, 130), bg);
            return;
        }
        let mut y = sh - 1;
        for (l, c, a, idx) in lines.iter().skip(self.chron_scroll).take(avail) {
            self.screen.text_attr(1, y, l, *c, bg, *a);
            self.chron_rows.push((y, *idx));
            if y == 1 {
                break;
            }
            y -= 1;
        }
    }

    fn render_help(&mut self) {
        let sw = self.screen.w;
        let sh = self.screen.h.saturating_sub(1);
        let bg = Rgb(14, 14, 20);
        let fg = Rgb(200, 200, 205);
        self.screen.fill(0, 0, sw, sh, ' ', fg, bg);
        for (k, l) in detail::HELP.iter().enumerate().take(sh) {
            let (c, a) = if l.starts_with("  ") || l.is_empty() {
                (fg, 0)
            } else {
                (Rgb(230, 200, 120), BOLD)
            };
            self.screen.text_clip(2, k, l, sw - 3, c, bg, a);
        }
    }

    fn render_fate(&mut self) {
        let p = match self.selected {
            Some(Ref::Polity(p)) => p,
            _ => return,
        };
        let lines = detail::fate_menu(&self.world, p);
        let w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(20) + 4;
        let h = lines.len() + 2;
        let (_, _, mw, mh) = self.map_rect();
        let x = mw.saturating_sub(w) / 2;
        let y = mh.saturating_sub(h) / 2;
        self.fate_rect = (x, y, w, h);
        let bg = Rgb(30, 24, 40);
        let fg = Rgb(230, 225, 235);
        self.screen.fill(x, y, w, h, ' ', fg, bg);
        self.screen
            .frame(x, y, w, h, "The Hand of Fate", Rgb(230, 200, 120), bg);
        for (k, l) in lines.iter().enumerate() {
            let attr = if k == 0 { BOLD } else { 0 };
            self.screen.text_attr(x + 2, y + 1 + k, l, fg, bg, attr);
        }
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
