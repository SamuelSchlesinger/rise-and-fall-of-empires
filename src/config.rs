//! User configuration: `~/.config/empires/config` with `key = value`
//! settings and vim-style `map <from> <to>` key remaps.

use crate::sim::tuning::UnknownField;
use crate::sim::Detail;
use crate::term::Key;
use std::path::PathBuf;

/// Why one `key = value` setting was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// No setting goes by that name.
    UnknownSetting(String),
    /// The setting exists but the value does not fit it.
    BadValue {
        /// The setting's name.
        key: String,
        /// What the value should have looked like, e.g. "low, medium or high".
        expected: &'static str,
    },
    /// A `tune.<field>` setting named a field that does not exist.
    UnknownTuning(UnknownField),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::UnknownSetting(k) => write!(f, "unknown setting '{}'", k),
            ConfigError::BadValue { key, expected } => write!(f, "{} must be {}", key, expected),
            ConfigError::UnknownTuning(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::UnknownTuning(e) => Some(e),
            _ => None,
        }
    }
}

impl From<UnknownField> for ConfigError {
    fn from(e: UnknownField) -> ConfigError {
        ConfigError::UnknownTuning(e)
    }
}

/// Why the commented config template could not be written.
#[derive(Debug)]
pub enum TemplateError {
    /// There is a config file there already; it is never overwritten.
    Exists(PathBuf),
    /// The directory or the file could not be written.
    Io(std::io::Error),
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateError::Exists(p) => write!(f, "{} already exists", p.display()),
            TemplateError::Io(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for TemplateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TemplateError::Io(e) => Some(e),
            TemplateError::Exists(_) => None,
        }
    }
}

impl From<std::io::Error> for TemplateError {
    fn from(e: std::io::Error) -> TemplateError {
        TemplateError::Io(e)
    }
}

/// Everything the config file can say. `None` means "the file did not
/// mention it", so a command-line flag or the default still wins.
#[derive(Default, Clone)]
pub struct Config {
    /// How much detail the simulation keeps.
    pub detail: Option<Detail>,
    /// Years per second at start.
    pub speed: Option<f64>,
    /// A [`crate::theme::Theme`] name.
    pub theme: Option<String>,
    /// Whether to capture the mouse.
    pub mouse: Option<bool>,
    /// Whether to draw with plain ASCII glyphs.
    pub ascii: Option<bool>,
    /// Years between autosaves, 0 for never.
    pub autosave: Option<i32>,
    /// Map width in cells.
    pub width: Option<usize>,
    /// Map height in cells.
    pub height: Option<usize>,
    /// Lowest importance shown in the event log.
    pub log: Option<u8>,
    /// Whether the cursor follows major events.
    pub follow: Option<bool>,
    /// World cells per character, 1 to 4.
    pub zoom: Option<usize>,
    /// `map <from> <to>` key remaps, in the order they were written.
    pub maps: Vec<(Key, Key)>,
    /// `tune.<field> = <number>` overrides for `sim::tuning::Tuning`.
    pub tunes: Vec<(String, f64)>,
    /// Complaints about the file, reported once the interface is up.
    pub errors: Vec<String>,
}

