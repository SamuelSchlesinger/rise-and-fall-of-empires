//! Terminal user interface: a scrollable world map with overlays, a
//! sidebar, a live event log, browsable lists, detail pages and the full
//! chronicle. Keys follow vim conventions: counts, gg/G, zz, ZZ, `:`
//! commands and `/` search, with mouse and modified arrows as well.

mod commands;
mod detail;
mod input;
mod render;

use crate::config::Config;
use crate::geo::Biome;
use crate::sim::chronicle::{EventKind, Ref};
use crate::sim::World;
use crate::term::{self, Input, Key, Rgb, Screen, BOLD, DIM, REVERSE};
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

    // -- the view: which part of the world is on screen -----------------------

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
        hits.sort_by(|a, b| b.0.cmp(&a.0));
        hits.truncate(60);
        hits.into_iter().map(|(_, r)| r).collect()
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

    fn choose_fate(&mut self, p: usize, choice: usize) {
        let msg = detail::hand_of_fate(&mut self.world, p, choice as u8);
        self.say(&msg);
        self.mode = self.prev_mode;
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
