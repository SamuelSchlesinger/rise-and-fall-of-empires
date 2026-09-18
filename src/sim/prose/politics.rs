//! States: how a realm is announced, how crowns pass, why realms rise in
//! rank, and how they break apart.

use super::{
    cap, capital_name, count, join_names, realm, realm_full, realm_full_cap, realm_it, realm_its,
    realm_was, who, years, Pick,
};
use crate::sim::dynasty::Kin;
use crate::sim::{PolityKind, World};

// ---------------------------------------------------------------------------
// Founding
// ---------------------------------------------------------------------------

/// A stateless people has raised a leader. Draws no random number: the
/// variant is chosen from the year and the realm's id.
pub fn polity_founded(
    w: &World,
    p: usize,
    kind: PolityKind,
    capital: usize,
    culture: usize,
    place: &str,
) -> String {
    let folk = &w.cultures[culture].plural;
    let seat = &w.cities[capital].name;
    let pick = Pick::stable(w.year, p);
    match kind {
        PolityKind::Horde => format!(
            "{} gathered the {} riders into a single host and pitched their camps at {}, {}. So began {}.",
            w.ruler_short(p),
            folk,
            seat,
            place,
            realm_full(w, p)
        ),
        _ if pick.index(2) == 0 => format!(
            "The {} living {} raised {} as their chief and built the settlement of {}. So began {}.",
            folk,
            place,
            w.ruler_short(p),
            seat,
            realm_full(w, p)
        ),
        _ => format!(
            "Living {}, the {} grew numerous enough to need a leader and took {}. They walled the settlement of {}, and so began {}.",
            place,
            folk,
            w.ruler_short(p),
            seat,
            realm_full(w, p)
        ),
    }
}

/// A realm founds a town. One draw, as before.
///
/// Why people went, not only that they went. Every town in the world was
/// raised "where the land would feed them", which is true of all of them
/// and therefore says nothing about any of them --- and a realm founds a
/// town every few years for six thousand years, so it was one of the three
/// or four most repeated clauses in the whole chronicle. What the realm was
/// doing at the time is known here and was never asked.
pub fn city_founded(w: &World, p: usize, city: usize, place: &str, pick: &Pick) -> String {
    let name = &w.cities[city].name;
    if pick.index(2) == 0 {
        return format!(
            "{} founded the town of {} {}.",
            w.ruler_title(p),
            name,
            place
        );
    }
    let pol = &w.polities[p];
    let realm = realm_full(w, p);
    // A town raised in wartime, in a crowded realm, in a devout one or in a
    // trading one is a different town, and the chronicle knows which it is.
    let why = if pol.at_war() {
        match pick.index(3) {
            0 => ", far enough from the fighting that people would go",
            1 => ", which filled with people who had somewhere to be that was not the border",
            _ => ", and the first winter there was better than the one they had left",
        }
    } else if pol.school.is_some() && pol.prestige > 25.0 {
        match pick.index(3) {
            0 => ", around a shrine that had been there before the town was",
            1 => ", which the priests had said would be a good place and which was",
            _ => ", and the temple was built before the granary, as it usually is",
        }
    } else if pol.cells > 200 {
        match pick.index(3) {
            0 => ", at the far end of a road the crown had just finished paying for",
            1 => ", because there was nowhere left nearer in",
            _ => ", the last of a long line of towns founded for the same reason",
        }
    } else {
        match pick.index(4) {
            0 => ", where the land would feed them",
            1 => ", on ground nobody had troubled to claim",
            2 => ", and the first houses were up before the year turned",
            _ => ", which they had walked a long way to find",
        }
    };
    format!(
        "Settlers out of {} raised the town of {} {}{}.",
        realm, name, place, why
    )
}

/// A realm learns to build seagoing ships.
pub fn learned_seafaring(w: &World, p: usize) -> String {
    format!(
        "The shipwrights of {} learned to build vessels fit for the open sea, and {} looked beyond {} own coasts.",
        realm_full(w, p),
        realm(w, p),
        realm_its(w, p)
    )
}

// ---------------------------------------------------------------------------
// Rulers
// ---------------------------------------------------------------------------

