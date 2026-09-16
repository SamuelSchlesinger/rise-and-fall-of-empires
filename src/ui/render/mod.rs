//! Composing a frame: which panels the current mode draws, the status
//! line, the shared chronicle-line builder and the theme's colour map.

mod map;
mod panels;
mod sidebar;

use crate::ui::*;

impl Ui {
    pub(super) fn render(&mut self) {
        self.compose();
        self.screen.flush();
    }

    pub(super) fn compose(&mut self) {
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

    /// The newest chronicle entries, wrapped to `width` and newest first, as
    /// `(line, colour, attributes, event index)`. Stops once `need` lines are
    /// gathered. The event log and the chronicle page both draw from this.
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
                    "       ".to_string()
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
