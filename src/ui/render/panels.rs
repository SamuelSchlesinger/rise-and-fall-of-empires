//! The event log under the map and the full-screen panels: lists, detail
//! pages, the chronicle, the help screen and the Hand of Fate.

use crate::ui::*;

impl Ui {
    pub(super) fn render_log(&mut self) {
        let (_, _, _, mh) = self.map_rect();
        let sw = self.screen.w;
        let y0 = mh;
        let h = self.content_height().saturating_sub(mh);
        self.log_rows.clear();
        if h == 0 {
            return;
        }
        let bg = Rgb(10, 10, 14);
        self.screen.fill(
            Rect::new(0, y0, sw, h),
            ' ',
            Style::new(Rgb(200, 200, 200), bg),
        );
        self.screen.hline(0, y0, sw, Rgb(60, 60, 70), bg);
        // What the feed is showing, in words as well as in the number `v`
        // and `:log` set: "importance ≥2" says nothing on its own.
        let title = format!(
            " Chronicle | c history | r recap | v {} (≥{}) ",
            words::log_level(self.log_min),
            self.log_min
        );
        let title = if self.ascii {
            title.replace('≥', ">=")
        } else {
            title
        };
        self.screen
            .text_attr(2, y0, &title, Rgb(230, 200, 120), bg, BOLD);
        let avail = h - 1;
        let width = sw.saturating_sub(10);
        let mut lines = self.chronicle_lines(self.log_min, "", width, avail);
        self.fold_battles(&mut lines, width);
        let shown: Vec<(String, Rgb, u8, usize)> = Ui::chron_window(&lines, avail).to_vec();
        let mut y = y0 + h - 1;
        for (l, c, a, idx) in shown {
            self.screen.text_attr(1, y, &l, c, bg, a);
            self.log_rows.push((y, idx));
            if y == y0 + 1 {
                break;
            }
            y -= 1;
        }
    }

