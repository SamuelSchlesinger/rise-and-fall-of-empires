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
impl super::super::Ui {
    /// How a realm stands, and the glyph that says which: either towards
    /// `from` when one is selected, or its own posture in the world when
    /// none is.
    ///
    /// The colours are the ones used everywhere else for these ideas — red
    /// for war, green for a sworn friend, violet for a marriage, blue for a
    /// tributary tie — so the layer needs learning once.
    fn relation_style(&self, from: Option<usize>, p: usize) -> (Rgb, Option<char>) {
        let w = &self.world;
        let ascii = self.ascii;
        let g = |uni: char, a: char| Some(if ascii { a } else { uni });
        let war = Rgb(200, 60, 55);
        let ally = Rgb(70, 190, 110);
        let wed = Rgb(180, 110, 210);
        let sworn_off = Rgb(205, 140, 60);
        let bound = Rgb(80, 130, 215);
        let calm = Rgb(95, 100, 110);
        let gold = Rgb(225, 190, 90);
        match from {
            // The world as one realm sees it.
            Some(me) => {
                // Gold alone for the realm being asked about. A glyph over
                // every cell it holds is five hundred marks on a screen, and
                // the political layer already says "this is the one you
                // picked" with brightness and nothing else.
                if me == p {
                    return (gold, None);
                }
                if w.war_between(me, p).is_some() {
                    return (war, g('\u{2715}', 'X'));
                }
                if w.polities[p].overlord == Some(me) {
                    return (bound, g('\u{25bc}', 'v'));
                }
                if w.polities[me].overlord == Some(p) {
                    return (bound, g('\u{25b2}', '^'));
                }
                match crate::sim::dynasty::stance_of(w, me, p) {
                    crate::sim::Stance::Allied => (ally, g('\u{2713}', '+')),
                    crate::sim::Stance::Married => (wed, g('\u{2740}', 'm')),
                    crate::sim::Stance::Rival => (sworn_off, g('\u{2260}', '!')),
                    crate::sim::Stance::Neutral => {
                        // Tension shades the neutrals, so the realms most
                        // likely to become the next war stand out from the
                        // ones that never think about each other.
                        let t = w.polities[me].tension.get(&p).copied().unwrap_or(0.0);
                        if t >= 0.35 {
                            (calm.mix(sworn_off, t.min(1.0)), None)
                        } else {
                            (calm, None)
                        }
                    }
                }
            }
            // Nothing selected: each realm's own standing in the world.
            None => {
                if crate::sim::war::hegemon(w) == Some(p) {
                    return (gold, g('\u{25c6}', '@'));
                }
                if w.polities[p].at_war() {
                    return (war, g('\u{2715}', 'X'));
                }
                if w.polities[p].overlord.is_some() {
                    return (bound, g('\u{25bc}', 'v'));
                }
                let stances = &w.polities[p].stance;
                if stances.values().any(|&s| s == crate::sim::Stance::Allied) {
                    return (ally, g('\u{2713}', '+'));
                }
                if stances.values().any(|&s| s == crate::sim::Stance::Married) {
                    return (wed, g('\u{2740}', 'm'));
                }
                let hottest = w.polities[p]
                    .tension
                    .values()
                    .copied()
                    .fold(0.0f32, f32::max);
                if hottest >= 0.35 {
                    (calm.mix(sworn_off, hottest.min(1.0)), None)
                } else {
                    (calm, None)
                }
            }
        }
    }
}

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
            // Relations needs the owner grid for the same reason political
            // does: it colours realms, so it has to know which realm each
            // cell belongs to. Leaving it out of this list was a silent
            // failure — every cell read as unheld and the whole layer drew
            // in one grey.
            Layer::Political | Layer::Relations => Some(self.owner_grid(true)),
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
        // The roads, rasterised once for the frame and only when they are
        // being drawn. A route is a pair of cities, not a path, so the line
        // between them has to be walked; the high bit records whether
        // anything is moving along it and the low bits how much, so that one
        // byte per cell carries both.
        // Where the chronicle says things happened, counted once for the
        // frame. Only built for the layer that draws it: walking sixty
        // thousand events every frame for a map nobody is looking at is
        // exactly what `stories()` was fixed for.
        let events: Vec<u32> = if self.layer == Layer::Memory {
            let mut v = vec![0u32; self.world.cells.len()];
            for e in &self.world.chronicle.events {
                if let Some(l) = e.loc.filter(|&l| l < v.len()) {
                    v[l] += 1 + u32::from(e.importance);
                }
            }
            v
        } else {
            Vec::new()
        };
        let roads: Vec<u8> = if self.layer == Layer::Trade {
            let mut v = vec![0u8; self.world.cells.len()];
            for r in &self.world.routes {
                let (ca, cb) = (self.world.cities[r.a].cell, self.world.cities[r.b].cell);
                if self.world.cities[r.a].destroyed.is_some()
                    || self.world.cities[r.b].destroyed.is_some()
                {
                    continue;
                }
                let weight = ((r.value * 6.0) as u32).min(0x7f) as u8;
                for cell in self.world.terrain.cells_between(ca, cb) {
                    let slot = &mut v[cell];
                    let carried = (*slot & 0x7f).saturating_add(weight).min(0x7f);
                    let open = (*slot & 0x80 != 0) || r.open;
                    *slot = carried | if open { 0x80 } else { 0 };
                }
            }
            v
        } else {
            Vec::new()
        };
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
                    // What the ground yields. `World::goods` has existed
                    // since trade was built and was never drawn, so the only
                    // way to learn that a stretch of country was salt land
                    // was to open every city on it one at a time.
                    Layer::Goods => {
                        if !water {
                            match self.world.goods.get(i).copied().flatten() {
                                Some(g) => {
                                    let (c, glyph) = good_style(g, self.ascii);
                                    bg = bg.mix(c, 0.32);
                                    fg = c;
                                    ch = glyph;
                                    attr |= BOLD;
                                }
                                // Ground that yields nothing worth carrying
                                // recedes, so the productive land reads as
                                // pattern rather than noise.
                                None => {
                                    bg = bg.scale(0.45);
                                    fg = bg.scale(1.5);
                                }
                            }
                        }
                    }
                    // What trade is worth and where it moves. The roads are
                    // the point: a realm's wealth is mostly a fact about
                    // what passes through it.
                    Layer::Trade => {
                        if !water {
                            let dev = cs.owner.map(|p| self.world.polities[p].dev).unwrap_or(0.0);
                            let warm = Rgb(30, 28, 34)
                                .mix(Rgb(230, 200, 120), (dev / 2.5).clamp(0.0, 1.0));
                            bg = bg.mix(warm, 0.7);
                            fg = bg.scale(1.3);
                            // The ground gives up its own glyph: on a layer
                            // about what moves, the network should be the
                            // only pattern on the screen. Cities are drawn
                            // after this and keep theirs.
                            ch = ' ';
                        }
                        if let Some(&carried) = roads.get(i) {
                            if carried > 0 {
                                // Brighter where more is carried; a road
                                // closed by war is drawn but dark, which is
                                // what makes a blockade legible.
                                let lit = (carried & 0x7f) as f32 / 40.0;
                                let open = carried & 0x80 != 0;
                                let road = if open {
                                    Rgb(255, 228, 150)
                                } else {
                                    Rgb(120, 70, 70)
                                };
                                bg = bg.mix(road, (0.25 + lit * 0.5).min(0.8));
                                fg = road;
                                // A busy road reads as a line, a quiet one
                                // as a track.
                                ch = match (open, lit > 0.5, self.ascii) {
                                    (true, true, false) => '━',
                                    (true, false, false) => '·',
                                    (false, _, false) => '×',
                                    (true, true, true) => '=',
                                    (true, false, true) => '-',
                                    (false, _, true) => 'x',
                                };
                                attr |= BOLD;
                            }
                        }
                    }
                    // What the land can feed: fertility, this century's
                    // weather and what its people know, multiplied. The one
                    // number that decides where anybody can live, and the
                    // only place the drifting climate field becomes visible.
                    Layer::Harvest => {
                        if !water {
                            let cap = (self.world.cell_capacity(i) / 6.0).clamp(0.0, 1.0);
                            let heat = Rgb(48, 30, 24)
                                .mix(Rgb(120, 170, 70), cap.sqrt())
                                .mix(Rgb(230, 240, 160), (cap - 0.65).max(0.0) * 2.4);
                            bg = bg.mix(heat, 0.85);
                            fg = bg.scale(1.3);
                        }
                    }
                    // What the ground remembers. Knowledge belongs to the
                    // land rather than the state, so this is the layer where
                    // a dark age looks like one: the bright core around old
                    // cities dims when the people who kept it leave.
                    Layer::Knowledge => {
                        if !water {
                            let y = self.world.cell_yield.get(i).copied().unwrap_or(1.0);
                            let k = ((y - 1.0) / 1.4).clamp(0.0, 1.0);
                            let lit = Rgb(26, 28, 44)
                                .mix(Rgb(90, 150, 230), k.sqrt())
                                .mix(Rgb(235, 245, 255), (k - 0.7).max(0.0) * 3.0);
                            bg = bg.mix(lit, 0.85);
                            fg = bg.scale(1.3);
                        }
                    }
                    // The motion layers. All four are signed or one-sided
                    // running totals rather than levels, so the question
                    // they answer is "what has been happening here", which
                    // no snapshot of the world can answer at all.
                    Layer::Settling => {
                        if !water {
                            let f = self.world.flows.settled.get(i).copied().unwrap_or(0.0);
                            // Scaled against the cell's own carrying
                            // capacity: a hundred people arriving on rich
                            // ground is nothing and on the tundra it is a
                            // migration.
                            let scale = self.world.cell_capacity(i).max(0.4);
                            let k = (f / scale).clamp(-1.0, 1.0);
                            let tint = if k >= 0.0 {
                                Rgb(60, 200, 120)
                            } else {
                                Rgb(210, 90, 70)
                            };
                            bg = Rgb(24, 24, 30).mix(tint, k.abs().sqrt());
                            fg = bg.scale(1.4);
                            if k.abs() > 0.25 {
                                ch = if k > 0.0 {
                                    if self.ascii {
                                        '+'
                                    } else {
                                        '▲'
                                    }
                                } else if self.ascii {
                                    '-'
                                } else {
                                    '▼'
                                };
                                attr |= BOLD;
                            }
                        }
                    }
                    Layer::Fighting => {
                        let f = self.world.flows.fought.get(i).copied().unwrap_or(0.0);
                        if f > 0.02 {
                            let k = (f / 3.0).clamp(0.0, 1.0);
                            bg = bg.mix(Rgb(200, 40, 40), 0.25 + k * 0.6);
                            fg = Rgb(255, 220, 190);
                            ch = if self.ascii { 'X' } else { '✕' };
                            attr |= BOLD;
                        } else if !water {
                            bg = bg.scale(0.4);
                            fg = bg.scale(1.5);
                            ch = ' ';
                        }
                    }
                    Layer::Frontier => {
                        if !water {
                            let churn = self.world.flows.changed.get(i).copied().unwrap_or(0.0);
                            // How long this ground has been held, beside how
                            // often it has changed hands lately. Old ground
                            // is calm blue, new ground is hot.
                            let held = (self.world.year - cs.since).max(0) as f32;
                            let fresh = (1.0 - held / 120.0).clamp(0.0, 1.0);
                            let k = (churn * 0.4 + fresh).clamp(0.0, 1.0);
                            bg = Rgb(28, 34, 54)
                                .mix(Rgb(120, 130, 180), 1.0 - k)
                                .mix(Rgb(255, 150, 60), k);
                            fg = bg.scale(1.35);
                            if churn > 1.5 {
                                ch = if self.ascii { '!' } else { '◆' };
                                attr |= BOLD;
                            }
                        }
                    }
                    Layer::Drift => {
                        if !water {
                            let d = self.world.climate_drift(i);
                            let k = (d / 0.25).clamp(-1.0, 1.0);
                            let tint = if k >= 0.0 {
                                // Wetting: the margins open up.
                                Rgb(70, 170, 220)
                            } else {
                                Rgb(220, 150, 60)
                            };
                            bg = Rgb(26, 26, 30).mix(tint, k.abs().sqrt());
                            fg = bg.scale(1.4);
                            if k.abs() > 0.35 {
                                ch = if k > 0.0 {
                                    if self.ascii {
                                        '~'
                                    } else {
                                        '≈'
                                    }
                                } else if self.ascii {
                                    ':'
                                } else {
                                    '∴'
                                };
                                attr |= BOLD;
                            }
                        }
                    }
                    // Who is bound to whom. With a realm selected this is
                    // the diplomatic world *from there* — the question a
                    // reader actually has is never "what are all the
                    // alliances" but "who would come in against me". With
                    // nothing selected it falls back to each realm's own
                    // posture, so the layer says something before anything
                    // is picked.
                    Layer::Relations => {
                        if !water {
                            let holder = borders
                                .as_ref()
                                .and_then(|(g, gw)| g[(sy + 1) * gw + sx + 1]);
                            ch = field_glyph(self.ascii, holder.is_some());
                            match holder {
                                Some(p) => {
                                    let (tint, mark) = self.relation_style(sel_polity, p);
                                    bg = bg.mix(tint, 0.6);
                                    fg = bg.mix(Rgb(255, 255, 255), 0.5);
                                    if let Some(g) = mark {
                                        ch = g;
                                        attr |= BOLD;
                                    }
                                }
                                None => bg = bg.scale(0.6),
                            }
                        }
                    }
                    // Where history happened. Built once for the frame from
                    // every event the chronicle still holds — which is only
                    // what compaction has spared, so this is the world's
                    // memory rather than its whole past, and the layer is
                    // named for that.
                    Layer::Memory => {
                        if let Some(&n) = events.get(i) {
                            if n > 0 {
                                let k = ((n as f32).ln_1p() / 3.5).clamp(0.0, 1.0);
                                bg = Rgb(20, 20, 28)
                                    .mix(Rgb(120, 110, 200), k.sqrt())
                                    .mix(Rgb(255, 240, 170), (k - 0.65).max(0.0) * 2.8);
                                fg = bg.scale(1.4);
                            } else if !water {
                                bg = bg.scale(0.35);
                                fg = bg.scale(1.5);
                                ch = ' ';
                            }
                        }
                    }
                    // How long the ground has been lived in. `since` is the
                    // year a cell last changed hands, which is the closest
                    // thing the world records to how settled a place is.
                    Layer::Settled => {
                        if !water {
                            if cs.pop > 0.02 {
                                let held = (self.world.year - cs.since).max(0) as f32;
                                let k = (held / 400.0).clamp(0.0, 1.0);
                                bg = Rgb(70, 40, 30)
                                    .mix(Rgb(200, 170, 90), k.sqrt())
                                    .mix(Rgb(235, 245, 225), (k - 0.7).max(0.0) * 3.0);
                                fg = bg.scale(1.35);
                            } else {
                                bg = bg.scale(0.3);
                                fg = bg.scale(1.5);
                                ch = ' ';
                            }
                        }
                    }
                    // Where the carrying trade has just been won or lost.
                    // Gold where a road has opened, dark red where a war has
                    // shut one — the corridor, not its two ends.
                    Layer::Carrying => {
                        let f = self.world.flows.carried.get(i).copied().unwrap_or(0.0);
                        if f.abs() > 0.02 {
                            let k = (f.abs() / 3.0).clamp(0.0, 1.0);
                            let tint = if f > 0.0 {
                                Rgb(240, 205, 110)
                            } else {
                                Rgb(190, 70, 60)
                            };
                            bg = bg.mix(tint, 0.25 + k * 0.6);
                            fg = bg.mix(Rgb(255, 255, 255), 0.5);
                            ch = if f > 0.0 {
                                if self.ascii {
                                    '+'
                                } else {
                                    '\u{25b8}'
                                }
                            } else if self.ascii {
                                'x'
                            } else {
                                '\u{00d7}'
                            };
                            attr |= BOLD;
                        } else if !water {
                            bg = bg.scale(0.4);
                            fg = bg.scale(1.5);
                            ch = ' ';
                        }
                    }
                    Layer::Terrain => {}
                }
                // A plague. It lasts three to six years, which at any speed
                // above a crawl is a flash of colour with no explanation —
                // so it carries a glyph as well as a wash, because a mark
                // reads at forty milliseconds and a hue does not.
                //
                // And the wash is held back on the layers whose own palette
                // already speaks green. On the settling layer green means
                // people arriving, so a plague — people dying — was washing
                // the ground in the colour of the opposite thing.
                if cs.plague > 0 && self.layer != Layer::Biomes {
                    // Sickly green, except on the layers whose own palette
                    // already speaks green: on the settling layer green
                    // means people *arriving*, so a plague was washing the
                    // ground in the colour of the opposite thing. There it
                    // takes a violet nothing else uses, rather than going
                    // unmarked.
                    let wash =
                        if matches!(self.layer, Layer::Goods | Layer::Harvest | Layer::Settling) {
                            Rgb(175, 90, 195)
                        } else {
                            Rgb(120, 200, 60)
                        };
                    bg = bg.mix(wash, 0.35);
                    // And a mark, because three to six years is forty
                    // milliseconds at the fastest speed, and a hue that
                    // brief reads as a flash with no explanation. Never over
                    // a standing city, which keeps its own glyph; the wash
                    // carries it there.
                    let in_town = cs
                        .city
                        .is_some_and(|c| self.world.cities[c].destroyed.is_none());
                    if !in_town {
                        ch = if self.ascii { '&' } else { '†' };
                        fg = bg.mix(Rgb(255, 255, 255), 0.55);
                        attr |= BOLD;
                    }
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
                    // Reverse video is not a cursor. It swaps the cell's own
                    // two colours, and on a shaded map those are often
                    // nearly the same: deep water is drawn in (5, 17, 41) on
                    // (4, 13, 31), eighteen apart out of seven hundred and
                    // sixty-five, and better than a quarter of the map is
                    // within a fifth of that. Swapping them changes nothing
                    // a player can see, so the cursor simply disappeared
                    // over open sea and over any flat stretch of country.
                    //
                    // So the pair is forced apart instead of exchanged:
                    // white behind a dark map and near-black behind a light
                    // one, with the glyph in the other. No terrain colour
                    // can hide that, and because both are ordinary colours
                    // the themes re-map them along with everything else.
                    let dark = Rgb(14, 14, 18);
                    let light = Rgb(255, 255, 255);
                    let (behind, front) = if bg.luma() > 0.45 {
                        (dark, light)
                    } else {
                        (light, dark)
                    };
                    bg = behind;
                    fg = front;
                    attr |= BOLD;
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
