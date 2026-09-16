//! Causes in plain words. Every function here is a pure reading of a
//! `&World`: why a realm holds together or does not, who is winning a war
//! and why, where a person stands. The weights mirror `politics::economy`
//! and `war::strength`, so the explanation and the simulation agree.

use crate::sim::{PolityKind, Role, WarKind, World};

/// One named pull on a number, with the size of its contribution.
#[derive(Clone, Debug)]
pub struct Factor {
    pub text: String,
    pub weight: f32,
}

/// What stability settles at with an average ruler and nothing else
/// pulling on it: the tuned floor plus a middling ruler's wisdom and charm.
pub fn stability_base(w: &World) -> f32 {
    let tn = &w.tuning;
    tn.stability_base + 0.5 * tn.stability_wisdom_weight + 0.5 * tn.stability_charisma_weight
}

fn push(out: &mut Vec<Factor>, weight: f32, text: String) {
    if weight.abs() >= 0.01 {
        out.push(Factor { text, weight });
    }
}

/// Average prosperity of a realm's cities, as `politics::economy` sees it.
fn avg_prosperity(w: &World, p: usize) -> f32 {
    let cities = &w.polities[p].cities;
    if cities.is_empty() {
        return 0.3;
    }
    cities.iter().map(|&c| w.cities[c].prosperity).sum::<f32>() / cities.len() as f32
}

/// How long the realm's longest running war has gone on.
fn war_years(w: &World, p: usize) -> i32 {
    w.polities[p]
        .wars
        .iter()
        .map(|&x| w.year - w.wars[x].started)
        .max()
        .unwrap_or(0)
        .max(1)
}

/// Everything pulling a realm's stability up or down, strongest first.
/// `stability_base` plus the weights is the value stability is drifting
/// towards, which is what `politics::economy` computes each year.
pub fn stability_factors(w: &World, p: usize) -> Vec<Factor> {
    let mut out: Vec<Factor> = Vec::new();
    if p >= w.polities.len() || !w.polities[p].alive() {
        return out;
    }
    let pol = &w.polities[p];
    let tn = &w.tuning;
    let vals = w.cultures[pol.culture].values;

    // The ruler.
    match pol.ruler {
        Some(r) => {
            let t = w.persons[r].traits;
            push(
                &mut out,
                (t.wisdom - 0.5) * tn.stability_wisdom_weight,
                if t.wisdom >= 0.5 {
                    "the ruler governs wisely".into()
                } else {
                    "the ruler is out of their depth".into()
                },
            );
            push(
                &mut out,
                (t.charisma - 0.5) * tn.stability_charisma_weight,
                if t.charisma >= 0.5 {
                    "the ruler is beloved".into()
                } else {
                    "the ruler is cold to the people".into()
                },
            );
        }
        None => push(
            &mut out,
            (0.3 - 0.5) * (tn.stability_wisdom_weight + tn.stability_charisma_weight),
            "there is no ruler, and the great men quarrel".into(),
        ),
    }

    // Overextension.
    let over = pol.overextension(tn);
    push(
        &mut out,
        -(over - 1.0).max(0.0) * tn.stability_overextension_weight,
        format!(
            "overextended: {} lands, but the crown can govern about {:.0}",
            pol.cells,
            pol.admin_capacity(tn)
        ),
    );

    // Foreign subjects.
    push(
        &mut out,
        -pol.foreign_share * tn.stability_foreign_weight,
        format!(
            "{} in every 10 subjects are of other peoples",
            (pol.foreign_share * 10.0).round().max(1.0)
        ),
    );

    // War-weariness.
    push(
        &mut out,
        -pol.exhaustion * tn.stability_exhaustion_weight,
        if pol.at_war() {
            format!(
                "war-weary after {} of fighting",
                crate::sim::prose::years(war_years(w, p) as i64)
            )
        } else {
            "still weary from the last war".into()
        },
    );

    // Decadence.
    push(
        &mut out,
        -pol.decadence * tn.stability_decadence_weight,
        if pol.decadence > 0.6 {
            "the court has rotted through with luxury".into()
        } else {
            "the dynasty has grown decadent".into()
        },
    );

    // The cities.
    let prosp = avg_prosperity(w, p);
    push(
        &mut out,
        (prosp - 0.4) * 0.15,
        if prosp >= 0.4 {
            "the cities are prosperous".into()
        } else {
            "the cities are poor".into()
        },
    );

    // Custom.
    push(
        &mut out,
        vals.tradition * 0.05,
        "old custom binds the people to the throne".into(),
    );

    // The treasury.
    push(
        &mut out,
        (pol.treasury / 600.0).max(-0.2) * 0.2,
        if pol.treasury >= 0.0 {
            "the treasury is full".into()
        } else {
            "the coffers are empty".into()
        },
    );

    // A state doctrine.
    let (_, stab_bonus, _, _) = w.school_effects(p);
    if let Some(s) = pol.school {
        push(
            &mut out,
            stab_bonus,
            format!("{} upholds the throne", w.schools[s].name),
        );
    }

    // What kind of thing it is.
    let (kind_stab, kind_text) = match pol.kind {
        PolityKind::Kingdom | PolityKind::Republic => (0.05, "settled law and long custom hold"),
        PolityKind::Empire => (-0.08, "an empire is a hard thing to hold together"),
        PolityKind::Horde => (-0.1, "a horde obeys only while it is winning"),
        PolityKind::Theocracy => (0.03, "the temple's authority steadies the realm"),
        _ => (0.0, ""),
    };
    push(&mut out, kind_stab, kind_text.to_string());

    out.sort_by(|a, b| b.weight.abs().total_cmp(&a.weight.abs()));
    out
}

