//! Composing a frame: which panels the current mode draws, the status
//! line, the shared chronicle-line builder and the theme's colour map.

mod map;
mod panels;
mod sidebar;

use crate::ui::*;

/// The ASCII stand-in for a glyph `--ascii` must not print.
///
/// The map's own biome glyphs are chosen by [`biome_style`] as it draws, and
/// the pairs below agree with it (`♣` is the forest's `T`, since that is what
/// the help page's symbol key means by it). Anything unrecognised becomes a
/// `?`, which is at least visibly a stand-in rather than a broken cell.
fn ascii_glyph(ch: char) -> char {
    match ch {
        '─' | '━' | '╌' => '-',
        '│' | '┃' | '╎' => '|',
        '┌' | '┐' | '└' | '┘' | '├' | '┤' | '┬' | '┴' | '┼' => '+',
        '█' | '▓' | '■' | '▪' => '#',
        '▒' => '+',
        '░' => '.',
        '•' | '◦' => '*',
        '·' => '|',
        '≈' => '~',
        '▲' | '△' | '^' => '^',
        '♣' => 'T',
        '♠' => 't',
        '∩' => 'n',
        '¤' => '%',
        '×' => 'x',
        '↑' => '+',
        '↓' => '-',
        '→' => '>',
        '←' => '<',
        '▶' | '▸' => '>',
        '◀' | '◂' => '<',
        '▼' => 'v',
        '…' => '.',
        '—' | '–' => '-',
        '“' | '”' => '"',
        '‘' | '’' => '\'',
        '≥' => '>',
        '≤' => '<',
        '°' => 'o',
        c if c.is_ascii() => c,
        _ => '?',
    }
}

impl Ui {
    pub(super) fn render(&mut self) {
        self.compose();
        self.screen.flush();
    }

