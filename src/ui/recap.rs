//! The digest: what happened in the last so-many years, for the whole
//! world or for one realm, grouped by kind and written as plain sentences.

use super::detail::Line;
use super::words;
use crate::sim::chronicle::{EventKind, Ref};
use crate::sim::{Role, World};
use crate::term::{Rgb, BOLD, DIM};

const FG: Rgb = Rgb(200, 200, 205);
const DIMC: Rgb = Rgb(130, 130, 140);
const ACCENT: Rgb = Rgb(230, 200, 120);

fn line(text: impl Into<String>, fg: Rgb, attr: u8) -> Line {
    Line {
        text: text.into(),
        fg,
        attr,
    }
}

/// A number short enough for a chart's margin.
///
/// The series share no units — people run to millions, decadence to one —
/// so the same column has to hold both without either becoming unreadable.
fn compact(v: f32) -> String {
    let a = v.abs();
    if a >= 1.0e6 {
        format!("{:.1}M", v / 1.0e6)
    } else if a >= 1.0e3 {
        format!("{:.1}k", v / 1.0e3)
    } else if a >= 10.0 {
        format!("{:.0}", v)
    } else {
        format!("{:.2}", v)
    }
}

/// The ramp a sparkline is drawn with, lowest first.
///
/// Both halves are needed, not one. The frame's ascii sweep would turn the
/// block elements into `'?'` — they have no arms in `ascii_glyph`, and the
/// test that forbids Unicode in ascii mode would still pass, because a
/// question mark is ASCII. A chart of question marks is not a chart.
fn ramp(ascii: bool) -> [char; 8] {
    if ascii {
        ['_', '.', '.', '-', '-', '=', '=', '#']
    } else {
        [
            '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}',
            '\u{2588}',
        ]
    }
}

/// One series as a row of blocks, scaled to its own range.
///
/// Each series gets its own scale because they share no units: a chart that
/// put population and the number of wars on one axis would be a flat line
/// and a flat line. The floor is zero rather than the minimum, so a series
/// that never varies reads as a steady band and not as noise.
fn spark(values: &[f32], width: usize, ascii: bool) -> String {
    if values.is_empty() || width == 0 {
        return String::new();
    }
    let top = values.iter().copied().fold(0.0f32, f32::max);
    let steps = ramp(ascii);
    // More samples than columns is the normal case in a long world, so each
    // column is the largest sample it covers: a chart of a world's history
    // should not lose the peak of a plague to averaging.
    (0..width)
        .map(|i| {
            let from = i * values.len() / width;
            let to = (((i + 1) * values.len()) / width)
                .max(from + 1)
                .min(values.len());
            let v = values[from..to].iter().copied().fold(0.0f32, f32::max);
            if top <= 0.0 {
                return steps[0];
            }
            let k = ((v / top) * (steps.len() - 1) as f32).round() as usize;
            steps[k.min(steps.len() - 1)]
        })
        .collect()
}

/// How the world has gone, series by series, as a block of sparklines.
///
/// Empty until a world has been measured a few times — there is nothing
/// useful to draw from two points — and empty for a world loaded from a file
/// written before it kept a record of itself.
pub fn world_over_time(w: &World, width: usize, ascii: bool) -> Vec<Line> {
    use crate::sim::history::SERIES;
    let mut out: Vec<Line> = Vec::new();
    let Some((from, to)) = w.history.span() else {
        return out;
    };
    if w.history.samples.len() < 4 {
        return out;
    }
    // Label, chart, and the current value with its peak. The chart takes
    // whatever is left over, and never less than a stub.
    let label_w = 13usize;
    // Exactly what the format below spends after the chart: two spaces, a
    // nine-wide figure, two spaces, "peak " and a seven-wide figure. Guess
    // it short and the peak is clipped in half by the page's own width.
    let tail_w = 2 + 9 + 2 + 5 + 7;
    let chart_w = width.saturating_sub(label_w + tail_w).max(8);
    out.push(line(
        format!("THE WORLD FROM YEAR {} TO {}", from, to),
        ACCENT,
        BOLD,
    ));
    for (name, read) in SERIES {
        let values: Vec<f32> = w.history.samples.iter().map(read).collect();
        let now = values.last().copied().unwrap_or(0.0);
        let peak = values.iter().copied().fold(0.0f32, f32::max);
        out.push(line(
            format!(
                "{:<label$}{}  {:>9}  peak {:>7}",
                name,
                spark(&values, chart_w, ascii),
                compact(now),
                compact(peak),
                label = label_w,
            ),
            FG,
            0,
        ));
    }
    out.push(line(
        format!(
            "each line is scaled to itself, one column per {} years",
            (w.history.samples.len() * crate::sim::history::SAMPLE_EVERY as usize)
                .div_euclid(chart_w.max(1))
                .max(crate::sim::history::SAMPLE_EVERY as usize)
        ),
        DIMC,
        DIM,
    ));
    out
}