/// How a ruler died, as a complete sentence beginning with their title.
/// `cause` is a verb phrase such as `"died of a fever."`.
pub fn ruler_died(w: &World, p: usize, ruler: usize, cause: &str, reign: i32) -> String {
    let mut text = format!("{} {}", w.ruler_title(p), cause);
    if reign >= 20 {
        let g = who(w, ruler);
        text.push_str(&format!(
            " {} had held the throne of {} for {}.",
            g.subject_cap(),
            realm(w, p),
            years(reign as i64)
        ));
    }
    text
}

/// An extra line about a ruler's passing, at high detail. One draw.
pub fn ruler_death_flourish(w: &World, p: usize, ruler: usize, reign: i32, pick: &Pick) -> String {
    let g = who(w, ruler);
    match pick.index(6) {
        0 if reign < 20 => format!(
            " {} had reigned only {}.",
            g.subject_cap(),
            years(reign.max(0) as i64)
        ),
        0 => String::new(),
        1 => format!(" The people of {} mourned for many days.", realm(w, p)),
        2 => format!(
            " {} {} laid in the tombs of {}.",
            g.subject_cap(),
            g.was(),
            capital_name(w, p)
        ),
        3 => " Few wept.".to_string(),
        4 => format!(
            " Songs were sung of {} reign for a generation.",
            g.possessive
        ),
        _ => String::new(),
    }
}

/// The cause of a natural death, as a verb phrase. One draw.
/// What was going on around a ruler when they died.
///
/// A death in a plague year, in the middle of a war, or after a lifetime on
/// the throne is a different death, and the world knows which it was.
/// Sixteen phrasings drawn from one hat gave a king at the height of a
/// siege the same end as one who died in a quiet century.
#[derive(Clone, Copy, Default)]
pub struct DeathContext {
    pub at_war: bool,
    pub plague: bool,
    pub reign_years: i32,
    pub cruel: bool,
}

/// The cause of a natural death, given what the realm was living through.
/// One draw.
pub fn natural_death_in(age: i32, cx: DeathContext, pick: &Pick) -> String {
    if cx.plague {
        return match pick.index(4) {
            0 => "died of the sickness that was taking everybody else.".to_string(),
            1 => "died in the plague year, and the court was too busy burying to make much of it."
                .to_string(),
            2 => format!(
                "died at {} of the pestilence, which did not ask what the crown was worth.",
                age
            ),
            _ => "was carried off by the sickness within a month of the physicians declaring it past its worst.".to_string(),
        };
    }
    if cx.at_war {
        return match pick.index(4) {
            0 => "died in camp, of a fever the army had brought with it.".to_string(),
            1 => "died with the war still running, and the campaign went on without noticing."
                .to_string(),
            2 => "died on the march, and was buried where the ground was soft enough.".to_string(),
            _ => "died of a wound that had been called slight when it was taken.".to_string(),
        };
    }
    if cx.reign_years >= 45 {
        return match pick.index(4) {
            0 => format!(
                "died at {}, having reigned so long that nobody at court could remember the forms for a funeral.",
                age
            ),
            1 => "died after a reign longer than most lives, and was mourned by people who had known no other ruler.".to_string(),
            2 => format!("died in bed at {}, with three generations of the court standing round it.", age),
            _ => "died at last, and the realm discovered how much of its arrangements had been personal.".to_string(),
        };
    }
    if cx.cruel {
        return match pick.index(3) {
            0 => "died in bed, which surprised a great many people.".to_string(),
            1 => "died of natural causes, and the bells were rung for longer than was strictly respectful.".to_string(),
            _ => format!("died at {}, unpleasantly and slowly, which the chroniclers recorded in more detail than was needed.", age),
        };
    }
    natural_death(age, pick)
}