    pub(super) fn render_list(&mut self) {
        let sw = self.screen.w;
        let sh = self.content_height();
        let bg = Rgb(14, 14, 20);
        let fg = Rgb(200, 200, 205);
        let accent = Rgb(230, 200, 120);
        self.screen
            .fill(Rect::new(0, 0, sw, sh), ' ', Style::new(fg, bg));
        // Tabs.
        let tabs = self.list_tab_layout();
        for &(i, x) in &tabs {
            let t = detail::LIST_TABS[i];
            let sel = i == self.list_tab;
            let (f, b, a) = if sel {
                (Rgb(10, 10, 10), accent, BOLD)
            } else {
                (fg, Rgb(30, 30, 40), 0)
            };
            self.screen.text_attr(x, 0, &format!(" {} ", t), f, b, a);
        }
        if tabs.first().map(|(i, _)| *i > 0).unwrap_or(false) {
            self.screen.put(0, 0, '<', accent, bg);
        }
        if tabs
            .last()
            .map(|(i, _)| *i + 1 < detail::LIST_TABS.len())
            .unwrap_or(false)
        {
            self.screen.put(sw - 1, 0, '>', accent, bg);
        }
        let rows = self.list_rows();
        let header = detail::list_header(self.list_tab);
        self.screen
            .text_attr(1, 1, header, Rgb(150, 150, 160), bg, BOLD);
        if !self.list_filter.is_empty() {
            let f = format!(" {} rows match \"{}\" ", rows.len(), self.list_filter);
            self.screen.text_attr(
                sw.saturating_sub(f.chars().count() + 1),
                sh.saturating_sub(1),
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
                self.screen
                    .fill(Rect::new(0, y, sw, 1), ' ', Style::new(f, b));
            }
            self.screen
                .put(1, y, if self.ascii { '#' } else { '■' }, color, b);
            self.screen
                .text_clip(3, y, row, sw - 4, Style::attr(f, b, a));
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

    pub(super) fn render_detail(&mut self) {
        let r = match self.selected {
            Some(r) => r,
            None => return,
        };
        let sw = self.screen.w;
        let sh = self.content_height();
        let bg = Rgb(14, 14, 20);
        let fg = Rgb(200, 200, 205);
        self.screen
            .fill(Rect::new(0, 0, sw, sh), ' ', Style::new(fg, bg));
        let width = sw.saturating_sub(4).max(20);
        let lines = detail::detail_lines(&self.world, r, width, self.ascii);
        let max_scroll = lines.len().saturating_sub(sh.saturating_sub(1));
        if self.detail_scroll > max_scroll {
            self.detail_scroll = max_scroll;
        }
        for (k, line) in lines.iter().skip(self.detail_scroll).take(sh).enumerate() {
            self.screen
                .text_clip(2, k, &line.text, width, Style::attr(line.fg, bg, line.attr));
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

    pub(super) fn render_chronicle(&mut self) {
        let sw = self.screen.w;
        let sh = self.content_height();
        let bg = Rgb(12, 12, 16);
        let fg = Rgb(200, 200, 205);
        self.screen
            .fill(Rect::new(0, 0, sw, sh), ' ', Style::new(fg, bg));
        self.chron_rows.clear();
        let filter = self.chron_filter.to_lowercase();
        let title = if filter.is_empty() {
            format!(
                " The Chronicle of the World — importance ≥{} ({}) ",
                self.chron_min,
                crate::sim::prose::count(self.world.chronicle.len() as i64, "entry")
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
        let need = avail.saturating_add(self.chron_scroll.min(1_000_000));
        let lines = self.chronicle_lines(self.chron_min, &filter, width, need);
        if self.chron_scroll > lines.len().saturating_sub(avail) {
            self.chron_scroll = lines.len().saturating_sub(avail);
        }
        if lines.is_empty() {
            self.screen
                .text(2, 2, "nothing matches", Rgb(120, 120, 130), bg);
            return;
        }
        let from = lines.split_at(self.chron_scroll.min(lines.len())).1;
        let mut y = sh - 1;
        for (l, c, a, idx) in Ui::chron_window(from, avail) {
            self.screen.text_attr(1, y, l, *c, bg, *a);
            self.chron_rows.push((y, *idx));
            if y == 1 {
                break;
            }
            y -= 1;
        }
    }

    pub(super) fn render_help(&mut self) {
        let sw = self.screen.w;
        let sh = self.content_height();
        let bg = Rgb(14, 14, 20);
        let fg = Rgb(200, 200, 205);
        self.screen
            .fill(Rect::new(0, 0, sw, sh), ' ', Style::new(fg, bg));
        // Folded to this terminal's width, then scrolled: the page is both
        // wider and taller than a small window, and the last row says where
        // in it the reader is rather than letting the text simply stop.
        let guide = self.mode == Mode::Guide;
        let lines = if guide {
            learn::guide_lines(sw.saturating_sub(3))
        } else {
            detail::help_lines(sw.saturating_sub(3))
        };
        let scroll = if guide {
            &mut self.guide_scroll
        } else {
            &mut self.help_scroll
        };
        let more = sh.saturating_sub(1).max(1);
        let top = (*scroll).min(lines.len().saturating_sub(more));
        *scroll = top;
        for (k, l) in lines.iter().skip(top).enumerate().take(more) {
            let body = if guide {
                l.chars().any(char::is_lowercase)
            } else {
                l.starts_with("  ")
            };
            let (c, a) = if body || l.is_empty() {
                (fg, 0)
            } else {
                (Rgb(230, 200, 120), BOLD)
            };
            self.screen
                .text_clip(2, k, l, sw.saturating_sub(3), Style::attr(c, bg, a));
        }
        let shown = top + more.min(lines.len().saturating_sub(top));
        let note = if lines.len() > more {
            format!(
                "-- {}-{} of {} · j k Ctrl-d Ctrl-u scroll · Esc back --",
                top + 1,
                shown,
                lines.len()
            )
        } else {
            "-- Esc back --".to_string()
        };
        self.screen.text_clip(
            2,
            sh.saturating_sub(1),
            &note,
            sw.saturating_sub(3),
            Style::attr(Rgb(140, 140, 150), bg, 0),
        );
    }

    /// A run of battles of one war reads as one line plus a count: the log
    /// is for noticing things, not for counting skirmishes.
    fn fold_battles(&self, lines: &mut Vec<(String, Rgb, u8, usize)>, width: usize) {
        let war_of = |idx: usize| -> Option<usize> {
            let e = &self.world.chronicle.events[idx];
            if e.kind != EventKind::Battle {
                return None;
            }
            e.refs.iter().find_map(|r| match r {
                Ref::War(x) => Some(*x),
                _ => None,
            })
        };
        let mut out: Vec<(String, Rgb, u8, usize)> = Vec::new();
        let mut i = 0;
        while i < lines.len() {
            let war = war_of(lines[i].3);
            let mut j = i + 1;
            if war.is_some() {
                while j < lines.len() && war_of(lines[j].3) == war && lines[j].3 != lines[i].3 {
                    j += 1;
                }
            }
            out.push(lines[i].clone());
            let folded = j - i - 1;
            if folded > 0 {
                let name = &self.world.wars[war.unwrap()].name;
                let text = format!(
                    "       and {} more battle{} of {}",
                    folded,
                    if folded == 1 { "" } else { "s" },
                    name
                );
                let text: String = text.chars().take(width + 7).collect();
                out.push((text, Rgb(150, 120, 110), DIM, lines[i].3));
            }
            i = j;
        }
        *lines = out;
    }

    /// The digest page: what has happened lately, in plain sentences.
    pub(super) fn render_recap(&mut self) {
        let sw = self.screen.w;
        let sh = self.content_height();
        let bg = Rgb(14, 14, 20);
        let fg = Rgb(200, 200, 205);
        self.screen
            .fill(Rect::new(0, 0, sw, sh), ' ', Style::new(fg, bg));
        let width = sw.saturating_sub(6).max(30);
        let lines = recap::lines(&self.world, self.recap_years, self.recap_scope, width, sh);
        let max_scroll = lines.len().saturating_sub(sh.saturating_sub(1));
        if self.recap_scroll > max_scroll {
            self.recap_scroll = max_scroll;
        }
        for (k, line) in lines.iter().skip(self.recap_scroll).take(sh).enumerate() {
            self.screen
                .text_clip(3, k, &line.text, width, Style::attr(line.fg, bg, line.attr));
        }
        if lines.len() > sh {
            let s = format!(
                " {}/{} ",
                self.recap_scroll + sh.min(lines.len()),
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

    /// The first-run card, over whatever is behind it.
    pub(super) fn render_tour(&mut self) {
        let sw = self.screen.w;
        let sh = self.screen.h;
        let inner = TOUR
            .iter()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(40)
            .min(sw.saturating_sub(6));
        let w = (inner + 6).min(sw);
        let lines: Vec<_> = TOUR
            .iter()
            .flat_map(|line| term::wrap(line, inner.max(4)))
            .collect();
        let h = (lines.len() + 4).min(sh);
        let x = (sw.saturating_sub(w)) / 2;
        let y = (sh.saturating_sub(h)) / 2;
        let bg = Rgb(26, 26, 38);
        let fg = Rgb(225, 225, 235);
        self.screen
            .fill(Rect::new(x, y, w, h), ' ', Style::new(fg, bg));
        self.screen.frame(
            Rect::new(x, y, w, h),
            "Rise and Fall of Empires",
            Style::new(Rgb(240, 210, 130), bg),
        );
        for (k, l) in lines.iter().enumerate() {
            if y + 2 + k >= y + h.saturating_sub(2) {
                break;
            }
            let (c, a) = if k == 0 {
                (Rgb(255, 230, 170), BOLD)
            } else if k + 1 == lines.len() {
                (Rgb(160, 210, 160), 0)
            } else {
                (fg, 0)
            };
            self.screen.text_clip(
                x + 3,
                y + 2 + k,
                l,
                w.saturating_sub(5),
                Style::attr(c, bg, a),
            );
        }
        self.screen.text_clip(
            x + 1,
            y + h.saturating_sub(2),
            "t tutorial p guide",
            w.saturating_sub(2),
            Style::new(Rgb(160, 210, 160), bg),
        );
    }

    pub(super) fn render_fate(&mut self) {
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
        self.screen
            .fill(Rect::new(x, y, w, h), ' ', Style::new(fg, bg));
        self.screen.frame(
            Rect::new(x, y, w, h),
            "The Hand of Fate",
            Style::new(Rgb(230, 200, 120), bg),
        );
        for (k, l) in lines.iter().enumerate() {
            let attr = if k == 0 { BOLD } else { 0 };
            self.screen.text_attr(x + 2, y + 1 + k, l, fg, bg, attr);
        }
    }
}
