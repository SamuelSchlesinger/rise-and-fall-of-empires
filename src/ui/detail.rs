//! Lists, detail pages, summaries, the help text and the hand of fate.

use super::words;
use crate::sim::chronicle::{EventKind, Ref};
use crate::sim::{dynasty, explain};
use crate::sim::{SchoolKind, Stance, World};
use crate::term::{self, Rgb, BOLD, DIM};
use std::cmp::Ordering;

pub const LIST_TABS: [&str; 14] = [
    "Realms",
    "Cities",
    "Peoples",
    "Schools",
    "Persons",
    "Wars",
    "Places",
    "Relics",
    "Prophecies",
    "Figures",
    "Houses",
    // Two views of the economy. Kept as their own pages rather than as more
    // columns on Realms and Cities, which are already as wide as a narrow
    // terminal will take — and because sorting realms by what they are
    // worth is a different question from sorting them by how large they are.
    "Wealth",
    "Roads",
    // The thing the game is named for, and the one view it could not draw.
    // Every realm has recorded its founding, its fall, its parent and the
    // year of its greatest extent since long before this page existed.
    "Timeline",
];

pub struct Line {
    pub text: String,
    pub fg: Rgb,
    pub attr: u8,
}

fn line(text: impl Into<String>, fg: Rgb, attr: u8) -> Line {
    Line {
        text: text.into(),
        fg,
        attr,
    }
}

/// Wrap `text` under a fixed-width label, keeping continuation lines indented.
fn labelled(out: &mut Vec<Line>, label: &str, text: &str, width: usize, fg: Rgb) {
    let lw = 14;
    let body = term::wrap(text, width.saturating_sub(lw).max(10));
    for (k, l) in body.into_iter().enumerate() {
        let pre = if k == 0 {
            format!("{:<lw$}", label, lw = lw)
        } else {
            " ".repeat(lw)
        };
        out.push(line(format!("{}{}", pre, l), fg, 0));
    }
}

fn article(word: &str) -> &'static str {
    if word.starts_with(|c: char| "aeiouAEIOU".contains(c)) {
        "An"
    } else {
        "A"
    }
}

const FG: Rgb = Rgb(200, 200, 205);
const DIMC: Rgb = Rgb(130, 130, 140);
const ACCENT: Rgb = Rgb(230, 200, 120);
const LINK: Rgb = Rgb(140, 190, 240);

pub fn ref_color(w: &World, r: Ref) -> Rgb {
    match r {
        Ref::Polity(p) => w.polities[p].color,
        Ref::City(c) => w.cities[c]
            .polity
            .map(|p| w.polities[p].color)
            .unwrap_or(DIMC),
        Ref::Culture(c) => w.cultures[c].color,
        Ref::School(s) => w.schools[s].color,
        Ref::Person(p) => w.persons[p]
            .polity
            .map(|p| w.polities[p].color)
            .unwrap_or(FG),
        Ref::War(_) => Rgb(240, 110, 100),
        Ref::Race(_) => ACCENT,
        Ref::Feature(_) => Rgb(120, 170, 240),
        Ref::Artifact(_) => Rgb(255, 200, 80),
        Ref::House(h) => w.houses[h]
            .realms
            .first()
            .map(|&p| w.polities[p].color)
            .unwrap_or(ACCENT),
    }
}

pub fn entity_name(w: &World, r: Ref) -> String {
    match r {
        Ref::Polity(p) => w.polities[p].name.clone(),
        Ref::City(c) => w.cities[c].name.clone(),
        Ref::Culture(c) => format!("the {}", w.cultures[c].plural),
        Ref::School(s) => w.schools[s].name.clone(),
        Ref::Person(p) => w.persons[p].full_name(),
        Ref::War(x) => w.wars[x].name.clone(),
        Ref::Race(x) => format!("the {}", w.races[x].plural),
        Ref::Feature(f) => w.terrain.features[f].display(),
        Ref::Artifact(a) => w.artifacts[a].name.clone(),
        Ref::House(h) => w.houses[h].name.clone(),
    }
}

pub fn entity_loc(w: &World, r: Ref) -> Option<usize> {
    match r {
        Ref::Polity(p) => w.capital_cell(p).or_else(|| w.cells_of(p).first().copied()),
        Ref::City(c) => Some(w.cities[c].cell),
        Ref::Culture(c) => Some(w.cultures[c].home),
        Ref::School(s) => Some(w.cities[w.schools[s].home_city].cell),
        Ref::Person(p) => {
            let per = &w.persons[p];
            per.city
                .map(|c| w.cities[c].cell)
                .or_else(|| per.polity.and_then(|pp| w.capital_cell(pp)))
        }
        Ref::War(x) => w
            .capital_cell(w.wars[x].defender)
            .or_else(|| w.capital_cell(w.wars[x].attacker)),
        Ref::Race(x) => Some(w.races[x].home),
        Ref::Feature(f) => {
            let c = w.terrain.features[f].center;
            Some(w.terrain.idx(c.0, c.1))
        }
        Ref::Artifact(a) => w.artifact_loc(a),
        // A house is wherever it last sat: the seat of the newest realm it
        // holds, or of the last one it held.
        Ref::House(h) => w.houses[h]
            .realms
            .iter()
            .rev()
            .find_map(|&p| w.capital_cell(p)),
    }
}

/// How wide a lifetime is drawn on the Timeline page.
///
/// Fixed, because `list_rows` builds finished strings and does not know the
/// terminal — the same bargain every other list page makes, with
/// `text_clip` cutting the tail on a narrow window.
const TIMELINE_WIDTH: usize = 44;

/// How many generations of successor states are shown beneath a realm.
const LINEAGE_DEPTH: usize = 2;

/// And how many of each generation. A realm that shattered into fifty
/// splinters would otherwise fill the page with its own wreckage; the rest
/// are listed further down on their own account, which is where a reader
/// looking for the second great power of the age expects to find it.
const LINEAGE_FANOUT: usize = 6;

/// A bar covering the years `from`..=`to` within a world `span` years old.
///
/// Unlike [`bar`], which fills from the left, this one is a *position*: the
/// point of the page is that two realms which lasted equally long but three
/// thousand years apart should not look the same.
fn span_bar(from: i32, to: i32, span: f32, width: usize) -> String {
    let at = |year: i32| -> usize {
        ((year.max(0) as f32 / span.max(1.0)) * width as f32).round() as usize
    };
    let (start, end) = (at(from).min(width), at(to).min(width));
    // A realm that rose and fell inside one column still gets a column: a
    // blank row would say it never existed.
    let end = end.max(start + 1).min(width);
    // Drawn in the two glyphs the frame's ascii sweep already knows how to
    // replace, so this needs no `ascii` flag of its own — `list_rows` builds
    // finished strings and is not told which mode it is in. The same bargain
    // `term::hline` makes.
    let (full, empty) = ('\u{2588}', '\u{2591}');
    (0..width)
        .map(|i| if i >= start && i < end { full } else { empty })
        .collect()
}

/// The chosen realms in lineage order, each with how deep it sits.
///
/// A realm records the realm it broke away from, so the ones that are
/// present in `chosen` form a forest. Roots — whose parent is not on the
/// page, because it was too small to make the cut or never existed — come
/// in the order they were given, and each realm's children follow it.
///
/// Bounded by construction: every realm is emitted at most once, guarded by
/// `seen`, so a malformed parent chain cannot loop.
fn lineage_order(w: &World, chosen: &[usize]) -> Vec<(usize, usize)> {
    let in_set: std::collections::BTreeSet<usize> = chosen.iter().copied().collect();
    let mut children: std::collections::BTreeMap<usize, Vec<usize>> =
        std::collections::BTreeMap::new();
    for &p in chosen {
        if let Some(parent) = w.polities[p].parent.filter(|q| in_set.contains(q)) {
            children.entry(parent).or_default().push(p);
        }
    }
    let mut out: Vec<(usize, usize)> = Vec::with_capacity(chosen.len());
    let mut seen: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    // An explicit stack rather than recursion: a chain of successor states
    // can be hundreds deep in a long game.
    for &root in chosen {
        if w.polities[root].parent.is_some_and(|q| in_set.contains(&q)) {
            continue;
        }
        let mut stack = vec![(root, 0usize)];
        while let Some((p, depth)) = stack.pop() {
            if !seen.insert(p) {
                continue;
            }
            out.push((p, depth));
            // Only the immediate family. Without a bound the greatest realm
            // brought its whole descent with it — hundreds of rows — and
            // the second greatest was pages away, which is the opposite of
            // what a page ranked by greatness is for. Anything deeper is
            // left for the sweep below, which lists it on its own account.
            if depth < LINEAGE_DEPTH {
                if let Some(kids) = children.get(&p) {
                    for &kid in kids.iter().take(LINEAGE_FANOUT).rev() {
                        stack.push((kid, depth + 1));
                    }
                }
            }
        }
    }
    // Everything not reached: too deep in somebody's descent, or in a
    // parent cycle. Listed on its own account, in the order it was given,
    // rather than silently dropped from a page that said it would show it.
    for &p in chosen {
        if seen.insert(p) {
            out.push((p, 0));
        }
    }
    out
}

fn bar(v: f32, width: usize, ascii: bool) -> String {
    let n = ((v.clamp(0.0, 1.0)) * width as f32).round() as usize;
    let (full, empty) = if ascii { ('#', '.') } else { ('█', '░') };
    let mut s = String::new();
    for i in 0..width {
        s.push(if i < n { full } else { empty });
    }
    s
}

/// Green when a realm is holding together, red when it is not.
pub fn stability_color(v: f32) -> Rgb {
    if v >= 0.65 {
        Rgb(150, 210, 150)
    } else if v >= 0.45 {
        Rgb(220, 210, 140)
    } else if v >= 0.25 {
        Rgb(230, 170, 110)
    } else {
        Rgb(240, 110, 100)
    }
}

/// Short summary for the sidebar.
pub fn summary(w: &World, r: Ref) -> Vec<(String, Rgb)> {
    let mut out = Vec::new();
    match r {
        Ref::House(h) => {
            let ho = &w.houses[h];
            out.push((ho.name.clone(), ACCENT));
            out.push((
                format!(
                    "{} of the {}",
                    if ho.alive() {
                        "a house"
                    } else {
                        "a spent house"
                    },
                    w.cultures[ho.culture].plural
                ),
                FG,
            ));
            out.push((
                format!(
                    "{} years · {} who ruled",
                    ho.span(w.year).max(0),
                    ho.seniors.len()
                ),
                DIMC,
            ));
        }
        Ref::Polity(p) => {
            let pol = &w.polities[p];
            out.push((pol.name.clone(), pol.color));
            if let Some(y) = pol.fell {
                out.push((format!("fell in year {}", y), DIMC));
            }
            if let Some(o) = pol.overlord {
                out.push((crate::sim::prose::tributary_line(w, o), Rgb(240, 160, 90)));
            }
            out.push((
                format!(
                    "{}, {}",
                    words::realm_size(pol.cells),
                    words::polity_kind(pol.kind)
                ),
                FG,
            ));
            out.push((w.ruler_short(p), FG));
            out.push((
                format!(
                    "{} lands · {} people",
                    pol.cells,
                    words::folk(pol.pop as f32)
                ),
                DIMC,
            ));
            if pol.alive() {
                out.push((
                    format!(
                        "{}, {}",
                        words::stability(pol.stability),
                        words::stability_trend(
                            explain::stability_drift(w, p),
                            explain::stability_target(w, p)
                        )
                    ),
                    stability_color(pol.stability),
                ));
                out.push((
                    format!(
                        "stability {:.0}% · treasury {} · army {}",
                        pol.stability * 100.0,
                        words::treasury(pol.treasury),
                        words::army_standing(w, p)
                    ),
                    DIMC,
                ));
                if let Some(f) = explain::stability_factors(w, p)
                    .into_iter()
                    .find(|f| f.weight < -0.04)
                {
                    out.push((format!("worst of it: {}", f.text), Rgb(210, 150, 120)));
                }
            }
            if let Some(s) = pol.school {
                out.push((
                    format!("{} ({})", w.schools[s].name, words::school_reach(w, s)),
                    w.schools[s].color,
                ));
            }
            for &wid in &pol.wars {
                let war = &w.wars[wid];
                let other = if war.attacker == p {
                    war.defender
                } else {
                    war.attacker
                };
                let lean = match explain::war_view(w, wid).leader {
                    Some(q) if q == p => ", and ahead",
                    Some(_) => ", and behind",
                    None => ", evenly",
                };
                out.push((
                    format!("at war with {}{}", w.polities[other].short, lean),
                    Rgb(240, 110, 100),
                ));
            }
        }
        Ref::City(c) => {
            let city = &w.cities[c];
            out.push((city.name.clone(), Rgb(255, 255, 255)));
            out.push((
                format!(
                    "{}, founded in {}",
                    words::city_size(city.pop),
                    city.founded
                ),
                FG,
            ));
            out.push((format!("{} people", words::folk(city.pop)), DIMC));
            if let Some(p) = city.polity {
                out.push((w.polities[p].name.clone(), w.polities[p].color));
            }
            for wn in &city.wonders {
                out.push((wn.clone(), ACCENT));
            }
        }
        Ref::Culture(c) => {
            let cu = &w.cultures[c];
            out.push((format!("the {}", cu.plural), cu.color));
            out.push((
                format!(
                    "{} people · {} lands",
                    crate::sim::prose::a(&w.races[cu.race].adj),
                    cu.cells
                ),
                FG,
            ));
            out.push((format!("speak {}", cu.lang.name), DIMC));
        }
        Ref::School(s) => {
            let sc = &w.schools[s];
            out.push((sc.name.clone(), sc.color));
            out.push((words::school_reach(w, s).to_string(), FG));
            out.push((format!("{} · {}", sc.kind.name(), sc.doctrine), DIMC));
            out.push((
                format!(
                    "followed in {} realms",
                    sc.influence
                        .iter()
                        .filter(|(&p, &v)| v > 0.1 && w.polities[p].alive())
                        .count()
                ),
                DIMC,
            ));
        }
        Ref::Person(p) => {
            let per = &w.persons[p];
            out.push((per.full_name(), FG));
            out.push((explain::standing(w, p), FG));
            out.push((format!("{} · born {}", per.role.name(), per.born), DIMC));
        }
        Ref::War(x) => {
            let war = &w.wars[x];
            out.push((war.name.clone(), Rgb(240, 110, 100)));
            out.push((
                format!(
                    "{} against {}",
                    w.polities[war.attacker].short, w.polities[war.defender].short
                ),
                FG,
            ));
            if war.alive() {
                let v = explain::war_view(w, x);
                out.push((
                    match v.leader {
                        Some(p) => format!("{} is winning", w.polities[p].short),
                        None => "neither side is winning".to_string(),
                    },
                    FG,
                ));
                out.push((
                    format!(
                        "{} years on; {}",
                        w.year - war.started,
                        explain::war_weariness(w, x)
                    ),
                    DIMC,
                ));
            } else {
                out.push((format!("it {}", war.result), DIMC));
            }
        }
        Ref::Race(x) => {
            out.push((format!("the {}", w.races[x].plural), ACCENT));
        }
        Ref::Feature(f) => {
            let ft = &w.terrain.features[f];
            out.push((ft.display(), LINK));
            out.push((
                format!("{} · {} cells", ft.kind.label(), ft.cells.len()),
                DIMC,
            ));
        }
        Ref::Artifact(a) => {
            let ar = &w.artifacts[a];
            out.push((ar.name.clone(), Rgb(255, 200, 80)));
            out.push((format!("held by {}", w.artifact_holder_name(a)), FG));
        }
    }
    out
}