pub fn natural_death(age: i32, pick: &Pick) -> String {
    // Sixteen ways to die, because seven was not enough: a quarter of every
    // monarch in the world fell off a horse, and the chronicle read like a
    // slot machine with seven symbols on the reel.
    match pick.index(16) {
        0 => "died of a fever.".to_string(),
        1 => format!("died in bed at the age of {}.", age),
        2 => "fell from a horse and did not rise.".to_string(),
        3 => "died of a wasting sickness.".to_string(),
        4 => "died in the night, and no one could say why.".to_string(),
        5 => format!("died, full of years, at the age of {}.", age),
        6 => "choked at a feast.".to_string(),
        7 => "took to bed in the autumn and did not get up again.".to_string(),
        8 => "died of a wound taken hunting that would not close.".to_string(),
        9 => format!(
            "died at {}, having outlived every physician who had treated them.",
            age
        ),
        10 => "died of the flux, which takes kings and carters alike.".to_string(),
        11 => "was found dead at the council table with the morning's business still before them."
            .to_string(),
        12 => "died on the road, a long way from anywhere worth dying.".to_string(),
        13 => format!(
            "was {} old, and had been failing for two years before anyone dared to say so.",
            years(age.max(1) as i64)
        ),
        14 => "died of a cough that the court had insisted for months was nothing.".to_string(),
        _ => "died in the bath, which the chroniclers found undignified and recorded anyway."
            .to_string(),
    }
}

/// Who killed a ruler. One draw.
pub fn assassin(w: &World, p: usize, pick: &Pick) -> String {
    match pick.index(10) {
        0 => "a cupbearer".to_string(),
        1 => "the palace guard".to_string(),
        2 => "a discontented noble".to_string(),
        3 => match w.polities[p].neighbors.first() {
            Some(&(q, _)) => format!("agents of {}", realm(w, q)),
            None => "an unknown hand".to_string(),
        },
        4 => "a masked assassin".to_string(),
        5 => "their own physician, who was paid in advance".to_string(),
        6 => "a groom with a grievance nobody had thought worth settling".to_string(),
        7 => "the captain of the household, in the household".to_string(),
        8 => match w.polities[p].neighbors.first() {
            Some(&(q, _)) => format!("a man who had been seen in {} that spring", realm(w, q)),
            None => "somebody the court never named".to_string(),
        },
        _ => "a kinsman who was next in line and did not care to wait".to_string(),
    }
}

/// A murdered ruler, as a verb phrase for [`ruler_died`].
pub fn murdered_by(by: &str) -> String {
    format!("was murdered by {}.", by)
}

// ---------------------------------------------------------------------------
// Succession
// ---------------------------------------------------------------------------

/// A realm without a dynasty chooses its next leader, naming the one who
/// died. No draw.
pub fn elected(w: &World, p: usize, kind: PolityKind, heir: usize, old: usize) -> String {
    let name = &w.persons[heir].name;
    let gone = &w.persons[old].name;
    match kind {
        PolityKind::Republic => format!(
            "The assemblies of {} elected {} consul after the death of {}.",
            realm_full(w, p),
            name,
            gone
        ),
        PolityKind::Magocracy => format!(
            "The towers of {} chose {} as archmage after the death of {}.",
            realm_full(w, p),
            name,
            gone
        ),
        PolityKind::Theocracy => format!(
            "The priests of {} raised {} to the hierarchy in place of {}.",
            realm_full(w, p),
            name,
            gone
        ),
        _ => {
            let people = &w.cultures[w.polities[p].culture].plural;
            match Pick::stable(w.year, p).index(6) {
                0 => format!(
                    "The {} chose {} to lead them after the death of {}.",
                    people, name, gone
                ),
                1 => format!(
                    "With {} dead, the {} met and came away with {}.",
                    gone, people, name
                ),
                2 => format!(
                    "The {} raised {} over them in place of {}.",
                    people, name, gone
                ),
                3 => format!(
                    "{} was dead; the elders of the {} argued it out and the argument ended on {}.",
                    cap(gone),
                    people,
                    name
                ),
                4 => format!(
                    "The {} were without a leader for a season after {} died, and then they were not: {} had the most spears behind {}.",
                    people,
                    gone,
                    name,
                    who(w, heir).object
                ),
                _ => format!(
                    "{} followed {} at the choosing of the {}.",
                    name, gone, people
                ),
            }
        }
    }
}

