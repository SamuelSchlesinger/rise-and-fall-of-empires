//! The sidebar: the year and era, world statistics, the storyteller, what
//! lies under the cursor, the selection and the great powers.

use crate::ui::*;

impl Ui {
    pub(super) fn render_sidebar(&mut self) {
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
}