/// The quantities a list page's rows can be compared on, for the hint line.
pub fn query_fields(tab: usize) -> String {
    let fields = crate::ui::query::fields_for(tab);
    if fields.is_empty() {
        return String::new();
    }
    let first = fields.split_whitespace().next().unwrap_or("");
    format!("filter by name, or {}>0 — {}", first, fields)
}

pub fn list_header(tab: usize) -> &'static str {
    match tab {
        0 => "  name                              kind          lands    people  stability          ruler",
        1 => "  name                    realm                            people   size              founded",
        2 => "  people                  race          lands    people   known for    language",
        3 => "  school                              kind         realms reach                         home",
        4 => "  name                          role         realm                  born   died",
        5 => "  war                                       attacker vs defender          years",
        6 => "  place                                   kind",
        7 => "  relic                                   kind      made   held by",
        9 => "  figure                        points  realm               lived      chiefly remembered for",
        10 => "  house                               people           ruled  thrones  span",
        11 => "  realm                             treasury   a year   devt   trade  prosperity",
        12 => "  road                                            worth  carrying          since",
        13 => "  realm                          rise and fall                                  peak",
        _ => "  prophecy                                                          seer                 by     outcome",
    }
}

/// How many rows a list page puts in order and renders.
///
/// A page is a window on a world's whole history, and history does not stop
/// growing: by the eightieth century of a large map a hundred thousand
/// people have lived and twenty-five thousand wars have been fought. Every
/// one of them was sorted and rendered to a line of text on every frame,
/// which cost the Wars page nine milliseconds a frame by itself — the
/// interface got slower the longer a game went on, for no reason a player
/// could see. Nobody scrolls past the first few hundred of anything; `/`
/// is what finds the rest.
const LIST_CAP: usize = 2000;

/// And for the Figures page, which is not a catalogue but a short list of
/// the people an age is remembered by — and which pays for every row it
/// shows, because each one carries the deed the person is known for.
const FIGURES_CAP: usize = 400;

/// Put the best `cap` of `v` in order and return how many there were.
///
/// The selection is linear in the length and the sort is over the survivors,
/// so a page costs what it shows rather than what the world remembers.
fn keep_best(
    v: &mut Vec<usize>,
    cap: usize,
    mut cmp: impl FnMut(&usize, &usize) -> Ordering,
) -> usize {
    let total = v.len();
    if total > cap {
        v.select_nth_unstable_by(cap - 1, &mut cmp);
        v.truncate(cap);
    }
    v.sort_by(cmp);
    total
}

/// The same, for a page ordered by a key rather than a comparison. The
/// index goes on the end of every key so that the order is total: an
/// unstable selection may reorder ties, and a list that shuffles as it is
/// scrolled is worse than a slow one.
fn keep_best_by_key<K: Ord>(
    v: &mut Vec<usize>,
    cap: usize,
    mut key: impl FnMut(usize) -> K,
) -> usize {
    keep_best(v, cap, |&a, &b| (key(a), a).cmp(&(key(b), b)))
}

pub fn list_rows(w: &World, tab: usize) -> (Vec<(String, Ref)>, usize) {
    let mut rows = Vec::new();
    // How many there were before the page was trimmed to [`LIST_CAP`], so
    // the footer can say that there is more than is shown.
    let total;
    match tab {
        0 => {
            let mut ps: Vec<usize> = (0..w.polities.len()).collect();
            total = keep_best_by_key(&mut ps, LIST_CAP, |p| {
                (
                    w.polities[p].fell.is_some(),
                    std::cmp::Reverse(w.polities[p].cells),
                    std::cmp::Reverse(w.polities[p].peak_cells),
                )
            });
            for p in ps {
                let pol = &w.polities[p];
                let status = match pol.fell {
                    Some(y) => format!("fell {}", y),
                    None => format!("{:>5}", pol.cells),
                };
                let row = format!(
                    "{:<34}{:<12}{:>7}  {:>7.0}k  {:>3.0}% {:<13} {}",
                    clip(&pol.name, 33),
                    pol.kind.name(),
                    status,
                    pol.pop,
                    pol.stability * 100.0,
                    if pol.alive() {
                        words::stability(pol.stability)
                    } else {
                        ""
                    },
                    if pol.alive() {
                        w.ruler_short(p)
                    } else {
                        String::new()
                    }
                );
                rows.push((row, Ref::Polity(p)));
            }
        }
        1 => {
            let mut cs: Vec<usize> = (0..w.cities.len()).collect();
            // Living cities first, largest first within each group.
            total = keep_best(&mut cs, LIST_CAP, |&a, &b| {
                let (ca, cb) = (&w.cities[a], &w.cities[b]);
                ca.destroyed
                    .is_some()
                    .cmp(&cb.destroyed.is_some())
                    .then_with(|| cb.pop.total_cmp(&ca.pop))
                    .then_with(|| a.cmp(&b))
            });
            for c in cs {
                let city = &w.cities[c];
                let realm = match city.destroyed {
                    Some(y) => format!("destroyed {}", y),
                    None => city
                        .polity
                        .map(|p| w.polities[p].name.clone())
                        .unwrap_or_else(|| "independent".into()),
                };
                let row = format!(
                    "{:<24}{:<32}{:>7.1}k  {:<18}{:>6}",
                    clip(&city.name, 23),
                    clip(&realm, 31),
                    city.pop,
                    if city.destroyed.is_none() {
                        words::city_size(city.pop)
                    } else {
                        "ruins"
                    },
                    city.founded
                );
                rows.push((row, Ref::City(c)));
            }
        }
        2 => {
            let mut cs: Vec<usize> = (0..w.cultures.len()).collect();
            total = keep_best_by_key(&mut cs, LIST_CAP, |c| {
                (
                    w.cultures[c].extinct.is_some(),
                    std::cmp::Reverse(w.cultures[c].cells),
                )
            });
            for c in cs {
                let cu = &w.cultures[c];
                let status = match cu.extinct {
                    Some(y) => format!("gone {}", y),
                    None => format!("{:>5}", cu.cells),
                };
                let (field, _) = cu.best_field();
                let row = format!(
                    "{:<24}{:<14}{:>6}  {:>7.0}k  {:<13}{}",
                    clip(&cu.plural, 23),
                    clip(&w.races[cu.race].name, 13),
                    status,
                    cu.pop,
                    field.name(),
                    cu.lang.name
                );
                rows.push((row, Ref::Culture(c)));
            }
        }
        3 => {
            let mut ss: Vec<usize> = (0..w.schools.len()).collect();
            // Living schools first, most influential first within each group.
            total = keep_best(&mut ss, LIST_CAP, |&a, &b| {
                let (sa, sb) = (&w.schools[a], &w.schools[b]);
                sa.extinct
                    .is_some()
                    .cmp(&sb.extinct.is_some())
                    .then_with(|| sb.total_influence().total_cmp(&sa.total_influence()))
                    .then_with(|| a.cmp(&b))
            });
            for s in ss {
                let sc = &w.schools[s];
                let realms = sc.influence.values().filter(|v| **v > 0.1).count();
                let status = match sc.extinct {
                    Some(y) => format!("gone {}", y),
                    None => format!("{:>5}", realms),
                };
                let row = format!(
                    "{:<36}{:<12}{:>6}  {:<30}{}",
                    clip(&sc.name, 35),
                    sc.kind.name(),
                    status,
                    words::school_reach(w, s),
                    w.cities[sc.home_city].name
                );
                rows.push((row, Ref::School(s)));
            }
        }
        4 => {
            let mut ps: Vec<usize> = (0..w.persons.len()).collect();
            // The living first, then by renown, then youngest first.
            total = keep_best(&mut ps, LIST_CAP, |&a, &b| {
                let (pa, pb) = (&w.persons[a], &w.persons[b]);
                pa.died
                    .is_some()
                    .cmp(&pb.died.is_some())
                    .then_with(|| pb.renown.total_cmp(&pa.renown))
                    .then_with(|| pb.born.cmp(&pa.born))
                    .then_with(|| a.cmp(&b))
            });
            for p in ps {
                let per = &w.persons[p];
                let realm = per
                    .polity
                    .map(|pp| w.polities[pp].short.clone())
                    .unwrap_or_default();
                let died = per
                    .died
                    .map(|y| y.to_string())
                    .unwrap_or_else(|| "alive".into());
                let row = format!(
                    "{:<30}{:<13}{:<22}{:>6}  {:>5}",
                    clip(&per.full_name(), 29),
                    per.role.name(),
                    clip(&realm, 21),
                    per.born,
                    died
                );
                rows.push((row, Ref::Person(p)));
            }
        }
        5 => {
            let mut ws: Vec<usize> = (0..w.wars.len()).collect();
            total = keep_best_by_key(&mut ws, LIST_CAP, |x| {
                (
                    w.wars[x].ended.is_some(),
                    std::cmp::Reverse(w.wars[x].started),
                )
            });
            for x in ws {
                let war = &w.wars[x];
                let years = match war.ended {
                    Some(e) => format!("{}-{}", war.started, e),
                    None => format!("{}-", war.started),
                };
                let row = format!(
                    "{:<42}{:<32}{}",
                    clip(&war.name, 41),
                    clip(
                        &format!(
                            "{} vs {}",
                            w.polities[war.attacker].short, w.polities[war.defender].short
                        ),
                        31
                    ),
                    years
                );
                rows.push((row, Ref::War(x)));
            }
        }
        7 => {
            let mut arts: Vec<usize> = (0..w.artifacts.len()).collect();
            total = keep_best_by_key(&mut arts, LIST_CAP, |a| {
                (
                    matches!(w.artifacts[a].holder, crate::sim::Holder::Lost),
                    w.artifacts[a].made,
                )
            });
            for a in arts {
                let ar = &w.artifacts[a];
                let row = format!(
                    "{:<40}{:<10}{:>5}   {}",
                    clip(&ar.name, 39),
                    ar.kind.word().to_lowercase(),
                    ar.made,
                    clip(&w.artifact_holder_name(a), 40)
                );
                rows.push((row, Ref::Artifact(a)));
            }
        }
        8 => {
            let mut prs: Vec<usize> = (0..w.prophecies.len()).collect();
            total = keep_best_by_key(&mut prs, LIST_CAP, |i| {
                (
                    w.prophecies[i].outcome.is_some(),
                    std::cmp::Reverse(w.prophecies[i].year),
                )
            });
            for i in prs {
                let pr = &w.prophecies[i];
                let outcome = match pr.outcome {
                    Some(true) => format!("came true {}", pr.resolved.unwrap_or(0)),
                    Some(false) => format!("failed {}", pr.resolved.unwrap_or(0)),
                    None => "pending".to_string(),
                };
                let row = format!(
                    "{:<66}{:<21}{:>6}  {}",
                    clip(&pr.what, 65),
                    clip(&w.persons[pr.seer].full_name(), 20),
                    pr.deadline,
                    outcome
                );
                rows.push((row, Ref::Person(pr.seer)));
            }
        }
        // The figures of the age: everyone the world has ever called great,
        // the living first. `Persons` lists everybody who was ever named;
        // this lists the handful who mattered, which is the list a reader
        // actually wants when they ask who is alive right now and why they
        // should care.
        9 => {
            let mut fs: Vec<usize> = (0..w.persons.len())
                .filter(|&i| w.persons[i].is_acclaimed() || w.persons[i].greatness >= 80.0)
                .collect();
            total = keep_best(&mut fs, FIGURES_CAP, |&a, &b| {
                let (pa, pb) = (&w.persons[a], &w.persons[b]);
                pa.died
                    .is_some()
                    .cmp(&pb.died.is_some())
                    .then_with(|| pb.greatness.total_cmp(&pa.greatness))
                    .then_with(|| a.cmp(&b))
            });
            for i in fs {
                let per = &w.persons[i];
                let realm = per
                    .polity
                    .map(|pp| w.polities[pp].short.clone())
                    .unwrap_or_default();
                let when = match per.died {
                    Some(y) => format!("{}-{}", per.born, y),
                    None => format!("{}-", per.born),
                };
                let deed = dynasty::standing(w, i)
                    .first()
                    .map(|c| c.what.clone())
                    .unwrap_or_default();
                let row = format!(
                    "{:<30}{:>5}  {:<20}{:<11}{}",
                    clip(&per.full_name(), 29),
                    per.greatness.round() as i32,
                    clip(&realm, 19),
                    when,
                    clip(&deed, 34)
                );
                rows.push((row, Ref::Person(i)));
            }
        }
        // Every ruling family the world has known, the living ones first
        // and the longest-lasting at the top of each group.
        10 => {
            let mut hs: Vec<usize> = (0..w.houses.len()).collect();
            total = keep_best(&mut hs, LIST_CAP, |&a, &b| {
                let (ha, hb) = (&w.houses[a], &w.houses[b]);
                ha.ended
                    .is_some()
                    .cmp(&hb.ended.is_some())
                    .then_with(|| hb.span(w.year).cmp(&ha.span(w.year)))
                    .then_with(|| a.cmp(&b))
            });
            for h in hs {
                let ho = &w.houses[h];
                let span = match ho.ended {
                    Some(y) => format!("{}-{}", ho.founded, y),
                    None => format!("{}-", ho.founded),
                };
                let thrones = ho
                    .realms
                    .iter()
                    .filter(|&&p| w.polities[p].alive() && w.polities[p].house == Some(h))
                    .count();
                let row = format!(
                    "{:<36}{:<17}{:>5}{:>9}  {}",
                    clip(&ho.name, 35),
                    clip(&w.cultures[ho.culture].plural, 16),
                    ho.seniors.len(),
                    thrones,
                    span
                );
                rows.push((row, Ref::House(h)));
            }
        }
        // What each realm is worth, richest first, with what it is gaining
        // or losing a year beside it: a realm with a full treasury and a
        // deficit is a different thing from one with the same treasury and
        // a surplus, and nothing in the interface said so.
        11 => {
            let mut ps: Vec<usize> = w.alive_polities.clone();
            // Treasury first, then what the year will bring. Most great
            // realms sit at the ceiling of what a treasury can hold, so
            // without the second key the page was ten realms tied at the
            // cap in arbitrary order — and a realm with a full treasury and
            // a deficit is a different thing from one with a surplus.
            total = keep_best_by_key(&mut ps, LIST_CAP, |p| {
                (
                    std::cmp::Reverse((w.polities[p].treasury * 100.0) as i64),
                    std::cmp::Reverse((explain::income_total(w, p) * 100.0) as i64),
                )
            });
            for p in ps {
                let pol = &w.polities[p];
                let prosp = if pol.cities.is_empty() {
                    0.0
                } else {
                    pol.cities
                        .iter()
                        .map(|&c| w.cities[c].prosperity)
                        .sum::<f32>()
                        / pol.cities.len() as f32
                };
                let row = format!(
                    "{:<34}{:>8.0}  {:>+7.1}  {:>5.2}  {:>6.1}  {:>7.0}",
                    clip(&pol.name, 33),
                    pol.treasury,
                    explain::income_total(w, p),
                    pol.dev,
                    w.realm_trade(p),
                    prosp * 100.0
                );
                rows.push((row, Ref::Polity(p)));
            }
        }
        // The roads themselves, busiest first. The Trade layer draws the
        // network; this is the same network as a list that can be sorted and
        // searched, which is the only way to find the one road a war has
        // shut.
        12 => {
            let mut rs: Vec<usize> = (0..w.routes.len())
                .filter(|&i| {
                    w.cities[w.routes[i].a].destroyed.is_none()
                        && w.cities[w.routes[i].b].destroyed.is_none()
                })
                .collect();
            total = keep_best_by_key(&mut rs, LIST_CAP, |i| {
                (
                    !w.routes[i].open,
                    std::cmp::Reverse((w.routes[i].value * 100.0) as i64),
                )
            });
            for i in rs {
                let r = &w.routes[i];
                let row = format!(
                    "{:<46}{:>7.1}  {:<16}{:>6}",
                    clip(
                        &format!(
                            "{} — {}{}",
                            w.cities[r.a].name,
                            w.cities[r.b].name,
                            if r.by_sea { " (by sea)" } else { "" }
                        ),
                        45
                    ),
                    r.value,
                    if r.open { "open" } else { "shut by war" },
                    r.since
                );
                rows.push((row, Ref::City(r.a)));
            }
        }
        // Every realm as a bar from its founding to its fall, the great
        // ones first and their successor states beneath them. A realm's
        // `parent` is recorded when it is founded, so a kingdom that broke
        // into four reads as a family rather than as four unrelated rows.
        13 => {
            let span = w.year.max(1) as f32;
            let mut ps: Vec<usize> = (0..w.polities.len()).collect();
            total = keep_best_by_key(&mut ps, LIST_CAP, |p| {
                std::cmp::Reverse(w.polities[p].peak_cells)
            });
            for (p, depth) in lineage_order(w, &ps) {
                let pol = &w.polities[p];
                let fell = pol.fell.unwrap_or(w.year);
                let indent = "  ".repeat(depth.min(4));
                let name = format!("{}{}", indent, clip(&pol.short, 29 - depth.min(4) * 2));
                let row = format!(
                    "{:<31}{} {:>5} @{:<6}{}",
                    name,
                    span_bar(pol.founded, fell, span, TIMELINE_WIDTH),
                    pol.peak_cells,
                    pol.peak_year,
                    match pol.fell {
                        Some(y) => format!("fell {}", y),
                        None => "standing".to_string(),
                    }
                );
                rows.push((row, Ref::Polity(p)));
            }
        }
        _ => {
            let mut fs: Vec<usize> = (0..w.terrain.features.len()).collect();
            total = keep_best_by_key(&mut fs, LIST_CAP, |f| {
                (
                    w.terrain.features[f].name.is_none(),
                    std::cmp::Reverse(w.terrain.features[f].cells.len()),
                )
            });
            for f in fs {
                let ft = &w.terrain.features[f];
                let row = format!("{:<40}{}", clip(&ft.display(), 39), ft.kind.label());
                rows.push((row, Ref::Feature(f)));
            }
        }
    }
    (rows, total)
}