/// No heir, so the strongest hand takes the throne. No draw.
pub fn succession_usurped(
    w: &World,
    p: usize,
    usurper: usize,
    new_dynasty: &str,
    old_dynasty: &str,
) -> String {
    let ended = if old_dynasty.trim().is_empty() {
        String::new()
    } else {
        format!(" {} was ended.", cap(old_dynasty))
    };
    let who_ = &w.persons[usurper].name;
    let realm = realm_full(w, p);
    match Pick::stable(w.year, p).index(5) {
        0 => format!(
            "The old ruler left no clear heir, and {} seized the throne of {} and founded {}.{}",
            who_, realm, new_dynasty, ended
        ),
        1 => format!(
            "The line failed, and {} was taken by {}, who had the men to take it. So began {}.{}",
            realm, who_, new_dynasty, ended
        ),
        2 => format!(
            "Nobody could name an heir to {} that anybody else agreed to, so {} stopped the argument and took the crown. {} dates from that day.{}",
            realm, who_, cap(new_dynasty), ended
        ),
        3 => format!(
            "{} was left without an heir. {} was in the capital with soldiers; the succession settled itself, and {} had its founder.{}",
            realm_full_cap(w, p),
            who_,
            new_dynasty,
            ended
        ),
        _ => format!(
            "There was no heir to {}, and there was {}. {} begins with a man who was simply there when it mattered.{}",
            realm,
            who_,
            cap(new_dynasty),
            ended
        ),
    }
}

/// A house returns to a throne it lost. One draw.
///
/// The one sentence in this world that reaches backwards further than a
/// lifetime, and the reason the restoration was worth building at all.
pub fn house_restored(
    w: &World,
    p: usize,
    heir: usize,
    last: usize,
    gap: i32,
    house: &str,
    pick: &Pick,
) -> String {
    let name = &w.persons[heir].name;
    let gone = &w.persons[last].name;
    let span = years(gap.max(1) as i64);
    let realm = realm_full(w, p);
    match pick.index(5) {
        0 => format!(
            "{} {} was crowned, and {} sat the throne of {} again after {}. \
             The last of the line to hold it had been {}.",
            w.honorific(p, w.persons[heir].gender),
            name,
            house,
            realm,
            span,
            gone
        ),
        1 => format!(
            "After {} in which other houses held it, the throne of {} came \
             back to {}: {} was crowned where {} had been the last of the \
             line to reign.",
            span, realm, house, name, gone
        ),
        2 => format!(
            "{} had been out of {} for {}. {} brought it back, and the heralds \
             had to be shown how the old arms were quartered.",
            cap(house),
            realm,
            span,
            name
        ),
        3 => format!(
            "The restoration of {} in {}: {} took the crown {} lost {} ago, \
             and the chroniclers of that realm dated everything from it for \
             a generation afterwards.",
            house, realm, name, gone, span
        ),
        _ => format!(
            "{} of {} was crowned in {}. It was the first time in {} that \
             anyone of that blood had been, and the older families in the \
             city remembered why.",
            name, house, realm, span
        ),
    }
}

/// The crown goes sideways, to the nearest of the house. One draw.
///
/// The relation is named because it is the whole point of the sentence: a
/// reader following a line wants to know that it held, and by what thread.
pub fn kin_succeeds(
    w: &World,
    p: usize,
    heir: usize,
    old: usize,
    tie: Kin,
    house: &str,
    pick: &Pick,
) -> String {
    let h = &w.persons[heir];
    let rel = tie.name(h.gender);
    let gone = &w.persons[old].name;
    let held = if tie.steps() >= 4 {
        format!(" {} held.", cap(house))
    } else {
        String::new()
    };
    match pick.index(3) {
        0 => format!(
            "{} of {} died without issue, and the throne passed to {} {}, {} {}.{}",
            cap(gone),
            realm(w, p),
            w.honorific(p, h.gender),
            h.name,
            article(rel),
            rel,
            held
        ),
        1 => format!(
            "No child of {} survived {}, so {} {} took the throne of {}, being {} {}.{}",
            gone,
            who(w, old).object,
            w.honorific(p, h.gender),
            h.name,
            realm(w, p),
            article(rel),
            rel,
            held
        ),
        _ => format!(
            "The crown of {} went sideways to {}, {} of {}, of {}.{}",
            realm(w, p),
            h.name,
            rel,
            gone,
            house,
            held
        ),
    }
}

