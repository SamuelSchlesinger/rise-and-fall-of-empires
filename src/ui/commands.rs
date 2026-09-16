//! The `:` command line. Each group of commands is one function; a command
//! that a group does not know falls through to the next one, so adding a
//! command means touching a single `match`.

use crate::sim::Detail;
use crate::ui::*;

/// What a command group did with a command line.
enum Cmd {
    /// Handled; carry on.
    Done,
    /// Not this group's command; try the next one.
    Unknown,
    /// Handled, and the game should quit.
    Quit,
}

impl Ui {
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

    /// Run a `:` command line. Returns false to quit.
    pub(super) fn run_command(&mut self, line: &str) -> bool {
        let mut parts = line.splitn(2, ' ');
        let cmd = parts.next().unwrap_or("").trim().to_lowercase();
        let arg = parts.next().unwrap_or("").trim().to_string();
        if cmd.is_empty() {
            return true;
        }
        const GROUPS: [fn(&mut Ui, &str, &str) -> Cmd; 6] = [
            Ui::cmd_files,
            Ui::cmd_sim,
            Ui::cmd_view,
            Ui::cmd_search,
            Ui::cmd_config,
            Ui::cmd_misc,
        ];
        for group in GROUPS {
            match group(self, &cmd, &arg) {
                Cmd::Quit => return false,
                Cmd::Done => return true,
                Cmd::Unknown => {}
            }
        }
        self.say(&format!("unknown command :{}  (try :help)", cmd));
        true
    }

    /// Saving, loading and the save directory.
    fn cmd_files(&mut self, cmd: &str, arg: &str) -> Cmd {
        match cmd {
            "q" | "quit" | "exit" => return Cmd::Quit,
            "q!" => {
                self.save_path = None;
                return Cmd::Quit;
            }
            "wq" | "x" => {
                if self
                    .save(if arg.is_empty() { None } else { Some(arg) })
                    .is_ok()
                {
                    return Cmd::Quit;
                }
            }
            "w" | "write" | "save" => {
                let _ = self.save(if arg.is_empty() { None } else { Some(arg) });
            }
            "saveas" => {
                if arg.is_empty() {
                    self.say("usage: :saveas NAME|PATH");
                } else {
                    let _ = self.save(Some(arg));
                }
            }
            "e" | "edit" | "load" | "open" => self.load(arg),
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
            _ => return Cmd::Unknown,
        }
        Cmd::Done
    }

    /// Running the world: pausing, speed, detail and moving through time.
    fn cmd_sim(&mut self, cmd: &str, arg: &str) -> Cmd {
        match cmd {
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
            _ => return Cmd::Unknown,
        }
        Cmd::Done
    }

    /// What is on screen: layers, zoom, theme, panels and feed filters.
    fn cmd_view(&mut self, cmd: &str, arg: &str) -> Cmd {
        match cmd {
            "layer" | "l" => match Layer::from_name(arg) {
                Some(l) => self.layer = l,
                None => self.say("layers: political terrain culture mana population biomes"),
            },
            "log" => match arg.parse::<u8>() {
                Ok(v) if v <= 3 => self.log_min = v,
                _ => self.say("usage: :log 0-3"),
            },
            "filter" => match arg.parse::<u8>() {
                Ok(v) if v <= 3 => self.chron_min = v,
                _ => self.say("usage: :filter 0-3"),
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
                self.chron_filter = arg.to_string();
                self.chron_scroll = 0;
            }
            "map" if arg.is_empty() => self.mode = Mode::Map,
            "theme" => {
                self.theme = if arg.is_empty() {
                    self.theme.next()
                } else {
                    Theme::from_name(arg).unwrap_or(self.theme)
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
            _ => return Cmd::Unknown,
        }
        Cmd::Done
    }

    /// Finding a thing by name.
    fn cmd_search(&mut self, cmd: &str, arg: &str) -> Cmd {
        match cmd {
            "find" | "go" | "goto" | "jump" | "f" => {
                let saved = self.mode;
                if matches!(saved, Mode::List | Mode::Chronicle) {
                    self.mode = Mode::Map;
                }
                self.submit_search(arg);
                if self.search_results.is_empty() {
                    self.mode = saved;
                }
            }
            _ => return Cmd::Unknown,
        }
        Cmd::Done
    }

    /// The runtime half of the config file: settings and key maps.
    fn cmd_config(&mut self, cmd: &str, arg: &str) -> Cmd {
        match cmd {
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
            "unmap" => match crate::config::parse_key(arg) {
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
            "mkconfig" => match crate::config::write_template() {
                Ok(p) => self.say(&format!("wrote {}", p.display())),
                Err(e) => self.say(&e),
            },
            "config" => self.say(&format!("{}", crate::config::config_path().display())),
            _ => return Cmd::Unknown,
        }
        Cmd::Done
    }

    /// Everything else: the Hand of Fate, muting and the storyteller.
    fn cmd_misc(&mut self, cmd: &str, arg: &str) -> Cmd {
        match cmd {
            "fate" => match (self.selected, arg.parse::<usize>()) {
                (Some(Ref::Polity(p)), Ok(c)) if (1..=6).contains(&c) => self.choose_fate(p, c - 1),
                (Some(Ref::Polity(_)), _) => self.open_fate(),
                _ => self.say("select a realm first"),
            },
            "mute" | "unmute" => {
                if arg.is_empty() {
                    let names: Vec<&str> = self.muted.iter().map(|k| k.name()).collect();
                    self.say(&if names.is_empty() { "nothing muted; :mute battle|death|founding|person|discovery|magic|politics|disaster|culture|wonder".to_string() } else { format!("muted: {}", names.join(" ")) });
                } else {
                    match EventKind::from_name(arg) {
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
            _ => return Cmd::Unknown,
        }
        Cmd::Done
    }
}