fn clip(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// Follow a single-letter link from a detail page.
/// The page a bracketed link leads to, or `None` if this page has no link on
/// that letter.
///
/// The letter in the brackets is the key to press, so it has to be one the
/// page's own key handler does not already spend: `k` and `j` scroll, which
/// is why the capital, the home city and a relic's maker are `[K]`.
pub fn follow_link(w: &World, r: Ref, c: char) -> Option<Ref> {
    match r {
        Ref::Polity(p) => {
            let pol = &w.polities[p];
            match c {
                'r' => pol.ruler.map(Ref::Person),
                'c' => Some(Ref::Culture(pol.culture)),
                'K' => pol.capital.map(Ref::City),
                's' => pol.school.map(Ref::School),
                'p' => pol.parent.map(Ref::Polity),
                'w' => pol.wars.first().map(|&x| Ref::War(x)),
                'H' => pol.house.map(Ref::House),
                'O' => pol.overlord.map(Ref::Polity),
                _ => None,
            }
        }
        Ref::City(ci) => {
            let city = &w.cities[ci];
            match c {
                'p' => city.polity.map(Ref::Polity),
                'c' => Some(Ref::Culture(city.culture)),
                _ => None,
            }
        }
        Ref::Culture(cu) => match c {
            'p' => w.cultures[cu].parent.map(Ref::Culture),
            'a' => Some(Ref::Race(w.cultures[cu].race)),
            _ => None,
        },
        Ref::School(s) => {
            let sc = &w.schools[s];
            match c {
                'f' => Some(Ref::Person(sc.founder)),
                'K' => Some(Ref::City(sc.home_city)),
                'p' => sc.parent.map(Ref::School),
                _ => None,
            }
        }
        Ref::Person(pi) => {
            let per = &w.persons[pi];
            match c {
                'p' => per.polity.map(Ref::Polity),
                'c' => Some(Ref::Culture(per.culture)),
                's' => per.school.map(Ref::School),
                'f' => per.parent.map(Ref::Person),
                'h' => w.children_of(pi).first().copied().map(Ref::Person),
                'H' => per.house.map(Ref::House),
                'm' => per.spouse.map(Ref::Person),
                'v' => per.served.map(Ref::Person),
                _ => None,
            }
        }
        Ref::War(x) => match c {
            'a' => Some(Ref::Polity(w.wars[x].attacker)),
            'd' => Some(Ref::Polity(w.wars[x].defender)),
            _ => None,
        },
        Ref::House(h) => {
            let ho = &w.houses[h];
            match c {
                'c' => Some(Ref::Culture(ho.culture)),
                'f' => ho.founder.map(Ref::Person),
                'b' => ho.parent.map(Ref::House),
                _ => None,
            }
        }
        Ref::Artifact(a) => {
            let ar = &w.artifacts[a];
            match c {
                'h' => match ar.holder {
                    crate::sim::Holder::Polity(p) => Some(Ref::Polity(p)),
                    crate::sim::Holder::Person(per) => Some(Ref::Person(per)),
                    crate::sim::Holder::City(ci) => Some(Ref::City(ci)),
                    crate::sim::Holder::Lost => None,
                },
                'K' => ar.maker.map(Ref::Person),
                'o' => ar.origin.map(Ref::Polity),
                _ => None,
            }
        }
        _ => None,
    }
}

fn history(w: &World, r: Ref, width: usize, out: &mut Vec<Line>, limit: usize) {
    let ids = w.chronicle.for_ref(r);
    out.push(line("", FG, 0));
    out.push(line(
        format!(
            "History ({})",
            crate::sim::prose::count(ids.len() as i64, "entry")
        ),
        ACCENT,
        BOLD,
    ));
    let start = ids.len().saturating_sub(limit);
    if start > 0 {
        out.push(line(
            format!("  … {} earlier entries omitted", start),
            DIMC,
            DIM,
        ));
    }
    for &id in &ids[start..] {
        let e = &w.chronicle.events[id];
        let (c, a) = super::event_style(e.kind, e.importance);
        for (k, l) in term::wrap(&e.text, width.saturating_sub(8))
            .into_iter()
            .enumerate()
        {
            let prefix = if k == 0 {
                format!("{:>5}  ", e.year)
            } else {
                "       ".into()
            };
            out.push(line(format!("{}{}", prefix, l), c, a));
        }
    }
}

/// Every line of the page for `r`, laid out for `width` columns.
///
/// One function per kind of thing below, because the pages have almost
/// nothing in common beyond their shape: a title, some labelled fields,
/// the bracketed links out, and the entity's own history at the end.
pub fn detail_lines(w: &World, r: Ref, width: usize, ascii: bool) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::new();
    let w2 = width.saturating_sub(2);
    match r {
        Ref::Polity(p) => realm_page(w, p, r, width, w2, ascii, &mut out),
        Ref::City(ci) => city_page(w, ci, r, width, &mut out),
        Ref::Culture(cu) => people_page(w, cu, r, width, w2, &mut out),
        Ref::School(s) => school_page(w, s, r, width, w2, ascii, &mut out),
        Ref::Person(pi) => person_page(w, pi, r, width, w2, ascii, &mut out),
        Ref::War(x) => war_page(w, x, r, width, w2, ascii, &mut out),
        Ref::Race(x) => race_page(w, x, r, width, w2, &mut out),
        Ref::Artifact(a) => relic_page(w, a, r, width, w2, ascii, &mut out),
        Ref::Feature(f) => place_page(w, f, r, width, w2, &mut out),
        Ref::House(h) => house_page(w, h, r, width, w2, ascii, &mut out),
    }
    out
}

