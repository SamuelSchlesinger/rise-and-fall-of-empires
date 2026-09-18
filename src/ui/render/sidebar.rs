//! The sidebar: the year and era, world statistics, the storyteller, what
//! lies under the cursor, the selection, and the key to the colours on
//! screen.

use crate::ui::*;

/// The mark that says a line was cut short.
fn ellipsis(ascii: bool) -> &'static str {
    // `…` is one character in the frame and `--ascii` would flatten it to a
    // bare full stop, which reads as the end of a sentence rather than the
    // middle of one. So ASCII gets three dots of its own.
    if ascii {
        "..."
    } else {
        "…"
    }
}

/// `line` with the mark on the end, shortened to fit `width`.
fn mark_cut(line: &str, width: usize, ascii: bool) -> String {
    let mark = ellipsis(ascii);
    let n = mark.chars().count();
    let mut s: String = line.chars().take(width.saturating_sub(n)).collect();
    while s.ends_with(' ') {
        s.pop();
    }
    s.push_str(mark);
    s
}

/// `line` cut to `width`, ending in an ellipsis if anything was lost.
fn ellipsize(line: &str, width: usize, ascii: bool) -> String {
    if line.chars().count() <= width {
        return line.to_string();
    }
    mark_cut(line, width, ascii)
}

/// `s` wrapped to `width`, at most `max` lines, with an ellipsis if it had
/// to be cut short.
///
/// The sidebar cannot scroll, so a sentence that simply stops mid-word reads
/// as a rendering fault. An ellipsis reads as a summary, which is what it is.
fn fold(s: &str, width: usize, max: usize, ascii: bool) -> Vec<String> {
    let width = width.max(4);
    let mut lines: Vec<String> = term::wrap(s, width)
        .into_iter()
        .map(|l| ellipsize(&l, width, ascii))
        .collect();
    if lines.len() > max {
        lines.truncate(max.max(1));
        if let Some(last) = lines.last_mut() {
            *last = mark_cut(last, width, ascii);
        }
    }
    lines
}

/// The rows of a sidebar block: every item wrapped to `width` and cut to
/// `max` rows, the cut marked with an ellipsis.
fn block_rows(
    items: &[(String, Rgb)],
    width: usize,
    max: usize,
    ascii: bool,
) -> Vec<(String, Rgb)> {
    let mut out: Vec<(String, Rgb)> = Vec::new();
    let mut cut = false;
    for (s, c) in items {
        for l in term::wrap(s, width) {
            if out.len() == max {
                cut = true;
                break;
            }
            out.push((ellipsize(&l, width, ascii), *c));
        }
        if cut {
            break;
        }
    }
    if cut {
        if let Some((last, _)) = out.last_mut() {
            *last = mark_cut(last, width, ascii);
        }
    }
    out
}

/// How many rows each block below the sidebar's header gets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Share {
    pub story: usize,
    pub here: usize,
    pub selected: usize,
    pub key: usize,
}

/// The most rows a block may take, however much it has to say.
/// The storyteller earns a couple more rows than the other blocks: it is
/// where a living figure of the age appears, and a reader who glances at the
/// sidebar once a century should find that name without hunting for it.
const STORY_CAP: usize = 6;
const HERE_CAP: usize = 7;
const SEL_CAP: usize = 8;
/// The key is the only thing on screen that says which realm is which
/// colour, so it keeps a floor of rows — a heading and four realms — even
/// when the blocks above it are full, and it never grows past the eight
/// realms it has room to name.
const KEY_MIN: usize = 6;
const KEY_CAP: usize = 10;