/// A section heading in the left column and its sentences on the right.
struct Section {
    label: &'static str,
    head: String,
    items: Vec<String>,
    color: Rgb,
}

fn clip(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    s.chars()
        .take(n.saturating_sub(1))
        .chain("…".chars())
        .collect()
}

/// Did this realm take part in the event?
fn touches(w: &World, e: usize, p: usize) -> bool {
    w.chronicle.events[e].refs.contains(&Ref::Polity(p))
}

/// `years` back from now, for the whole world or one realm, laid out for a
/// `width` by `height` page.
///
/// The height is what the digest spends: it fills the page it is given and
/// says "there was more" when it runs out, rather than stopping at a fixed
/// thirty lines and leaving the bottom third of a tall terminal blank.
/// How the covenant stands: who the watcher serves, how those people fare
/// against the world, and what has been spent on them.
///
/// The simulation does not know or care that anyone is watching, which is
/// the point of it — but the one thing a watcher has staked anything on
/// deserves an answer in plain words rather than a number in a corner of
/// the status bar.
fn covenant_report(w: &World) -> Vec<Line> {
    let mut out = Vec::new();
    let Some(c) = w.fate.patron.filter(|&c| c < w.cultures.len()) else {
        out.push(line("THE COVENANT", ACCENT, BOLD));
        out.push(line(
            "You are bound to no one and gather almost nothing. :bind <people>.",
            DIMC,
            DIM,
        ));
        out.push(line("", FG, 0));
        return out;
    };
    let cul = &w.cultures[c];
    let share = crate::sim::fate::devotion(w) * 100.0;
    let realms: Vec<usize> = w
        .alive_polities
        .iter()
        .copied()
        .filter(|&p| w.polities[p].culture == c)
        .collect();
    let greatest = realms
        .iter()
        .copied()
        .max_by_key(|&p| w.polities[p].cells)
        .map(|p| {
            format!(
                "{} ({})",
                w.polities[p].name,
                words::realm_size(w.polities[p].cells)
            )
        });
    out.push(line(
        format!("THE {}", cul.plural.to_uppercase()),
        ACCENT,
        BOLD,
    ));
    let standing = match cul.extinct {
        Some(y) => format!("gone since year {}. There is nothing left to be god of.", y),
        None => format!(
            "{} people, {:.1}% of the world, in {}.",
            words::folk(cul.pop as f32),
            share,
            count(realms.len(), "realm")
        ),
    };
    out.push(line(standing, FG, 0));
    if let Some(g) = greatest {
        out.push(line(format!("Their greatest realm is {}.", g), FG, 0));
    }
    out.push(line(
        format!(
            "Bound in year {}. {:.0} in hand, {:+.1} a year, {:.0} spent over {}.",
            w.fate.bound,
            w.fate.power,
            w.fate.income,
            w.fate.spent,
            count(w.fate.acts as usize, "act"),
        ),
        DIMC,
        DIM,
    ));
    // What you did with it. Without this the acts vanish into fifty
    // thousand events and the loop has no visible end.
    let mine: Vec<&crate::sim::chronicle::Event> = w
        .chronicle
        .events
        .iter()
        .rev()
        .filter(|e| e.by_fate)
        .take(4)
        .collect();
    if !mine.is_empty() {
        out.push(line("Your hand:", DIMC, DIM));
        for e in mine {
            out.push(line(format!("  {}  {}", e.year, clip(&e.text, 96)), FG, 0));
        }
    }
    out.push(line("", FG, 0));
    out
}