/// The page for a realm.
fn realm_page(
    w: &World,
    p: usize,
    r: Ref,
    width: usize,
    w2: usize,
    ascii: bool,
    out: &mut Vec<Line>,
) {
    let pol = &w.polities[p];
    out.push(line(pol.name.to_uppercase(), pol.color, BOLD));
    // The article has to agree with whatever word comes next, which is an
    // adjective coined from a language that knows nothing about English.
    let status = match pol.fell {
        Some(y) => format!(
            "{} that {}; founded {}, fell {}.",
            crate::sim::prose::cap_a(pol.kind.name()),
            pol.fall_cause.trim_end_matches('.'),
            pol.founded,
            y
        ),
        None => format!(
            "{} {} founded in year {}.",
            crate::sim::prose::cap_a(&w.cultures[pol.culture].adj),
            pol.kind.name(),
            pol.founded
        ),
    };
    for l in term::wrap(&status, w2) {
        out.push(line(l, FG, 0));
    }
    out.push(line("", FG, 0));
    if pol.alive() {
        out.push(line(format!("[r] Ruler     {}", w.ruler_title(p)), LINK, 0));
        if let Some(rl) = pol.ruler {
            let per = &w.persons[rl];
            out.push(line(
                format!(
                    "              {} · age {} · {} years on the throne",
                    per.traits.describe(),
                    w.year - per.born,
                    (w.year - pol.reign_start).max(0)
                ),
                DIMC,
                0,
            ));
        }
        if !pol.dynasty.is_empty() {
            let label = if pol.house.is_some() {
                "[H] House    "
            } else {
                "    Dynasty  "
            };
            out.push(line(
                format!("{} {}", label, pol.dynasty),
                if pol.house.is_some() { LINK } else { FG },
                0,
            ));
        }
    }
    out.push(line(
        format!(
            "[c] People    the {} ({} folk), who speak {}",
            w.cultures[pol.culture].plural,
            w.races[w.cultures[pol.culture].race].adj,
            w.cultures[pol.culture].lang.name
        ),
        LINK,
        0,
    ));
    if let Some(cap) = pol.capital {
        out.push(line(
            format!(
                "[K] Capital   {} ({:.1}k)",
                w.cities[cap].name, w.cities[cap].pop
            ),
            LINK,
            0,
        ));
    }
    if let Some(s) = pol.school {
        out.push(line(
            format!(
                "[s] Doctrine  {} ({})",
                w.schools[s].name,
                w.schools[s].kind.name()
            ),
            LINK,
            0,
        ));
    }
    if let Some(pp) = pol.parent {
        out.push(line(
            format!("[p] Sprang from {}", w.polities[pp].name),
            LINK,
            0,
        ));
    }
    if pol.alive() {
        out.push(line("", FG, 0));
        out.push(line(
            format!(
                "    Lands     {} — {} (peak {} in year {})   Cities {}",
                pol.cells,
                words::realm_size(pol.cells),
                pol.peak_cells,
                pol.peak_year,
                pol.cities.len()
            ),
            FG,
            0,
        ));
        out.push(line(
            format!(
                "    People    {}   Army {:.1} ({})   Treasury {:.0} ({})   Development {:.2}",
                words::folk(pol.pop as f32),
                pol.army,
                words::army_standing(w, p),
                pol.treasury,
                words::treasury(pol.treasury),
                pol.dev
            ),
            FG,
            0,
        ));
        // The word for where it is going comes from the same target the Why
        // block below totals up, and says the number, so the two lines
        // cannot contradict one another.
        out.push(line(
            format!(
                "    Stability {} {} ({:.0}%), {}",
                bar(pol.stability, 20, ascii),
                words::stability(pol.stability),
                pol.stability * 100.0,
                words::stability_trend(
                    explain::stability_drift(w, p),
                    explain::stability_target(w, p)
                )
            ),
            stability_color(pol.stability),
            0,
        ));
        // Why it stands where it does: the same weights the
        // simulation uses each year, ranked and in plain words.
        let factors = explain::stability_factors(w, p);
        if !factors.is_empty() {
            out.push(line("", FG, 0));
            out.push(line("Why", ACCENT, BOLD));
            for f in factors.iter().take(8) {
                let (mark, c) = if f.weight > 0.0 {
                    (if ascii { '+' } else { '▲' }, Rgb(150, 210, 150))
                } else {
                    (if ascii { '-' } else { '▼' }, Rgb(230, 150, 120))
                };
                for (k, l) in term::wrap(&f.text, w2.saturating_sub(18))
                    .into_iter()
                    .enumerate()
                {
                    let pre = if k == 0 {
                        format!("  {} {:>5}  ", mark, format!("{:+.0}", f.weight * 100.0))
                    } else {
                        "           ".to_string()
                    };
                    out.push(line(format!("{}{}", pre, l), c, 0));
                }
            }
            out.push(line(
                format!(
                    "  everything together pulls stability towards {:.0}%",
                    explain::stability_target(w, p) * 100.0
                ),
                DIMC,
                DIM,
            ));
            out.push(line("", FG, 0));
        }
        // And where the money comes from. Stability had an explanation from
        // the beginning and money never did, which left the two economic
        // entries in the block above — the full treasury, the prosperous
        // cities — as the only visible economics in the game, both of them
        // effects with no stated cause.
        let money = explain::income_shown(w, p, 5);
        if !money.is_empty() {
            out.push(line("Money", ACCENT, BOLD));
            // Every line of it: `income_factors` already trims the
            // revenues and keeps all the costs, so the figures shown add
            // up to the total printed beneath them.
            for f in money.iter() {
                let (mark, c) = if f.weight > 0.0 {
                    (if ascii { '+' } else { '▲' }, Rgb(230, 200, 120))
                } else {
                    (if ascii { '-' } else { '▼' }, Rgb(200, 140, 120))
                };
                for (k, l) in term::wrap(&f.text, w2.saturating_sub(18))
                    .into_iter()
                    .enumerate()
                {
                    let pre = if k == 0 {
                        format!("  {} {:>6}  ", mark, format!("{:+.2}", f.weight))
                    } else {
                        "            ".to_string()
                    };
                    out.push(line(format!("{}{}", pre, l), c, 0));
                }
            }
            let net = explain::income_total(w, p);
            let years_to_empty = if net < -0.01 {
                let t = w.polities[p].treasury;
                if t > 0.0 {
                    format!(", and empty in {:.0} years at that rate", t / -net)
                } else {
                    ", and already in debt".to_string()
                }
            } else {
                String::new()
            };
            out.push(line(
                format!(
                    "  the treasury {} about {:.2} a year{}",
                    if net >= 0.0 { "gains" } else { "loses" },
                    net.abs(),
                    years_to_empty
                ),
                DIMC,
                DIM,
            ));
            out.push(line("", FG, 0));
        }
        // Four separate pressures, so no single bar can stand for them: the
        // one that used to sit here drew from overextension alone and was
        // full for any realm past its reach, above three numbers it said
        // nothing about.
        out.push(line(
            format!(
                "    Strain    overextension {:.0}% of what the crown can govern",
                pol.overextension(&w.tuning) * 100.0
            ),
            FG,
            0,
        ));
        out.push(line(
            format!(
                "              foreign subjects {:.0}%   war-weariness {:.0}%   decadence {:.0}%",
                pol.foreign_share * 100.0,
                pol.exhaustion * 100.0,
                pol.decadence * 100.0
            ),
            FG,
            0,
        ));
        let mut flags = Vec::new();
        if pol.seafaring {
            flags.push("seafaring");
        }
        if pol.cultures_within > 1 {
            flags.push("many peoples");
        }
        if !flags.is_empty() {
            out.push(line(format!("    Notes     {}", flags.join(", ")), DIMC, 0));
        }
        if !pol.wars.is_empty() {
            out.push(line("", FG, 0));
            for &wid in &pol.wars {
                let war = &w.wars[wid];
                let other = if war.attacker == p {
                    war.defender
                } else {
                    war.attacker
                };
                let lean = if (war.score > 0.0) == (war.attacker == p) {
                    "winning"
                } else {
                    "losing"
                };
                out.push(line(
                    format!(
                        "[w] At war    {} against {} since {} ({}, {} battles)",
                        war.name, w.polities[other].name, war.started, lean, war.battles
                    ),
                    Rgb(240, 110, 100),
                    0,
                ));
            }
        }
        // What this realm knows. The count alone is the headline; the most
        // recent few say what kind of realm it is becoming.
        let known = w.tech_count(p);
        if known > 0 {
            let mut held = w.tech_list(p);
            held.sort_by_key(|&t| std::cmp::Reverse(w.tech_first_year[t]));
            let recent: Vec<String> = held
                .iter()
                .take(3)
                .map(|&t| format!("{} ({})", w.techs[t].name, w.techs[t].effect.label()))
                .collect();
            out.push(line(
                format!("    Knows     {} of {} arts", known, w.techs.len()),
                FG,
                0,
            ));
            labelled(out, "    Lately", &recent.join(", "), w2, DIMC);
        }
        // How the throne passes is the single most consequential thing
        // about a realm that cannot be seen on the map: it decides whether
        // a conqueror's work survives them.
        out.push(line(
            format!(
                "    Succession {} — {}",
                pol.inheritance.name(),
                pol.inheritance.describe()
            ),
            DIMC,
            0,
        ));
        if !pol.heirs.is_empty() {
            let names: Vec<String> = pol
                .heirs
                .iter()
                .take(5)
                .map(|&h| {
                    let age = w.persons[h].age(w.year);
                    format!("{} ({})", w.persons[h].name, age)
                })
                .collect();
            labelled(out, "    Heirs     ", &names.join(", "), w2, DIMC);
        }
        // Standing arrangements, which the border colours cannot show.
        if let Some(o) = pol.overlord {
            out.push(line(
                format!("[O] Overlord   {}", w.polities[o].name),
                Rgb(240, 160, 90),
                0,
            ));
        }
        if !pol.tributaries.is_empty() {
            let names: Vec<String> = pol
                .tributaries
                .iter()
                .filter(|&&t| w.polities[t].alive())
                .map(|&t| w.polities[t].short.clone())
                .collect();
            if !names.is_empty() {
                labelled(
                    out,
                    "    Tributaries",
                    &names.join(", "),
                    w2,
                    Rgb(240, 200, 120),
                );
            }
        }
        let allies: Vec<String> = pol
            .stance
            .iter()
            .filter(|&(&q, &st)| st == Stance::Allied && w.polities[q].alive())
            .map(|(&q, _)| w.polities[q].short.clone())
            .collect();
        if !allies.is_empty() {
            labelled(
                out,
                "    Allied with",
                &allies.join(", "),
                w2,
                Rgb(120, 210, 150),
            );
        }
        let kin: Vec<String> = pol
            .stance
            .iter()
            .filter(|&(&q, &st)| st == Stance::Married && w.polities[q].alive())
            .map(|(&q, _)| w.polities[q].short.clone())
            .collect();
        if !kin.is_empty() {
            labelled(
                out,
                "    Married to",
                &kin.join(", "),
                w2,
                Rgb(200, 170, 220),
            );
        }
        let mut nb: Vec<(usize, u32)> = pol.neighbors.clone();
        nb.sort_by_key(|&(_, l)| std::cmp::Reverse(l));
        if !nb.is_empty() {
            let s: Vec<String> = nb
                .iter()
                .take(6)
                .map(|&(q, _)| {
                    // A stance outranks a temperature: an ally is an ally
                    // even in a year when the border is tense.
                    let mood = match pol.stance.get(&q).copied().unwrap_or(Stance::Neutral) {
                        // A declared arrangement outranks a temperature: an
                        // ally is an ally even in a tense year.
                        st @ (Stance::Allied | Stance::Rival) => st.name().to_string(),
                        Stance::Married => "kin".to_string(),
                        Stance::Neutral => {
                            let t = pol.tension.get(&q).copied().unwrap_or(0.0);
                            if t > 0.5 {
                                "hostile"
                            } else if t > 0.3 {
                                "wary"
                            } else {
                                "calm"
                            }
                            .to_string()
                        }
                    };
                    format!("{} ({})", w.polities[q].short, mood)
                })
                .collect();
            labelled(out, "    Bordering", &s.join(", "), w2, DIMC);
        }
        if !pol.cities.is_empty() {
            let names: Vec<String> = pol
                .cities
                .iter()
                .map(|&c| format!("{} {:.0}k", w.cities[c].name, w.cities[c].pop))
                .collect();
            labelled(out, "    Cities", &names.join(", "), w2, DIMC);
        }
        let schools: Vec<String> = w
            .schools
            .iter()
            .filter(|s| s.alive())
            .filter_map(|s| s.influence.get(&p).map(|v| (s, *v)))
            .filter(|(_, v)| *v > 0.1)
            .map(|(s, v)| format!("{} {:.0}%", s.short, v * 100.0))
            .collect();
        if !schools.is_empty() {
            labelled(out, "    Beliefs", &schools.join(", "), w2, DIMC);
        }
        let relics: Vec<String> = w
            .artifacts_of(p)
            .iter()
            .map(|&a| w.artifacts[a].name.clone())
            .collect();
        if !relics.is_empty() {
            labelled(out, "    Relics", &relics.join(", "), w2, Rgb(255, 200, 80));
        }
        let pending: Vec<String> = w.prophecies.iter().filter(|pr| pr.outcome.is_none() && matches!(pr.kind, crate::sim::ProphecyKind::RealmFalls(q) | crate::sim::ProphecyKind::CrownOfEmpire(q) | crate::sim::ProphecyKind::RulerMurdered(q) | crate::sim::ProphecyKind::RelicReturns(_, q) if q == p)).map(|pr| format!("{} (by {})", pr.what, pr.deadline)).collect();
        if !pending.is_empty() {
            labelled(
                out,
                "    Foretold",
                &pending.join("; "),
                w2,
                Rgb(200, 140, 240),
            );
        }
        out.push(line("", FG, 0));
        out.push(line(
            "[x] Intervene with the Hand of Fate",
            Rgb(200, 140, 240),
            0,
        ));
    }
    if pol.rulers.len() > 1 {
        out.push(line("", FG, 0));
        out.push(line("Rulers", ACCENT, BOLD));
        for &rl in pol.rulers.iter().rev().take(12) {
            let per = &w.persons[rl];
            let end = per
                .died
                .map(|y| y.to_string())
                .unwrap_or_else(|| "now".into());
            out.push(line(
                format!(
                    "  {:<34} {}",
                    per.full_name(),
                    if per.died.is_some() {
                        format!("{}: {}", end, per.death)
                    } else {
                        "reigning".into()
                    }
                ),
                FG,
                0,
            ));
        }
    }
    history(w, r, width, out, 400);
}

/// The page for a city.
fn city_page(w: &World, ci: usize, r: Ref, width: usize, out: &mut Vec<Line>) {
    let w2 = width.saturating_sub(2);
    let city = &w.cities[ci];
    out.push(line(city.name.to_uppercase(), Rgb(255, 255, 255), BOLD));
    let mut desc = format!(
        "A city of the {}, founded in year {}",
        w.cultures[city.culture].plural, city.founded
    );
    if let Some(y) = city.destroyed {
        desc.push_str(&format!(", destroyed in year {}", y));
    }
    desc.push('.');
    out.push(line(desc, FG, 0));
    out.push(line(
        format!("    Terrain   {}", w.terrain.describe_cell(city.cell)),
        DIMC,
        0,
    ));
    // What its own country yields, and what passes through. A city's wealth
    // is as much about where it sits as what grows around it, and this is
    // where a reader can see which of the two it is living on.
    let goods = w.city_goods(ci);
    if !goods.is_empty() {
        labelled(
            out,
            "    Yields",
            &crate::sim::prose::city_produces(&goods),
            w2,
            DIMC,
        );
    }
    let partners = w.trade_partners(ci);
    if !partners.is_empty() {
        let takings = w.city_trade(ci);
        out.push(line(
            format!(
                "    Trade     {:.1} a year over {}",
                takings,
                crate::sim::prose::count(partners.len() as i64, "road")
            ),
            Rgb(230, 200, 120),
            0,
        ));
        let names: Vec<String> = partners
            .iter()
            .take(4)
            .map(|&(c, v, sea)| {
                format!(
                    "{} ({:.1}{})",
                    w.cities[c].name,
                    v,
                    if sea { ", by sea" } else { "" }
                )
            })
            .collect();
        labelled(out, "    With", &names.join(", "), w2, DIMC);
        // A city that lives on the carrying trade rather than its own land.
        if let Some(p) = city.polity {
            if takings > city.pop * 0.35 && takings > 6.0 {
                for l in term::wrap(&crate::sim::prose::city_entrepot(w, ci, p), w2) {
                    out.push(line(l, Rgb(230, 200, 120), 0));
                }
            }
        }
    }
    out.push(line("", FG, 0));
    if let Some(p) = city.polity {
        let cap = if w.polities[p].capital == Some(ci) {
            " (capital)"
        } else {
            ""
        };
        out.push(line(
            format!("[p] Realm     {}{}", w.polities[p].name, cap),
            LINK,
            0,
        ));
    }
    out.push(line(
        format!("[c] People    the {}", w.cultures[city.culture].plural),
        LINK,
        0,
    ));
    out.push(line(
        format!(
            "    Size      {} of {} people (at its greatest {})",
            words::capitalize(words::city_size(city.pop)),
            words::folk(city.pop),
            words::folk(city.peak_pop)
        ),
        FG,
        0,
    ));
    out.push(line(
        format!(
            // An index where a hundred is an ordinary town, not a share of
            // anything: prosperity runs to two and a half, so printing it
            // with a percent sign produced "194% prosperity", which invites
            // a reader to wonder 194% of what.
            "    Fortunes  {} (prosperity {:.0}, ordinary is 100)   walls {}   sacked {}",
            if city.prosperity > 0.8 {
                "thriving"
            } else if city.prosperity > 0.45 {
                "comfortable"
            } else if city.prosperity > 0.25 {
                "getting by"
            } else {
                "wretched"
            },
            city.prosperity * 100.0,
            if city.walls > 0.6 {
                "strong"
            } else if city.walls > 0.3 {
                "serviceable"
            } else {
                "thin"
            },
            crate::sim::prose::count(city.times_sacked as i64, "time")
        ),
        FG,
        0,
    ));
    // The hand of fate reaches towns now, so the page that describes one
    // should say so: a reader who has never pressed `x` will not guess that
    // it does anything here.
    out.push(line("[x] Intervene with the Hand of Fate", DIMC, DIM));
    for wn in &city.wonders {
        out.push(line(format!("    Wonder    {}", wn), ACCENT, 0));
    }
    let schools: Vec<String> = w
        .schools
        .iter()
        .filter(|s| s.home_city == ci)
        .map(|s| s.name.clone())
        .collect();
    if !schools.is_empty() {
        out.push(line(
            format!("    Birthplace of {}", schools.join(", ")),
            Rgb(200, 140, 240),
            0,
        ));
    }
    history(w, r, width, out, 300);
}

/// The page for a people.
/// A people's bent, as a sentence: what they take to and what passes them by.
fn bent_line(w: &World, cu: usize) -> String {
    use crate::sim::tech::FIELDS;
    let mut fields: Vec<(crate::sim::tech::Field, f32)> = FIELDS
        .into_iter()
        .map(|f| (f, w.cultures[cu].bent(f)))
        .collect();
    fields.sort_by(|a, b| b.1.total_cmp(&a.1));
    let strong: Vec<&str> = fields.iter().take(2).map(|&(f, _)| f.name()).collect();
    let weak: Vec<&str> = fields
        .iter()
        .rev()
        .take(2)
        .map(|&(f, _)| f.name())
        .collect();
    format!(
        "take to {} and {}; {} and {} have never much interested them",
        strong[0], strong[1], weak[0], weak[1]
    )
}

