//! The world map: terrain colouring, the overlay layers, cities, event
//! markers and the scroll hints.

use crate::ui::*;

/// A cell's neighbour to the north belongs to somebody else.
pub(crate) const EDGE_N: u8 = 1;
/// ...to the east.
pub(crate) const EDGE_E: u8 = 2;
/// ...to the south.
pub(crate) const EDGE_S: u8 = 4;
/// ...to the west.
pub(crate) const EDGE_W: u8 = 8;

/// Which sides of the cell at (`x`, `y`) face a different owner.
///
/// `grid` is row-major and `gw` wide; each entry is whoever holds that cell
/// (a realm, a people — anything comparable), or `None` for nobody. A
/// neighbour off the edge of the grid is never a border, so a caller that
/// pads the grid with a repeat of the rim gets no fence drawn around the
/// world itself.
pub(crate) fn owner_edges(grid: &[Option<usize>], gw: usize, x: usize, y: usize) -> u8 {
    if gw == 0 {
        return 0;
    }
    let gh = grid.len() / gw;
    if x >= gw || y >= gh {
        return 0;
    }
    let me = grid[y * gw + x];
    let at = |x: isize, y: isize| -> Option<Option<usize>> {
        if x < 0 || y < 0 || x >= gw as isize || y >= gh as isize {
            None
        } else {
            Some(grid[y as usize * gw + x as usize])
        }
    };
    let (x, y) = (x as isize, y as isize);
    let mut bits = 0u8;
    if matches!(at(x, y - 1), Some(o) if o != me) {
        bits |= EDGE_N;
    }
    if matches!(at(x + 1, y), Some(o) if o != me) {
        bits |= EDGE_E;
    }
    if matches!(at(x, y + 1), Some(o) if o != me) {
        bits |= EDGE_S;
    }
    if matches!(at(x - 1, y), Some(o) if o != me) {
        bits |= EDGE_W;
    }
    bits
}

/// The glyph that draws those edges inside a single character cell, or
/// `None` where the cell is well inside its owner's land.
///
/// One character cannot show four separate edges, so this draws the shape
/// they make: a rule along a lone edge, a corner where two meet, and a
/// crossing where the land narrows to a point with three or four sides
/// exposed. Under `--ascii` the same shapes come out as `- | +`.
pub(crate) fn border_glyph(bits: u8, ascii: bool) -> Option<char> {
    if bits == 0 {
        return None;
    }
    let (n, e, s, w) = (
        bits & EDGE_N != 0,
        bits & EDGE_E != 0,
        bits & EDGE_S != 0,
        bits & EDGE_W != 0,
    );
    let ch = match (n, e, s, w) {
        (true, false, false, true) => '┌',
        (true, true, false, false) => '┐',
        (false, false, true, true) => '└',
        (false, true, true, false) => '┘',
        (_, false, _, false) => '─',
        (false, _, false, _) => '│',
        _ => '┼',
    };
    Some(if ascii {
        match ch {
            '─' => '-',
            '│' => '|',
            _ => '+',
        }
    } else {
        ch
    })
}