/// "a nephew", "an uncle" — the article a relation takes.
fn article(word: &str) -> &'static str {
    match word.as_bytes().first() {
        Some(b'a') | Some(b'e') | Some(b'i') | Some(b'o') | Some(b'u') => "an",
        _ => "a",
    }
}

/// A child inherits with a kinsman standing behind the throne. One draw.
pub fn succession_regent(w: &World, p: usize, child: usize, regent: usize, pick: &Pick) -> String {
    let g = who(w, child);
    let name = &w.persons[child].name;
    let reg = &w.persons[regent].name;
    let age = w.persons[child].age(w.year).max(1);
    match pick.index(4) {
        0 => format!(
            "{} of {} was {} old at {} accession, and {} governed for {} \
             until such time as {} should be fit to.",
            name,
            realm_full(w, p),
            years(age as i64),
            g.possessive,
            reg,
            g.object,
            g.subject
        ),
        1 => format!(
            "The crown of {} came to the child {}, and {} took the regency. \
             Regents have been known to find the arrangement agreeable.",
            realm_full(w, p),
            name,
            reg
        ),
        2 => format!(
            "{} inherited {} at {}, too young to hold it. {} ruled in {} \
             name, and the court took the measure of them both.",
            name,
            realm_full(w, p),
            years(age as i64),
            reg,
            g.possessive
        ),
        _ => format!(
            "A child sat the throne of {}: {}, {} old, with {} of the house \
             standing behind it.",
            realm_full(w, p),
            name,
            years(age as i64),
            reg
        ),
    }
}

/// A grown kinsman takes what a child was left. One draw.
pub fn uncle_takes_throne(
    w: &World,
    p: usize,
    taker: usize,
    child: usize,
    tie: Kin,
    house: &str,
    pick: &Pick,
) -> String {
    let rel = tie.name(w.persons[taker].gender);
    let g = who(w, child);
    let name = &w.persons[child].name;
    let age = w.persons[child].age(w.year).max(1);
    match pick.index(3) {
        0 => format!(
            "The heir of {} was {} old. {} {}, {} of the late ruler, \
             observed that the realm could not be governed from a nursery, \
             and took the throne in the child's place. The crown stayed in \
             {}, at least.",
            realm_full(w, p),
            years(age as i64),
            w.honorific(p, w.persons[taker].gender),
            w.persons[taker].name,
            rel,
            house
        ),
        1 => format!(
            "{} of {} was {} old, and was passed over for the late ruler's \
             {}, {}, who wore the crown instead. The child was not spoken \
             of again for some years.",
            cap(name),
            realm_full(w, p),
            years(age as i64),
            rel,
            w.persons[taker].name
        ),
        _ => format!(
            "{} {} set aside the child {} and took the throne of {}. \
             {} is said to have wept; {} did not.",
            w.honorific(p, w.persons[taker].gender),
            w.persons[taker].name,
            name,
            realm_full(w, p),
            cap(w.persons[taker].name.as_str()),
            g.subject
        ),
    }
}

/// A child inherits and the great houses rule in their name. No draw.
pub fn succession_regency(w: &World, p: usize, regent: usize) -> String {
    let g = who(w, regent);
    let name = &w.persons[regent].name;
    let realm = realm_full(w, p);
    match Pick::stable(w.year, p).index(5) {
        0 => format!(
            "The child {} inherited {}, being the only heir of age to claim it. The great houses ruled in {} name and quarrelled over the spoils.",
            name, realm, g.possessive
        ),
        1 => format!(
            "{} passed to the child {}, and with it to whichever of the great houses could hold the nursery door.",
            realm_full_cap(w, p),
            name
        ),
        2 => format!(
            "There was no one else: the child {} took {}, and a council of lords governed in {} name, each of them for a different reason.",
            name, realm, g.possessive
        ),
        3 => format!(
            "{} {} inherited by a child. {} would reign for years before {} ruled for a day.",
            realm_full_cap(w, p),
            realm_was(w, p),
            name,
            g.subject
        ),
        _ => format!(
            "The crown of {} came to {}, who was too young to lift it. The court discovered that it liked governing without a ruler.",
            realm, name
        ),
    }
}