    pub(super) fn compose(&mut self) {
        self.note_mode_change();
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
            Mode::Help | Mode::Guide => self.render_help(),
            Mode::Recap => self.render_recap(),
        }
        self.render_status();
        if self.tour {
            self.render_tour();
        }
        if self.tutorial.is_some() {
            self.render_tutorial();
        }
        if !self.theme.is_identity() {
            let t = self.theme;
            for c in self.screen.cells_mut() {
                c.fg = t.map(c.fg, false);
                c.bg = t.map(c.bg, true);
            }
        }
        if self.ascii {
            // A last sweep, in the same spirit as the theme pass above.
            // Individual widgets pick ASCII glyphs as they draw, but `--ascii`
            // is a promise about the whole frame, and box rules, separators
            // and the help page's own symbol key were all still coming out in
            // Unicode. Doing it here means a new widget cannot break it.
            for c in self.screen.cells_mut() {
                if !c.ch.is_ascii() {
                    c.ch = ascii_glyph(c.ch);
                }
            }
        }
    }

    fn render_status(&mut self) {
        let sw = self.screen.w;
        let y = self.screen.h - 1;
        let bg = Rgb(40, 40, 52);
        let fg = Rgb(220, 220, 225);
        let key = Rgb(255, 220, 120);
        self.screen
            .fill(Rect::new(0, y, sw, 1), ' ', Style::new(fg, bg));
        if matches!(self.mode, Mode::Help | Mode::Guide) {
            let hint = if self.mode == Mode::Guide {
                "Esc back | j/k scroll | ? keys | t tutorial"
            } else {
                "Esc back | j/k scroll | p guide | t tutorial"
            };
            self.screen
                .text_clip(1, y, hint, sw.saturating_sub(2), Style::new(key, bg));
            return;
        }
        if self.tutorial.is_some() {
            let state = if self.paused {
                "paused"
            } else {
                "time running"
            };
            self.screen.text_clip(
                1,
                y,
                &format!("Ctrl-g skip | {}", state),
                sw.saturating_sub(2),
                Style::new(key, bg),
            );
            return;
        }
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
            "paused".to_string()
        } else {
            format!("{} years/sec", SPEEDS[self.speed_idx])
        };
        let left = format!(" year {} · {} ", self.world.year, state);
        let mut x = self.screen.text_attr(0, y, &left, key, bg, BOLD);
        let rest = format!(
            "· {} map · detail {} · seed {} · {:.1}ms/y {}fps",
            self.layer.name(),
            self.world.detail.name(),
            self.world.seed,
            self.world.ticks_ms,
            self.fps
        );
        x = self.screen.text(x, y, &rest, Rgb(170, 170, 185), bg);
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
            Mode::Help => "j/k scroll  p guide  t tutorial  Esc back",
            Mode::Guide => "j/k scroll  ? keys  t tutorial  Esc back",
            Mode::Fate => "1-6 choose  Esc cancel",
            Mode::Recap => "j/k scroll  :recap N for a longer look  Esc back",
        };
        // Progressively shorter hints, because a terminal with no room for
        // the full set is exactly the one whose reader most needs to be told
        // how to get out. The last is two words long and always drawn.
        let mut hints = hints;
        let mut hx = sw.saturating_sub(hints.chars().count() + 1);
        for shorter in ["? help  :q quit", "? :q"] {
            if hx > x + 2 {
                break;
            }
            hints = shorter;
            hx = sw.saturating_sub(hints.chars().count() + 1);
        }
        // Still no room beside what is already there: give the hints the end
        // of the line anyway and let the seed and the frame rate go instead.
        // Painting over the tail of a truncated line beats leaving the reader
        // with no way out on screen at all.
        if hx <= x + 2 {
            hx = sw.saturating_sub(hints.chars().count().min(sw));
        }
        self.screen
            .fill(Rect::new(hx, y, sw - hx, 1), ' ', Style::new(fg, bg));
        self.screen.text(hx, y, hints, Rgb(170, 170, 180), bg);
    }

    /// Remember what the viewer missed while they were off the map, and
    /// hand them one line about it when they come back.
    fn note_mode_change(&mut self) {
        let was = self.last_mode;
        self.last_mode = self.mode;
        if self.mode == was {
            return;
        }
        if was == Mode::Map && self.mode != Mode::Fate {
            self.away_mark = Some(self.world.chronicle.len());
            return;
        }
        if self.mode == Mode::Map {
            if let Some(mark) = self.away_mark.take() {
                self.since_note = self.missed_since(mark);
            }
        }
    }

    /// One plain sentence about the major events after chronicle entry
    /// `mark`, or nothing if the world stayed quiet.
    fn missed_since(&self, mark: usize) -> Option<String> {
        let events = &self.world.chronicle.events;
        if mark >= events.len() {
            return None;
        }
        let major: Vec<&crate::sim::chronicle::Event> = events[mark..]
            .iter()
            .filter(|e| e.importance >= 2 && !self.muted.contains(&e.kind))
            .collect();
        let first = major.first()?;
        let years = self.world.year - first.year;
        let when = if years <= 1 {
            "in the year".to_string()
        } else {
            format!("over the {} years", years)
        };
        Some(if major.len() == 1 {
            format!("While you were away: {}", first.text)
        } else {
            format!(
                "{} notable things {} you were away. The first: {}",
                major.len(),
                when,
                first.text
            )
        })
    }

    /// The newest chronicle entries, wrapped to `width` and newest first, as
    /// `(line, colour, attributes, event index)`. Stops once `need` lines are
    /// gathered. The event log and the chronicle page both draw from this.
    /// The indent [`Ui::chronicle_lines`] puts on a wrapped continuation
    /// line, where a first line carries the year instead.
    pub(super) const CHRON_CONT: &'static str = "       ";

    /// Whether a chronicle row is the start of an entry rather than the
    /// middle of a wrapped one.
    pub(super) fn chron_head(l: &str) -> bool {
        !l.starts_with(Ui::CHRON_CONT)
    }

    /// Which of the chronicle's rows to draw in `avail` rows of panel.
    ///
    /// The rows come newest first and are drawn bottom-up, so the end of the
    /// slice is the panel's top row. Cutting there at an arbitrary point
    /// opened the panel on the tail of a wrapped sentence — no year, no
    /// subject, half an idea. So the slice ends on the start of an entry;
    /// and when no whole entry fits at all, it shows the newest entry from
    /// its first line down rather than its last lines.
    pub(super) fn chron_window(
        rows: &[(String, Rgb, u8, usize)],
        avail: usize,
    ) -> &[(String, Rgb, u8, usize)] {
        if rows.is_empty() || avail == 0 {
            return &[];
        }
        let head = |i: &usize| Ui::chron_head(&rows[*i].0);
        let top = (0..rows.len().min(avail))
            .rev()
            .find(head)
            .or_else(|| (0..rows.len()).find(head))
            .unwrap_or(rows.len() - 1);
        &rows[(top + 1).saturating_sub(avail)..=top]
    }

    fn chronicle_lines(
        &self,
        min: u8,
        filter: &str,
        width: usize,
        need: usize,
    ) -> Vec<(String, Rgb, u8, usize)> {
        let mut lines: Vec<(String, Rgb, u8, usize)> = Vec::new();
        let w = &self.world;
        for (idx, e) in w.chronicle.events.iter().enumerate().rev() {
            if e.importance < min || self.muted.contains(&e.kind) {
                continue;
            }
            if !filter.is_empty() && !e.text.to_lowercase().contains(filter) {
                continue;
            }
            let (color, attr) = event_style(e.kind, e.importance);
            let wrapped = term::wrap(&e.text, width);
            for (k, l) in wrapped.iter().enumerate().rev() {
                let prefix = if k == 0 {
                    format!("{:>5}  ", e.year)
                } else {
                    Ui::CHRON_CONT.to_string()
                };
                lines.push((format!("{}{}", prefix, l), color, attr, idx));
            }
            if lines.len() >= need {
                break;
            }
        }
        lines
    }
}