/// The neutral fill the political and culture layers put under their
/// colours, in place of the terrain glyph: land held by somebody, land held
/// by nobody, and open water.
pub(crate) fn field_glyph(ascii: bool, held: bool) -> char {
    match (held, ascii) {
        (false, _) => ' ',
        (true, true) => '.',
        (true, false) => '·',
    }
}

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

    /// Who holds each character cell of the pane, plus a ring of one cell
    /// around it, so [`owner_edges`] can look at every neighbour.
    ///
    /// The ring repeats the rim of the world rather than reading `None` off
    /// the end of it: the edge of the map is not a frontier.
    fn owner_grid(&self, political: bool) -> (Vec<Option<usize>>, usize) {
        let (_, _, mw, mh) = self.map_rect();
        let (padx, pady) = self.view_pad();
        let (ox, oy) = self.view;
        let (tw, th) = (self.world.terrain.w, self.world.terrain.h);
        let z = self.zoom;
        let (gw, gh) = (mw + 2, mh + 2);
        let mut grid = vec![None; gw * gh];
        let axis = |s: isize, pad: usize, o: usize, len: usize| -> usize {
            let v = o as isize + (s - pad as isize) * z as isize;
            v.clamp(0, len as isize - 1) as usize
        };
        for gy in 0..gh {
            let y0 = axis(gy as isize - 1, pady, oy, th);
            for gx in 0..gw {
                let x0 = axis(gx as isize - 1, padx, ox, tw);
                grid[gy * gw + gx] = self.block_holder(x0, y0, political);
            }
        }
        (grid, gw)
    }

    /// Whoever holds most of the `zoom`×`zoom` block at (`x0`, `y0`): the
    /// realm, on the political layer, or the people on the culture one.
    ///
    /// Only dry land counts. A realm's coastal waters are still its own, but
    /// drawing them would rub out the coastline, which is the one edge of a
    /// realm the eye already knows.
    fn block_holder(&self, x0: usize, y0: usize, political: bool) -> Option<usize> {
        let (tw, th) = (self.world.terrain.w, self.world.terrain.h);
        let z = self.zoom;
        let mut best: Option<(usize, usize)> = None;
        let mut tally: Vec<(usize, usize)> = Vec::new();
        for y in y0..(y0 + z).min(th) {
            for x in x0..(x0 + z).min(tw) {
                if self.world.terrain.biome[y * tw + x].is_water() {
                    continue;
                }
                let cs = &self.world.cells[y * tw + x];
                let who = if political { cs.owner } else { cs.culture };
                let Some(who) = who else { continue };
                let n = match tally.iter_mut().find(|(k, _)| *k == who) {
                    Some(e) => {
                        e.1 += 1;
                        e.1
                    }
                    None => {
                        tally.push((who, 1));
                        1
                    }
                };
                if best.map_or(true, |(_, c)| n > c) {
                    best = Some((who, n));
                }
            }
        }
        best.map(|(who, _)| who)
    }

    /// The grid itself: one character per zoom block, coloured by the
    /// current layer and overlaid with rivers, cities, borders, event
    /// markers and the cursor.
    fn draw_cells(&mut self, marks: &[(usize, u8)]) {
        let (mx, my, mw, mh) = self.map_rect();
        let (padx, pady) = self.view_pad();
        let (ox, oy) = self.view;
        let tw = self.world.terrain.w;
        let th = self.world.terrain.h;
        let borders = match self.layer {
            Layer::Political => Some(self.owner_grid(true)),
            Layer::Culture => Some(self.owner_grid(false)),
            _ => None,
        };
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
        for sy in pady..mh {
            let y0 = oy + (sy - pady) * z;
            if y0 >= th {
                break;
            }
            for sx in padx..mw {
                let x0 = ox + (sx - padx) * z;
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
                    // Ownership, not terrain in other colours: a plain field
                    // of dots under the realm's colour, ruled off from its
                    // neighbours, so the frontiers read in black and white.
                    Layer::Political => {
                        let holder = borders
                            .as_ref()
                            .and_then(|(g, gw)| g[(sy + 1) * gw + sx + 1]);
                        if !water {
                            ch = field_glyph(self.ascii, holder.is_some());
                            match holder {
                                Some(p) => {
                                    let pc = self.world.polities[p].color;
                                    let mut k = 0.55;
                                    if let Some(sp) = sel_polity {
                                        if sp == p {
                                            k += 0.2;
                                        } else {
                                            k *= 0.55;
                                        }
                                    }
                                    bg = bg.mix(pc, k);
                                    fg = bg.mix(Rgb(255, 255, 255), 0.45);
                                }
                                None => bg = bg.scale(0.75),
                            }
                            if self.world.terrain.river[i] >= 1 {
                                ch = if self.ascii { '~' } else { '≈' };
                                fg = Rgb(120, 170, 240);
                            }
                            // Only on the held side of the seam: an outline
                            // in the realm's own colour, wilderness plain.
                            if let (Some(p), Some((grid, gw))) = (holder, &borders) {
                                let bits = owner_edges(grid, *gw, sx + 1, sy + 1);
                                if let Some(b) = border_glyph(bits, self.ascii) {
                                    ch = b;
                                    fg = self.world.polities[p].color.mix(Rgb(255, 255, 255), 0.6);
                                    attr = BOLD;
                                }
                            }
                        }
                    }
                    Layer::Culture => {
                        let holder = borders
                            .as_ref()
                            .and_then(|(g, gw)| g[(sy + 1) * gw + sx + 1]);
                        if !water {
                            ch = field_glyph(self.ascii, holder.is_some());
                            match holder {
                                Some(c) => {
                                    let cc = self.world.cultures[c].color;
                                    let mut k = (0.35 + cs.pop / max_pop * 0.45).min(0.8);
                                    if let Some(sc) = sel_culture {
                                        if sc == c {
                                            k = 0.9;
                                        } else {
                                            k *= 0.45;
                                        }
                                    }
                                    bg = bg.mix(cc, k);
                                    fg = bg.mix(Rgb(255, 255, 255), 0.45);
                                }
                                None => bg = bg.scale(0.75),
                            }
                            if let (Some(c), Some((grid, gw))) = (holder, &borders) {
                                let bits = owner_edges(grid, *gw, sx + 1, sy + 1);
                                if let Some(b) = border_glyph(bits, self.ascii) {
                                    ch = b;
                                    fg = self.world.cultures[c].color.mix(Rgb(255, 255, 255), 0.6);
                                    attr = BOLD;
                                }
                            }
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
                // Only where the wastes are drawn as themselves: on the
                // political and culture layers the glyph belongs to a realm
                // or a border, and must keep its colour.
                if self.world.terrain.biome[i] == Biome::Wastes
                    && matches!(
                        self.layer,
                        Layer::Terrain | Layer::Biomes | Layer::Magic | Layer::Population
                    )
                {
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
        let (padx, pady) = self.view_pad();
        let (ox, oy) = self.view;
        let tw = self.world.terrain.w;
        let th = self.world.terrain.h;
        let z = self.zoom;
        let hint = Rgb(180, 180, 180);
        let dark = Rgb(20, 20, 24);
        // Only along the edges the world actually reaches: when it is
        // smaller than the pane it sits centred, with blank margins that
        // nothing carries on into.
        let rows: Vec<usize> = (pady..mh.saturating_sub(pady)).step_by(4).collect();
        let cols: Vec<usize> = (padx..mw.saturating_sub(padx)).step_by(8).collect();
        if ox > 0 {
            for &sy in &rows {
                self.screen.put(mx + padx, my + sy, '<', hint, dark);
            }
        }
        if ox + (mw - padx) * z < tw {
            for &sy in &rows {
                self.screen.put(mx + mw - 1, my + sy, '>', hint, dark);
            }
        }
        if oy > 0 {
            for &sx in &cols {
                self.screen.put(mx + sx, my + pady, '^', hint, dark);
            }
        }
        if oy + (mh - pady) * z < th {
            for &sx in &cols {
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
        let (padx, pady) = self.view_pad();
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
            let (sx, sy) = (cx - ox + padx, cy - oy + pady);
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
        self.screen.fill(
            Rect::new(mx, y, mw, 1),
            ' ',
            Style::new(Rgb(170, 170, 180), bg),
        );
        self.screen
            .text_attr(mx + 1, y, self.layer.name(), Rgb(235, 205, 130), bg, BOLD);
        let x = mx + 2 + self.layer.name().chars().count();
        let room = mw.saturating_sub(x - mx + 1);
        let text = words::legend(self.layer, self.ascii, room);
        self.screen
            .text_clip(x, y, &text, room, Style::new(Rgb(165, 165, 178), bg));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A grid written as rows of characters: `.` is nobody's land, any other
    /// character is a holder of its own.
    fn grid(rows: &[&str]) -> (Vec<Option<usize>>, usize) {
        let gw = rows[0].chars().count();
        let mut g = Vec::new();
        for r in rows {
            assert_eq!(r.chars().count(), gw);
            for c in r.chars() {
                g.push(if c == '.' { None } else { Some(c as usize) });
            }
        }
        (g, gw)
    }

    /// The whole grid as it would be drawn, for reading a case at a glance.
    fn drawn(rows: &[&str], ascii: bool) -> Vec<String> {
        let (g, gw) = grid(rows);
        (0..g.len() / gw)
            .map(|y| {
                (0..gw)
                    .map(|x| border_glyph(owner_edges(&g, gw, x, y), ascii).unwrap_or(' '))
                    .collect()
            })
            .collect()
    }

    #[test]
    fn one_realm_has_no_inner_borders() {
        let (g, gw) = grid(&["aaa", "aaa", "aaa"]);
        assert_eq!(owner_edges(&g, gw, 1, 1), 0);
        assert_eq!(border_glyph(0, false), None);
    }

    #[test]
    fn the_rim_of_the_grid_is_not_a_frontier() {
        // Neighbours off the grid are ignored, so a caller that pads with a
        // repeat of the rim gets no fence drawn around the world itself.
        let (g, gw) = grid(&["aa", "aa"]);
        assert_eq!(owner_edges(&g, gw, 0, 0), 0);
        assert_eq!(owner_edges(&g, gw, 9, 9), 0);
    }

    #[test]
    fn a_neighbour_of_another_realm_is_an_edge() {
        let (g, gw) = grid(&["aab", "aab", "aab"]);
        assert_eq!(owner_edges(&g, gw, 1, 1), EDGE_E);
        assert_eq!(owner_edges(&g, gw, 2, 1), EDGE_W);
        assert_eq!(border_glyph(EDGE_E, false), Some('│'));
    }

    #[test]
    fn unclaimed_land_is_a_frontier_too() {
        let (g, gw) = grid(&["aa.", "aa.", "..."]);
        assert_eq!(owner_edges(&g, gw, 1, 1), EDGE_E | EDGE_S);
        assert_eq!(border_glyph(EDGE_E | EDGE_S, false), Some('┘'));
    }

    #[test]
    fn corners_and_crossings_get_their_own_glyphs() {
        assert_eq!(border_glyph(EDGE_N | EDGE_W, false), Some('┌'));
        assert_eq!(border_glyph(EDGE_N | EDGE_E, false), Some('┐'));
        assert_eq!(border_glyph(EDGE_S | EDGE_W, false), Some('└'));
        assert_eq!(border_glyph(EDGE_S | EDGE_E, false), Some('┘'));
        assert_eq!(border_glyph(EDGE_N, false), Some('─'));
        assert_eq!(border_glyph(EDGE_N | EDGE_S, false), Some('─'));
        assert_eq!(border_glyph(EDGE_E | EDGE_W, false), Some('│'));
        assert_eq!(border_glyph(EDGE_N | EDGE_E | EDGE_W, false), Some('┼'));
        assert_eq!(border_glyph(0xf, false), Some('┼'));
    }

    #[test]
    fn ascii_draws_the_same_shapes_in_ascii() {
        assert_eq!(border_glyph(EDGE_N, true), Some('-'));
        assert_eq!(border_glyph(EDGE_W, true), Some('|'));
        assert_eq!(border_glyph(EDGE_N | EDGE_W, true), Some('+'));
        for bits in 1..16u8 {
            assert!(border_glyph(bits, true).unwrap().is_ascii(), "{}", bits);
        }
    }

    /// Two realms side by side come out as a rule down the seam, drawn from
    /// both sides of it: that is what makes a frontier visible with the
    /// colour turned off.
    #[test]
    fn two_realms_are_ruled_apart() {
        assert_eq!(
            drawn(&["aabb", "aabb", "aabb"], true),
            [" || ", " || ", " || "]
        );
    }

    /// A realm alone in the wilderness is outlined all the way round, coast
    /// included. (The map draws the outline only on the held side of the
    /// seam, so the wilderness itself stays blank.)
    #[test]
    fn a_lone_realm_is_outlined() {
        assert_eq!(
            drawn(&["....", ".aa.", "...."], true),
            [" -- ", "|++|", " -- "]
        );
    }

    #[test]
    fn the_field_is_neutral_but_present() {
        assert_eq!(field_glyph(false, true), '·');
        assert_eq!(field_glyph(true, true), '.');
        assert_eq!(field_glyph(false, false), ' ');
        assert_eq!(field_glyph(true, false), ' ');
    }
}