/// The claim a rebel claimant fights under.
pub fn succession_claim(w: &World, p: usize, claimant: &str) -> String {
    format!("the claim of {} to the throne of {}", claimant, realm(w, p))
}

// ---------------------------------------------------------------------------
// Rank
// ---------------------------------------------------------------------------

/// What a realm was and what it has become, for [`rank_changed`].
pub struct RankChange<'a> {
    /// The kind of thing it was.
    pub old: PolityKind,
    /// The kind of thing it now is.
    pub new: PolityKind,
    /// What it was called before.
    pub old_name: &'a str,
    /// What it is called now.
    pub new_name: &'a str,
    /// The ruler presiding over the change.
    pub ruler: &'a str,
    /// The capital it happened in.
    pub capital: &'a str,
}

/// A realm changes what kind of thing it is. Returns importance and text.
/// No draws.
pub fn rank_changed(w: &World, p: usize, c: &RankChange) -> (u8, String) {
    let RankChange {
        old,
        new,
        old_name,
        new_name,
        ruler,
        capital,
    } = *c;
    let short = realm(w, p);
    // The old name may be plural ("the Velenic Clans are now ...").
    let was_named = if super::name_is_plural(old_name, old) {
        "are"
    } else {
        "is"
    };
    match new {
        PolityKind::Chiefdom => (
            0,
            format!(
                "The clans of {} bent the knee to a single chieftain, having tired of raiding one another. {} {} now {}.",
                short,
                cap(old_name),
                was_named,
                new_name
            ),
        ),
        PolityKind::Kingdom if old == PolityKind::Empire => (
            2,
            format!(
                "Shrunken and humbled, {} {} an empire no longer. Its rulers styled it {}.",
                old_name,
                if was_named == "are" { "were" } else { "was" },
                new_name
            ),
        ),
        PolityKind::Kingdom => (
            1,
            format!(
                "{} was crowned in {}, and the chiefdom of {} became {}.",
                ruler, capital, short, new_name
            ),
        ),
        PolityKind::Republic => (
            1,
            format!(
                "The merchant houses of {} cast down their chieftain, being rich enough to do without one, and proclaimed {}.",
                capital, new_name
            ),
        ),
        PolityKind::Empire => (
            3,
            format!(
                "{} {} proclaimed {}. {} took the imperial diadem before the assembled peoples of {}.",
                cap(old_name),
                if was_named == "are" { "were" } else { "was" },
                new_name,
                ruler,
                count(w.polities[p].cells as i64, "land")
            ),
        ),
        PolityKind::Theocracy => (
            2,
            format!(
                "The priests of {} took the crown for their own, the ruler being devout and the faith strong. {} {} now {}.",
                short,
                cap(old_name),
                was_named,
                new_name
            ),
        ),
        PolityKind::Magocracy => (
            2,
            format!(
                "The mages who counselled the throne of {} dispensed with the throne. {} {} now {}.",
                short,
                cap(old_name),
                was_named,
                new_name
            ),
        ),
        _ => (
            1,
            format!("{} {} now {}.", cap(old_name), was_named, new_name),
        ),
    }
}

// ---------------------------------------------------------------------------
// Unrest and endings
// ---------------------------------------------------------------------------

/// Why a province rose against its ruler.
pub fn revolt_cause(w: &World, p: usize, rebel_culture: usize, civil: bool, cruel: bool) -> String {
    if civil {
        if cruel {
            format!("the cruelty of {}", w.ruler_short(p))
        } else {
            format!("the misrule of {}", w.ruler_short(p))
        }
    } else {
        format!(
            "the {} wish to be free of {}",
            w.cultures[rebel_culture].adj,
            realm(w, p)
        )
    }
}