/// Where stability is heading, given everything pulling on it.
pub fn stability_target(w: &World, p: usize) -> f32 {
    (stability_base(w)
        + stability_factors(w, p)
            .iter()
            .map(|f| f.weight)
            .sum::<f32>())
    .clamp(0.0, 1.0)
}

/// How a realm's stability is moving: up, down or holding.
pub fn stability_drift(w: &World, p: usize) -> &'static str {
    let target = stability_target(w, p);
    let now = w.polities[p].stability;
    if target > now + 0.06 {
        "and recovering"
    } else if target < now - 0.06 {
        "and getting worse"
    } else {
        "and holding there"
    }
}

// ---------------------------------------------------------------------------
// War
// ---------------------------------------------------------------------------

/// A reading of a war: how the two sides compare, who is ahead and what a
/// peace signed this year would look like.
pub struct WarView {
    pub attacker_strength: f32,
    pub defender_strength: f32,
    /// Polity ahead on the war score, or `None` if it is even.
    pub leader: Option<usize>,
    pub reasons: Vec<String>,
    pub peace: String,
}

/// Field strength, as `war::strength` computes it before terrain.
fn field_strength(w: &World, p: usize) -> f32 {
    let pol = &w.polities[p];
    let t = crate::sim::politics::traits_of(w, p);
    let generals = pol
        .generals
        .iter()
        .filter(|&&g| w.persons[g].alive())
        .count()
        .min(2) as f32;
    (pol.army * (0.7 + t.valor * 0.6) * (0.55 + pol.stability * 0.5) * (1.0 + generals * 0.15))
        .max(0.05)
}

fn living_generals(w: &World, p: usize) -> usize {
    w.polities[p]
        .generals
        .iter()
        .filter(|&&g| w.persons[g].alive())
        .count()
        .min(2)
}

/// Average defensive value of the land a realm holds along its borders.
fn home_defense(w: &World, p: usize) -> f32 {
    let mut sum = 0.0;
    let mut n = 0usize;
    for i in 0..w.cells.len() {
        if w.cells[i].owner != Some(p) {
            continue;
        }
        if w.terrain
            .neighbors4(i)
            .any(|nb| w.cells[nb].owner != Some(p))
        {
            sum += w.terrain.biome[i].defense();
            n += 1;
            if n >= 200 {
                break;
            }
        }
    }
    if n == 0 {
        1.0
    } else {
        sum / n as f32
    }
}

