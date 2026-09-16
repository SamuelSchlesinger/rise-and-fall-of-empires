//! Peoples: assimilation, a realm changing its own character, a tongue
//! dying out, and a community drifting far enough to become a people of
//! its own.

use super::{cap, realm, realm_full, years};
use crate::sim::World;

/// A city's people have gone over to the culture of the realm that rules
/// them. Says how long it took and who rules, so the cause is on the page.
pub fn city_assimilated(
    w: &World,
    city: usize,
    ruler_polity: usize,
    new_culture: usize,
    old_culture: usize,
    tenure: i32,
) -> String {
    format!(
        "After {} under the crown of {}, the people of {} count themselves among the {}; the {} tongue is heard there no more.",
        years(tenure.max(1) as i64),
        realm_full(w, ruler_polity),
        w.cities[city].name,
        w.cultures[new_culture].plural,
        w.cultures[old_culture].name
    )
}

/// A realm has more foreign subjects than its own people, and takes their
/// culture for its own.
pub fn realm_changes_culture(w: &World, p: usize, old_culture: usize, new_culture: usize) -> String {
    format!(
        "The rulers of {} had long spoken {} at court, but their subjects were {}. {} now counts itself {}, and the court speaks as the country does.",
        realm_full(w, p),
        w.cultures[old_culture].name,
        w.cultures[new_culture].plural,
        realm(w, p),
        w.cultures[new_culture].adj
    )
}

/// The last speakers of a language are gone.
pub fn culture_extinct(w: &World, c: usize) -> String {
    format!(
        "The last speakers of {} died, and with them the {} passed out of the world.",
        w.cultures[c].lang.name, w.cultures[c].plural
    )
}

/// A cut-off community has become a people in its own right.
pub fn culture_diverged(
    w: &World,
    parent: usize,
    child: usize,
    place: &str,
    realm_that_followed: Option<usize>,
) -> String {
    let mut text = format!(
        "Cut off from their kin, the {} folk living {} drifted in speech and custom until they were a people apart: the {}, who call themselves {}.",
        w.cultures[parent].name,
        place,
        w.cultures[child].plural,
        w.cultures[child].name
    );
    if let Some(p) = realm_that_followed {
        text.push_str(&format!(
            " {} counts itself {} from this year on.",
            cap(&realm_full(w, p)),
            w.cultures[child].adj
        ));
    }
    text
}
