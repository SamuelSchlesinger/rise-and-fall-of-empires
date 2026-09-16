//! The event log under the map and the full-screen panels: lists, detail
//! pages, the chronicle, the help screen and the Hand of Fate.

use crate::ui::*;

impl Ui {
    pub(super) fn render_log(&mut self) {
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
        let mut lines = self.chronicle_lines(self.log_min, "", width, avail);
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

    pub(super) fn render_list(&mut self) {
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

    pub(super) fn render_detail(&mut self) {
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

    pub(super) fn render_chronicle(&mut self) {
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

    pub(super) fn render_help(&mut self) {
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
        self.screen.fill(x, y, w, h, ' ', fg, bg);
        self.screen
            .frame(x, y, w, h, "The Hand of Fate", Rgb(230, 200, 120), bg);
        for (k, l) in lines.iter().enumerate() {
            let attr = if k == 0 { BOLD } else { 0 };
            self.screen.text_attr(x + 2, y + 1 + k, l, fg, bg, attr);
        }
    }
}