pub fn war_view(w: &World, wid: usize) -> WarView {
    let war = &w.wars[wid];
    let (a, d) = (war.attacker, war.defender);
    let (sa, sd) = (field_strength(w, a), field_strength(w, d));
    let (da, dd) = (home_defense(w, a), home_defense(w, d));
    let mut reasons: Vec<String> = Vec::new();

    let (an, dn) = (w.polities[a].short.clone(), w.polities[d].short.clone());
    let ratio = sa / sd.max(0.001);
    // Negating Range::contains would also include NaN; keep ordered comparisons.
    #[allow(clippy::manual_range_contains)]
    if ratio > 1.2 || ratio < 0.83 {
        let (big, small, big_s, small_s) = if ratio > 1.0 {
            (&an, &dn, sa, sd)
        } else {
            (&dn, &an, sd, sa)
        };
        reasons.push(format!(
            "{} brings the heavier host into the field ({:.0} against {:.0})",
            big, big_s, small_s
        ));
        let _ = small;
        let _ = small_s;
    } else {
        reasons.push("the two hosts are evenly matched in the field".into());
    }
    let (ga, gd) = (living_generals(w, a), living_generals(w, d));
    if ga != gd {
        let (who, n) = if ga > gd { (&an, ga) } else { (&dn, gd) };
        reasons.push(format!(
            "{} has {} of note to command",
            who,
            crate::sim::prose::count(n as i64, "general")
        ));
    }
    if (da - dd).abs() > 0.08 {
        let (who, def) = if dd > da { (&dn, dd) } else { (&an, da) };
        reasons.push(format!(
            "the marches of {} are hard country: a defender there fights as though {:.0}% stronger",
            who,
            (def - 1.0) * 100.0
        ));
    }
    let (stab_a, stab_d) = (w.polities[a].stability, w.polities[d].stability);
    if (stab_a - stab_d).abs() > 0.2 {
        let (who, other) = if stab_a < stab_d {
            (&an, &dn)
        } else {
            (&dn, &an)
        };
        reasons.push(format!(
            "{} is troubled at home while {} is not",
            who, other
        ));
    }
    if war.cells_taken.abs() >= 3 {
        let (who, n) = if war.cells_taken > 0 {
            (&an, war.cells_taken)
        } else {
            (&dn, -war.cells_taken)
        };
        reasons.push(format!(
            "{} fought, and {} holds {} it did not hold before",
            crate::sim::prose::count(war.battles as i64, "battle"),
            who,
            crate::sim::prose::count(n as i64, "land")
        ));
    }

    let leader = if war.score > 0.15 {
        Some(a)
    } else if war.score < -0.15 {
        Some(d)
    } else {
        None
    };

    let rebellion = matches!(
        war.kind,
        WarKind::Rebellion | WarKind::CivilWar | WarKind::Succession
    );
    let peace = if rebellion {
        if war.score < -0.4 {
            format!(
                "Were it settled now, {} would be beaten and {} would be no more.",
                crate::sim::prose::insurgent(war.kind),
                w.polities[a].name
            )
        } else if war.score > 0.8 && w.polities[a].cells > w.polities[d].cells {
            format!(
                "Were it settled now, {} would take the old capital and rule in its place.",
                w.polities[a].name
            )
        } else {
            format!(
                "Were it settled now, {} would recognise the independence of {}.",
                dn, w.polities[a].name
            )
        }
    } else if war.score > 0.5 {
        format!(
            "Were peace signed now, {} would keep what it has taken and {} would pay tribute.",
            an, dn
        )
    } else if war.score < -0.5 {
        format!(
            "Were peace signed now, {} would give back its gains and pay {} tribute.",
            an, dn
        )
    } else {
        "Were peace signed now, neither side would have anything to show for it.".into()
    };

    WarView {
        attacker_strength: sa,
        defender_strength: sd,
        leader,
        reasons,
        peace,
    }
}

/// How close the fighting is to ending, in words.
pub fn war_weariness(w: &World, wid: usize) -> &'static str {
    let war = &w.wars[wid];
    if war.ended.is_some() {
        return "over";
    }
    let ex = (w.polities[war.attacker].exhaustion + w.polities[war.defender].exhaustion) / 2.0;
    if w.year - war.started < 3 {
        "neither side will yet talk of peace"
    } else if ex > 0.8 {
        "both sides are spent, and peace cannot be far off"
    } else if ex > 0.4 {
        "the armies are tiring"
    } else {
        "both sides still have the stomach for it"
    }
}

// ---------------------------------------------------------------------------
// People
// ---------------------------------------------------------------------------

/// Small numbers read better as words.
pub fn spell(n: i32) -> String {
    const ONES: [&str; 20] = [
        "no",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];
    const TENS: [&str; 10] = [
        "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
    ];
    if !(0..100).contains(&n) {
        return n.to_string();
    }
    if n < 20 {
        return ONES[n as usize].to_string();
    }
    let (t, o) = ((n / 10) as usize, (n % 10) as usize);
    if o == 0 {
        TENS[t].to_string()
    } else {
        format!("{}-{}", TENS[t], ONES[o])
    }
}