/// A province has raised the banner of revolt. No draw.
pub fn revolt(w: &World, p: usize, rebel: usize, leader: usize, seat: &str, cause: &str) -> String {
    format!(
        "{} raised the banner of revolt at {} against {}, blaming {}. The rebels named their realm {}.",
        w.persons[leader].name,
        seat,
        realm_full(w, p),
        cause,
        realm_full(w, rebel)
    )
}

/// A discontented notable gathers the aggrieved. One draw for the
/// rebel's station in life.
pub fn rebellion_of_the_wronged(
    w: &World,
    p: usize,
    rebel: usize,
    leader: usize,
    pick: &Pick,
) -> String {
    let station = pick.text(&[
        "a soldier",
        "a farmer's child",
        "a minor noble",
        "a priest",
        "a bandit",
        "a tax-collector",
    ]);
    format!(
        "{}, {} of {} who had been wronged by the crown, gathered the discontented and proclaimed {}.",
        w.persons[leader].name,
        station,
        realm_full(w, p),
        realm_full(w, rebel)
    )
}

/// Where a realm stands against everything this world has already buried.
///
/// The chronicle gave the fall of a realm of two thousand lands and five
/// thousand years exactly the sentence it gave a chiefdom of forty --- the
/// world knew perfectly well which was which and had no way of saying so.
/// Importance decided whether an event was written down; it should also
/// decide how it is said.
struct Standing {
    /// Nothing this world has ever held has been larger.
    largest_ever: bool,
    /// Nothing that has ended in this world stood for longer.
    longest_ever: bool,
    /// Years since anything this size last ended, if it has been a while.
    since_its_like: Option<i32>,
}

fn standing(w: &World, p: usize) -> Standing {
    let peak = w.polities[p].peak_cells;
    let age = w.year - w.polities[p].founded;
    let mut largest = 0usize;
    let mut oldest = 0i32;
    let mut last_like: Option<i32> = None;
    for q in 0..w.polities.len() {
        if q == p {
            continue;
        }
        let pol = &w.polities[q];
        largest = largest.max(pol.peak_cells);
        if let Some(end) = pol.fell {
            oldest = oldest.max(end - pol.founded);
            // Something of this weight, gone before. Four fifths counts as
            // its like: a realm is not measured to the acre.
            if pol.peak_cells * 5 >= peak * 4 {
                last_like = Some(last_like.map_or(end, |y: i32| y.max(end)));
            }
        }
    }
    Standing {
        largest_ever: peak > 0 && peak >= largest,
        longest_ever: age > 0 && age >= oldest,
        since_its_like: last_like
            .map(|y| w.year - y)
            .filter(|&gap| gap >= 200 && peak > 120),
    }
}

/// The obituary of a realm. `cause` is a verb phrase such as
/// `"was overrun and destroyed."`.
pub fn realm_fell(w: &World, p: usize, cause: &str, cities_built: usize) -> String {
    let mut text = format!("{} {}", realm_full_cap(w, p), cause);
    let peak = w.polities[p].peak_cells;
    let age = w.year - w.polities[p].founded;
    if peak > 40 {
        text.push_str(&format!(
            " At {} height in year {}, {} had ruled {} and {}.",
            realm_its(w, p),
            w.polities[p].peak_year,
            realm_it(w, p),
            count(peak as i64, "land"),
            count(cities_built as i64, "city")
        ));
    }
    // How long it stood. Nowhere in six thousand years did the chronicle
    // say this, which is why the death of a five-thousand-year-old realm
    // read exactly like the death of a fifty-year-old one.
    if age >= 150 {
        text.push_str(&format!(" It had stood {}.", years(age as i64)));
    }
    // And what its passing was worth against the rest of the record.
    let st = standing(w, p);
    let pick = Pick::stable(w.year, p);
    if st.largest_ever && peak > 120 {
        text.push_str(match pick.index(3) {
            0 => " Nothing so large had ever stood in the world, and nothing so large had ever fallen.",
            1 => " It was the largest thing this world had made, and the mapmakers had no practice at unmaking it.",
            _ => " No realm in the history of the world had held more.",
        });
    } else if st.longest_ever && age >= 400 {
        text.push_str(match pick.index(3) {
            0 => " Nothing that has ended in this world had stood so long.",
            1 => " No other realm had lasted as many years before ending.",
            _ => " It had outlived everything that had gone into the ground before it.",
        });
    } else if let Some(gap) = st.since_its_like {
        text.push_str(&format!(
            " Nothing of its size had ended in {}.",
            years(gap as i64)
        ));
    }
    text
}

