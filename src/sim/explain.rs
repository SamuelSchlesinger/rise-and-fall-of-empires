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

/// Where a realm's money comes from and where it goes, largest first.
///
/// The mirror of the income arithmetic in `politics::economy`, in the same
/// shape as [`stability_factors`]: a list of named pulls whose weights sum
/// to what the treasury gains this year. That sum is the point — a reader
/// can check the parts against the whole, so a realm that is quietly going
/// broke says which of its costs is doing it.
///
/// Stability had an explanation from the beginning and money never did,
/// which left the two economic entries in the stability list — "the
/// treasury is full", "the cities are prosperous" — as the only visible
/// economics in the game, both of them effects with no stated cause.
pub fn income_factors(w: &World, p: usize) -> Vec<Factor> {
    let mut out: Vec<Factor> = Vec::new();
    if p >= w.polities.len() || !w.polities[p].alive() {
        return out;
    }
    let tn = &w.tuning;
    let pol = &w.polities[p];
    // The sources are what each place is assessed for, and what never
    // arrives is a line of its own below. The other way round — netting the
    // loss off each source — hides the thing the reader wants to know,
    // which is whether the realm is poor or merely badly run.
    let tech = w.tech_income_mult(p);
    // Cities, each one's own line when it is worth a line, because the
    // interesting case is a realm living off one port.
    let mut from_cities: Vec<(f32, usize)> = pol
        .cities
        .iter()
        .map(|&c| {
            (
                w.cities[c].pop * w.cities[c].prosperity * tn.city_income_factor * tech,
                c,
            )
        })
        .collect();
    from_cities.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let named = from_cities.len().min(3);
    for &(v, c) in from_cities.iter().take(named) {
        push(&mut out, v, format!("taxes from {}", w.cities[c].name));
    }
    let rest: f32 = from_cities.iter().skip(named).map(|x| x.0).sum();
    push(
        &mut out,
        rest,
        format!(
            "taxes from {}",
            crate::sim::prose::count((from_cities.len() - named) as i64, "smaller town")
        ),
    );
    // The land itself, which is what a realm has instead of cities.
    push(
        &mut out,
        pol.cells as f32 * tn.cell_income_factor * (1.0 + pol.dev) * tech,
        format!("{} lands under tax", pol.cells),
    );
    // What the crown takes from what passes through.
    let tolls = w.realm_trade(p) * tn.trade_toll_factor * tech;
    push(
        &mut out,
        tolls,
        "tolls on the roads and the harbours".to_string(),
    );
    // What never arrives. Stated as its own loss rather than folded
    // silently into the sources above, because the reader's question is
    // whether the realm is poor or merely badly run.
    let gross: f32 = out.iter().map(|f| f.weight).sum();
    let skim = crate::sim::corruption_share(tn, pol.decadence, pol.sprawl);
    if skim > 0.0 {
        let lost = gross * skim;
        let why = if pol.decadence * tn.corruption_decadence_weight
            > (pol.sprawl - 1.0).max(0.0) * tn.corruption_sprawl_weight
        {
            "taken by the court before it arrives"
        } else {
            "lost on the long road to the capital"
        };
        push(&mut out, -lost, format!("{:.0}% {}", skim * 100.0, why));
    }
    // And what it all costs.
    push(
        &mut out,
        -(pol.army * tn.army_upkeep_factor),
        format!("keeping {:.0} under arms", pol.army),
    );
    push(
        &mut out,
        -(pol.cells as f32 * tn.cell_upkeep_factor),
        "governors, garrisons and roads".to_string(),
    );
    // What the court will spend of what is already in hand. Not a cost of
    // government but of splendour, and the reason a treasury has no ceiling.
    let spend = crate::sim::court_spending(tn, pol.treasury);
    push(
        &mut out,
        -spend,
        "the court's own expenses and splendour".to_string(),
    );
    // Gains largest first, then the costs, rather than everything ranked by
    // size together: a page shows the first handful of a block, and the two
    // costs otherwise sank below the small revenues of a large realm.
    // Nothing is dropped here — see [`income_shown`], which is where the
    // trimming belongs, because a total that depended on how many lines fit
    // would not be a total.
    let (mut gains, mut costs): (Vec<Factor>, Vec<Factor>) =
        out.into_iter().partition(|f| f.weight > 0.0);
    gains.sort_by(|a, b| b.weight.total_cmp(&a.weight));
    costs.sort_by(|a, b| a.weight.total_cmp(&b.weight));
    gains.extend(costs);
    gains
}