fn people_page(w: &World, cu: usize, r: Ref, width: usize, w2: usize, out: &mut Vec<Line>) {
    let c = &w.cultures[cu];
    out.push(line(
        format!("THE {}", c.plural.to_uppercase()),
        c.color,
        BOLD,
    ));
    let race = &w.races[c.race];
    let mut desc = format!(
        "{} people. They speak {} and call themselves {}.",
        crate::sim::prose::cap_a(&race.adj),
        c.lang.name,
        c.name
    );
    if let Some(y) = c.extinct {
        desc.push_str(&format!(" They vanished from the world in year {}.", y));
    }
    for l in term::wrap(&desc, w2) {
        out.push(line(l, FG, 0));
    }
    for l in term::wrap(
        &format!("The {} are {}.", race.plural, race.description),
        w2,
    ) {
        out.push(line(l, DIMC, 0));
    }
    // What this people takes to. It decides what they will ever work out
    // and what they will refuse to copy from a neighbour, so it is as much
    // a fact about them as their language.
    for l in term::wrap(&format!("They {}.", bent_line(w, cu)), w2) {
        out.push(line(l, ACCENT, 0));
    }
    out.push(line("", FG, 0));
    out.push(line(format!("[a] Race      the {}", race.plural), LINK, 0));
    if let Some(p) = c.parent {
        out.push(line(
            format!("[p] Descended from the {}", w.cultures[p].plural),
            LINK,
            0,
        ));
    }
    let v = c.values;
    out.push(line(format!("    Values    militarism {:.0}%  mysticism {:.0}%  trade {:.0}%  tradition {:.0}%  openness {:.0}%", v.militarism * 100.0, v.mysticism * 100.0, v.mercantilism * 100.0, v.tradition * 100.0, v.openness * 100.0), FG, 0));
    out.push(line(
        format!(
            "    Lands     {}   People {:.0}k   Since year {}",
            c.cells, c.pop, c.founded
        ),
        FG,
        0,
    ));
    let realms: Vec<String> = w
        .polities
        .iter()
        .filter(|p| p.alive() && p.culture == cu)
        .map(|p| p.name.clone())
        .collect();
    if !realms.is_empty() {
        labelled(out, "    Realms", &realms.join(", "), w2, DIMC);
    }
    let children: Vec<String> = w
        .cultures
        .iter()
        .filter(|x| x.parent == Some(cu))
        .map(|x| x.plural.clone())
        .collect();
    if !children.is_empty() {
        labelled(out, "    Offshoots", &children.join(", "), w2, DIMC);
    }
    out.push(line("", FG, 0));
    let sample: Vec<String> = (0..6).map(|_| c.lang.place(&w.rng)).collect();
    out.push(line(
        format!("    Names in {}: {}", c.lang.name, sample.join(", ")),
        DIMC,
        0,
    ));
    history(w, r, width, out, 200);
}

/// The page for a school of thought.
fn school_page(
    w: &World,
    s: usize,
    r: Ref,
    width: usize,
    w2: usize,
    ascii: bool,
    out: &mut Vec<Line>,
) {
    let sc = &w.schools[s];
    out.push(line(sc.name.to_uppercase(), sc.color, BOLD));
    let mut desc = format!(
        "{} {} founded in year {} in {}; it {}.",
        article(sc.kind.name()),
        sc.kind.name(),
        sc.founded,
        w.cities[sc.home_city].name,
        sc.doctrine
    );
    if let Some(y) = sc.extinct {
        desc.push_str(&format!(
            " Its last {} died in year {}.",
            sc.kind.follower(),
            y
        ));
    }
    for l in term::wrap(&desc, w2) {
        out.push(line(l, FG, 0));
    }
    out.push(line("", FG, 0));
    out.push(line(
        format!("[f] Founder   {}", w.persons[sc.founder].full_name()),
        LINK,
        0,
    ));
    out.push(line(
        format!("[K] Home      {}", w.cities[sc.home_city].name),
        LINK,
        0,
    ));
    if let Some(p) = sc.parent {
        out.push(line(
            format!("[p] Schism of {}", w.schools[p].name),
            LINK,
            0,
        ));
    }
    out.push(line(
        format!(
            "    Reach     {} — followed in {} realms, {} of them by law",
            words::capitalize(words::school_reach(w, s)),
            sc.influence
                .iter()
                .filter(|(&p, &v)| v > 0.1 && w.polities[p].alive())
                .count(),
            w.polities
                .iter()
                .filter(|p| p.alive() && p.school == Some(s))
                .count()
        ),
        FG,
        0,
    ));
    out.push(line("", FG, 0));
    out.push(line("Tenets", ACCENT, BOLD));
    for t in &sc.tenets {
        for l in term::wrap(&format!("  “{}”", t), w2) {
            out.push(line(l, FG, 0));
        }
    }
    out.push(line("", FG, 0));
    let mut infl: Vec<(usize, f32)> = sc
        .influence
        .iter()
        .map(|(&p, &v)| (p, v))
        .filter(|(p, v)| *v > 0.05 && w.polities[*p].alive())
        .collect();
    infl.sort_by(|a, b| b.1.total_cmp(&a.1));
    out.push(line(
        format!("Following (hostility {:.0}%)", sc.hostility * 100.0),
        ACCENT,
        BOLD,
    ));
    for (p, v) in infl.iter().take(15) {
        let state = if w.polities[*p].school == Some(s) {
            " (state doctrine)"
        } else {
            ""
        };
        // The percentage is right-aligned so the realm names start in
        // one column rather than three.
        out.push(line(
            format!(
                "  {} {:>4}%  {}{}",
                bar(*v, 10, ascii),
                (v * 100.0).round() as i32,
                w.polities[*p].name,
                state
            ),
            FG,
            0,
        ));
    }
    let children: Vec<String> = w
        .schools
        .iter()
        .filter(|x| x.parent == Some(s))
        .map(|x| x.name.clone())
        .collect();
    if !children.is_empty() {
        for l in term::wrap(&format!("  Offshoots: {}", children.join(", ")), w2) {
            out.push(line(l, DIMC, 0));
        }
    }
    history(w, r, width, out, 200);
}

/// The page for a person.
fn person_page(
    w: &World,
    pi: usize,
    r: Ref,
    width: usize,
    w2: usize,
    ascii: bool,
    out: &mut Vec<Line>,
) {
    let per = &w.persons[pi];
    out.push(line(per.full_name().to_uppercase(), FG, BOLD));
    let mut desc = format!(
        "{} {} {} of the {}, born in year {}",
        article(&per.traits.describe()),
        per.traits.describe(),
        per.role.name(),
        w.cultures[per.culture].plural,
        per.born
    );
    match per.died {
        Some(y) => desc.push_str(&format!(
            "; {} in year {} at the age of {}.",
            per.death.trim_end_matches('.'),
            y,
            y - per.born
        )),
        None => desc.push_str(&format!(", now aged {}.", w.year - per.born)),
    }
    for l in term::wrap(&desc, w2) {
        out.push(line(l, FG, 0));
    }
    if per.alive() {
        out.push(line("[x] Intervene with the Hand of Fate", DIMC, DIM));
    }
    for l in term::wrap(&explain::standing(w, pi), w2) {
        out.push(line(l, ACCENT, 0));
    }
    if let Some(t) = &per.title {
        out.push(line(
            format!("    Styled    {}", t),
            Rgb(255, 210, 90),
            BOLD,
        ));
    }
    if let Some(y) = per.acclaimed {
        out.push(line(
            format!(
                "    Acclaimed in year {}, at the age of {}",
                y,
                (y - per.born).max(0)
            ),
            Rgb(255, 210, 90),
            0,
        ));
    }
    out.push(line("", FG, 0));
    if let Some(p) = per.polity {
        out.push(line(
            format!("[p] Realm     {}", w.polities[p].name),
            LINK,
            0,
        ));
    }
    out.push(line(
        format!("[c] People    the {}", w.cultures[per.culture].plural),
        LINK,
        0,
    ));
    if let Some(s) = per.school {
        out.push(line(
            format!("[s] School    {}", w.schools[s].name),
            LINK,
            0,
        ));
    }
    if let Some(h) = per.house {
        out.push(line(format!("[H] House     {}", w.houses[h].name), LINK, 0));
    }
    if per.parent.is_some() {
        out.push(line(format!("[f] Family    {}", w.lineage(pi)), LINK, 0));
    }
    let kids = w.children_of(pi);
    if !kids.is_empty() {
        let names: Vec<String> = kids.iter().map(|&k| w.persons[k].full_name()).collect();
        labelled(out, "[h] Children", &names.join(", "), w2, LINK);
    }
    // Blood. What a person carries without showing is the thing a reader
    // following a house for two centuries is actually watching for.
    let carried = crate::sim::blood::carried(&per.genes);
    if !carried.is_empty() {
        let names: Vec<&str> = carried
            .iter()
            .map(|&t| crate::sim::blood::trait_name(t))
            .collect();
        labelled(
            out,
            "    Carries",
            &names.join(", "),
            w2,
            Rgb(200, 140, 240),
        );
    }
    if per.inbred > 0.12 {
        out.push(line(
            format!(
                "    Blood     parents of one line ({:.0}% shared), and {} for it",
                per.inbred * 100.0,
                if per.vigour < 0.85 {
                    "the weaker"
                } else {
                    "no weaker"
                }
            ),
            Rgb(220, 140, 130),
            0,
        ));
    }
    if let Some(sp) = per.spouse {
        out.push(line(
            format!("[m] Married   {}", w.persons[sp].full_name()),
            LINK,
            0,
        ));
    }
    if let Some(sv) = per.served {
        out.push(line(
            format!("[v] Served    {}", w.persons[sv].full_name()),
            LINK,
            0,
        ));
    }
    // What this life actually amounted to, in the same points the
    // simulation uses to decide whether the world calls them great. The
    // list is the greatness score itself, itemised, so a reader can see
    // exactly why one ruler is remembered and another is not.
    let deeds = dynasty::standing(w, pi);
    if !deeds.is_empty() {
        out.push(line("", FG, 0));
        out.push(line(
            format!("Standing — {:.0} points", per.greatness),
            ACCENT,
            BOLD,
        ));
        for d in deeds.iter().take(8) {
            let colour = if d.points >= 0.0 {
                FG
            } else {
                Rgb(220, 120, 110)
            };
            out.push(line(
                format!("  {:>+5.0}  {}", d.points, words::capitalize(&d.what)),
                colour,
                0,
            ));
        }
    }
    let held: Vec<String> = w
        .artifacts
        .iter()
        .filter(|ar| ar.holder == crate::sim::Holder::Person(pi))
        .map(|ar| ar.name.clone())
        .collect();
    if !held.is_empty() {
        labelled(out, "    Bears", &held.join(", "), w2, Rgb(255, 200, 80));
    }
    let prophecies: Vec<String> = w
        .prophecies
        .iter()
        .filter(|pr| pr.seer == pi)
        .map(|pr| {
            format!(
                "{} ({})",
                pr.what,
                match pr.outcome {
                    Some(true) => "came true",
                    Some(false) => "failed",
                    None => "pending",
                }
            )
        })
        .collect();
    if !prophecies.is_empty() {
        labelled(
            out,
            "    Foretold",
            &prophecies.join("; "),
            w2,
            Rgb(200, 140, 240),
        );
    }
    // Two rows of three, every column the same width, so the bars
    // line up under one another instead of drifting a space with the
    // length of the name beside them.
    let t = per.traits;
    let cell = |name: &str, v: f32| format!("{:<9}{}  ", name, bar(v, 8, ascii));
    for row in [
        [
            ("ambition", t.ambition),
            ("valor", t.valor),
            ("wisdom", t.wisdom),
        ],
        [
            ("piety", t.piety),
            ("cruelty", t.cruelty),
            ("charisma", t.charisma),
        ],
    ] {
        let text: String = row.iter().map(|&(n, v)| cell(n, v)).collect();
        out.push(line(format!("    {}", text.trim_end()), DIMC, 0));
    }
    history(w, r, width, out, 200);
}

/// The page for a war.
fn war_page(
    w: &World,
    x: usize,
    r: Ref,
    width: usize,
    w2: usize,
    ascii: bool,
    out: &mut Vec<Line>,
) {
    let war = &w.wars[x];
    out.push(line(war.name.to_uppercase(), Rgb(240, 110, 100), BOLD));
    let status = match war.ended {
        Some(e) => format!(
            "Fought {}-{} between {} and {}. It {}",
            war.started,
            e,
            w.polities[war.attacker].name,
            w.polities[war.defender].name,
            war.result
        ),
        None => format!(
            "Fought since {} between {} and {}. {} battles so far.",
            war.started, w.polities[war.attacker].name, w.polities[war.defender].name, war.battles
        ),
    };
    for l in term::wrap(&status, w2) {
        out.push(line(l, FG, 0));
    }
    for l in term::wrap(&format!("Cause: {}.", war.cause), w2) {
        out.push(line(l, DIMC, 0));
    }
    out.push(line("", FG, 0));
    out.push(line(
        format!("[a] Attacker  {}", w.polities[war.attacker].name),
        LINK,
        0,
    ));
    out.push(line(
        format!("[d] Defender  {}", w.polities[war.defender].name),
        LINK,
        0,
    ));
    // Who is ahead, why, and what a peace would cost.
    let v = explain::war_view(w, x);
    out.push(line("", FG, 0));
    out.push(line("Who is winning", ACCENT, BOLD));
    let lean = match v.leader {
        Some(p) if war.alive() => format!("  {} is ahead.", w.polities[p].name),
        Some(p) => format!("  {} had the better of it.", w.polities[p].name),
        None => "  Neither side has the better of it.".to_string(),
    };
    out.push(line(lean, FG, BOLD));
    // One bar pulled between the two sides, rather than two bars
    // the eye has to compare.
    let total = v.attacker_strength + v.defender_strength;
    let n = 24usize;
    let a = ((v.attacker_strength / total) * n as f32).round() as usize;
    let (mine, theirs) = if ascii { ('#', '-') } else { ('█', '▒') };
    out.push(line(
        format!(
            "  {} {}{} {}   ({:.0} against {:.0} in the field)",
            w.polities[war.attacker].short,
            mine.to_string().repeat(a.min(n)),
            theirs.to_string().repeat(n - a.min(n)),
            w.polities[war.defender].short,
            v.attacker_strength,
            v.defender_strength
        ),
        DIMC,
        0,
    ));
    for r in &v.reasons {
        for (k, l) in term::wrap(r, w2.saturating_sub(4)).into_iter().enumerate() {
            out.push(line(
                format!("{}{}", if k == 0 { "  · " } else { "    " }, l),
                FG,
                0,
            ));
        }
    }
    if war.alive() {
        out.push(line(
            format!(
                "  After {} years of it, {}.",
                (w.year - war.started).max(1),
                explain::war_weariness(w, x)
            ),
            DIMC,
            0,
        ));
        out.push(line("", FG, 0));
        out.push(line("If it ended now", ACCENT, BOLD));
        for l in term::wrap(&v.peace, w2.saturating_sub(2)) {
            out.push(line(format!("  {}", l), FG, 0));
        }
    }
    history(w, r, width, out, 300);
}

/// The page for a race.
fn race_page(w: &World, x: usize, r: Ref, width: usize, w2: usize, out: &mut Vec<Line>) {
    let race = &w.races[x];
    out.push(line(
        format!("THE {}", race.plural.to_uppercase()),
        ACCENT,
        BOLD,
    ));
    for l in term::wrap(
        &format!("The {} are {}.", race.plural, race.description),
        w2,
    ) {
        out.push(line(l, FG, 0));
    }
    out.push(line(
        format!("    Typical lifespan {} years.", race.lifespan as i32),
        DIMC,
        0,
    ));
    let peoples: Vec<String> = w
        .cultures
        .iter()
        .filter(|c| c.race == x && c.extinct.is_none())
        .map(|c| c.plural.clone())
        .collect();
    labelled(out, "    Peoples", &peoples.join(", "), w2, DIMC);
    history(w, r, width, out, 100);
}