/// `$XDG_CONFIG_HOME/empires`, or `~/.config/empires`.
pub fn config_dir() -> PathBuf {
    if let Ok(x) = std::env::var("XDG_CONFIG_HOME") {
        if !x.is_empty() {
            return PathBuf::from(x).join("empires");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config/empires")
}

/// The config file itself: [`config_dir`] plus `config`.
pub fn config_path() -> PathBuf {
    config_dir().join("config")
}

/// Read [`config_path`]. A missing file is not an error, and a bad line is
/// collected into `Config::errors` rather than stopping the rest.
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
        if let Some(rest) = line
            .strip_prefix("map ")
            .or_else(|| line.strip_prefix("noremap "))
        {
            let mut it = rest.split_whitespace();
            match (it.next().and_then(parse_key), it.next().and_then(parse_key)) {
                (Some(a), Some(b)) => c.maps.push((a, b)),
                _ => c
                    .errors
                    .push(format!("line {}: cannot parse map '{}'", n + 1, rest)),
            }
            continue;
        }
        let line = line.strip_prefix("set ").unwrap_or(line);
        let (k, v) = match line.split_once('=') {
            Some((k, v)) => (k.trim().to_lowercase(), v.trim().to_string()),
            None => match line.split_once(' ') {
                Some((k, v)) => (k.trim().to_lowercase(), v.trim().to_string()),
                None => {
                    c.errors
                        .push(format!("line {}: expected 'key = value'", n + 1));
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
    /// Apply one `key = value` setting.
    ///
    /// # Errors
    ///
    /// [`ConfigError`] naming the setting, if it is unknown or the value does
    /// not fit it.
    pub fn set(&mut self, k: &str, v: &str) -> Result<(), ConfigError> {
        // A value that does not parse: "<k> must be <expected>".
        let bad = |expected: &'static str| ConfigError::BadValue {
            key: k.to_string(),
            expected,
        };
        match k {
            "detail" => {
                self.detail = Some(match v.to_lowercase().as_str() {
                    "low" | "l" => Detail::Low,
                    "medium" | "med" | "m" => Detail::Medium,
                    "high" | "h" => Detail::High,
                    _ => return Err(bad("low, medium or high")),
                });
            }
            "speed" => self.speed = Some(v.parse().map_err(|_| bad("a number"))?),
            // Checked here rather than shrugged off later: the point of the
            // config file is that a name it does not know is reported.
            "theme" => {
                let name = v.to_lowercase();
                if crate::theme::Theme::from_name(&name).is_none() {
                    return Err(bad("default, phosphor, amber, paper or dusk"));
                }
                self.theme = Some(name);
            }
            "mouse" => self.mouse = Some(parse_bool(v).ok_or_else(|| bad("on or off"))?),
            "ascii" => self.ascii = Some(parse_bool(v).ok_or_else(|| bad("on or off"))?),
            "autosave" => self.autosave = Some(v.parse().map_err(|_| bad("a number of years"))?),
            "width" => self.width = Some(v.parse().map_err(|_| bad("a number"))?),
            "height" => self.height = Some(v.parse().map_err(|_| bad("a number"))?),
            // The ranges the template advertises, enforced. Both used to be
            // parsed as bare numbers and silently clamped on the way in.
            "log" => {
                let n: u8 = v.parse().map_err(|_| bad("1-3"))?;
                if n > 3 {
                    return Err(bad("1-3"));
                }
                // 0 is accepted for old config files and taken as 1: one
                // line in the whole simulation is logged below importance 1.
                self.log = Some(n.max(1));
            }
            "follow" => self.follow = Some(parse_bool(v).ok_or_else(|| bad("on or off"))?),
            "zoom" => {
                let n: usize = v.parse().map_err(|_| bad("1-4"))?;
                if !(1..=4).contains(&n) {
                    return Err(bad("1-4"));
                }
                self.zoom = Some(n);
            }
            _ => match k.strip_prefix("tune.") {
                Some(field) => {
                    let value: f64 = v.parse().map_err(|_| bad("a number"))?;
                    // Check the name now so mistakes are reported where they are made.
                    crate::sim::tuning::Tuning::default().set(field, value)?;
                    self.tunes.push((field.to_string(), value));
                }
                None => return Err(ConfigError::UnknownSetting(k.to_string())),
            },
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
        // The ctrl-arrows are keys in their own right, not control
        // characters, so they have to be recognised before the fallback
        // below — which otherwise turns `<C-Up>` into `<C-u>`.
        match rest {
            "up" => return Some(Key::CtrlUp),
            "down" => return Some(Key::CtrlDown),
            "left" => return Some(Key::CtrlLeft),
            "right" => return Some(Key::CtrlRight),
            _ => {}
        }
        let mut it = rest.chars();
        let c = it.next()?;
        // `<C-foo>` is a mistake, not Ctrl+F: say so rather than guess.
        return it.next().is_none().then_some(Key::Ctrl(c));
    }
    let shift = lower.starts_with("s-");
    let base = if shift { &lower[2..] } else { lower.as_str() };
    let k = match base {
        "up" => {
            if shift {
                Key::ShiftUp
            } else {
                Key::Up
            }
        }
        "down" => {
            if shift {
                Key::ShiftDown
            } else {
                Key::Down
            }
        }
        "left" => {
            if shift {
                Key::ShiftLeft
            } else {
                Key::Left
            }
        }
        "right" => {
            if shift {
                Key::ShiftRight
            } else {
                Key::Right
            }
        }
        "enter" | "cr" | "return" => Key::Enter,
        "esc" | "escape" => Key::Esc,
        "tab" => {
            if shift {
                Key::BackTab
            } else {
                Key::Tab
            }
        }
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

/// The vim-style name of a key, the inverse of [`parse_key`].
pub fn key_name(k: &Key) -> String {
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
        Key::Paste(_) => "<Paste>".into(),
    }
}

/// The commented config file `--mkconfig` writes.
pub const TEMPLATE: &str = "# Rise and Fall of Empires configuration\n# Settings take `key = value`; keys can be remapped vim-style with `map <from> <to>`.\n# Uncomment what you want. A value outside the range in the comment, or a name\n# this build does not know, is reported when the game starts.\n\n# detail = medium        # low | medium | high\n# speed = 5              # years per second at start; snapped to the nearest of\n#                        # 0.5 1 2 5 10 25 50 100\n# theme = default        # default | phosphor | amber | paper | dusk\n# mouse = on\n# ascii = off\n# autosave = 100         # years between autosaves when a save file is set (0 = off)\n# width = 160            # 40-600\n# height = 64            # 20-300\n# log = 2                # least importance shown in the event log: 1 everything,\n#                        # 2 the notable (the default), 3 only the great\n# follow = on            # jump the cursor to major events\n# zoom = 1               # 1-4, how many world cells per character\n\n# tune.decadence_growth = 0.0045   # override any field of sim::tuning::Tuning\n\n# map w k                # examples: map <S-Up> K, map <C-p> :, map ; :\n";

/// Write [`TEMPLATE`] to [`config_path`] and return where it landed.
///
/// # Errors
///
/// [`TemplateError::Exists`] rather than overwriting a config someone has
/// already written, or [`TemplateError::Io`] if the write fails.
pub fn write_template() -> Result<PathBuf, TemplateError> {
    let path = config_path();
    if path.exists() {
        return Err(TemplateError::Exists(path));
    }
    std::fs::create_dir_all(config_dir())?;
    std::fs::write(&path, TEMPLATE)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key `key_name` can print must parse back to the key it named.
    /// `:maps` prints these names, so a name that does not round-trip is one
    /// a reader cannot copy into their config file.
    #[test]
    fn key_names_round_trip() {
        let keys = [
            Key::Char('j'),
            Key::Char(' '),
            Key::Char('<'),
            Key::Ctrl('d'),
            Key::Up,
            Key::Down,
            Key::Left,
            Key::Right,
            Key::ShiftUp,
            Key::ShiftDown,
            Key::ShiftLeft,
            Key::ShiftRight,
            Key::CtrlUp,
            Key::CtrlDown,
            Key::CtrlLeft,
            Key::CtrlRight,
            Key::PageUp,
            Key::PageDown,
            Key::Home,
            Key::End,
            Key::Enter,
            Key::Esc,
            Key::Tab,
            Key::BackTab,
            Key::Backspace,
            Key::Delete,
            Key::F(1),
            Key::F(12),
        ];
        for k in keys {
            let name = key_name(&k);
            assert_eq!(
                parse_key(&name),
                Some(k.clone()),
                "{:?} prints as {} which parses back as {:?}",
                k,
                name,
                parse_key(&name)
            );
        }
    }

    #[test]
    fn nonsense_key_names_are_refused() {
        for s in ["<C-foo>", "<nope>", "<>", "<C->", "<S-nope>"] {
            assert_eq!(parse_key(s), None, "{} should not parse", s);
        }
    }

    #[test]
    fn settings_reject_values_they_cannot_honour() {
        let mut c = Config::default();
        // Ranges the template advertises are enforced, not clamped later.
        assert!(c.set("zoom", "0").is_err());
        assert!(c.set("zoom", "5").is_err());
        assert!(c.set("zoom", "4").is_ok());
        assert!(c.set("log", "4").is_err());
        assert!(c.set("log", "3").is_ok());
        assert!(c.set("theme", "bogus").is_err());
        assert!(c.set("theme", "Paper").is_ok());
        assert!(c.set("detail", "sideways").is_err());
        assert!(c.set("nosuchthing", "1").is_err());
        assert!(c.set("tune.no_such_field", "1").is_err());
    }

    /// Every setting the template shows must be one `set` accepts, and every
    /// value it shows must be one `set` takes. A template that does not load
    /// is worse than none.
    #[test]
    fn the_template_is_a_valid_config() {
        let mut c = Config::default();
        let mut seen = 0;
        for raw in TEMPLATE.lines() {
            let line = raw.trim_start_matches('#').trim();
            let line = line.split('#').next().unwrap_or("").trim();
            // A line is a setting only if what comes before the `=` is one
            // bare name; the rest of the file is prose about the file.
            let is_setting = line.split_once('=').is_some_and(|(k, _)| {
                let k = k.trim();
                !k.is_empty()
                    && k.chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
            });
            if line.is_empty() || !(is_setting || line.starts_with("map ")) {
                continue;
            }
            if let Some(rest) = line.strip_prefix("map ") {
                let mut it = rest.split_whitespace();
                assert!(
                    it.next().and_then(parse_key).is_some(),
                    "template map line does not parse: {}",
                    raw
                );
                seen += 1;
                continue;
            }
            let (k, v) = line.split_once('=').expect(raw);
            assert!(
                c.set(k.trim(), v.trim()).is_ok(),
                "template line is not a setting this build accepts: {}",
                raw
            );
            seen += 1;
        }
        assert!(seen > 10, "only {} template lines were checked", seen);
    }

    /// The player guide reproduces the template. It has drifted from it before —
    /// showing settings uncommented, and ranges the parser did not enforce —
    /// so the two are checked against each other rather than by eye.
    #[test]
    fn the_guide_quotes_the_template_it_has() {
        let guide = include_str!("../docs/GUIDE.md");
        let mut checked = 0;
        for l in TEMPLATE.lines() {
            let l = l.trim_end();
            let body = l.trim_start_matches('#').trim_start();
            // Only the settings themselves and the remap example; the file's
            // own header prose is the guide's to word as it likes.
            let names_a_setting = body.split_once('=').is_some_and(|(k, _)| {
                let k = k.trim();
                !k.is_empty()
                    && k.chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
            });
            if !(names_a_setting || body.starts_with("map ")) {
                continue;
            }
            checked += 1;
            assert!(
                guide.contains(l),
                "docs/GUIDE.md does not show this line of the config template:\n  {}",
                l
            );
        }
        assert!(checked > 10, "only {} template lines were checked", checked);
    }
}
