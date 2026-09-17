//! Filtering a list by what things *are*, not only what they are called.
//!
//! The list pages have always had a filter, and it matched the text of a
//! rendered row: type `khax` and see the rows mentioning Khax. That answers
//! one question well and every other question not at all. "Which realms are
//! running a deficit", "which of my cities are worth more than forty a year
//! in trade", "who is over sixty and has never held a throne" — none of
//! those are in the text of a row, and all of them are in the world.
//!
//! So a filter term is now either a word to look for, as before, or a
//! comparison against a named quantity: `lands>200`, `stability<0.3`,
//! `treasury>=100`. Terms are separated by spaces and all of them must hold,
//! which is enough to ask a surprisingly specific question and needs no
//! parser worth the name.
//!
//! The fields are per kind of thing — a realm has lands and a treasury, a
//! war has battles and years — and [`fields_for`] lists the ones that apply
//! so the interface can say what may be asked. A term naming a field that
//! does not apply matches nothing, which is the honest answer: no city has a
//! war-weariness.

use crate::sim::chronicle::Ref;
use crate::sim::{explain, World};

/// How a value is compared.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Op {
    Gt,
    Ge,
    Lt,
    Le,
    Eq,
}

impl Op {
    fn holds(self, a: f32, b: f32) -> bool {
        match self {
            Op::Gt => a > b,
            Op::Ge => a >= b,
            Op::Lt => a < b,
            Op::Le => a <= b,
            // Deliberately loose: nobody means a bit-exact float when they
            // type `prosperity=1`.
            Op::Eq => (a - b).abs() < 0.005 + b.abs() * 0.005,
        }
    }
}

/// One condition a row must satisfy.
#[derive(Clone, Debug)]
pub enum Term {
    /// A word that must appear in the row's text.
    Text(String),
    /// A named quantity compared against a number.
    Compare { field: String, op: Op, value: f32 },
}

/// Break a filter into its terms.
///
/// Anything that does not parse as a comparison is a word to look for, so a
/// half-typed query behaves like the plain text filter it used to be rather
/// than matching nothing while the number is still being entered.
pub fn parse(s: &str) -> Vec<Term> {
    s.split_whitespace().map(parse_term).collect()
}

fn parse_term(t: &str) -> Term {
    // Longest operators first, or `>=` parses as `>` and leaves an `=`.
    for (sym, op) in [
        (">=", Op::Ge),
        ("<=", Op::Le),
        (">", Op::Gt),
        ("<", Op::Lt),
        ("=", Op::Eq),
    ] {
        if let Some((name, rest)) = t.split_once(sym) {
            if let Ok(v) = rest.trim().parse::<f32>() {
                if !name.trim().is_empty() {
                    return Term::Compare {
                        field: name.trim().to_lowercase(),
                        op,
                        value: v,
                    };
                }
            }
        }
    }
    Term::Text(t.to_lowercase())
}

/// Whether a row passes every term.
pub fn matches(w: &World, r: Ref, text: &str, terms: &[Term]) -> bool {
    let lower = text.to_lowercase();
    terms.iter().all(|t| match t {
        Term::Text(s) => lower.contains(s),
        Term::Compare { field, op, value } => {
            value_of(w, r, field).is_some_and(|v| op.holds(v, *value))
        }
    })
}

/// The named quantities of whatever this row points at.
///
/// Everything is scaled the way the interface prints it — `stability<30`
/// rather than `0.3`, `prosperity>150` rather than `1.5` — because a filter
/// that disagreed with the column beside it would be a trap. Prosperity is
/// an index where a hundred is an ordinary town rather than a share of
/// anything, and it runs past two hundred.
pub fn value_of(w: &World, r: Ref, field: &str) -> Option<f32> {
    match r {
        Ref::Polity(p) if p < w.polities.len() => {
            let pol = &w.polities[p];
            Some(match field {
                "lands" | "cells" => pol.cells as f32,
                "people" | "pop" => pol.pop as f32,
                "stability" => pol.stability * 100.0,
                "treasury" | "gold" => pol.treasury,
                "income" => explain::income_total(w, p),
                "devt" | "dev" | "development" => pol.dev,
                "army" => pol.army,
                "cities" | "towns" => pol.cities.len() as f32,
                "trade" => w.realm_trade(p),
                "wars" => pol.wars.iter().filter(|&&x| w.wars[x].alive()).count() as f32,
                "age" => (w.year - pol.founded) as f32,
                "peak" => pol.peak_cells as f32,
                "sprawl" => pol.sprawl * 100.0,
                "reach" => pol.tech_admin,
                _ => return None,
            })
        }
        Ref::City(c) if c < w.cities.len() => {
            let city = &w.cities[c];
            Some(match field {
                "people" | "pop" => city.pop,
                "prosperity" => city.prosperity * 100.0,
                "walls" => city.walls * 100.0,
                "trade" => w.city_trade(c),
                "roads" => w.trade_partners(c).len() as f32,
                "founded" => city.founded as f32,
                "age" => (w.year - city.founded) as f32,
                "sacked" => city.times_sacked as f32,
                "wonders" => city.wonders.len() as f32,
                "peak" => city.peak_pop,
                _ => return None,
            })
        }
        Ref::Person(i) if i < w.persons.len() => {
            let per = &w.persons[i];
            Some(match field {
                "renown" => per.renown,
                "greatness" | "points" => per.greatness,
                "born" => per.born as f32,
                "age" => per.age(w.year) as f32,
                "reign" | "years" => per.reign_years as f32,
                "battles" | "won" => per.battles_won as f32,
                "children" | "heirs" => per.children.len() as f32,
                "taken" | "lands" => per.taken as f32,
                _ => return None,
            })
        }
        Ref::War(x) if x < w.wars.len() => {
            let war = &w.wars[x];
            Some(match field {
                "years" => (war.ended.unwrap_or(w.year) - war.started) as f32,
                "battles" => war.battles as f32,
                "started" => war.started as f32,
                "score" => war.score * 100.0,
                "lands" | "taken" => war.cells_taken as f32,
                "allies" => (war.allies_a.len() + war.allies_d.len()) as f32,
                _ => return None,
            })
        }
        Ref::House(h) if h < w.houses.len() => {
            let house = &w.houses[h];
            Some(match field {
                "span" | "years" => house.span(w.year) as f32,
                "people" | "members" => house.members.len() as f32,
                "thrones" | "realms" => house.realms.len() as f32,
                "rulers" | "ruled" => house.seniors.len() as f32,
                _ => return None,
            })
        }
        Ref::Culture(c) if c < w.cultures.len() => {
            let cu = &w.cultures[c];
            Some(match field {
                "lands" | "cells" => cu.cells as f32,
                "people" | "pop" => cu.pop as f32,
                _ => return None,
            })
        }
        _ => None,
    }
}

/// What may be asked about the rows of a given list page, for the hint line.
pub fn fields_for(tab: usize) -> &'static str {
    match tab {
        0 | 11 => {
            "lands people stability treasury income devt army cities trade wars age peak sprawl"
        }
        1 => "people prosperity walls trade roads founded age sacked wonders peak",
        2 => "lands people",
        4 | 9 => "renown greatness born age reign battles children lands",
        5 => "years battles started score lands allies",
        10 => "span people thrones rulers",
        12 => "people prosperity trade roads",
        _ => "",
    }
}