pub fn lines(
    w: &World,
    years: i32,
    only: Option<usize>,
    width: usize,
    height: usize,
    ascii: bool,
) -> Vec<Line> {
    let since = (w.year - years.max(1)).max(0);
    let mut out: Vec<Line> = Vec::new();
    let title = match only {
        Some(p) => format!(
            "{} — THE LAST {} YEARS",
            w.polities[p].name.to_uppercase(),
            years
        ),
        None => format!("THE WORLD — THE LAST {} YEARS", years),
    };
    out.push(line(title, ACCENT, BOLD));
    out.push(line(
        format!(
            "year {} to year {}   ·   :recap N for a longer look   ·   Esc to go back",
            since, w.year
        ),
        DIMC,
        DIM,
    ));
    out.push(line("", FG, 0));

    // The one section that is about the reader. It goes first, above the
    // world's own accounts, because a covenant is a wager and the first
    // thing anybody wants from a summary of the last two centuries is
    // whether they are winning it.
    if only.is_none() {
        out.extend(covenant_report(w));
    }

    // How the world has gone, before what it has been doing lately. Only
    // for the world as a whole: the series are world totals, and a realm's
    // digest asking after the world's population would be answering a
    // question nobody put.
    if only.is_none() {
        let chart = world_over_time(w, width, ascii);
        if !chart.is_empty() {
            out.extend(chart);
            out.push(line("", FG, 0));
        }
    }

    let mine = |p: usize| only.map(|q| q == p).unwrap_or(true);
    let mut sections: Vec<Section> = Vec::new();

    // Realms founded and fallen.
    let born: Vec<usize> = w
        .polities
        .iter()
        .filter(|p| p.founded >= since && (mine(p.id) || p.parent.map(mine).unwrap_or(false)))
        .map(|p| p.id)
        .collect();
    let gone: Vec<usize> = w
        .polities
        .iter()
        .filter(|p| p.fell.map(|y| y >= since).unwrap_or(false) && mine(p.id))
        .map(|p| p.id)
        .collect();
    if !born.is_empty() || !gone.is_empty() {
        let mut items = Vec::new();
        let mut big: Vec<usize> = born.clone();
        big.sort_by_key(|&p| std::cmp::Reverse(w.polities[p].peak_cells));
        for &p in big.iter().take(3) {
            items.push(format!(
                "{} rose in {} and is now {}.",
                w.polities[p].name,
                w.polities[p].founded,
                if w.polities[p].alive() {
                    words::realm_size(w.polities[p].cells).to_string()
                } else {
                    format!("gone, since {}", w.polities[p].fell.unwrap_or(0))
                }
            ));
        }
        let mut fallen: Vec<usize> = gone.clone();
        fallen.sort_by_key(|&p| std::cmp::Reverse(w.polities[p].peak_cells));
        for &p in fallen.iter().take(3) {
            items.push(format!(
                "{} fell in {}: it {}",
                w.polities[p].name,
                w.polities[p].fell.unwrap_or(0),
                w.polities[p].fall_cause
            ));
        }
        sections.push(Section {
            label: "Realms",
            head: format!("{} rose, {} fell.", count(born.len(), "realm"), gone.len()),
            items,
            color: FG,
        });
    }

    // Wars.
    let begun: Vec<usize> = w
        .wars
        .iter()
        .filter(|x| x.started >= since && (mine(x.attacker) || mine(x.defender)))
        .map(|x| x.id)
        .collect();
    let ended: Vec<usize> = w
        .wars
        .iter()
        .filter(|x| {
            x.ended.map(|y| y >= since).unwrap_or(false) && (mine(x.attacker) || mine(x.defender))
        })
        .map(|x| x.id)
        .collect();
    let burning: Vec<usize> = w
        .wars
        .iter()
        .filter(|x| x.alive() && (mine(x.attacker) || mine(x.defender)))
        .map(|x| x.id)
        .collect();
    if !begun.is_empty() || !ended.is_empty() || !burning.is_empty() {
        let mut items = Vec::new();
        let mut done: Vec<usize> = ended.clone();
        done.sort_by_key(|&x| std::cmp::Reverse(w.wars[x].battles));
        for &x in done.iter().take(3) {
            items.push(format!(
                "{} {}",
                crate::lang::capitalize(&w.wars[x].name),
                w.wars[x].result
            ));
        }
        let mut now: Vec<usize> = burning.clone();
        now.sort_by_key(|&x| std::cmp::Reverse(w.wars[x].battles));
        for &x in now.iter().take(2) {
            let v = crate::sim::explain::war_view(w, x);
            let lean = match v.leader {
                Some(p) => format!("{} is ahead", w.polities[p].short),
                None => "neither side is ahead".into(),
            };
            items.push(format!(
                "{} is still being fought, {} years in: {}.",
                crate::lang::capitalize(&w.wars[x].name),
                w.year - w.wars[x].started,
                lean
            ));
        }
        sections.push(Section {
            label: "Wars",
            // Deliberately not "began minus ended": wars that ended in this
            // window may have started before it, and the ones still burning
            // may have too, so the three numbers do not subtract.
            head: format!(
                "{} began and {} ended; {} being fought now.",
                count(begun.len(), "war"),
                count(ended.len(), "war"),
                count(burning.len(), "war")
            ),
            items,
            color: Rgb(240, 110, 100),
        });
    }

    // Rulers who died.
    let dead: Vec<usize> = w
        .persons
        .iter()
        .filter(|p| {
            p.role == Role::Ruler
                && p.died.map(|y| y >= since).unwrap_or(false)
                && p.polity.map(mine).unwrap_or(false)
        })
        .map(|p| p.id)
        .collect();
    if !dead.is_empty() {
        let mut best = dead.clone();
        best.sort_by(|&a, &b| w.persons[b].renown.total_cmp(&w.persons[a].renown));
        let items: Vec<String> = best
            .iter()
            .take(4)
            .map(|&i| {
                format!(
                    "{} of {}, {}: {}.",
                    w.persons[i].full_name(),
                    w.persons[i]
                        .polity
                        .map(|p| w.polities[p].short.clone())
                        .unwrap_or_default(),
                    w.persons[i].died.unwrap_or(0),
                    w.persons[i].death.trim_end_matches('.')
                )
            })
            .collect();
        sections.push(Section {
            label: "Rulers",
            head: format!("{} died.", count(dead.len(), "ruler")),
            items,
            color: Rgb(180, 160, 160),
        });
    }

    // Cities founded and lost.
    let founded: Vec<usize> = w
        .cities
        .iter()
        .filter(|c| c.founded >= since && c.polity.map(mine).unwrap_or(only.is_none()))
        .map(|c| c.id)
        .collect();
    let razed: Vec<usize> = w
        .cities
        .iter()
        .filter(|c| {
            c.destroyed.map(|y| y >= since).unwrap_or(false)
                && c.polity.map(mine).unwrap_or(only.is_none())
        })
        .map(|c| c.id)
        .collect();
    let sacked: Vec<usize> = w
        .chronicle
        .events
        .iter()
        .enumerate()
        .filter(|(_, e)| e.year >= since && e.text.contains("sack"))
        .filter(|(i, _)| only.map(|p| touches(w, *i, p)).unwrap_or(true))
        .map(|(i, _)| i)
        .collect();
    if !founded.is_empty() || !razed.is_empty() || !sacked.is_empty() {
        let mut items = Vec::new();
        let mut biggest = founded.clone();
        biggest.sort_by(|&a, &b| w.cities[b].pop.total_cmp(&w.cities[a].pop));
        for &c in biggest.iter().take(2) {
            items.push(format!(
                "{} was founded in {} and is {} of {} people.",
                w.cities[c].name,
                w.cities[c].founded,
                words::city_size(w.cities[c].pop),
                words::folk(w.cities[c].pop)
            ));
        }
        for &c in razed.iter().take(2) {
            items.push(format!(
                "{} was destroyed in {}.",
                w.cities[c].name,
                w.cities[c].destroyed.unwrap_or(0)
            ));
        }
        for &i in sacked.iter().rev().take(2) {
            items.push(w.chronicle.events[i].text.clone());
        }
        sections.push(Section {
            label: "Cities",
            head: format!(
                "{} founded, {} destroyed, {} sacked.",
                count(founded.len(), "city"),
                razed.len(),
                sacked.len()
            ),
            items,
            color: Rgb(255, 255, 255),
        });
    }

    // Schools of thought.
    let new_schools: Vec<usize> = w
        .schools
        .iter()
        .filter(|s| {
            s.founded >= since
                && only
                    .map(|p| {
                        s.influence.contains_key(&p) || w.cities[s.home_city].polity == Some(p)
                    })
                    .unwrap_or(true)
        })
        .map(|s| s.id)
        .collect();
    let doctrine: Vec<usize> = w
        .chronicle
        .events
        .iter()
        .enumerate()
        .filter(|(_, e)| e.year >= since && e.kind == EventKind::Magic && e.importance >= 2)
        .filter(|(i, _)| only.map(|p| touches(w, *i, p)).unwrap_or(true))
        .map(|(i, _)| i)
        .collect();
    if !new_schools.is_empty() || !doctrine.is_empty() {
        let mut items = Vec::new();
        for &s in new_schools.iter().take(2) {
            items.push(format!(
                "{} was founded in {} at {}; it is now {}.",
                w.schools[s].name,
                w.schools[s].founded,
                w.cities[w.schools[s].home_city].name,
                words::school_reach(w, s)
            ));
        }
        for &i in doctrine.iter().rev().take(3) {
            items.push(w.chronicle.events[i].text.clone());
        }
        sections.push(Section {
            label: "Schools",
            head: format!(
                "{} founded; {} turns of doctrine.",
                count(new_schools.len(), "school"),
                doctrine.len()
            ),
            items,
            color: Rgb(200, 140, 240),
        });
    }

    // Who is growing and who is shrinking.
    let mut movers: Vec<usize> = w
        .living_polities()
        .into_iter()
        .filter(|&p| mine(p) && w.polities[p].reign_start >= since)
        .collect();
    movers.sort_by_key(|&p| std::cmp::Reverse(w.polities[p].reign_gained.abs()));
    let peaks: Vec<usize> = w
        .living_polities()
        .into_iter()
        .filter(|&p| mine(p) && w.polities[p].peak_year >= since && w.polities[p].peak_cells >= 40)
        .collect();
    if !movers.is_empty() || !peaks.is_empty() {
        let mut items = Vec::new();
        for &p in movers.iter().take(3) {
            let pol = &w.polities[p];
            if pol.reign_gained.abs() < 6 {
                continue;
            }
            items.push(format!(
                "{} has {} {} lands under {}, and is now {}.",
                pol.short,
                if pol.reign_gained > 0 { "won" } else { "lost" },
                pol.reign_gained.abs(),
                w.ruler_short(p),
                words::realm_size(pol.cells)
            ));
        }
        for &p in peaks.iter().take(2) {
            let pol = &w.polities[p];
            if pol.peak_cells > pol.cells + pol.cells / 8 {
                items.push(format!(
                    "{} was at its greatest in {} with {} lands; it holds {} now.",
                    pol.short, pol.peak_year, pol.peak_cells, pol.cells
                ));
            }
        }
        if !items.is_empty() {
            sections.push(Section {
                label: "Fortunes",
                head: String::new(),
                items,
                color: Rgb(150, 220, 150),
            });
        }
    }

    // The figures of the period, and the houses that ended in it. This is
    // the section a reader scans first: a fifty-year digest that names no
    // people is a weather report.
    {
        let mut items: Vec<String> = Vec::new();
        let mut named: Vec<usize> = (0..w.persons.len())
            .filter(|&i| {
                w.persons[i].acclaimed.map(|y| y >= since).unwrap_or(false)
                    && mine(w.persons[i].polity.unwrap_or(usize::MAX))
            })
            .collect();
        named.sort_by(|&a, &b| w.persons[b].greatness.total_cmp(&w.persons[a].greatness));
        for &i in named.iter().take(3) {
            let per = &w.persons[i];
            let realm = per
                .polity
                .map(|p| w.polities[p].short.clone())
                .unwrap_or_default();
            let deed = crate::sim::dynasty::standing(w, i)
                .first()
                .map(|c| c.what.clone())
                .unwrap_or_default();
            items.push(format!(
                "{} of {} was called great in {}, having {}.{}",
                per.full_name(),
                realm,
                per.acclaimed.unwrap_or(0),
                deed,
                if per.alive() { " Still living." } else { "" }
            ));
        }
        // Somebody still alive and still worth watching, even if the world
        // named them before this period began.
        let mut living: Vec<usize> = (0..w.persons.len())
            .filter(|&i| {
                w.persons[i].alive()
                    && w.persons[i].is_acclaimed()
                    && w.persons[i].acclaimed.map(|y| y < since).unwrap_or(false)
                    && mine(w.persons[i].polity.unwrap_or(usize::MAX))
            })
            .collect();
        living.sort_by(|&a, &b| w.persons[b].greatness.total_cmp(&w.persons[a].greatness));
        for &i in living.iter().take(2) {
            let per = &w.persons[i];
            items.push(format!(
                "{} still holds {}, {} years on the throne.",
                per.full_name(),
                per.polity
                    .map(|p| w.polities[p].short.clone())
                    .unwrap_or_default(),
                per.reign_years
            ));
        }
        let ended: Vec<usize> = (0..w.houses.len())
            .filter(|&h| {
                w.houses[h].ended.map(|y| y >= since).unwrap_or(false)
                    && w.houses[h].seniors.len() >= 3
            })
            .collect();
        for &h in ended.iter().take(2) {
            let ho = &w.houses[h];
            items.push(format!(
                "{} died out after {} years and {} who ruled.",
                ho.name,
                ho.span(w.year).max(0),
                ho.seniors.len()
            ));
        }
        if !items.is_empty() {
            sections.push(Section {
                label: "Figures",
                head: String::new(),
                items,
                color: Rgb(255, 210, 90),
            });
        }
    }

    // The loudest things the chronicle itself recorded.
    let loud: Vec<usize> = w
        .chronicle
        .events
        .iter()
        .enumerate()
        .filter(|(_, e)| e.year >= since && e.importance >= 3)
        .filter(|(i, _)| only.map(|p| touches(w, *i, p)).unwrap_or(true))
        .map(|(i, _)| i)
        .collect();
    if !loud.is_empty() {
        let items: Vec<String> = loud
            .iter()
            .rev()
            .take(4)
            .map(|&i| {
                format!(
                    "{}  {}",
                    w.chronicle.events[i].year, w.chronicle.events[i].text
                )
            })
            .collect();
        sections.push(Section {
            label: "Loudest",
            head: format!("{} the world could not ignore.", count(loud.len(), "thing")),
            items,
            color: ACCENT,
        });
    }

    if sections.is_empty() {
        out.push(line(
            "Nothing of note. The world kept turning and no one wrote it down.",
            DIMC,
            0,
        ));
        return out;
    }

    // A digest that runs past a screen is not a digest: every section gets
    // its heading, and the detail underneath shares what is left.
    let lw = 12usize;
    let body = width.saturating_sub(lw + 4).max(24);
    // Two rows per section go on its heading and the blank line after it;
    // the rest is shared out between them.
    let budget = height.saturating_sub(4).clamp(12, 80);
    let spare = budget.saturating_sub(sections.len() * 2);
    let per_section = (spare / sections.len().max(1)).max(1);
    let mut cut = false;
    // The share-out above can round up — `per_section` has a floor of one
    // line, so enough sections will still overrun a short page. The page is
    // a promise, so it is also enforced here: once there is only room for
    // the "there was more" line, the digest stops.
    let room = |out: &Vec<Line>| out.len() + 1 < height;
    for s in sections {
        if !room(&out) {
            cut = true;
            break;
        }
        if !s.head.is_empty() {
            out.push(line(
                format!("{:<lw$}{}", s.label, s.head, lw = lw),
                s.color,
                BOLD,
            ));
        } else {
            out.push(line(format!("{:<lw$}", s.label), s.color, BOLD));
        }
        let mut used = 0usize;
        for it in s.items {
            if used >= per_section || !room(&out) {
                cut = true;
                break;
            }
            for (k, l) in crate::term::wrap(&clip(&it, body * 2), body)
                .into_iter()
                .take(2)
                .enumerate()
            {
                let pre = if k == 0 {
                    format!("{:<lw$}  ", "", lw = lw)
                } else {
                    format!("{:<lw$}    ", "", lw = lw)
                };
                out.push(line(format!("{}{}", pre, l), FG, 0));
                used += 1;
            }
        }
        out.push(line("", FG, 0));
    }
    // Trailing blank lines are not worth a row of a page this tight.
    while out.last().map(|l| l.text.is_empty()).unwrap_or(false) {
        out.pop();
    }
    if cut && out.len() < height {
        out.push(line(
            "There was more. The chronicle (c) has all of it.",
            DIMC,
            DIM,
        ));
    }
    // The page is a promise: whatever the share-out above worked out, the
    // digest never returns more rows than it was given.
    out.truncate(height);
    out
}