/// One sentence placing a person in the world.
pub fn standing(w: &World, id: usize) -> String {
    let per = &w.persons[id];
    let t = per.traits;
    let name_of = |p: usize| w.polities[p].short.clone();
    // A ruler: how long, and how remembered.
    if let Some(p) = per.polity {
        if w.polities[p].ruler == Some(id) {
            let years = (w.year - w.polities[p].reign_start).max(0);
            let word = if t.cruelty > 0.7 {
                "a feared"
            } else if t.charisma > 0.7 {
                "a beloved"
            } else if t.wisdom > 0.7 {
                "a wise"
            } else if t.wisdom < 0.3 {
                "a hapless"
            } else {
                "an unremarkable"
            };
            let gained = w.polities[p].reign_gained;
            let tail = if gained > 30 {
                format!(", who has won {} lands for the realm", gained)
            } else if gained < -30 {
                format!(", who has lost {} lands of it", -gained)
            } else {
                String::new()
            };
            return format!(
                "{} ruler of {} of {} year{}{}.",
                word,
                name_of(p),
                spell(years),
                if years == 1 { "" } else { "s" },
                tail
            );
        }
    }
    match per.role {
        Role::General => {
            if per.battles_won >= 5 {
                format!(
                    "A general of {} victories, whose name is known far past the border.",
                    spell(per.battles_won as i32)
                )
            } else if per.battles_won > 0 {
                format!(
                    "A general with {} victor{} to their name.",
                    spell(per.battles_won as i32),
                    if per.battles_won == 1 { "y" } else { "ies" }
                )
            } else {
                "A general who has yet to be tested in the field.".into()
            }
        }
        Role::Ruler => match per.polity {
            Some(p) => format!(
                "Once of {}; the throne has since passed on.",
                w.polities[p].name
            ),
            None => "A ruler of a realm that is no longer.".into(),
        },
        Role::Mage | Role::Prophet | Role::Philosopher => {
            let founded = w.schools.iter().any(|s| s.founder == id);
            let seen = w.prophecies.iter().filter(|pr| pr.seer == id).count();
            if founded {
                "The founder of a school of thought that outlived them.".into()
            } else if seen > 0 {
                format!(
                    "A seer who has spoken {} prophec{}.",
                    spell(seen as i32),
                    if seen == 1 { "y" } else { "ies" }
                )
            } else {
                format!("{} of some standing.", title_case(per.role.name()))
            }
        }
        Role::Poet => "A poet, remembered for words rather than deeds.".into(),
        Role::Rebel => "A rebel, whose cause the chronicle judges by its end.".into(),
        Role::Martyr => "A martyr, killed for what they would not give up.".into(),
        Role::Explorer => "An explorer, who went where the maps stopped.".into(),
    }
}

fn title_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{Detail, World};

    fn world() -> World {
        let mut w = World::new(7, 60, 30, Detail::Medium);
        for _ in 0..200 {
            w.tick();
        }
        w
    }

    #[test]
    fn spelling_small_numbers() {
        assert_eq!(spell(0), "no");
        assert_eq!(spell(1), "one");
        assert_eq!(spell(20), "twenty");
        assert_eq!(spell(21), "twenty-one");
        assert_eq!(spell(99), "ninety-nine");
        assert_eq!(spell(100), "100");
        assert_eq!(spell(-3), "-3");
    }

    #[test]
    fn factors_sum_to_the_target() {
        let w = world();
        for p in w.living_polities() {
            let sum: f32 = stability_factors(&w, p).iter().map(|f| f.weight).sum();
            let target = (stability_base(&w) + sum).clamp(0.0, 1.0);
            assert!((stability_target(&w, p) - target).abs() < 1e-5);
            // Nothing tiny survives the filter.
            for f in stability_factors(&w, p) {
                assert!(f.weight.abs() >= 0.01, "{} is too small to mention", f.text);
                assert!(!f.text.is_empty());
            }
        }
    }

    #[test]
    fn factors_are_ranked() {
        let w = world();
        for p in w.living_polities() {
            let fs = stability_factors(&w, p);
            for pair in fs.windows(2) {
                assert!(pair[0].weight.abs() >= pair[1].weight.abs());
            }
        }
    }

    #[test]
    fn overextension_pulls_down() {
        let w = world();
        for p in w.living_polities() {
            if w.polities[p].overextension(&w.tuning) > 1.5 {
                let fs = stability_factors(&w, p);
                let over = fs.iter().find(|f| f.text.starts_with("overextended"));
                assert!(over.is_some());
                assert!(over.unwrap().weight < 0.0);
            }
        }
    }

    #[test]
    fn every_war_reads_as_a_sentence() {
        let w = world();
        for x in 0..w.wars.len() {
            let v = war_view(&w, x);
            assert!(v.attacker_strength > 0.0 && v.defender_strength > 0.0);
            assert!(!v.reasons.is_empty());
            assert!(v.peace.ends_with('.'));
        }
    }

    #[test]
    fn every_person_has_a_standing() {
        let w = world();
        for i in 0..w.persons.len() {
            let s = standing(&w, i);
            assert!(s.ends_with('.'), "{}", s);
            assert!(
                s.chars().next().unwrap().is_uppercase() || s.starts_with('a'),
                "{}",
                s
            );
        }
    }
}