/// [`income_factors`] trimmed to fit a page, without changing what it sums
/// to.
///
/// At most `gains` revenue lines survive; the rest are folded into one, and
/// every cost is kept. So the figures a reader sees still add up to the
/// figure printed under them, which is the only reason to show them as a
/// column at all.
pub fn income_shown(w: &World, p: usize, gains: usize) -> Vec<Factor> {
    let all = income_factors(w, p);
    let (revenue, costs): (Vec<Factor>, Vec<Factor>) =
        all.into_iter().partition(|f| f.weight > 0.0);
    let folded: f32 = revenue.iter().skip(gains).map(|f| f.weight).sum();
    let dropped = revenue.len().saturating_sub(gains);
    let mut out: Vec<Factor> = revenue.into_iter().take(gains).collect();
    if dropped > 0 {
        out.push(Factor {
            text: format!(
                "{} besides",
                crate::sim::prose::count(dropped as i64, "smaller source")
            ),
            weight: folded,
        });
    }
    out.extend(costs);
    out
}

/// What the treasury will gain this year, as [`income_factors`] totals it.
pub fn income_total(w: &World, p: usize) -> f32 {
    income_factors(w, p).iter().map(|f| f.weight).sum()
}

/// Why a realm is no longer there, in one sentence.
///
/// A fallen realm's page could say when it ended and not why, though the
/// world records everything needed to say it: how far past its peak it was,
/// how long it had been shrinking, what it was overextended by, whether it
/// was at war, and what its last recorded moments were. A reader who has
/// just watched a five-hundred-year empire vanish deserves better than a
/// year.
pub fn epitaph(w: &World, p: usize) -> Option<String> {
    let pol = w.polities.get(p)?;
    let fell = pol.fell?;
    let lasted = fell - pol.founded;
    // What the chronicle remembers of its last years, which is the best
    // evidence there is for how it went.
    let last: Vec<&str> = w
        .chronicle
        .for_ref(crate::sim::chronicle::Ref::Polity(p))
        .iter()
        .rev()
        .filter_map(|&i| w.chronicle.events.get(i))
        .filter(|e| e.year >= fell - 20)
        .map(|e| e.text.as_str())
        .collect();
    let violent = last.iter().any(|t| {
        t.contains("sack") || t.contains("Battle") || t.contains("conquer") || t.contains("took")
    });
    let split = last
        .iter()
        .any(|t| t.contains("came apart") || t.contains("broke") || t.contains("proclaim"));
    let shrank = pol.peak_cells > 0 && pol.peak_year < fell - 50;
    let how = if split {
        "It came apart rather than being conquered"
    } else if violent {
        "It was taken"
    } else if shrank {
        "It had been shrinking for a long while before the end"
    } else {
        "It simply stopped"
    };
    let years = crate::sim::prose::years(lasted.max(0) as i64);
    let decline = (fell - pol.peak_year).max(0);
    let arc = if pol.peak_cells > 0 && decline > 0 {
        format!(
            ", and spent the last {} of them past its height of {} lands",
            crate::sim::prose::years(decline as i64),
            pol.peak_cells
        )
    } else if pol.peak_cells > 0 {
        format!(
            ", and was at its greatest of {} lands at the end",
            pol.peak_cells
        )
    } else {
        String::new()
    };
    Some(format!("It stood for {}{}. {}.", years, arc, how))
}