/// Share `avail` rows out between the blocks by what each one actually has
/// to say, rather than by a fixed reserve each.
///
/// Every argument is the rows that block would use if the sidebar were
/// endless, headings included. Here and Selected take what they ask for up
/// to a cap, the storyteller takes what is left of its own, and the key
/// takes the remainder — but never less than [`KEY_MIN`] rows while it has
/// that much to say, which is what the fixed reserves used to eat.
pub(crate) fn share_rows(
    avail: usize,
    story: usize,
    here: usize,
    selected: usize,
    key: usize,
) -> Share {
    fn take(want: usize, cap: usize, left: &mut usize) -> usize {
        let got = want.min(cap).min(*left);
        *left -= got;
        got
    }
    let floor = KEY_MIN.min(key).min(avail);
    let mut left = avail - floor;
    let mut s = Share {
        here: take(here, HERE_CAP, &mut left),
        selected: take(selected, SEL_CAP, &mut left),
        story: take(story, STORY_CAP, &mut left),
        key: 0,
    };
    // A block needs a rule, a heading and a line of its own to be worth
    // drawing at all. One that came out shorter than that hands its rows on
    // rather than leaving a gap in the middle of the sidebar.
    for r in [&mut s.story, &mut s.here, &mut s.selected] {
        if *r < 3 {
            left += *r;
            *r = 0;
        }
    }
    s.key = (floor + left).min(key).min(KEY_CAP);
    s
}

impl Ui {
    /// Which realms hold land inside the map pane right now, the largest
    /// realm first, with how many of the cells on screen each one holds.
    pub(super) fn realms_on_screen(&self) -> Vec<(usize, usize)> {
        let (_, _, mw, mh) = self.map_rect();
        let (padx, pady) = self.view_pad();
        let (ox, oy) = self.view;
        let (tw, th) = (self.world.terrain.w, self.world.terrain.h);
        let z = self.zoom;
        let x1 = (ox + (mw - padx.min(mw)) * z).min(tw);
        let y1 = (oy + (mh - pady.min(mh)) * z).min(th);
        let mut seen: Vec<(usize, usize)> = Vec::new();
        for y in oy..y1 {
            for x in ox..x1 {
                if let Some(p) = self.world.cells[y * tw + x].owner {
                    match seen.iter_mut().find(|(q, _)| *q == p) {
                        Some(e) => e.1 += 1,
                        None => seen.push((p, 1)),
                    }
                }
            }
        }
        // Ordered by the whole realm, not by the sliver of it on screen, so
        // the sizes beside the names read down the list in order.
        seen.sort_by_key(|&(p, n)| {
            (
                std::cmp::Reverse(self.world.polities[p].cells),
                std::cmp::Reverse(n),
                p,
            )
        });
        seen
    }

