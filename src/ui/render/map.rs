//! The world map: terrain colouring, the overlay layers, cities, event
//! markers and the scroll hints.

use crate::ui::*;

impl Ui {
    fn terrain_style(&self, i: usize) -> (char, Rgb, Rgb) {
        let t = &self.world.terrain;
        let b = t.biome[i];
        let hn = t.height_norm(i);
        let (mut bg, glyph) = biome_style(b, self.ascii);
        match b {
            Biome::DeepOcean | Biome::Ocean | Biome::Shallows => {
                let depth = ((t.sea - t.elev[i]) / t.sea).clamp(0.0, 1.0);
                bg = bg.scale(1.15 - depth * 0.6);
            }
            Biome::Mountain | Biome::Peak | Biome::Hills => {
                bg = bg.mix(Rgb(235, 235, 235), (hn - 0.35).max(0.0) * 0.8);
            }
            _ => {
                bg = bg.scale(0.85 + hn * 0.6);
            }
        }
        let mut fg = bg.scale(1.35);
        let mut ch = glyph;
        if t.river[i] >= 1 && !b.is_water() {
            ch = if self.ascii { '~' } else { '≈' };
            fg = Rgb(70, 130, 220);
            if t.river[i] >= 3 {
                fg = Rgb(90, 160, 250);
            }
        }
        (ch, fg, bg)
    }

    pub(super) fn render_map(&mut self) {
        let (mx, my, mw, mh) = self.map_rect();
        let marks = self.recent_marks();
        self.draw_cells(&marks);
        self.draw_scroll_hints(mx, my, mw, mh);
        // Labels last: a scroll hint may hide behind a name, but a name
        // must never be broken by one.
        self.render_map_labels(mx, my, mw, mh);
        self.render_legend(mx, my, mw, mh);
    }

    /// Where the last two years' notable events happened, for the markers
    /// the map stamps over the terrain.
    fn recent_marks(&self) -> Vec<(usize, u8)> {
        let mut marks: Vec<(usize, u8)> = Vec::new();
        let year = self.world.year;
        for e in self.world.chronicle.events.iter().rev() {
            if e.year < year - 1 {
                break;
            }
            if e.importance >= 2 {
                if let Some(l) = e.loc {
                    marks.push((l, e.importance));
                }
            }
        }
        marks
    }

