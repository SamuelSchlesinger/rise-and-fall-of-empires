//! Sentences about what the land yields and what moves between the places
//! that have it and the places that do not.
//!
//! The closing of a road is the one that matters. It is the sentence that
//! makes interdependence visible: two cities that never saw a soldier grow
//! poorer because somebody else went to war.

use super::{realm, World};

/// A great road stops carrying anything.
pub fn route_closed(w: &World, a: usize, b: usize, at_war: bool) -> String {
    let (na, nb) = (&w.cities[a].name, &w.cities[b].name);
    if at_war {
        format!(
            "The road between {} and {} closed. The caravans turned back at the border, \
             and the markets at both ends felt it within the season.",
            na, nb
        )
    } else {
        format!(
            "Trade between {} and {} ceased. What had gone one way and come back the other \
             for generations simply stopped.",
            na, nb
        )
    }
}

/// And starts again.
pub fn route_reopened(w: &World, a: usize, b: usize) -> String {
    format!(
        "The road between {} and {} was open again, and the first caravans through it were \
         met as though they had been mourned.",
        w.cities[a].name, w.cities[b].name
    )
}

/// What a city's own country yields, for its page.
pub fn city_produces(goods: &[crate::sim::trade::Good]) -> String {
    let names: Vec<&str> = goods.iter().map(|g| g.name()).collect();
    match names.len() {
        0 => "little worth carrying anywhere".to_string(),
        _ => super::list_with_and(&names),
    }
}

/// A city grown rich on the carrying trade rather than on its own country.
pub fn city_entrepot(w: &World, city: usize, p: usize) -> String {
    format!(
        "{} lives on what passes through it rather than on what grows around it, \
         which is the making of {} and would be its unmaking if the roads closed.",
        w.cities[city].name,
        realm(w, p)
    )
}
