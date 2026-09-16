//! Lists, detail pages, summaries, the help text and the hand of fate.

use super::words;
use crate::sim::chronicle::{EventKind, Ref};
use crate::sim::explain;
use crate::sim::{SchoolKind, World};
use crate::term::{self, Rgb, BOLD, DIM};

pub const LIST_TABS: [&str; 9] = [
    "Realms",
    "Cities",
    "Peoples",
    "Schools",
    "Persons",
    "Wars",
    "Places",
    "Relics",
    "Prophecies",
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
    }
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
        Ref::Polity(p) => {
            let pol = &w.polities[p];
            out.push((pol.name.clone(), pol.color));
            if let Some(y) = pol.fell {
                out.push((format!("fell in year {}", y), DIMC));
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
                        "{} {}",
                        words::stability(pol.stability),
                        explain::stability_drift(w, p)
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
                format!("a {} people · {} lands", w.races[cu.race].adj, cu.cells),
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

pub fn list_header(tab: usize) -> &'static str {
    match tab {
        0 => "  name                              kind          lands    people  stability          ruler",
        1 => "  name                    realm                            people   size              founded",
        2 => "  people                  race          lands   people   language",
        3 => "  school                              kind         realms reach                         home",
        4 => "  name                          role         realm                  born   died",
        5 => "  war                                       attacker vs defender          years",
        6 => "  place                                   kind",
        7 => "  relic                                   kind      made   held by",
        _ => "  prophecy                                                          seer                 by     outcome",
    }
}

pub fn list_rows(w: &World, tab: usize) -> Vec<(String, Ref)> {
    let mut rows = Vec::new();
    match tab {
        0 => {
            let mut ps: Vec<usize> = (0..w.polities.len()).collect();
            ps.sort_by_key(|&p| {
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
            cs.sort_by(|&a, &b| {
                let (ca, cb) = (&w.cities[a], &w.cities[b]);
                ca.destroyed
                    .is_some()
                    .cmp(&cb.destroyed.is_some())
                    .then_with(|| cb.pop.total_cmp(&ca.pop))
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
            cs.sort_by_key(|&c| {
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
                let row = format!(
                    "{:<24}{:<14}{:>6}  {:>7.0}k  {}",
                    clip(&cu.plural, 23),
                    clip(&w.races[cu.race].name, 13),
                    status,
                    cu.pop,
                    cu.lang.name
                );
                rows.push((row, Ref::Culture(c)));
            }
        }
        3 => {
            let mut ss: Vec<usize> = (0..w.schools.len()).collect();
            // Living schools first, most influential first within each group.
            ss.sort_by(|&a, &b| {
                let (sa, sb) = (&w.schools[a], &w.schools[b]);
                sa.extinct
                    .is_some()
                    .cmp(&sb.extinct.is_some())
                    .then_with(|| sb.total_influence().total_cmp(&sa.total_influence()))
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
            ps.sort_by(|&a, &b| {
                let (pa, pb) = (&w.persons[a], &w.persons[b]);
                pa.died
                    .is_some()
                    .cmp(&pb.died.is_some())
                    .then_with(|| pb.renown.total_cmp(&pa.renown))
                    .then_with(|| pb.born.cmp(&pa.born))
            });
            for p in ps.into_iter().take(400) {
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
            ws.sort_by_key(|&x| {
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
            arts.sort_by_key(|&a| {
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
            prs.sort_by_key(|&i| {
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
        _ => {
            let mut fs: Vec<usize> = (0..w.terrain.features.len()).collect();
            fs.sort_by_key(|&f| {
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
    rows
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
                _ => None,
            }
        }
        Ref::War(x) => match c {
            'a' => Some(Ref::Polity(w.wars[x].attacker)),
            'd' => Some(Ref::Polity(w.wars[x].defender)),
            _ => None,
        },
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
        format!("History ({} entries)", ids.len()),
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
    let status = match pol.fell {
        Some(y) => format!(
            "A {} that {}; founded {}, fell {}.",
            pol.kind.name(),
            pol.fall_cause.trim_end_matches('.'),
            pol.founded,
            y
        ),
        None => format!(
            "A {} {} founded in year {}.",
            w.cultures[pol.culture].adj,
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
            out.push(line(format!("    Dynasty   {}", pol.dynasty), FG, 0));
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
        out.push(line(
            format!(
                "    Stability {} {} ({:.0}%), {}",
                bar(pol.stability, 20, ascii),
                words::stability(pol.stability),
                pol.stability * 100.0,
                explain::stability_drift(w, p)
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
        out.push(line(format!("    Strain    {} overextension {:.0}%, foreign subjects {:.0}%, war-weariness {:.0}%, decadence {:.0}%", bar((pol.overextension(&w.tuning) - 0.5).clamp(0.0, 1.0), 20, ascii), pol.overextension(&w.tuning) * 100.0, pol.foreign_share * 100.0, pol.exhaustion * 100.0, pol.decadence * 100.0), FG, 0));
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
        let mut nb: Vec<(usize, u32)> = pol.neighbors.clone();
        nb.sort_by_key(|&(_, l)| std::cmp::Reverse(l));
        if !nb.is_empty() {
            let s: Vec<String> = nb
                .iter()
                .take(6)
                .map(|&(q, _)| {
                    let t = pol.tension.get(&q).copied().unwrap_or(0.0);
                    let mood = if t > 0.5 {
                        "hostile"
                    } else if t > 0.3 {
                        "wary"
                    } else {
                        "calm"
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
            "    Fortunes  {} ({:.0}% prosperity)   walls {}   sacked {} times",
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
            city.times_sacked
        ),
        FG,
        0,
    ));
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
fn people_page(w: &World, cu: usize, r: Ref, width: usize, w2: usize, out: &mut Vec<Line>) {
    let c = &w.cultures[cu];
    out.push(line(
        format!("THE {}", c.plural.to_uppercase()),
        c.color,
        BOLD,
    ));
    let race = &w.races[c.race];
    let mut desc = format!(
        "A {} people. They speak {} and call themselves {}.",
        race.adj, c.lang.name, c.name
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
    for l in term::wrap(&explain::standing(w, pi), w2) {
        out.push(line(l, ACCENT, 0));
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
    if per.parent.is_some() {
        out.push(line(format!("[f] Family    {}", w.lineage(pi)), LINK, 0));
    }
    let kids = w.children_of(pi);
    if !kids.is_empty() {
        let names: Vec<String> = kids.iter().map(|&k| w.persons[k].full_name()).collect();
        labelled(out, "[h] Children", &names.join(", "), w2, LINK);
    }
    if per.battles_won > 0 {
        out.push(line(
            format!("    Battles   {} won", per.battles_won),
            DIMC,
            0,
        ));
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
        "A {}, {}. Made in year {}",
        ar.kind.word().to_lowercase(),
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
    out.push(line(
        format!("[h] Held by   {}", w.artifact_holder_name(a)),
        LINK,
        0,
    ));
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
    let mut desc = format!("A {} of {} cells.", ft.kind.label(), ft.cells.len());
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
    "             v filter the event log by importance (0-2)   x the Hand of Fate for that realm",
    "",
    "Search       /name finds realms, cities, people, peoples, schools, wars, places and relics;",
    "             Enter jumps, n N cycle.  In lists and the chronicle, / filters the rows instead.",
    "",
    "Commands     :w [name]  :e name  :saveas name  :saves  :autosave N   (saves live in",
    "             ~/.local/share/empires; --save FILE or --load FILE on the command line)",
    "             :speed 25  :layer culture  :detail high  :zoom 2  :theme paper  :find Velen",
    "             :new [seed]  :until YEAR  :step N  :follow on  :log 2  :mute battle  :story",
    "             :recap 100 (the last N years)   :legend (the key under the map)   :tour",
    "             :set key value  :map <from> <to>  :unmap key  :maps  :mkconfig  :config",
    "             :fate 3  :q  :wq  :q!",
    "",
    "Browsing     e lists (realms, cities, peoples, schools, persons, wars, places, relics,",
    "             prophecies)   c the chronicle (f or v cycles importance 0-3, / filters by text)",
    "             Everywhere: j k scroll, Ctrl-d Ctrl-u half a page, Ctrl-f Ctrl-b a page,",
    "             gg G top / bottom, m show on map, q or Esc back.",
    "             In a list: Tab or ] [ change tab, Enter opens, n N move, x the Hand of Fate.",
    "             On a page: the bracketed letter opens that link ([r] ruler, [c] people,",
    "             [K] capital, [f] family), Backspace retraces, Enter shows it on the map.",
    "",
    "Mouse        click selects, click again opens, wheel scrolls, clicking a chronicle line or",
    "             a sidebar entry jumps there.  --no-mouse or :mouse leaves it to the terminal.",
    "Config       ~/.config/empires/config: key = value settings (detail speed theme mouse ascii",
    "             autosave width height log follow zoom), tune.<field> for any balance constant,",
    "             and vim-style remaps: map <S-Up> K.  :mkconfig writes a commented template.",
    "Themes       default  phosphor  amber  paper  dusk   (:theme cycles)",
    "Quitting     :q  ZZ  Ctrl-C  or q twice on the map, all of which save if a save file is",
    "             set.  :q! or ZQ quit without saving.",
    "Symbols      @ capital  # city  × ruins  ! a recent event  ≈ river  ▲ mountains  ♣ forest",
    "             At zoom 1 a realm's name stands beside its capital; the line under the map says",
    "             what the colours of the layer mean (:legend turns it off).",
    "Plain words  stability reads steady / restless / troubled / on the brink, a treasury bankrupt /",
    "             poor / solvent / rich, an army outmatched / matched / formidable. A realm's page",
    "             has a Why block: what pulls its stability up or down, ranked, in words.",
];

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

#[cfg(test)]
mod tests {
    use super::*;

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