    /// The grid itself: one character per zoom block, coloured by the
    /// current layer and overlaid with rivers, cities, borders, event
    /// markers and the cursor.
    fn draw_cells(&mut self, marks: &[(usize, u8)]) {
        let (mx, my, mw, mh) = self.map_rect();
        let (ox, oy) = self.view;
        let tw = self.world.terrain.w;
        let th = self.world.terrain.h;
        let sel_polity = match self.selected {
            Some(Ref::Polity(p)) => Some(p),
            _ => None,
        };
        let sel_culture = match self.selected {
            Some(Ref::Culture(c)) => Some(c),
            _ => None,
        };
        let sel_school = match self.selected {
            Some(Ref::School(s)) => Some(s),
            _ => None,
        };
        let max_pop = 8.0f32;
        let z = self.zoom;
        for sy in 0..mh {
            let y0 = oy + sy * z;
            if y0 >= th {
                break;
            }
            for sx in 0..mw {
                let x0 = ox + sx * z;
                if x0 >= tw {
                    break;
                }
                // Representative cell of the z×z block: a city if there is one, else the centre.
                let mut i = (y0 + (z / 2).min(th - 1 - y0)) * tw + (x0 + (z / 2).min(tw - 1 - x0));
                let mut has_cursor = false;
                let mut mark: Option<u8> = None;
                if z > 1 {
                    let mut best_city = false;
                    for yy in y0..(y0 + z).min(th) {
                        for xx in x0..(x0 + z).min(tw) {
                            let j = yy * tw + xx;
                            if (xx, yy) == self.cursor {
                                has_cursor = true;
                            }
                            if let Some(c) = self.world.cells[j].city {
                                if self.world.cities[c].destroyed.is_none() && !best_city {
                                    i = j;
                                    best_city = true;
                                }
                            }
                            for &(l, imp) in marks {
                                if l == j {
                                    mark = Some(mark.unwrap_or(0).max(imp));
                                }
                            }
                        }
                    }
                } else {
                    has_cursor = (x0, y0) == self.cursor;
                    for &(l, imp) in marks {
                        if l == i {
                            mark = Some(mark.unwrap_or(0).max(imp));
                        }
                    }
                }
                let (mut ch, mut fg, mut bg) = self.terrain_style(i);
                let cs = &self.world.cells[i];
                let water = self.world.terrain.biome[i].is_water();
                let mut attr = 0u8;
                match self.layer {
                    Layer::Political => {
                        if let Some(p) = cs.owner {
                            let pc = self.world.polities[p].color;
                            let border = self
                                .world
                                .terrain
                                .neighbors4(i)
                                .any(|nb| self.world.cells[nb].owner != Some(p));
                            let mut k = if border { 0.8 } else { 0.42 };
                            if let Some(sp) = sel_polity {
                                if sp == p {
                                    k += 0.15;
                                } else {
                                    k *= 0.5;
                                }
                            }
                            bg = bg.mix(pc, k);
                            fg = bg.scale(1.3);
                            if self.world.terrain.river[i] >= 1 {
                                fg = Rgb(120, 170, 240);
                            }
                        }
                    }
                    Layer::Culture => {
                        if let Some(c) = cs.culture {
                            let cc = self.world.cultures[c].color;
                            let mut k = (0.25 + cs.pop / max_pop * 0.5).min(0.75);
                            if let Some(sc) = sel_culture {
                                if sc == c {
                                    k = 0.85;
                                } else {
                                    k *= 0.4;
                                }
                            }
                            bg = bg.mix(cc, k);
                            fg = bg.scale(1.3);
                        }
                    }
                    Layer::Magic => {
                        let m = self.world.terrain.mana[i];
                        let purple = Rgb(170, 60, 230);
                        if !water {
                            bg = bg.mix(purple, (m * m) * 0.9);
                        }
                        if let Some(p) = cs.owner {
                            if let Some(s) = self.world.polities[p].school {
                                let sc = self.world.schools[s].color;
                                let infl = self.world.schools[s]
                                    .influence
                                    .get(&p)
                                    .copied()
                                    .unwrap_or(0.0);
                                let mut k = 0.15 + infl * 0.35;
                                if sel_school == Some(s) {
                                    k += 0.3;
                                }
                                bg = bg.mix(sc, k);
                            }
                        }
                        fg = bg.scale(1.3);
                    }
                    Layer::Population => {
                        if !water {
                            let p = (cs.pop / max_pop).clamp(0.0, 1.0);
                            let heat = Rgb(40, 40, 50)
                                .mix(Rgb(255, 210, 60), p.sqrt())
                                .mix(Rgb(255, 60, 40), (p - 0.6).max(0.0) * 2.0);
                            bg = bg.mix(heat, 0.85);
                            fg = bg.scale(1.3);
                        }
                    }
                    Layer::Biomes => {
                        let (flat, g) = biome_style(self.world.terrain.biome[i], self.ascii);
                        bg = flat;
                        fg = flat.scale(1.4);
                        ch = g;
                    }
                    Layer::Terrain => {}
                }
                if cs.plague > 0 && self.layer != Layer::Biomes {
                    bg = bg.mix(Rgb(120, 200, 60), 0.35);
                }
                // Cities.
                if let Some(c) = cs.city {
                    let city = &self.world.cities[c];
                    if city.destroyed.is_none() {
                        let is_cap = city
                            .polity
                            .map(|p| self.world.polities[p].capital == Some(c))
                            .unwrap_or(false);
                        ch = if is_cap { '@' } else { '#' };
                        fg = if bg.luma() > 0.5 {
                            Rgb(10, 10, 10)
                        } else {
                            Rgb(255, 255, 255)
                        };
                        attr = BOLD;
                        if self.layer == Layer::Magic {
                            let is_home = self
                                .world
                                .schools
                                .iter()
                                .any(|s| s.alive() && s.home_city == c);
                            if is_home {
                                ch = '*';
                                fg = Rgb(255, 240, 120);
                            }
                        }
                    } else {
                        ch = if self.ascii { 'x' } else { '×' };
                        fg = Rgb(120, 110, 100);
                    }
                }
                if self.world.terrain.biome[i] == Biome::Wastes {
                    fg = Rgb(200, 120, 220);
                }
                // Event markers.
                if let Some(imp) = mark {
                    ch = '!';
                    fg = if imp >= 3 {
                        Rgb(255, 80, 80)
                    } else {
                        Rgb(255, 220, 80)
                    };
                    attr = BOLD;
                }
                if has_cursor {
                    attr |= REVERSE;
                }
                self.screen.put_attr(mx + sx, my + sy, ch, fg, bg, attr);
            }
        }
    }