    pub(super) fn render_sidebar(&mut self) {
        let (_, _, mw, mh) = self.map_rect();
        let sw = self.screen.w;
        if sw <= mw {
            return;
        }
        let ascii = self.ascii;
        let x0 = mw;
        let width = sw - mw;
        let bg = Rgb(18, 18, 24);
        let fg = Rgb(200, 200, 205);
        let dim = Rgb(120, 120, 130);
        let accent = Rgb(230, 200, 120);
        self.screen
            .fill(Rect::new(x0, 0, width, mh), ' ', Style::new(fg, bg));
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
        for l in fold(&era, tx, 1, ascii) {
            self.screen.text_clip(x, y, &l, tx, Style::new(dim, bg));
            y += 1;
        }
        for l in fold(&era_desc, tx, 2, ascii) {
            self.screen
                .text_clip(x, y, &l, tx, Style::new(Rgb(95, 95, 105), bg));
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
            self.screen
                .text_clip(x, y, &ellipsize(l, tx, ascii), tx, Style::new(fg, bg));
            y += 1;
        }
        // What happened while the viewer was on another screen.
        if let Some(note) = self.since_note.clone() {
            self.screen
                .text_attr(x, y, "Since you last looked", Rgb(255, 220, 120), bg, BOLD);
            y += 1;
            for l in fold(&note, tx, 3, ascii) {
                self.screen
                    .text_clip(x, y, &l, tx, Style::new(Rgb(235, 215, 165), bg));
                y += 1;
            }
            y += 1;
        }

        // What each block below would use if the sidebar were endless, so
        // the rows can be shared out by need instead of by reserve.
        let story_items: Vec<(String, Ref, Rgb)> = super::super::stories(&self.world)
            .into_iter()
            .map(|(text, r)| {
                let c = detail::ref_color(&self.world, r);
                (text, r, c)
            })
            .collect();
        let story_lines: usize = story_items
            .iter()
            .map(|(t, _, _)| term::wrap(t, tx.saturating_sub(2)).len().min(2))
            .sum();
        let here_items = self.here_items(fg, dim);
        let here_lines: usize = here_items
            .iter()
            .map(|(s, _)| term::wrap(s, tx).len())
            .sum();
        let sel_items: Vec<(String, Rgb)> = match self.selected {
            Some(r) => detail::summary(&self.world, r),
            None => Vec::new(),
        };
        let sel_lines: usize = sel_items.iter().map(|(s, _)| term::wrap(s, tx).len()).sum();
        let on_screen = self.realms_on_screen();
        let share = share_rows(
            mh.saturating_sub(y),
            if story_items.is_empty() {
                0
            } else {
                story_lines + 2
            },
            here_lines + 2,
            if sel_items.is_empty() {
                0
            } else {
                sel_lines + 2
            },
            if on_screen.is_empty() {
                0
            } else {
                on_screen.len().min(8) + 2
            },
        );

        // The storyteller.
        self.story_rows.clear();
        if share.story >= 3 {
            let stop = y + share.story;
            self.screen.hline(x0 + 1, y, width - 1, Rgb(60, 60, 70), bg);
            y += 1;
            self.screen
                .text_attr(x, y, "Now | t visit | click a story", accent, bg, BOLD);
            y += 1;
            let room = tx.saturating_sub(2);
            for (text, r, color) in story_items.iter() {
                // Two rows each, or one when that is all that is left: a
                // headline elided to fit still reads, where one that simply
                // stops in a panel nobody can scroll reads as a fault.
                let max = stop.saturating_sub(y).min(2);
                if max == 0 {
                    break;
                }
                for (k, l) in fold(text, room, max, ascii).into_iter().enumerate() {
                    if k == 0 {
                        self.screen
                            .put(x, y, if ascii { '*' } else { '•' }, *color, bg);
                    }
                    self.screen
                        .text_clip(x + 2, y, &l, room, Style::new(fg, bg));
                    self.story_rows.push((y, *r));
                    y += 1;
                }
            }
            y = stop;
        }

        // Under the cursor.
        if share.here >= 3 {
            let stop = y + share.here;
            self.screen.hline(x0 + 1, y, width - 1, Rgb(60, 60, 70), bg);
            y += 1;
            self.screen
                .text_attr(x, y, "Here | Enter inspect", accent, bg, BOLD);
            y += 1;
            for (l, c) in block_rows(&here_items, tx, stop - y, ascii) {
                self.screen.text_clip(x, y, &l, tx, Style::new(c, bg));
                y += 1;
            }
            y = stop;
        }

        // The selection.
        if share.selected >= 3 {
            let stop = y + share.selected;
            self.screen.hline(x0 + 1, y, width - 1, Rgb(60, 60, 70), bg);
            y += 1;
            // What `x` would act on. The footer says "x intervene" and can
            // say no more — it is a fixed pair of strings — but the Hand of
            // Fate offers a different six for a realm, a town, a person and
            // a stretch of country, so pressing it without knowing which is
            // selected is a guess.
            //
            // Kept short and clipped to the sidebar. The first attempt at
            // this wrote "x befalls the realm" into a heading drawn
            // straight into a column thirty-three wide, and the tail simply
            // ran off the edge; the second put it in the block below, where
            // it was the first line trimmed when a realm had a lot to say.
            let head = match self
                .selected
                .filter(|&r| detail::fate_reaches(&self.world, r))
            {
                Some(Ref::Polity(_)) => "Selected | r recap | x realm",
                Some(Ref::City(_)) => "Selected | r recap | x town",
                Some(Ref::Person(_)) => "Selected | r recap | x person",
                Some(Ref::Feature(_)) => "Selected | r recap | x land",
                _ => "Selected | r recap",
            };
            self.screen
                .text_clip(x, y, head, tx, Style::attr(accent, bg, BOLD));
            y += 1;
            for (l, c) in block_rows(&sel_items, tx, stop - y, ascii) {
                self.screen.text_clip(x, y, &l, tx, Style::new(c, bg));
                y += 1;
            }
            y = stop;
        }

        // The key to the colours: which realm is which, out of what is on
        // screen right now.
        self.power_rows.clear();
        if share.key >= 3 {
            let stop = (y + share.key).min(mh);
            self.screen.hline(x0 + 1, y, width - 1, Rgb(60, 60, 70), bg);
            y += 1;
            self.screen.text_attr(x, y, "On screen", accent, bg, BOLD);
            y += 1;
            for &(p, _) in on_screen.iter().take(stop.saturating_sub(y)) {
                let (color, war, cells, name) = {
                    let pol = &self.world.polities[p];
                    (pol.color, pol.at_war(), pol.cells, pol.name.clone())
                };
                let arrow = match words::trend(&self.world, p) {
                    Some("rising") => {
                        if ascii {
                            "up"
                        } else {
                            "↑ "
                        }
                    }
                    Some(_) => {
                        if ascii {
                            "dn"
                        } else {
                            "↓ "
                        }
                    }
                    None => "  ",
                };
                self.power_rows.push((y, p));
                // A realm of your own people is marked, because a covenant
                // you cannot see on the map is a number in a corner of the
                // status bar and nothing more.
                let mine = self.world.fate.patron == Some(self.world.polities[p].culture);
                let swatch = match (mine, ascii) {
                    (true, true) => '*',
                    (true, false) => '\u{25c6}',
                    (false, true) => '#',
                    (false, false) => '\u{25a0}',
                };
                self.screen.put(x, y, swatch, color, bg);
                // swatch, name, lands, war, trend: the name gives up what
                // the numbers need, and says so with an ellipsis.
                let namew = tx.saturating_sub(11);
                let name = ellipsize(&name, namew, ascii);
                let s = format!(
                    "{:<w$} {:>4}{} {}",
                    name,
                    cells,
                    if war { "!" } else { " " },
                    arrow,
                    w = namew
                );
                self.screen
                    .text_clip(x + 2, y, &s, tx - 2, Style::new(fg, bg));
                y += 1;
            }
        }
    }