/// Everything making a city rich or poor, strongest first.
///
/// Prosperity became the most interesting number in the game when trade
/// began to feed it and a city's income began to depend on it, and it was
/// the one such number with no explanation at all. The weights mirror the
/// target in `politics::economy`, and like that target they are a sum which
/// is then *bent* towards a ceiling — so above the knee they no longer add
/// to where the city is heading, which is what [`prosperity_raw`] and
/// [`prosperity_target`] are for.
pub fn prosperity_factors(w: &World, c: usize) -> Vec<Factor> {
    use crate::sim::politics::{TRADE_PROSPERITY_HALF, TRADE_PROSPERITY_MAX, TRADE_TO_PROSPERITY};
    let mut out: Vec<Factor> = Vec::new();
    if c >= w.cities.len() || w.cities[c].destroyed.is_some() {
        return out;
    }
    let city = &w.cities[c];
    let cell = city.cell;
    let t = &w.terrain;
    push(
        &mut out,
        0.3,
        "a town is worth something wherever it is".into(),
    );
    if t.coast[cell] {
        let merc = w.cultures[city.culture].values.mercantilism;
        push(
            &mut out,
            0.25 * (0.5 + merc),
            "it sits on the sea, and its people put out from it".into(),
        );
    }
    if t.river[cell] >= 1 {
        push(
            &mut out,
            t.river[cell] as f32 * 0.07,
            "a river runs through it".into(),
        );
    }
    // Averaged over the ring the same way `economy` averages it.
    let mut minerals = 0.0;
    for nb in t.neighbors8(cell) {
        minerals += t.minerals[nb];
    }
    minerals /= 8.0;
    push(
        &mut out,
        minerals * 0.25,
        "there is ore in the ground".into(),
    );

    if let Some(p) = city.polity.filter(|&p| w.polities[p].alive()) {
        let pol = &w.polities[p];
        push(
            &mut out,
            pol.dev * 0.3,
            format!("{} is a developed realm", pol.short),
        );
        let peace = pol
            .neighbors
            .iter()
            .filter(|&&(q, _)| w.war_between(p, q).is_none())
            .count() as f32;
        push(
            &mut out,
            (peace * 0.04).min(0.2),
            "its neighbours are at peace with it".into(),
        );
        let (_, _, _, prosp_bonus) = w.school_effects(p);
        push(
            &mut out,
            prosp_bonus,
            "its doctrine favours industry".into(),
        );
        push(
            &mut out,
            w.tech_prosperity_bonus(p),
            "what the realm knows is worth money".into(),
        );
        if pol.capital == Some(c) {
            push(&mut out, 0.1, "the crown sits here".into());
        }
        if pol.at_war() {
            push(&mut out, -0.15, "the realm is at war".into());
        }
    }
    push(
        &mut out,
        city.wonders.len() as f32 * 0.08,
        if city.wonders.len() == 1 {
            "a wonder here draws people from everywhere".to_string()
        } else {
            format!(
                "{} here draw people from everywhere",
                crate::sim::prose::count(city.wonders.len() as i64, "wonder")
            )
        },
    );
    let passing = w.city_trade(c) * TRADE_TO_PROSPERITY;
    push(
        &mut out,
        TRADE_PROSPERITY_MAX * passing / (passing + TRADE_PROSPERITY_HALF),
        format!("{:.0} a year passes through it", w.city_trade(c)),
    );
    out.sort_by(|a, b| b.weight.abs().total_cmp(&a.weight.abs()));
    out
}

/// What those advantages add to, before the ceiling bends them.
pub fn prosperity_raw(w: &World, c: usize) -> f32 {
    prosperity_factors(w, c).iter().map(|f| f.weight).sum()
}