    /// Arrows along the edges the map carries on past.
    fn draw_scroll_hints(&mut self, mx: usize, my: usize, mw: usize, mh: usize) {
        let (ox, oy) = self.view;
        let tw = self.world.terrain.w;
        let th = self.world.terrain.h;
        let z = self.zoom;
        let hint = Rgb(180, 180, 180);
        let dark = Rgb(20, 20, 24);
        if ox > 0 {
            for sy in (0..mh).step_by(4) {
                self.screen.put(mx, my + sy, '<', hint, dark);
            }
        }
        if ox + mw * z < tw {
            for sy in (0..mh).step_by(4) {
                self.screen.put(mx + mw - 1, my + sy, '>', hint, dark);
            }
        }
        if oy > 0 {
            for sx in (0..mw).step_by(8) {
                self.screen.put(mx + sx, my, '^', hint, dark);
            }
        }
        if oy + mh * z < th {
            for sx in (0..mw).step_by(8) {
                self.screen.put(mx + sx, my + mh - 1, 'v', hint, dark);
            }
        }
    }

    /// Write each sizeable realm's short name beside its capital, where it
    /// fits without covering another label, a city or the sidebar. Only at
    /// zoom 1: closer in there is no room, further out the names collide.
    fn render_map_labels(&mut self, mx: usize, my: usize, mw: usize, mh: usize) {
        if self.zoom != 1 || mw < 40 {
            return;
        }
        let (ox, oy) = self.view;
        let tw = self.world.terrain.w;
        // Biggest realms first, so the small fry give way to the great.
        let mut ps: Vec<usize> = self.world.living_polities();
        ps.sort_by_key(|&p| std::cmp::Reverse(self.world.polities[p].cells));
        // Which columns of each row a label has already taken.
        let mut taken: Vec<Vec<bool>> = vec![vec![false; mw]; mh];
        for &p in &ps {
            let pol = &self.world.polities[p];
            if pol.cells < 12 {
                continue;
            }
            let cell = match self.world.capital_cell(p) {
                Some(c) => c,
                None => continue,
            };
            let (cx, cy) = (cell % tw, cell / tw);
            if cx < ox || cy < oy {
                continue;
            }
            let (sx, sy) = (cx - ox, cy - oy);
            if sx >= mw || sy >= mh {
                continue;
            }
            let label = format!(" {}", pol.short);
            let n = label.chars().count();
            // Right of the capital, and never over the sidebar edge.
            let x0 = sx + 1;
            if x0 + n + 1 > mw {
                continue;
            }
            if (x0..x0 + n + 1).any(|x| taken[sy][x]) {
                continue;
            }
            taken[sy][x0..x0 + n + 1].fill(true);
            let color = pol.color;
            let fg = color.mix(Rgb(255, 255, 255), 0.55);
            for (k, ch) in label.chars().enumerate() {
                let bg = self.screen.cell(mx + x0 + k, my + sy).bg.scale(0.45);
                self.screen.put_attr(mx + x0 + k, my + sy, ch, fg, bg, BOLD);
            }
        }
    }

    /// The one-line key to the layer, along the bottom of the map.
    fn render_legend(&mut self, mx: usize, my: usize, mw: usize, mh: usize) {
        if !self.show_legend || mh < 6 || mw < 30 {
            return;
        }
        let y = my + mh - 1;
        let bg = Rgb(24, 24, 30);
        let text = words::legend(self.layer, self.ascii);
        self.screen.fill(
            Rect::new(mx, y, mw, 1),
            ' ',
            Style::new(Rgb(170, 170, 180), bg),
        );
        self.screen
            .text_attr(mx + 1, y, self.layer.name(), Rgb(235, 205, 130), bg, BOLD);
        let x = mx + 2 + self.layer.name().chars().count();
        self.screen.text_clip(
            x,
            y,
            &text,
            mw.saturating_sub(x - mx + 1),
            Style::new(Rgb(165, 165, 178), bg),
        );
    }
}