    /// The "Here" block's lines: one plain sentence about the cursor's cell,
    /// then the particulars.
    fn here_items(&self, fg: Rgb, dim: Rgb) -> Vec<(String, Rgb)> {
        let w = &self.world;
        let i = w.terrain.idx(self.cursor.0, self.cursor.1);
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
        // The particulars answer the layer being read. This block used to
        // say mana and fertility whatever was on the screen, which meant the
        // trade map could tell a reader where the roads were and nothing at
        // all about what moved along them.
        here.extend(self.here_particulars(i, dim));
        here
    }

    /// The numbers under the cursor that bear on the layer being read.
    fn here_particulars(&self, i: usize, dim: Rgb) -> Vec<(String, Rgb)> {
        let w = &self.world;
        let land = w.terrain.is_land(i);
        let mut out: Vec<(String, Rgb)> = Vec::new();
        match self.layer {
            Layer::Goods => {
                match w.goods.get(i).copied().flatten() {
                    Some(g) => {
                        let (c, _) = crate::ui::good_style(g, self.ascii);
                        out.push((format!("yields {}", g.name()), c));
                    }
                    None if land => out.push(("yields nothing worth carrying".into(), dim)),
                    None => {}
                }
                // Who can actually sell it: a good with no city in reach is
                // a fact about the map and not about anybody's economy.
                if let Some((city, d)) = self.nearest_living_city(i) {
                    out.push((format!("{} is {} away", w.cities[city].name, d), dim));
                }
            }
            Layer::Trade => {
                if let Some(c) = w.cells[i].city.filter(|&c| w.cities[c].destroyed.is_none()) {
                    let partners = w.trade_partners(c);
                    out.push((
                        format!(
                            "{:.1} a year over {}",
                            w.city_trade(c),
                            crate::sim::prose::count(partners.len() as i64, "road")
                        ),
                        Rgb(230, 200, 120),
                    ));
                    let shut = w
                        .routes
                        .iter()
                        .filter(|r| !r.open && (r.a == c || r.b == c))
                        .count();
                    if shut > 0 {
                        out.push((
                            format!(
                                "{} closed by war",
                                crate::sim::prose::count(shut as i64, "road")
                            ),
                            Rgb(220, 120, 110),
                        ));
                    }
                }
                if let Some(p) = w.cells[i].owner {
                    out.push((
                        format!(
                            "{} keeps {:.0} in its treasury, development {:.2}",
                            w.polities[p].short, w.polities[p].treasury, w.polities[p].dev
                        ),
                        dim,
                    ));
                }
            }
            Layer::Harvest => {
                if land {
                    // The three factors, then their product, because the
                    // interest is in which of them is the binding one.
                    out.push((
                        format!(
                            "fertility {:.0}%  weather {:+.0}%  known {:+.0}%",
                            w.terrain.fertility[i] * 100.0,
                            (w.climate_mult(i) - 1.0) * 100.0,
                            (w.cell_yield.get(i).copied().unwrap_or(1.0) - 1.0) * 100.0
                        ),
                        dim,
                    ));
                    out.push((
                        format!(
                            "feeds {:.1}, holding {:.1}",
                            w.cell_capacity(i),
                            w.cells[i].pop
                        ),
                        dim,
                    ));
                }
            }
            Layer::Knowledge => {
                if land {
                    let y = w.cell_yield.get(i).copied().unwrap_or(1.0);
                    out.push((
                        format!("harvests {:+.0}% for what is known here", (y - 1.0) * 100.0),
                        dim,
                    ));
                    out.push((
                        format!(
                            "{} known on this ground",
                            crate::sim::prose::count(
                                w.known.get(i).copied().unwrap_or(0).count_ones() as i64,
                                "thing"
                            )
                        ),
                        dim,
                    ));
                }
            }
            // The motion layers report a rate and how it compares with what
            // is there, because "forty people left" means one thing in a
            // village and another in a province.
            Layer::Settling => {
                if land {
                    let f = w.flows.settled.get(i).copied().unwrap_or(0.0);
                    let word = if f > 0.01 {
                        "filling up"
                    } else if f < -0.01 {
                        "emptying out"
                    } else {
                        "settled"
                    };
                    out.push((
                        format!(
                            "{} ({:+.2} against {:.1} living here)",
                            word, f, w.cells[i].pop
                        ),
                        if f < -0.01 { Rgb(210, 120, 100) } else { dim },
                    ));
                }
            }
            Layer::Fighting => {
                let f = w.flows.fought.get(i).copied().unwrap_or(0.0);
                if f > 0.02 {
                    out.push((
                        format!("fought over ({:.1} of a lifetime's worth)", f),
                        Rgb(220, 120, 110),
                    ));
                } else if land {
                    out.push(("no fighting in living memory".into(), dim));
                }
                if let Some(p) = w.cells[i].owner {
                    let at = w.polities[p]
                        .wars
                        .iter()
                        .filter(|&&x| w.wars[x].alive())
                        .count();
                    if at > 0 {
                        out.push((
                            format!(
                                "{} is in {}",
                                w.polities[p].short,
                                crate::sim::prose::count(at as i64, "war")
                            ),
                            dim,
                        ));
                    }
                }
            }
            Layer::Frontier => {
                if land {
                    match w.cells[i].owner {
                        Some(p) => out.push((
                            format!(
                                "{} has held this since {}",
                                w.polities[p].short, w.cells[i].since
                            ),
                            dim,
                        )),
                        None => out.push(("held by nobody".into(), dim)),
                    }
                    let churn = w.flows.changed.get(i).copied().unwrap_or(0.0);
                    if churn > 0.5 {
                        out.push((
                            format!("changed hands {:.1} times in living memory", churn),
                            Rgb(255, 170, 90),
                        ));
                    }
                }
            }
            Layer::Carrying => {
                let f = w.flows.carried.get(i).copied().unwrap_or(0.0);
                out.push((
                    if f > 0.02 {
                        format!("the caravans have found this way ({:+.1})", f)
                    } else if f < -0.02 {
                        format!("a road through here has shut ({:+.1})", f)
                    } else {
                        "nothing has changed along here lately".to_string()
                    },
                    if f < -0.02 { Rgb(220, 120, 110) } else { dim },
                ));
            }
            Layer::Memory => {
                let n = w
                    .chronicle
                    .events
                    .iter()
                    .filter(|e| e.loc == Some(i))
                    .count();
                out.push((
                    if n == 0 {
                        "nothing the chronicle still remembers".to_string()
                    } else {
                        format!(
                            "{} remembered here",
                            crate::sim::prose::count(n as i64, "thing")
                        )
                    },
                    dim,
                ));
                if w.chronicle.dropped > 0 {
                    out.push((
                        format!(
                            "{} forgotten world-wide",
                            crate::sim::prose::count(w.chronicle.dropped as i64, "entry")
                        ),
                        dim,
                    ));
                }
            }
            Layer::Settled => {
                if land {
                    match w.cells[i].owner {
                        Some(p) => out.push((
                            format!(
                                "held by {} since {} — {}",
                                w.polities[p].short,
                                w.cells[i].since,
                                crate::sim::prose::years((w.year - w.cells[i].since).max(0) as i64)
                            ),
                            dim,
                        )),
                        None => out.push(("held by nobody".into(), dim)),
                    }
                }
            }
            Layer::Relations => {
                if let Some(p) = w.cells[i].owner {
                    let pol = &w.polities[p];
                    out.push((w.polities[p].short.clone(), pol.color));
                    let wars = pol.wars.iter().filter(|&&x| w.wars[x].alive()).count();
                    let allies = pol
                        .stance
                        .values()
                        .filter(|&&s| s == crate::sim::Stance::Allied)
                        .count();
                    let subjects = w
                        .alive_polities
                        .iter()
                        .filter(|&&q| w.polities[q].overlord == Some(p))
                        .count();
                    let mut said: Vec<String> = Vec::new();
                    if wars > 0 {
                        said.push(crate::sim::prose::count(wars as i64, "war"));
                    }
                    if allies > 0 {
                        said.push(format!(
                            "{} sworn",
                            crate::sim::prose::count(allies as i64, "friend")
                        ));
                    }
                    if subjects > 0 {
                        said.push(format!(
                            "{} paying tribute",
                            crate::sim::prose::count(subjects as i64, "realm")
                        ));
                    }
                    if let Some(over) = pol.overlord {
                        said.push(format!("pays tribute to {}", w.polities[over].short));
                    }
                    out.push((
                        if said.is_empty() {
                            "no wars, no friends, no subjects".to_string()
                        } else {
                            said.join(", ")
                        },
                        dim,
                    ));
                    // Whoever it is angriest with, since that is the next
                    // war if there is going to be one.
                    if let Some((&q, &t)) = pol
                        .tension
                        .iter()
                        .max_by(|a, b| a.1.total_cmp(b.1).then(b.0.cmp(a.0)))
                    {
                        if t >= 0.2 && w.polities[q].alive() {
                            out.push((
                                format!(
                                    "most angry with {} ({:.0}%)",
                                    w.polities[q].short,
                                    t * 100.0
                                ),
                                Rgb(220, 160, 90),
                            ));
                        }
                    }
                }
            }
            Layer::Drift => {
                if land {
                    let d = w.climate_drift(i);
                    let word = if d > 0.02 {
                        "wetter than a century ago"
                    } else if d < -0.02 {
                        "drier than a century ago"
                    } else {
                        "the same as a century ago"
                    };
                    out.push((format!("{} ({:+.0}% on harvests)", word, d * 100.0), dim));
                    out.push((
                        format!(
                            "feeds {:.1} now, holding {:.1}",
                            w.cell_capacity(i),
                            w.cells[i].pop
                        ),
                        dim,
                    ));
                }
            }
            _ => out.push((
                format!(
                    "mana {:.0}%  fertility {:.0}%",
                    w.terrain.mana[i] * 100.0,
                    w.terrain.fertility[i] * 100.0
                ),
                dim,
            )),
        }
        out
    }