fn count(n: usize, noun: &str) -> String {
    let plural = if n == 1 {
        noun.to_string()
    } else if let Some(stem) = noun.strip_suffix('y') {
        format!("{}ies", stem)
    } else {
        format!("{}s", noun)
    };
    format!("{} {}", n, plural)
}

#[cfg(test)]
mod chart_tests {
    use super::*;
    use crate::sim::Detail;

    /// A sparkline must be the width it was asked for, keep the peak, and
    /// draw something for a series that never moves.
    #[test]
    fn a_sparkline_fits_and_keeps_its_peak() {
        let steps = ramp(false);
        // More samples than columns is the normal case in a long world, and
        // a spike must survive the squeeze: a plague that halved a
        // population should not be averaged away.
        let mut values = vec![1.0f32; 200];
        values[137] = 100.0;
        let s = spark(&values, 40, false);
        assert_eq!(s.chars().count(), 40);
        assert!(
            s.contains(steps[7]),
            "the peak was lost when 200 samples were squeezed into 40 columns"
        );
        // Fewer samples than columns is fine too.
        assert_eq!(spark(&[1.0, 2.0, 3.0], 30, false).chars().count(), 30);
        // A flat series reads as a band, not as noise, and a series of
        // nothing at all does not divide by zero.
        let flat = spark(&[4.0; 20], 12, false);
        assert_eq!(flat.chars().count(), 12);
        assert!(flat.chars().all(|c| c == steps[7]));
        assert_eq!(spark(&[0.0; 20], 12, false).chars().count(), 12);
        assert_eq!(spark(&[], 12, false), "");
        assert_eq!(spark(&[1.0], 0, false), "");
        // And the ascii ramp is ascii, because the frame's sweep would turn
        // the block elements into question marks and still pass the test
        // that forbids Unicode.
        assert!(ramp(true).iter().all(char::is_ascii));
        assert!(spark(&[1.0, 5.0, 3.0], 20, true).is_ascii());
    }