/// The page for a relic.
fn relic_page(
    w: &World,
    a: usize,
    r: Ref,
    width: usize,
    w2: usize,
    ascii: bool,
    out: &mut Vec<Line>,
) {
    let ar = &w.artifacts[a];
    out.push(line(ar.name.to_uppercase(), Rgb(255, 200, 80), BOLD));
    let mut desc = format!(
        "{}, {}. Made in year {}",
        crate::sim::prose::cap_a(&ar.kind.word().to_lowercase()),
        ar.description,
        ar.made
    );
    if let Some(m) = ar.maker {
        desc.push_str(&format!(" in the time of {}", w.persons[m].full_name()));
    }
    desc.push_str(&format!("; it has changed hands {} times.", ar.hands));
    for l in term::wrap(&desc, w2) {
        out.push(line(l, FG, 0));
    }
    out.push(line("", FG, 0));
    // A lost relic has no holder to open, so it is not offered as a link.
    let held = w.artifact_holder_name(a);
    if ar.holder == crate::sim::Holder::Lost {
        out.push(line(format!("    Held by   {}", held), FG, 0));
    } else {
        out.push(line(format!("[h] Held by   {}", held), LINK, 0));
    }
    if let Some(m) = ar.maker {
        out.push(line(
            format!("[K] Maker     {}", w.persons[m].full_name()),
            LINK,
            0,
        ));
    }
    if let Some(o) = ar.origin {
        out.push(line(
            format!("[o] Origin    {}", w.polities[o].name),
            LINK,
            0,
        ));
    }
    out.push(line(
        format!("    Power     {}", bar(ar.power, 10, ascii)),
        DIMC,
        0,
    ));
    history(w, r, width, out, 200);
}

/// The page for a named place.
fn place_page(w: &World, f: usize, r: Ref, width: usize, w2: usize, out: &mut Vec<Line>) {
    let ft = &w.terrain.features[f];
    out.push(line(ft.display().to_uppercase(), LINK, BOLD));
    let mut desc = format!(
        "{} of {} cells.",
        crate::sim::prose::cap_a(ft.kind.label()),
        ft.cells.len()
    );
    if let Some(c) = ft.named_by {
        desc.push_str(&format!(" Named by the {}.", w.cultures[c].plural));
    }
    out.push(line(desc, FG, 0));
    let mut realms: Vec<String> = Vec::new();
    for &c in ft.cells.iter().step_by(3) {
        if let Some(p) = w.cells[c].owner {
            let n = w.polities[p].name.clone();
            if !realms.contains(&n) {
                realms.push(n);
            }
        }
    }
    if !realms.is_empty() {
        labelled(out, "    Realms", &realms.join(", "), w2, DIMC);
    }
    history(w, r, width, out, 100);
}