    /// The nearest standing city, and how far off it is.
    ///
    /// Bounded: it walks the living cities, of which there are a couple of
    /// thousand at most, rather than searching outward over the map.
    fn nearest_living_city(&self, i: usize) -> Option<(usize, usize)> {
        let w = &self.world;
        w.cities
            .iter()
            .filter(|c| c.destroyed.is_none())
            .map(|c| (c.id, w.terrain.dist(i, c.cell)))
            .min_by_key(|&(id, d)| (d, id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cut_line_says_so() {
        assert_eq!(ellipsize("short", 10, false), "short");
        assert_eq!(ellipsize("a longer line", 8, false), "a longe…");
        assert_eq!(ellipsize("a longer line", 8, true), "a lon...");
        assert!(ellipsize("a longer line", 8, true).chars().count() <= 8);
    }

    #[test]
    fn a_folded_block_never_stops_mid_sentence() {
        let s = "The Kingdom of Safa is on the brink, and its governors are \
                 already writing to its neighbours.";
        for width in [20usize, 33, 40] {
            for max in [1usize, 2, 3] {
                let lines = fold(s, width, max, false);
                assert!(lines.len() <= max);
                assert!(lines.iter().all(|l| l.chars().count() <= width));
                let whole = term::wrap(s, width).len() <= max;
                assert_eq!(
                    lines.last().unwrap().ends_with('…'),
                    !whole,
                    "{:?} at {} in {} rows",
                    lines,
                    width,
                    max
                );
            }
        }
    }

    #[test]
    fn a_cut_block_ends_in_an_ellipsis() {
        let items: Vec<(String, Rgb)> = vec![
            (
                "Grassland on the coast, where Fil stands.".into(),
                Rgb(1, 1, 1),
            ),
            ("ruled by the Lystzylese Commonwealth".into(), Rgb(2, 2, 2)),
        ];
        let rows = block_rows(&items, 20, 2, false);
        assert_eq!(rows.len(), 2);
        assert!(rows[1].0.ends_with('…'), "{:?}", rows);
        let all = block_rows(&items, 20, 40, false);
        assert!(!all.last().unwrap().0.ends_with('…'), "{:?}", all);
        assert!(block_rows(&items, 20, 40, true)
            .iter()
            .all(|(l, _)| l.is_ascii()));
    }

    /// The bug this replaced: fixed reserves for Here, Selected and the
    /// storyteller left the key with a single row on a 44-row terminal.
    #[test]
    fn the_key_keeps_its_floor() {
        // A tall sidebar: everybody gets what they asked for.
        let s = share_rows(40, 4, 7, 8, 10);
        assert_eq!(
            s,
            Share {
                story: 4,
                here: 7,
                selected: 8,
                key: 10
            }
        );
        // A crowded one: the key still gets four rows.
        let s = share_rows(20, 9, 9, 14, 10);
        assert!(s.key >= KEY_MIN, "{:?}", s);
        assert_eq!(s.story + s.here + s.selected + s.key, 20);
        // Nothing selected and no stories: the key takes the room.
        let s = share_rows(20, 0, 6, 0, 10);
        assert_eq!(s.here, 6);
        assert_eq!(s.key, KEY_CAP);
        // A block left with too few rows to draw hands them to the key
        // instead of leaving a gap where it would have been.
        let s = share_rows(18, 4, 7, 8, 10);
        assert_eq!(s.story, 0);
        assert_eq!(s.here + s.selected + s.key, 18);
    }

    #[test]
    fn blocks_never_take_more_than_they_asked_for() {
        for avail in 0..30usize {
            for &(story, here, sel, key) in &[
                (0usize, 3usize, 0usize, 0usize),
                (4, 6, 9, 10),
                (20, 20, 20, 20),
                (2, 2, 2, 2),
            ] {
                let s = share_rows(avail, story, here, sel, key);
                assert!(s.story <= story && s.here <= here && s.selected <= sel);
                assert!(s.key <= key);
                assert!(s.story + s.here + s.selected + s.key <= avail, "{:?}", s);
                if avail >= KEY_MIN && key >= KEY_MIN {
                    assert!(s.key >= KEY_MIN, "{} rows: {:?}", avail, s);
                }
            }
        }
    }
}