    /// The world's record of itself is drawn once there is enough of it,
    /// and says nothing at all before that.
    #[test]
    fn the_world_over_time_waits_until_it_has_something_to_say() {
        let mut w = crate::sim::World::new(29, 60, 30, Detail::Medium);
        assert!(
            world_over_time(&w, 100, false).is_empty(),
            "a world with no history drew a chart of it"
        );
        for _ in 0..600 {
            w.tick();
        }
        let lines = world_over_time(&w, 100, false);
        // A heading, every series, and the note about the scale.
        assert_eq!(lines.len(), crate::sim::history::SERIES.len() + 2);
        for (name, _) in crate::sim::history::SERIES {
            assert!(
                lines.iter().any(|l| l.text.starts_with(name)),
                "no row for {}",
                name
            );
        }
        // Every row fits the width it was given, or the page would spill
        // into the margin and the peak would be clipped in half.
        for l in &lines {
            assert!(
                l.text.chars().count() <= 100,
                "a {} row is {} wide against a page of 100",
                l.text.split_whitespace().next().unwrap_or(""),
                l.text.chars().count()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::Detail;

    #[test]
    fn counts_are_english() {
        assert_eq!(count(1, "war"), "1 war");
        assert_eq!(count(3, "war"), "3 wars");
        assert_eq!(count(1, "city"), "1 city");
        assert_eq!(count(0, "city"), "0 cities");
    }

    #[test]
    fn a_recap_stays_short_and_readable() {
        let mut w = World::new(4, 60, 30, Detail::Medium);
        for _ in 0..300 {
            w.tick();
        }
        for years in [10, 50, 100, 1000] {
            let ls = lines(&w, years, None, 100, 44, false);
            assert!(ls.len() <= 44, "{} lines for {} years", ls.len(), years);
            for l in &ls {
                assert!(l.text.chars().count() <= 100, "{}", l.text);
            }
        }
        // And for one realm.
        if let Some(&p) = w.living_polities().first() {
            let ls = lines(&w, 100, Some(p), 100, 44, false);
            assert!(!ls.is_empty());
            assert!(ls[0].text.contains("THE LAST 100 YEARS"));
        }
    }

    #[test]
    fn an_empty_world_still_says_something() {
        let w = World::new(3, 40, 20, Detail::Low);
        let ls = lines(&w, 50, None, 80, 24, false);
        assert!(ls.len() >= 3);
    }
}