/// [`HELP`] folded to fit `width` columns, keeping each entry under its own
/// heading.
///
/// The table is written for a wide terminal; on a narrow one the lines used
/// to be chopped off mid-word, which is a poor showing for the page whose
/// whole job is to be read. A line that overflows is split at its gutter —
/// the heading column the continuation lines already use — and the remainder
/// is wrapped and re-indented to line up under it.
pub fn help_lines(width: usize) -> Vec<String> {
    /// The column the headings' text starts in, which is also the indent of
    /// their continuation lines.
    const GUTTER: usize = 13;
    // A heading line puts its name in the gutter and its text after it, so
    // the gutter column is blank. Anything else — the title, the opening
    // paragraph — is prose in its own right.
    let heading = |l: &str| {
        l.starts_with(|c: char| !c.is_whitespace())
            && l.is_char_boundary(GUTTER)
            && l[..GUTTER].ends_with(' ')
    };

    let mut out = Vec::new();
    let mut i = 0;
    while i < HELP.len() {
        if HELP[i].trim().is_empty() {
            out.push(String::new());
            i += 1;
            continue;
        }
        let first = HELP[i];
        let lead = first.len() - first.trim_start().len();
        let gutter = if heading(first) { GUTTER } else { lead };
        // An entry is its opening line plus every line indented to its gutter.
        let mut last = i + 1;
        while last < HELP.len()
            && !HELP[last].trim().is_empty()
            && !heading(HELP[last])
            && HELP[last].len() - HELP[last].trim_start().len() == gutter
        {
            last += 1;
        }
        let group = &HELP[i..last];
        i = last;
        // Wide enough for the table as it was written: use it unchanged, so
        // the hand-made alignment survives wherever it can.
        if group.iter().all(|l| l.chars().count() <= width) {
            out.extend(group.iter().map(|l| (*l).to_string()));
            continue;
        }
        let head: String = first.chars().take(gutter).collect();
        let body = group
            .iter()
            .enumerate()
            .map(|(k, l)| {
                if k == 0 {
                    l[gutter.min(l.len())..].trim()
                } else {
                    l.trim()
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        let pad = " ".repeat(gutter);
        for (k, part) in term::wrap(&body, width.saturating_sub(gutter))
            .into_iter()
            .enumerate()
        {
            out.push(format!("{}{}", if k == 0 { &head } else { &pad }, part));
        }
    }
    out
}

pub const HELP: &[&str] = &[
    "RISE AND FALL OF EMPIRES",
    "",
    "Learn        p player guide   t interactive tutorial   Esc back",
    "             Reopen these any time with :guide or :tutorial. Ctrl-g skips a tutorial.",
    "",
    "  A passive world simulator. Leave it running; peoples, realms, faiths and magical orders",
    "  rise and fall on their own. Look closer whenever you like. Keys work like vim: a number",
    "  before a motion repeats it (12j, 3], 5.).",
    "",
    "Time         Space pause    + - speed    . step a year    D detail    :until 900   :step 50",
    "             ? or F1 this page.  Counts work everywhere: 12j, 3], 5.",
    "",
    "Map          h j k l / arrows move (H L 8 cells, K J 4 rows, Shift+arrows the same;",
    "             Ctrl+arrows 20 and 10)   0 $ left and right edge   Home centre of the world",
    "             zz centre on cursor   zt cursor to the top   zi zo zoom in / out (or :zoom N)",
    "             Tab / Shift+Tab next / previous layer   Enter open   s select   Esc clear",
    "             gg or G jump to selection   f follow events   PageUp PageDown half a screen",
    "             ] [ next / previous realm   } { next / previous city   t go to the top story",
    "             r a recap of the last fifty years (of the selected realm or city, if selected)",
    "             v the event log's level: 1 everything, 2 the notable (the default), 3 the great",
    "             x the Hand of Fate: a realm, a town, a person or a region",
    "",
    "Search       /name finds realms, cities, people, peoples, schools, wars, places and relics;",
    "             Enter jumps, n N cycle.  In lists and the chronicle, / filters the rows instead.",
    "",
    "Commands     :w [name]  :e name  :saveas name  :saves  :autosave N   (saves live in",
    "             ~/.local/share/empires; --save FILE or --load FILE on the command line)",
    "             :speed 25  :layer culture  :detail high  :zoom 2  :theme paper  :find Velen",
    "             :new [seed]  :until YEAR  :step N  :follow on  :log 1-3  :mute battle  :story",
    "             :recap 100 (the last N years)   :legend (the key under the map)   :tour",
    "             :set key value  :map <from> <to>  :unmap key  :maps  :mkconfig  :config",
    "             :fate 3  :q  :wq  :q!",
    "             :export chronicle|map|timeline|realms|wealth|cities|roads|persons|wars|houses",
    "             (Markdown for the history, HTML for the map, CSV for a table)",
    "",
    "Browsing     e lists (realms, cities, peoples, schools, persons, wars, places, relics,",
    "             prophecies, figures, houses, wealth, roads)   In a list, / filters by name or",
    "             by number: lands>200, income<0, prosperity>=150. The footer says what a page",
    "             will answer to.   c the chronicle (f or v cycles importance 0-3,",
    "             / filters by text)   Figures are those the world called great; Houses are the",
    "             ruling families, each with its whole line of succession.",
    "             Everywhere: j k scroll, Ctrl-d Ctrl-u half a page, Ctrl-f Ctrl-b a page,",
    "             gg G top / bottom, m show on map, q or Esc back.",
    "             In a list: Tab or ] [ change tab, Enter opens, n N move, x the Hand of Fate.",
    "             On a page: the bracketed letter opens that link ([r] ruler, [c] people,",
    "             [K] capital, [f] family, [H] house, [m] spouse, [O] overlord), Backspace",
    "             retraces, Enter shows it on the map.",
    "",
    "Mouse        click selects, click again opens, wheel scrolls, clicking a chronicle line or",
    "             a sidebar entry jumps there.  --no-mouse or :mouse leaves it to the terminal.",
    "Config       ~/.config/empires/config: key = value settings (detail speed theme mouse ascii",
    "             autosave width height log follow zoom), tune.<field> for any balance constant,",
    "             and vim-style remaps: map <S-Up> K.  :mkconfig writes a commented template.",
    "Themes       default  phosphor  amber  paper  dusk   (:theme cycles)",
    "Quitting     :q  ZZ  Ctrl-C  or q twice on the map, all of which save if a save file is",
    "             set.  :q! or ZQ quit without saving.",
    "Symbols      @ capital  # city  × ruins  ! a recent event  ≈ river  ▲ mountains  ∩ hills",
    "             ♣ forest  ♠ taiga  \" grassland  , steppe  : desert  . tundra  ¤ wastes  * ice",
    "             On the political and culture layers the land is a plain field of · under the",
    "             colours, ruled off with │ ─ ┼ wherever the realm (or the people) changes, and",
    "             blank where nobody holds it — so the frontiers read without colour at all.",
    "             At zoom 1 a realm's name stands beside its capital; the line under the map says",
    "             what the colours of the layer mean (:legend turns it off). The sidebar's",
    "             On screen block names the largest realms in view, with their colour.",
    "Plain words  stability reads steady / restless / troubled / on the brink, a treasury bankrupt /",
    "             poor / solvent / rich, an army outmatched / matched / formidable. A realm's page",
    "             has a Why block: what pulls its stability up or down, ranked, in words.",
];

/// Whether the Hand of Fate has anything to say about a thing.
///
/// It used to reach realms and nothing else, which meant the only way to
/// touch a city, a person or a stretch of country was to find the realm that
/// happened to hold it and act on the whole of that instead. A city is the
/// unit most of this world's history actually happens to.
pub fn fate_reaches(w: &World, r: Ref) -> bool {
    match r {
        Ref::Polity(p) => p < w.polities.len() && w.polities[p].alive(),
        Ref::City(c) => c < w.cities.len() && w.cities[c].destroyed.is_none(),
        Ref::Person(i) => i < w.persons.len() && w.persons[i].alive(),
        Ref::Feature(f) => f < w.terrain.features.len(),
        _ => false,
    }
}

/// The menu for whatever is selected, or an empty list if nothing fitting is.
pub fn fate_menu_for(w: &World, r: Ref) -> Vec<String> {
    match r {
        Ref::Polity(p) if fate_reaches(w, r) => fate_menu(w, p),
        Ref::City(c) if fate_reaches(w, r) => vec![
            format!("What befalls {}?", w.cities[c].name),
            String::new(),
            "1  A boom: the town fills, and the money with it".into(),
            "2  A fire: half of it burns, and the people scatter".into(),
            "3  Walls: masons raise them higher than the crown could afford".into(),
            "4  A wonder: something is built here that outlasts the realm".into(),
            "5  A sickness: it comes with the ships and empties the streets".into(),
            "6  A great road: the carrying trade finds this town".into(),
        ],
        Ref::Person(i) if fate_reaches(w, r) => vec![
            format!("What befalls {}?", w.persons[i].full_name()),
            String::new(),
            "1  Renown: their name is suddenly on every tongue".into(),
            "2  Ruin: they are disgraced, and everybody remembers why".into(),
            "3  Brilliance: wisdom and presence beyond their years".into(),
            "4  Ambition: they begin to want what is not theirs".into(),
            "5  A knife: they do not see the year out".into(),
            "6  An heir: a child is born to them who will be remarkable".into(),
        ],
        Ref::Feature(f) if fate_reaches(w, r) => vec![
            format!("What befalls {}?", w.terrain.features[f].display()),
            String::new(),
            "1  Good earth: the land grows kinder for a long age".into(),
            "2  Exhaustion: the soil gives out and the people drift away".into(),
            "3  A blight: the country is poisoned and left to the waste".into(),
            "4  A lode: ore is struck, and everything follows from that".into(),
            "5  A quickening: the ley runs strong here now".into(),
            "6  Emptying: whoever lived here leaves, and the ground forgets".into(),
        ],
        _ => Vec::new(),
    }
}

/// Work the chosen intervention on whatever is selected.
pub fn hand_of_fate_on(w: &mut World, r: Ref, choice: u8) -> String {
    if !fate_reaches(w, r) {
        return "that is beyond reach now".into();
    }
    match r {
        Ref::Polity(p) => hand_of_fate(w, p, choice),
        Ref::City(c) => city_fate(w, c, choice),
        Ref::Person(i) => person_fate(w, i, choice),
        Ref::Feature(f) => land_fate(w, f, choice),
        _ => "nothing answers".into(),
    }
}

/// What can be done to a single town.
fn city_fate(w: &mut World, c: usize, choice: u8) -> String {
    let name = w.cities[c].name.clone();
    let cell = w.cities[c].cell;
    let at = Some(cell);
    let refs = |w: &World| -> Vec<Ref> {
        let mut v = vec![Ref::City(c)];
        if let Some(p) = w.cities[c].polity {
            v.push(Ref::Polity(p));
        }
        v
    };
    match choice {
        0 => {
            w.cities[c].pop *= 1.6;
            w.cities[c].prosperity = (w.cities[c].prosperity + 0.5).min(2.5);
            let r = refs(w);
            let text = format!(
                "For a generation everybody who could get to {} went there, and \
                 the town could not build houses fast enough.",
                name
            );
            w.log(2, EventKind::Founding, &r, at, text);
            format!("{} is booming", name)
        }
        1 => {
            w.cities[c].pop *= 0.5;
            w.cities[c].prosperity *= 0.6;
            w.cities[c].walls *= 0.5;
            let r = refs(w);
            let text = format!(
                "The fire began in the warehouses of {} and did not stop until \
                 there was nothing left in that quarter to burn.",
                name
            );
            w.log(2, EventKind::Disaster, &r, at, text);
            format!("{} has burned", name)
        }
        2 => {
            w.cities[c].walls = (w.cities[c].walls + 1.2).min(3.0);
            let r = refs(w);
            let text = format!(
                "The new walls of {} were the talk of the age, and its \
                 neighbours drew their own conclusions.",
                name
            );
            w.log(1, EventKind::Wonder, &r, at, text);
            format!("{} is walled", name)
        }
        3 => {
            let pick = crate::sim::prose::Pick::rolled(&w.rng);
            let wonder = crate::sim::prose::wonder_name(&name, &pick);
            w.cities[c].wonders.push(wonder.clone());
            w.cities[c].prosperity = (w.cities[c].prosperity + 0.2).min(2.5);
            if let Some(p) = w.cities[c].polity {
                w.polities[p].prestige += 25.0;
            }
            let r = refs(w);
            let text = format!(
                "{} was finished in a single reign, which nobody had thought \
                 possible, and stood long after the realm that raised it.",
                wonder
            );
            w.log(2, EventKind::Wonder, &r, at, text);
            format!("{} now stands", wonder)
        }
        4 => {
            w.cells[cell].plague = 6;
            w.cities[c].pop *= 0.75;
            let polities = w.cities[c].polity.map(|p| vec![p]).unwrap_or_default();
            w.plagues.push(crate::sim::Plague {
                name: "the Harbour Fever".into(),
                years_left: 4,
                polities,
                deaths: 0.0,
            });
            let r = refs(w);
            let text = format!(
                "It came into {} on a ship nobody thought to turn away.",
                name
            );
            w.log(2, EventKind::Disaster, &r, at, text);
            format!("sickness is loose in {}", name)
        }
        _ => {
            // Every road this town works, made worth far more. The network
            // is rebuilt every twentieth year, so this lasts until then and
            // then settles wherever the town's size now justifies.
            let mut touched = 0;
            for route in w.routes.iter_mut() {
                if route.a == c || route.b == c {
                    route.value *= 2.5;
                    route.open = true;
                    touched += 1;
                }
            }
            crate::sim::trade::reckon(w);
            w.cities[c].prosperity = (w.cities[c].prosperity + 0.3).min(2.5);
            let r = refs(w);
            let text = format!(
                "The caravans changed their route that year and came through \
                 {} instead, and went on coming.",
                name
            );
            w.log(2, EventKind::Founding, &r, at, text);
            format!(
                "{} carries {}",
                name,
                crate::sim::prose::count(touched as i64, "road")
            )
        }
    }
}
/// What can be done to one person.
fn person_fate(w: &mut World, i: usize, choice: u8) -> String {
    let who = w.persons[i].full_name();
    let at = w.persons[i].polity.and_then(|p| w.capital_cell(p));
    let refs = |w: &World| -> Vec<Ref> {
        let mut v = vec![Ref::Person(i)];
        if let Some(p) = w.persons[i].polity {
            v.push(Ref::Polity(p));
        }
        v
    };
    match choice {
        0 => {
            w.persons[i].renown += 6.0;
            w.persons[i].greatness += 40.0;
            let r = refs(w);
            let text = format!(
                "Whatever {} had done, the story of it reached places {} had \
                 never been, and grew in the telling.",
                who, who
            );
            w.log(2, EventKind::Person, &r, at, text);
            format!("{} is renowned", who)
        }
        1 => {
            w.persons[i].renown = (w.persons[i].renown - 6.0).max(0.0);
            w.persons[i].greatness = (w.persons[i].greatness - 40.0).max(0.0);
            if let Some(p) = w.persons[i].polity {
                w.polities[p].stability = (w.polities[p].stability - 0.05).max(0.0);
            }
            let r = refs(w);
            let text = format!(
                "What {} had done in private was suddenly known, and nobody \
                 who had praised them would admit to it afterwards.",
                who
            );
            w.log(2, EventKind::Person, &r, at, text);
            format!("{} is disgraced", who)
        }
        2 => {
            let t = &mut w.persons[i].traits;
            t.wisdom = (t.wisdom + 0.4).min(1.0);
            t.charisma = (t.charisma + 0.4).min(1.0);
            let r = refs(w);
            let text = format!(
                "{} spoke, and people who had come to argue found they agreed.",
                who
            );
            w.log(1, EventKind::Person, &r, at, text);
            format!("{} is brilliant", who)
        }
        3 => {
            let t = &mut w.persons[i].traits;
            t.ambition = 1.0;
            t.cruelty = (t.cruelty + 0.2).min(1.0);
            let r = refs(w);
            let text = format!(
                "{} began to speak of what was owed to them, and to count who \
                 had not paid it.",
                who
            );
            w.log(1, EventKind::Person, &r, at, text);
            format!("{} is ambitious", who)
        }
        4 => {
            w.persons[i].died = Some(w.year);
            w.persons[i].death = "was found dead, and no one was ever charged.".into();
            if let Some(h) = w.persons[i].house {
                w.close_house_if_spent(h);
            }
            let r = refs(w);
            let text = format!("{} was found dead, and no one was ever charged.", who);
            w.log(2, EventKind::Death, &r, at, text);
            format!("{} is dead", who)
        }
        _ => {
            let Some(p) = w.persons[i].polity.filter(|&p| w.polities[p].alive()) else {
                return format!("{} belongs to no realm that could raise a child", who);
            };
            let culture = w.polities[p].culture;
            let child = w.new_person(culture, crate::sim::Role::Noble, Some(p), w.year, None);
            // Given the blood of somebody remarkable rather than a roll of
            // the dice: this is an intervention, and the point of it is that
            // the child will be worth watching.
            w.persons[child].parent = Some(i);
            let t = &mut w.persons[child].traits;
            t.ambition = (t.ambition + 0.35).min(1.0);
            t.wisdom = (t.wisdom + 0.3).min(1.0);
            t.charisma = (t.charisma + 0.3).min(1.0);
            w.persons[child].vigour = 1.15;
            if let Some(h) = w.persons[i].house {
                w.join_house(child, h);
            }
            w.persons[i].children.push(child);
            let born = w.persons[child].full_name();
            let text = format!(
                "A child was born to {} that the midwives talked about for \
                 years afterwards, though none of them could say why.",
                who
            );
            w.log(
                2,
                EventKind::Person,
                &[Ref::Person(child), Ref::Person(i), Ref::Polity(p)],
                at,
                text,
            );
            format!("{} is born", born)
        }
    }
}

/// What can be done to a stretch of country.
///
/// The one kind of intervention that outlives everybody it touches: the
/// ground keeps what is done to it, and knowledge belongs to the ground, so
/// emptying a region is how a dark age is started on purpose.
fn land_fate(w: &mut World, f: usize, choice: u8) -> String {
    let name = w.terrain.features[f].display();
    let cells: Vec<usize> = w.terrain.features[f].cells.clone();
    let at = {
        let c = w.terrain.features[f].center;
        Some(w.terrain.idx(c.0, c.1))
    };
    let land: Vec<usize> = cells
        .iter()
        .copied()
        .filter(|&i| w.terrain.is_land(i))
        .collect();
    if land.is_empty() {
        return format!("{} is all water", name);
    }
    let refs = vec![Ref::Feature(f)];
    match choice {
        0 => {
            for &i in &land {
                w.terrain.fertility[i] = (w.terrain.fertility[i] + 0.25).min(1.0);
            }
            let text = format!(
                "The rains came right for a lifetime over {}, and then kept \
                 coming right.",
                name
            );
            w.log(2, EventKind::Disaster, &refs, at, text);
            format!("{} is fertile", name)
        }
        1 => {
            for &i in &land {
                w.terrain.fertility[i] *= 0.45;
            }
            let text = format!(
                "The fields of {} gave less every year until the people \
                 stopped pretending it was the weather.",
                name
            );
            w.log(2, EventKind::Disaster, &refs, at, text);
            format!("{} is exhausted", name)
        }
        2 => {
            w.terrain.blight(&land);
            for &i in &land {
                w.cells[i].pop *= 0.3;
            }
            let text = format!(
                "Whatever was done in {}, nothing has grown there since, and \
                 the few who go in do not stay.",
                name
            );
            w.log(3, EventKind::Magic, &refs, at, text);
            format!("{} is blighted", name)
        }
        3 => {
            for &i in &land {
                w.terrain.minerals[i] = (w.terrain.minerals[i] + 0.4).min(1.0);
            }
            // The goods of a cell are a pure reading of its terrain, so
            // changing the terrain means reading them again.
            w.goods = crate::sim::trade::goods_of(&w.terrain);
            let text = format!(
                "Somebody sank a shaft in {} on a hunch, and a hundred years \
                 of everybody's iron came out of it.",
                name
            );
            w.log(2, EventKind::Discovery, &refs, at, text);
            format!("{} has ore", name)
        }
        4 => {
            for &i in &land {
                w.terrain.mana[i] = (w.terrain.mana[i] + 0.35).min(1.0);
            }
            // The goods of a cell are a pure reading of its terrain, so
            // changing the terrain means reading them again.
            w.goods = crate::sim::trade::goods_of(&w.terrain);
            let text = format!(
                "The old stones of {} began to be warm to the touch, and the \
                 dogs would not go near them.",
                name
            );
            w.log(2, EventKind::Magic, &refs, at, text);
            format!("the ley runs strong in {}", name)
        }
        _ => {
            // Knowledge belongs to the ground and ground that empties of
            // people forgets, so this is the one way to start a dark age on
            // purpose.
            for &i in &land {
                w.cells[i].pop = 0.0;
                w.known[i] = 0;
            }
            for &i in &land {
                w.refresh_yield(i);
            }
            let text = format!(
                "Within a generation there was nobody left in {} who \
                 remembered why anyone had ever lived there.",
                name
            );
            w.log(3, EventKind::Disaster, &refs, at, text);
            format!("{} is empty", name)
        }
    }
}

/// The six things that can be done to a whole realm.
pub fn fate_menu(w: &World, p: usize) -> Vec<String> {
    let pol = &w.polities[p];
    vec![
        format!("What befalls {}?", pol.name),
        String::new(),
        "1  A bountiful omen: the harvests swell and the people take heart".into(),
        "2  A dark omen: sickness and doubt spread through the realm".into(),
        "3  A vision: a prophet or mage arises in the capital".into(),
        "4  A gift: gold fills the treasury and the masons are set to work".into(),
        "5  A whisper of war: the ruler's ambition is inflamed".into(),
        "6  A quiet death: the ruler passes and the succession begins".into(),
    ]
}

/// Work one of them.
pub fn hand_of_fate(w: &mut World, p: usize, choice: u8) -> String {
    if !w.polities[p].alive() {
        return "that realm is gone".into();
    }
    let name = w.polities[p].name.clone();
    let cap = w.capital_cell(p);
    match choice {
        0 => {
            w.polities[p].stability = (w.polities[p].stability + 0.2).min(1.0);
            for i in w.cells_of(p) {
                w.cells[i].pop *= 1.15;
            }
            for c in w.polities[p].cities.clone() {
                w.cities[c].pop *= 1.1;
            }
            let text = format!("A white stag was seen in the fields of {}, and the harvest that year was the best in living memory.", w.polities[p].short);
            w.log(2, EventKind::Disaster, &[Ref::Polity(p)], cap, text);
            format!("blessed {}", name)
        }
        1 => {
            w.polities[p].stability = (w.polities[p].stability - 0.25).max(0.0);
            w.plagues.push(crate::sim::Plague {
                name: "the Whispering Sickness".into(),
                years_left: 3,
                polities: vec![p],
                deaths: 0.0,
            });
            let text = format!(
                "Crows gathered on the roofs of {} for a month. Then the coughing began.",
                w.polities[p]
                    .capital
                    .map(|c| w.cities[c].name.clone())
                    .unwrap_or_else(|| w.polities[p].short.clone())
            );
            w.log(2, EventKind::Disaster, &[Ref::Polity(p)], cap, text);
            format!("cursed {}", name)
        }
        2 => {
            if let Some(c) = w.polities[p].capital {
                let mana = w.terrain.mana[w.cities[c].cell];
                let kind = if mana > 0.45 {
                    SchoolKind::Arcane
                } else if w.rng.chance(0.5) {
                    SchoolKind::Divine
                } else {
                    SchoolKind::Philosophical
                };
                let s = crate::sim::magic::found_school(w, c, kind, None, None);
                let founder = w.schools[s].founder;
                let text = format!("{} of {} woke from a dream that would not leave them and began to teach. So began {}, which {}.", w.persons[founder].name, w.cities[c].name, w.schools[s].name, w.schools[s].doctrine);
                w.log(
                    2,
                    EventKind::Magic,
                    &[
                        Ref::School(s),
                        Ref::Person(founder),
                        Ref::Polity(p),
                        Ref::City(c),
                    ],
                    cap,
                    text,
                );
                format!("inspired {}", w.persons[founder].name)
            } else {
                "no capital to inspire".into()
            }
        }
        3 => {
            w.polities[p].treasury += 200.0;
            w.polities[p].dev = (w.polities[p].dev + 0.1).min(3.0);
            let text = format!("A ploughman of {} turned up a hoard of old gold; the crown claimed it and spent it well.", w.polities[p].short);
            w.log(1, EventKind::Wonder, &[Ref::Polity(p)], cap, text);
            format!("enriched {}", name)
        }
        4 => {
            if let Some(r) = w.polities[p].ruler {
                w.persons[r].traits.ambition = 1.0;
                w.persons[r].traits.valor = (w.persons[r].traits.valor + 0.3).min(1.0);
            }
            for t in w.polities[p].tension.values_mut() {
                *t = (*t + 0.4).min(1.0);
            }
            let text = format!("{} dreamed three nights running of a crown of many crowns, and woke each time hungry.", w.ruler_title(p));
            w.log(1, EventKind::Politics, &[Ref::Polity(p)], cap, text);
            format!("inflamed {}", w.ruler_short(p))
        }
        _ => {
            let who = w.ruler_short(p);
            crate::sim::politics::ruler_dies(
                w,
                p,
                "died quietly in the night, as if called away.",
                1,
            );
            format!("{} has died", who)
        }
    }
}

// ---------------------------------------------------------------------------
// Houses
// ---------------------------------------------------------------------------

/// Glyphs for the family tree, in Unicode and in the ASCII fallback.
struct TreeGlyphs {
    first: char,
    mid: char,
    last: char,
    only: char,
    branch: char,
    stub: char,
    vert: char,
}

fn tree_glyphs(ascii: bool) -> TreeGlyphs {
    if ascii {
        TreeGlyphs {
            first: 'o',
            mid: '+',
            last: '`',
            only: 'o',
            branch: '>',
            stub: '-',
            vert: '|',
        }
    } else {
        TreeGlyphs {
            first: '┬',
            mid: '├',
            last: '┴',
            only: '●',
            branch: '╞',
            stub: '─',
            vert: '│',
        }
    }
}

/// How `child` is related to the person before them in the line of
/// succession, in one word.
fn kin_word(w: &World, prev: Option<usize>, who: usize) -> &'static str {
    let Some(prev) = prev else { return "" };
    if w.persons[who].parent == Some(prev) {
        return "child";
    }
    if w.persons[prev].parent == Some(who) {
        return "parent";
    }
    let (a, b) = (w.persons[who].parent, w.persons[prev].parent);
    if a.is_some() && a == b {
        return "sibling";
    }
    if let Some(sp) = w.persons[prev].spouse {
        if sp == who {
            return "consort";
        }
    }
    // Same house, no traceable link: a cousin from a cadet line.
    "kinsman"
}

/// The page for a ruling house: a thousand years of one family, drawn so
/// that it fits a terminal.
///
/// The hard part of a dynasty at this scale is that an ordinary indented
/// tree is unreadable: thirty generations is thirty levels of indentation,
/// and a house with a hundred members cannot be drawn at all on eighty
/// columns. So the tree here is drawn down **time**, not down descent — the
/// line of succession is one straight column with a year beside each
/// accession, and everything else hangs off it as a note. That keeps the
/// drawing exactly as wide as one name however many centuries it runs, and
/// it matches what a reader actually wants to follow: who held the throne,
/// what they were to the one before, and where the line forked.
fn house_page(
    w: &World,
    h: usize,
    r: Ref,
    width: usize,
    w2: usize,
    ascii: bool,
    out: &mut Vec<Line>,
) {
    let ho = &w.houses[h];
    let g = tree_glyphs(ascii);
    out.push(line(ho.name.to_uppercase(), ACCENT, BOLD));

    let mut desc = format!("A house of the {}", w.cultures[ho.culture].plural);
    if let Some(f) = ho.founder {
        desc.push_str(&format!(", founded by {}", w.persons[f].full_name()));
    }
    desc.push_str(&format!(" in year {}", ho.founded));
    match ho.ended {
        Some(y) => desc.push_str(&format!(", and spent by year {}.", y)),
        None => desc.push('.'),
    }
    for l in term::wrap(&desc, w2) {
        out.push(line(l, FG, 0));
    }
    let living = ho.members.iter().filter(|&&m| w.persons[m].alive()).count();
    out.push(line(
        format!(
            "{} years · {} who ruled · {} of the line · {} living",
            ho.span(w.year).max(0),
            ho.seniors.len(),
            ho.members.len(),
            living
        ),
        DIMC,
        0,
    ));
    out.push(line("", FG, 0));

    out.push(line(
        format!("[c] People    the {}", w.cultures[ho.culture].plural),
        LINK,
        0,
    ));
    if let Some(f) = ho.founder {
        out.push(line(
            format!("[f] Founder   {}", w.persons[f].full_name()),
            LINK,
            0,
        ));
    }
    if let Some(pa) = ho.parent {
        out.push(line(
            format!("[b] Branch of {}", w.houses[pa].name),
            LINK,
            0,
        ));
    }

    // The thrones it has held, and when.
    if !ho.realms.is_empty() {
        out.push(line("", FG, 0));
        out.push(line("Thrones held", ACCENT, BOLD));
        for &p in ho.realms.iter().take(12) {
            let pol = &w.polities[p];
            let still = pol.alive() && pol.house == Some(h);
            let mark = if still { "·" } else { " " };
            // Just whether the house holds it now: dating the tenure would
            // mean recording when each accession happened in each realm,
            // and "since <the realm's founding>" was simply wrong for a
            // throne the house came to later.
            let when = if still {
                "holds it".to_string()
            } else if pol.alive() {
                "lost it".to_string()
            } else {
                format!("fell {}", pol.fell.unwrap_or(0))
            };
            let nw = w2.saturating_sub(when.len() + 6).clamp(10, 34);
            out.push(line(
                format!("  {} {:<nw$} {}", mark, clip(&pol.name, nw), when, nw = nw),
                if still { FG } else { DIMC },
                0,
            ));
        }
        if ho.peak_realms > 1 {
            out.push(line(
                format!(
                    "    at its height it held {} thrones at once",
                    ho.peak_realms
                ),
                DIMC,
                DIM,
            ));
        }
    }

    // The line of succession: the spine of the tree.
    out.push(line("", FG, 0));
    out.push(line("The line", ACCENT, BOLD));
    // The table lays itself out from the columns it is given rather than
    // assuming a wide terminal: the name takes what is left after the fixed
    // columns, and the realm and the relation are dropped in that order
    // when there is no room for them. A house page on eighty columns is the
    // common case and on sixty it still has to be a table.
    let spine_fixed = 2 + 4 + 1 + 1 + 1 + 5 + 2; // indent, year, gaps, glyph, reign
    let room = w2.saturating_sub(spine_fixed);
    let show_realm = room >= 40;
    let show_kin = room >= 58;
    let realm_w = if show_realm { 15 } else { 0 };
    let kin_w = if show_kin { 18 } else { 0 };
    let name_w = room
        .saturating_sub(realm_w + kin_w + usize::from(show_realm) + usize::from(show_kin))
        .clamp(8, 34);
    if ho.seniors.is_empty() {
        out.push(line("  never came to a throne", DIMC, DIM));
    } else {
        let mut head = format!("  year  {:<name_w$}", "who", name_w = name_w);
        if show_realm {
            head.push_str(&format!(" {:<realm_w$}", "realm", realm_w = realm_w));
        }
        head.push_str("  reign");
        if show_kin {
            head.push_str("  kin");
        }
        out.push(line(head, DIMC, DIM));
    }
    let mut spine: Vec<usize> = ho.seniors.clone();
    spine.sort_by_key(|&x| (w.persons[x].crowned.unwrap_or(w.persons[x].born), x));
    let n = spine.len();
    let mut prev: Option<usize> = None;
    for (i, &who) in spine.iter().enumerate() {
        let per = &w.persons[who];
        // Their accession year, recorded when they were crowned. The realm's
        // own `reign_start` describes whoever sits there now, so it cannot
        // date a predecessor and the line came out in the wrong order.
        let came = per.crowned.unwrap_or(per.born).max(0);
        let realm = per
            .polity
            .map(|p| w.polities[p].short.clone())
            .unwrap_or_default();
        let glyph = if n == 1 {
            g.only
        } else if i == 0 {
            g.first
        } else if i + 1 == n {
            g.last
        } else {
            g.mid
        };
        let reign = if per.alive() {
            format!("{}y…", per.reign_years)
        } else {
            format!("{}y", per.reign_years)
        };
        let kin = kin_word(w, prev, who);
        let siblings = per
            .parent
            .map(|pa| w.persons[pa].children.iter().filter(|&&c| c != who).count())
            .unwrap_or(0);
        let extra = if siblings > 0 {
            format!("{}, +{} more", kin, siblings)
        } else {
            kin.to_string()
        };
        let mut row = format!(
            "  {:>4} {} {:<name_w$}",
            came,
            glyph,
            clip(&per.full_name(), name_w),
            name_w = name_w
        );
        if show_realm {
            row.push_str(&format!(
                " {:<realm_w$}",
                clip(&realm, realm_w),
                realm_w = realm_w
            ));
        }
        row.push_str(&format!(" {:>5}", reign));
        if show_kin {
            row.push_str(&format!("  {}", clip(&extra, kin_w)));
        }
        out.push(line(
            row,
            if per.alive() { FG } else { DIMC },
            if per.is_acclaimed() { BOLD } else { 0 },
        ));
        // A member who founded a house of their own is where the tree
        // actually forks, and that is worth a line even though it leaves
        // this page.
        for (bi, br) in w
            .houses
            .iter()
            .enumerate()
            .filter(|(_, x)| x.parent == Some(h) && x.founder == Some(who))
        {
            let _ = bi;
            out.push(line(
                format!("       {} {}{} {}", g.vert, g.branch, g.stub, br.name),
                Rgb(200, 170, 220),
                0,
            ));
        }
        prev = Some(who);
    }

    // Everyone else: born to the house, never held a throne. At millennia
    // scale this is the long tail, so it is counted rather than listed in
    // full, with the most recent shown.
    let others: Vec<usize> = ho
        .members
        .iter()
        .copied()
        .filter(|m| !spine.contains(m))
        .collect();
    if !others.is_empty() {
        out.push(line("", FG, 0));
        out.push(line(
            format!("Others of the line ({})", others.len()),
            ACCENT,
            BOLD,
        ));
        let start = others.len().saturating_sub(24);
        if start > 0 {
            out.push(line(format!("  … {} earlier omitted", start), DIMC, DIM));
        }
        for &m in &others[start..] {
            let per = &w.persons[m];
            let when = match per.died {
                Some(y) => format!("{}–{}", per.born, y),
                None => format!("{}–, living", per.born),
            };
            let nw = w2
                .saturating_sub(when.len() + per.role.name().len() + 5)
                .clamp(10, 32);
            out.push(line(
                format!(
                    "  {:<nw$} {:<14} {}",
                    clip(&per.full_name(), nw),
                    when,
                    per.role.name(),
                    nw = nw
                ),
                if per.alive() { FG } else { DIMC },
                0,
            ));
        }
    }
    history(w, r, width, out, 200);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The timeline lists every realm it was given, once, with successors
    /// under the realm they broke away from.
    ///
    /// This is the page the game is named for and it could not be drawn
    /// until now, though every field it needs — `founded`, `fell`,
    /// `parent`, `peak_cells`, `peak_year` — has been recorded since long
    /// before. The ordering is the whole value: a kingdom that shattered
    /// into four should read as a family, not as four unrelated rows.
    #[test]
    fn the_timeline_lists_every_realm_once_under_its_parent() {
        use crate::sim::Detail;
        let mut w = crate::sim::World::new(17, 120, 60, Detail::Medium);
        for _ in 0..900 {
            w.tick();
        }
        let chosen: Vec<usize> = (0..w.polities.len()).collect();
        let order = lineage_order(&w, &chosen);

        // Every realm, exactly once.
        assert_eq!(
            order.len(),
            chosen.len(),
            "the page lost or doubled a realm"
        );
        let mut seen: Vec<usize> = order.iter().map(|&(p, _)| p).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), chosen.len(), "a realm appears twice");

        // Depth is bounded, and a realm shown as a successor really is one,
        // and really does come after its parent.
        let at: std::collections::BTreeMap<usize, usize> = order
            .iter()
            .enumerate()
            .map(|(i, &(p, _))| (p, i))
            .collect();
        for (i, &(p, depth)) in order.iter().enumerate() {
            assert!(depth <= LINEAGE_DEPTH, "realm {} sits {} deep", p, depth);
            if depth > 0 {
                let parent = w.polities[p]
                    .parent
                    .expect("only a realm with a parent is indented");
                let parent_at = *at
                    .get(&parent)
                    .expect("an indented realm's parent is on the page");
                assert!(
                    parent_at < i,
                    "realm {} is drawn above the realm it broke away from",
                    p
                );
                assert_eq!(
                    order[parent_at].1,
                    depth - 1,
                    "realm {} sits {} deep under a parent {} deep",
                    p,
                    depth,
                    order[parent_at].1
                );
            }
        }

        // A bar is drawn for every realm, including one that rose and fell
        // inside a single column: a blank row would say it never existed.
        let span = w.year as f32;
        for &p in &chosen {
            let pol = &w.polities[p];
            let bar = span_bar(
                pol.founded,
                pol.fell.unwrap_or(w.year),
                span,
                TIMELINE_WIDTH,
            );
            assert_eq!(bar.chars().count(), TIMELINE_WIDTH);
            assert!(
                bar.contains('\u{2588}'),
                "realm {} founded {} fell {:?} has an empty lifetime",
                p,
                pol.founded,
                pol.fell
            );
        }
    }

    /// A parent that points at itself, or a ring of them, must not hang the
    /// page or drop the realms caught in it.
    #[test]
    fn a_tangled_descent_still_lists_everybody() {
        use crate::sim::Detail;
        let mut w = crate::sim::World::new(19, 60, 30, Detail::Medium);
        for _ in 0..200 {
            w.tick();
        }
        let chosen: Vec<usize> = (0..w.polities.len()).collect();
        assert!(chosen.len() >= 3, "need a few realms to tangle");
        w.polities[chosen[0]].parent = Some(chosen[0]);
        w.polities[chosen[1]].parent = Some(chosen[2]);
        w.polities[chosen[2]].parent = Some(chosen[1]);
        let order = lineage_order(&w, &chosen);
        assert_eq!(order.len(), chosen.len(), "a cycle swallowed a realm");
    }

    /// The help page has to be readable on the narrowest terminal the README
    /// claims to support, and nothing in it may be dropped on the way.
    #[test]
    fn help_fits_any_terminal_without_losing_a_word() {
        let all: Vec<String> = HELP
            .iter()
            .flat_map(|l| l.split_whitespace())
            .map(str::to_string)
            .collect();
        for width in [30, 40, 60, 77, 80, 100, 157, 200] {
            let lines = help_lines(width);
            for l in &lines {
                // A line may only overflow because its last word is one the
                // width cannot hold at all — `~/.config/empires/config:` on a
                // thirty-column terminal. Anything else is a wrapping bug.
                let fits = l.chars().count() <= width.max(4);
                let one_long_word = l
                    .trim_end()
                    .rsplit_once(' ')
                    .map_or(true, |(head, _)| head.chars().count() < width.max(4));
                assert!(
                    fits || one_long_word,
                    "at width {} this line is {} long: {:?}",
                    width,
                    l.chars().count(),
                    l
                );
            }
            let words: Vec<String> = lines
                .iter()
                .flat_map(|l| l.split_whitespace())
                .map(str::to_string)
                .collect();
            assert_eq!(words, all, "width {} lost or reordered words", width);
        }
    }

    /// Wide enough for the table as written, and it is left exactly as
    /// written: the alignment is hand-made and worth keeping.
    #[test]
    fn a_wide_terminal_gets_the_table_verbatim() {
        let widest = HELP.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let lines = help_lines(widest);
        assert_eq!(lines.len(), HELP.len());
        for (a, b) in lines.iter().zip(HELP.iter()) {
            assert_eq!(a, b);
        }
    }

    /// Every bracketed marker a page draws must be a letter that does
    /// something when pressed: either `follow_link` opens a page for it, or
    /// the page's own key handler spends it on an action. `[k]` used to be
    /// drawn for the capital, and `k` scrolls.
    #[test]
    fn every_bracketed_link_letter_is_one_follow_link_knows() {
        // Letters `Ui::key_detail` handles itself before `follow_link` ever
        // sees them. Keep this in step with `src/ui/input.rs`.
        const ACTIONS: &[char] = &['x', 'm'];
        use crate::sim::{Detail, World};
        let mut w = World::new(4, 48, 24, Detail::Medium);
        for _ in 0..120 {
            w.tick();
        }
        let refs: Vec<Ref> = (0..w.polities.len())
            .map(Ref::Polity)
            .chain((0..w.cities.len()).map(Ref::City))
            .chain((0..w.cultures.len()).map(Ref::Culture))
            .chain((0..w.schools.len()).map(Ref::School))
            .chain((0..w.persons.len().min(200)).map(Ref::Person))
            .chain((0..w.wars.len()).map(Ref::War))
            .chain((0..w.artifacts.len()).map(Ref::Artifact))
            .collect();
        let mut checked = 0;
        for r in refs {
            for l in detail_lines(&w, r, 100, false) {
                let bytes: Vec<char> = l.text.chars().collect();
                for (i, c) in bytes.iter().enumerate() {
                    if *c != '[' || i + 2 >= bytes.len() || bytes[i + 2] != ']' {
                        continue;
                    }
                    let letter = bytes[i + 1];
                    if !letter.is_alphabetic() {
                        continue;
                    }
                    checked += 1;
                    assert!(
                        follow_link(&w, r, letter).is_some() || ACTIONS.contains(&letter),
                        "{:?} draws [{}] but pressing it does nothing: {:?}",
                        r,
                        letter,
                        l.text
                    );
                }
            }
        }
        assert!(checked > 20, "only {} link markers were checked", checked);
    }
}