/// A great realm breaks into pieces. Draws no random number: the variant
/// comes from the year and the realm, so the loudest sentence in the
/// chronicle is not the same one every time.
pub fn shattered(
    w: &World,
    p: usize,
    old_name: &str,
    successors: &[usize],
    capital: &str,
) -> String {
    // Name whoever crowned themselves where there is somebody to name: a
    // successor state with a face is a thread the reader can follow, and an
    // anonymous one is a line of scenery.
    let successor_names: Vec<String> = successors
        .iter()
        .map(|&sp| match w.polities[sp].ruler {
            Some(r) => format!("{} under {}", w.polities[sp].name, w.persons[r].name),
            None => w.polities[sp].name.clone(),
        })
        .collect();
    let successor_names = &successor_names[..];
    let now = realm_full(w, p);
    let remnant = if now == old_name {
        format!("What remained held only the lands about {}.", capital)
    } else {
        format!(
            "What remained of the old realm, now {}, held only the lands about {}.",
            now, capital
        )
    };
    let risen = join_names(successor_names);
    let opening = match Pick::stable(w.year, p).index(4) {
        0 => format!(
            "{} shattered, its throne too weak to hold its provinces. Its governors and generals each crowned themselves, and from its ruin rose {}.",
            cap(old_name),
            risen
        ),
        1 => format!(
            "The provinces of {} stopped waiting for orders that never came. Within a year the tax rolls were being read out in the names of {}.",
            old_name, risen
        ),
        2 => format!(
            "{} came apart along every seam it had ever been sewn along. Where one realm had been there were now {}.",
            cap(old_name),
            risen
        ),
        _ => format!(
            "No single blow broke {}: the garrisons simply began to obey the nearest lord instead of the furthest. Out of that habit came {}.",
            old_name, risen
        ),
    };
    format!("{} {}", opening, remnant)
}

/// A realm with nothing left to divide. No draw.
pub fn collapsed_into_lawlessness(w: &World, p: usize) -> String {
    format!(
        "collapsed into lawlessness, {} lords each seizing what they could.",
        realm_its(w, p)
    )
}

/// A realm that simply ran out of land. No draw.
pub fn faded_away(w: &World, p: usize, at_war: bool) -> String {
    if at_war {
        format!("{} overrun and destroyed.", realm_was(w, p))
    } else {
        format!("faded away, {} last lands abandoned.", realm_its(w, p))
    }
}

// ---------------------------------------------------------------------------
// Tyrants and usurping generals
// ---------------------------------------------------------------------------

/// A ruler earns a reputation for cruelty.
pub fn tyrant(w: &World, p: usize, ruler: usize, nobles: usize) -> String {
    let g = who(w, ruler);
    format!(
        "The people of {} spoke of {} only in whispers. {} had {} put to death in a single winter for plotting that no one could prove, and the roads were lined with what remained.",
        realm(w, p),
        w.ruler_title(p),
        g.subject_cap(),
        count(nobles as i64, "noble")
    )
}

/// A realm's army, so that an adjective is never used as a noun:
/// "the Velenic host".
pub fn host(w: &World, p: usize) -> String {
    format!("the {} host", w.polities[p].adj)
}

/// A general takes the throne by force. No draw.
pub fn general_usurps(w: &World, p: usize, general: usize, capital: &str) -> String {
    let g = who(w, general);
    format!(
        "The general {} marched on {} and took the throne of {} for {}, the realm being too weak to stop {}.",
        w.persons[general].name,
        capital,
        realm_full(w, p),
        g.reflexive,
        g.object
    )
}

/// The epithet and death notice of a ruler deposed by their own general.
pub fn deposed_by(general: &str) -> String {
    format!("was deposed and killed by the general {}.", general)
}