/// Where a city's prosperity is actually heading.
pub fn prosperity_target(w: &World, c: usize) -> f32 {
    use crate::sim::politics::{PROSPERITY_KNEE, PROSPERITY_MAX};
    crate::sim::soft_ceiling(prosperity_raw(w, c), PROSPERITY_KNEE, PROSPERITY_MAX)
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

    // Sheer size. Mirrors the hegemony term in `politics::economy`.
    let share = crate::sim::dynasty::world_share(w, p);
    push(
        &mut out,
        -(share - tn.hegemony_free_share).max(0.0) * tn.hegemony_weight,
        format!(
            "it rules {:.0}% of the settled world, which is more than any crown governs easily",
            share * 100.0
        ),
    );

    // Distance. Mirrors the sprawl term in `politics::economy`; the two are
    // written to be read side by side and must change together.
    push(
        &mut out,
        -(pol.sprawl - 1.0).max(0.0) * tn.stability_sprawl_weight,
        format!(
            "its provinces lie {:.0}% beyond the reach the crown can comfortably govern",
            (pol.sprawl - 1.0).max(0.0) * 100.0
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

    // Regalia. Mirrors the `artifact_stability` term in `politics::economy`.
    push(
        &mut out,
        w.artifact_stability(p),
        "the old regalia still command respect".into(),
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

    // The treasury. Through the shared curve, not a ratio of its own: this
    // was a second copy of the old one, dividing by a ceiling of six hundred
    // that no longer exists, and when the ceiling went it went on dividing —
    // so a realm sitting on three thousand was told its full treasury was
    // worth a hundred and four points of stability, five times more than
    // anything can be worth.
    push(
        &mut out,
        crate::sim::treasury_confidence(&w.tuning, pol.treasury) * 0.2,
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
/// Everything the factors add up to, before the ceiling bends it.
///
/// Worth showing beside the target when the two differ, because a realm
/// whose advantages add to 130 and one whose add to 101 both settle near the
/// top and are not at all the same realm.
pub fn stability_raw(w: &World, p: usize) -> f32 {
    stability_base(w)
        + stability_factors(w, p)
            .iter()
            .map(|f| f.weight)
            .sum::<f32>()
}

/// Where stability is actually heading.
///
/// Through the same bend the simulation applies. This used to clamp the sum
/// while `politics::economy` clamped the *value*, so above 1.0 the
/// explanation and the world disagreed and the reported target understated
/// the real pull towards the ceiling.
pub fn stability_target(w: &World, p: usize) -> f32 {
    crate::sim::soft_ceiling(
        stability_raw(w, p),
        crate::sim::politics::STABILITY_KNEE,
        1.0,
    )
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
        Role::Noble => match per.house {
            Some(h) => format!("Of {}, and never came to a throne.", w.houses[h].name),
            None => "Of a ruling house, and never came to a throne.".into(),
        },
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

    /// The listed pulls add up to the number the page prints beside them,
    /// and the ceiling bends that number rather than clipping it.
    #[test]
    fn factors_sum_to_the_target() {
        let w = world();
        for p in w.living_polities() {
            let sum: f32 = stability_factors(&w, p).iter().map(|f| f.weight).sum();
            // The raw total is exactly what the reader can add up from the
            // column of figures.
            assert!((stability_raw(&w, p) - (stability_base(&w) + sum)).abs() < 1e-5);
            // And the target is that total bent towards the ceiling, never
            // above it and never below what the total was.
            let raw = stability_raw(&w, p);
            let target = stability_target(&w, p);
            assert!((0.0..=1.0).contains(&target));
            if raw > 0.0 && raw <= 1.0 {
                assert!(
                    target <= raw + 1e-5,
                    "realm {} was bent upwards: {} to {}",
                    p,
                    raw,
                    target
                );
            }
            // Nothing tiny survives the filter.
            for f in stability_factors(&w, p) {
                assert!(f.weight.abs() >= 0.01, "{} is too small to mention", f.text);
                assert!(!f.text.is_empty());
            }
        }
    }

    /// A realm whose advantages far exceed what stability can hold must not
    /// read the same as one that only just clears it.
    ///
    /// The thirteen terms of the stability target are summed and were then
    /// clamped, so the top tenth of realms in a mature world all sat at
    /// exactly 1.0: the same drift word, the same "pulls towards 100%", for
    /// a realm at 1.02 and one at 1.6. The fifth and last instance of the
    /// mistake this simulation kept making.
    #[test]
    fn a_realm_with_every_advantage_is_still_told_apart() {
        let mut w = world();
        let ps = w.living_polities();
        let (a, b) = (ps[0], ps[1]);
        // Two realms differing only in what their treasuries buy them,
        // both far past what the ceiling used to allow.
        w.polities[a].decadence = 0.0;
        w.polities[b].decadence = 0.0;
        w.polities[a].exhaustion = 0.0;
        w.polities[b].exhaustion = 0.0;
        w.polities[a].treasury = 200.0;
        w.polities[b].treasury = 50_000.0;
        let (ra, rb) = (stability_raw(&w, a), stability_raw(&w, b));
        assert!(rb > ra, "the richer realm has no more going for it");
        let (ta, tb) = (stability_target(&w, a), stability_target(&w, b));
        assert!(ta <= 1.0 && tb <= 1.0);
        assert!(
            tb > ta,
            "two realms with different advantages ({:.3} and {:.3}) settle at \
             the same place ({:.3})",
            ra,
            rb,
            ta
        );
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
