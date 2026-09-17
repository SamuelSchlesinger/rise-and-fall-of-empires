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
        let bg = Rgb(30, 32, 43);
        let fg = Rgb(215, 218, 228);
        let key = Rgb(255, 220, 120);
        let top = self.content_height();
        self.screen.fill(
            Rect::new(0, top, sw, self.screen.h - top),
            ' ',
            Style::new(fg, bg),
        );
        for (i, row) in self.navigation_lines().iter().enumerate() {
            let mut x = 1;
            for action in row.split("  ") {
                let (shortcut, label) = action.split_once(' ').unwrap_or((action, ""));
                x = self.screen.text_attr(x, top + i, shortcut, key, bg, BOLD);
                x = self
                    .screen
                    .text(x, top + i, &format!(" {}  ", label), fg, bg);
            }
        }
        let exit = if self.tutorial.is_some() {
            "Ctrl-g skip"
        } else if self.prompt != Prompt::None {
            "Enter apply  Esc cancel"
        } else if self.mode == Mode::Fate {
            "Esc cancel"
        } else if self.mode == Mode::Map {
            "? help  :q quit"
        } else {
            "Esc back  ? help"
        };
        let exit = if exit.len() + 2 > sw {
            "Esc back"
        } else {
            exit
        };
        let hx = sw.saturating_sub(exit.len() + 1);
        let room = hx.saturating_sub(2);
        if self.prompt != Prompt::None {
            let lead = if self.prompt == Prompt::Command {
                ":"
            } else {
                "/"
            };
            // Keep the end of long commands visible while typing.
            let prompt = format!("{}{}", lead, self.prompt_text);
            let chars: Vec<_> = prompt.chars().collect();
            let start = chars.len().saturating_sub(room.saturating_sub(1));
            let visible: String = chars[start..].iter().collect();
            let x = self
                .screen
                .text_clip(1, y, &visible, room, Style::new(key, bg));
            if x < hx {
                self.screen.put_attr(x, y, ' ', fg, bg, REVERSE);
            }
        } else {
            let state = if self.paused {
                "paused".to_string()
            } else {
                format!("{} years/sec", SPEEDS[self.speed_idx])
            };
            let status = format!(
                "Year {} | {} | Follow {}",
                self.world.year,
                state,
                if self.follow { "on" } else { "off" }
            );
            self.screen
                .text_clip(1, y, &status, room, Style::attr(key, bg, BOLD));
            let x = status.chars().count() + 3;
            if x < hx {
                let info = format!("{} map | seed {}", self.layer.name(), self.world.seed);
                self.screen
                    .text_clip(x, y, &info, hx.saturating_sub(x + 1), Style::new(fg, bg));
            }
        }
        self.screen.text(hx, y, exit, key, bg);

        // Feedback uses a navigation row temporarily, leaving the world's
        // speed/follow state and the primary screen shortcuts in place.
        let feedback = if self.count.is_some() || self.pending.is_some() {
            Some(format!(
                "Keys: {}{}",
                self.count.map(|n| n.to_string()).unwrap_or_default(),
                self.pending.map(|c| c.to_string()).unwrap_or_default()
            ))
        } else if self.prompt == Prompt::Search {
            Some(match self.mode {
                Mode::List | Mode::Chronicle => {
                    "Type to filter | Enter keep | Esc clear".to_string()
                }
                _ => {
                    let hits = self.search(&self.prompt_text);
                    hits.first()
                        .map(|r| {
                            format!(
                                "{} matches | Enter visits {}",
                                hits.len(),
                                detail::entity_name(&self.world, *r)
                            )
                        })
                        .unwrap_or_else(|| {
                            if self.prompt_text.is_empty() {
                                "Type a name to find a realm, city or person".to_string()
                            } else {
                                "No matches: try another name".to_string()
                            }
                        })
                }
            })
        } else if self.prompt == Prompt::Command {
            Some(":w save | :guide | :tutorial | :speed N | :follow off".to_string())
        } else if Instant::now() < self.msg_until {
            Some(self.msg.clone())
        } else {
            None
        };
        if let Some(message) = feedback {
            if top == y && self.prompt != Prompt::None {
                return;
            }
            let row = if y > top { y - 1 } else { y };
            let width = if row == y { room } else { sw.saturating_sub(2) };
            self.screen
                .fill(Rect::new(0, row, width + 1, 1), ' ', Style::new(fg, bg));
            self.screen
                .text_clip(1, row, &message, width, Style::new(Rgb(160, 255, 180), bg));
        }
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
            self.away_year = Some(self.world.year);
            return;
        }
        if self.mode == Mode::Map {
            if let Some(mark) = self.away_year.take() {
                self.since_note = self.missed_since(mark);
            }
        }
    }

    /// One plain sentence about the major events since year `mark`, or
    /// nothing if the world stayed quiet.
    ///
    /// Takes a year rather than a chronicle index, because compaction
    /// removes events and slides every later index backwards. The old
    /// guard here — refuse when the index is past the end — protected
    /// against the rarer half of that: the common case is an index that
    /// still lands *inside* the vector but seven thousand entries too
    /// early, and the note then cheerfully reported a thousand things over
    /// nine hundred years for a two-minute absence.
    fn missed_since(&self, mark: i32) -> Option<String> {
        let events = &self.world.chronicle.events;
        let major: Vec<&crate::sim::chronicle::Event> = events
            .iter()
            .filter(|e| e.year > mark && e.importance >= 2 && !self.muted.contains(&e.kind))
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
