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
        for l in term::wrap(&era_desc, tx).into_iter().take(1) {
            self.screen.text_clip(x, y, &l, tx, Rgb(95, 95, 105), bg, 0);
            y += 1;
        }
        y += 1;
        // Two counts to a line: the sidebar has better uses for the rows.
        let st = &w.stats;
        let lines = [
            format!("{} people", words::folk(st.pop as f32)),
            format!("{} realms · {} cities", st.polities_alive, st.cities_alive),
            format!(
                "{} peoples · {} schools · {} wars",
                st.cultures_alive, st.schools_alive, st.wars_active
            ),
        ];
        for l in lines.iter() {
            self.screen.text_clip(x, y, l, tx, fg, bg, 0);
            y += 1;
        }
        // What happened while the viewer was on another screen.
        if let Some(note) = self.since_note.clone() {
            self.screen
                .text_attr(x, y, "Since you last looked", Rgb(255, 220, 120), bg, BOLD);
            y += 1;
            for l in term::wrap(&note, tx).into_iter().take(3) {
                self.screen
                    .text_clip(x, y, &l, tx, Rgb(235, 215, 165), bg, 0);
                y += 1;
            }
            y += 1;
        }
        // Every block below wants room; share it out in order of what a
        // newcomer needs, so the last block is never starved by the first.
        let mut avail = mh.saturating_sub(y);
        let take = |avail: &mut usize, want: usize| -> usize {
            let got = want.min(*avail);
            *avail -= got;
            got
        };
        let here_need = take(&mut avail, 8);
        let sel_need = if self.selected.is_some() {
            take(&mut avail, 10)
        } else {
            0
        };
        let story_room = take(&mut avail, 5);
        // The great powers list runs to the bottom, so it only needs a
        // reserve here; anything left over goes to it.
        let powers_need = take(&mut avail, 5);
        let story_floor = y + story_room;
        // The storyteller.
        self.story_rows.clear();
        let stories = self.stories();
        if !stories.is_empty() && y + 4 < story_floor {
            self.screen.hline(x0 + 1, y, width - 1, Rgb(60, 60, 70), bg);
            y += 1;
            self.screen.text_attr(x, y, "Now", accent, bg, BOLD);
            y += 1;
            for (text, r) in stories {
                let color = detail::ref_color(&self.world, r);
                let lines = term::wrap(&text, tx.saturating_sub(2));
                for (k, l) in lines.into_iter().take(2).enumerate() {
                    if y >= story_floor {
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
        // One sentence first, so a newcomer never has to decode a column
        // of bare nouns; the particulars follow it.
        here.push((words::here_sentence(w, i), fg));
        if let Some(p) = cs.owner {
            here.push((w.ruler_short(p), w.polities[p].color));
        }
        if let Some(f) = w.terrain.feature_at(i) {
            if w.terrain.features[f].name.is_some() {
                here.push((w.terrain.features[f].display(), dim));
            }
        }
        here.push((
            format!(
                "mana {:.0}%  fertility {:.0}%",
                w.terrain.mana[i] * 100.0,
                w.terrain.fertility[i] * 100.0
            ),
            dim,
        ));
        let here_stop = (y + here_need).min(mh.saturating_sub(sel_need + powers_need).max(y + 3));
        for (s, c) in here {
            for l in term::wrap(&s, tx) {
                if y >= here_stop {
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
            let sel_stop = (y + sel_need).min(mh.saturating_sub(powers_need).max(y + 3));
            for (s, c) in detail::summary(w, r) {
                for l in term::wrap(&s, tx) {
                    if y >= sel_stop {
                        break;
                    }
                    self.screen.text_clip(x, y, &l, tx, c, bg, 0);
                    y += 1;
                }
            }
            y += 1;
        }
        // Great powers.
        if y + 2 < mh {
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
                let arrow = match words::trend(w, p) {
                    Some("rising") => {
                        if self.ascii {
                            "up  "
                        } else {
                            "↑   "
                        }
                    }
                    Some(_) => {
                        if self.ascii {
                            "down"
                        } else {
                            "↓   "
                        }
                    }
                    None => "    ",
                };
                let namew = tx.saturating_sub(13);
                let name: String = if pol.name.chars().count() > namew {
                    pol.name
                        .chars()
                        .take(namew.saturating_sub(1))
                        .chain(std::iter::once('…'))
                        .collect()
                } else {
                    pol.name.clone()
                };
                let s = format!("{} {:<w$} {:>4} {}", war, name, pol.cells, arrow, w = namew);
                self.screen.text_clip(x + 2, y, &s, tx - 2, fg, bg, 0);
                y += 1;
            }
        }
    }
}
