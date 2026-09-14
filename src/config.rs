//! User configuration: `~/.config/empires/config` with `key = value`
//! settings and vim-style `map <from> <to>` key remaps.

use crate::sim::Detail;
use crate::term::Key;
use std::path::PathBuf;

#[derive(Default, Clone)]
pub struct Config {
    pub detail: Option<Detail>,
    pub speed: Option<f64>,
    pub theme: Option<String>,
    pub mouse: Option<bool>,
    pub ascii: Option<bool>,
    pub autosave: Option<i32>,
    pub width: Option<usize>,
    pub height: Option<usize>,
    pub log: Option<u8>,
    pub follow: Option<bool>,
    pub zoom: Option<usize>,
    pub maps: Vec<(Key, Key)>,
    pub errors: Vec<String>,
}

pub fn config_dir() -> PathBuf {
    if let Ok(x) = std::env::var("XDG_CONFIG_HOME") {
        if !x.is_empty() {
            return PathBuf::from(x).join("empires");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config/empires")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config")
}

pub fn load() -> Config {
    let mut c = Config::default();
    let text = match std::fs::read_to_string(config_path()) {
        Ok(t) => t,
        Err(_) => return c,
    };
    for (n, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("map ").or_else(|| line.strip_prefix("noremap ")) {
            let mut it = rest.split_whitespace();
            match (it.next().and_then(parse_key), it.next().and_then(parse_key)) {
                (Some(a), Some(b)) => c.maps.push((a, b)),
                _ => c.errors.push(format!("line {}: cannot parse map '{}'", n + 1, rest)),
            }
            continue;
        }
        let line = line.strip_prefix("set ").unwrap_or(line);
        let (k, v) = match line.split_once('=') {
            Some((k, v)) => (k.trim().to_lowercase(), v.trim().to_string()),
            None => match line.split_once(' ') {
                Some((k, v)) => (k.trim().to_lowercase(), v.trim().to_string()),
                None => {
                    c.errors.push(format!("line {}: expected 'key = value'", n + 1));
                    continue;
                }
            },
        };
        if let Err(e) = c.set(&k, &v) {
            c.errors.push(format!("line {}: {}", n + 1, e));
        }
    }
    c
}

fn parse_bool(v: &str) -> Option<bool> {
    match v.to_lowercase().as_str() {
        "on" | "true" | "yes" | "1" => Some(true),
        "off" | "false" | "no" | "0" => Some(false),
        _ => None,
    }
}

impl Config {
    /// Apply one `key = value` setting. Returns an error message for bad input.
    pub fn set(&mut self, k: &str, v: &str) -> Result<(), String> {
        match k {
            "detail" => {
                self.detail = Some(match v.to_lowercase().as_str() {
                    "low" | "l" => Detail::Low,
                    "medium" | "med" | "m" => Detail::Medium,
                    "high" | "h" => Detail::High,
                    _ => return Err("detail must be low, medium or high".into()),
                })
            }
            "speed" => self.speed = Some(v.parse().map_err(|_| "speed must be a number")?),
            "theme" => self.theme = Some(v.to_lowercase()),
            "mouse" => self.mouse = Some(parse_bool(v).ok_or("mouse must be on or off")?),
            "ascii" => self.ascii = Some(parse_bool(v).ok_or("ascii must be on or off")?),
            "autosave" => self.autosave = Some(v.parse().map_err(|_| "autosave must be a number of years")?),
            "width" => self.width = Some(v.parse().map_err(|_| "width must be a number")?),
            "height" => self.height = Some(v.parse().map_err(|_| "height must be a number")?),
            "log" => self.log = Some(v.parse().map_err(|_| "log must be 0-3")?),
            "follow" => self.follow = Some(parse_bool(v).ok_or("follow must be on or off")?),
            "zoom" => self.zoom = Some(v.parse().map_err(|_| "zoom must be 1-4")?),
            _ => return Err(format!("unknown setting '{}'", k)),
        }
        Ok(())
    }
}

/// Parse a vim-style key name: `j`, `<Up>`, `<S-Left>`, `<C-d>`, `<Enter>`, `<Space>`.
pub fn parse_key(s: &str) -> Option<Key> {
    let mut chars = s.chars();
    let first = chars.next()?;
    if chars.next().is_none() {
        return Some(match first {
            ' ' => Key::Char(' '),
            '\t' => Key::Tab,
            c => Key::Char(c),
        });
    }
    let inner = s.strip_prefix('<')?.strip_suffix('>')?;
    let lower = inner.to_lowercase();
    if let Some(rest) = lower.strip_prefix("c-") {
        let c = rest.chars().next()?;
        return Some(Key::Ctrl(c));
    }
    let shift = lower.starts_with("s-");
    let base = if shift { &lower[2..] } else { lower.as_str() };
    let k = match base {
        "up" => if shift { Key::ShiftUp } else { Key::Up },
        "down" => if shift { Key::ShiftDown } else { Key::Down },
        "left" => if shift { Key::ShiftLeft } else { Key::Left },
        "right" => if shift { Key::ShiftRight } else { Key::Right },
        "c-up" => Key::CtrlUp,
        "enter" | "cr" | "return" => Key::Enter,
        "esc" | "escape" => Key::Esc,
        "tab" => if shift { Key::BackTab } else { Key::Tab },
        "space" => Key::Char(' '),
        "bs" | "backspace" => Key::Backspace,
        "del" | "delete" => Key::Delete,
        "pageup" | "pgup" => Key::PageUp,
        "pagedown" | "pgdn" => Key::PageDown,
        "home" => Key::Home,
        "end" => Key::End,
        "lt" => Key::Char('<'),
        "gt" => Key::Char('>'),
        "bar" => Key::Char('|'),
        b if b.starts_with('f') && b[1..].parse::<u8>().is_ok() => Key::F(b[1..].parse().unwrap()),
        _ => return None,
    };
    Some(k)
}

pub fn key_name(k: Key) -> String {
    match k {
        Key::Char(' ') => "<Space>".into(),
        Key::Char('<') => "<lt>".into(),
        Key::Char(c) => c.to_string(),
        Key::Ctrl(c) => format!("<C-{}>", c),
        Key::Up => "<Up>".into(),
        Key::Down => "<Down>".into(),
        Key::Left => "<Left>".into(),
        Key::Right => "<Right>".into(),
        Key::ShiftUp => "<S-Up>".into(),
        Key::ShiftDown => "<S-Down>".into(),
        Key::ShiftLeft => "<S-Left>".into(),
        Key::ShiftRight => "<S-Right>".into(),
        Key::CtrlUp => "<C-Up>".into(),
        Key::CtrlDown => "<C-Down>".into(),
        Key::CtrlLeft => "<C-Left>".into(),
        Key::CtrlRight => "<C-Right>".into(),
        Key::PageUp => "<PageUp>".into(),
        Key::PageDown => "<PageDown>".into(),
        Key::Home => "<Home>".into(),
        Key::End => "<End>".into(),
        Key::Enter => "<Enter>".into(),
        Key::Esc => "<Esc>".into(),
        Key::Tab => "<Tab>".into(),
        Key::BackTab => "<S-Tab>".into(),
        Key::Backspace => "<BS>".into(),
        Key::Delete => "<Del>".into(),
        Key::F(n) => format!("<F{}>", n),
        Key::Mouse(_) => "<Mouse>".into(),
    }
}

pub const TEMPLATE: &str = "# Rise and Fall of Empires configuration\n# Settings take `key = value`; keys can be remapped vim-style with `map <from> <to>`.\n\n# detail = medium        # low | medium | high\n# speed = 5              # years per second at start (0.5 1 2 5 10 25 50 100)\n# theme = default        # default | phosphor | amber | paper | dusk\n# mouse = on\n# ascii = off\n# autosave = 100         # years between autosaves when a save file is set (0 = off)\n# width = 160\n# height = 64\n# log = 1                # minimum importance shown in the event log (0-3)\n# follow = on            # jump the cursor to major events\n# zoom = 1               # 1-4, how many world cells per character\n\n# map w k                # examples: map <S-Up> K, map <C-p> :, map ; :\n";

pub fn write_template() -> Result<PathBuf, String> {
    let path = config_path();
    if path.exists() {
        return Err(format!("{} already exists", path.display()));
    }
    std::fs::create_dir_all(config_dir()).map_err(|e| e.to_string())?;
    std::fs::write(&path, TEMPLATE).map_err(|e| e.to_string())?;
    Ok(path)
}
