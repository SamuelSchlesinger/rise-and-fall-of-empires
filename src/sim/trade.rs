//! What the land produces, and what moves between the places that have it
//! and the places that do not.
//!
//! Before this, "trade" existed in the world as two things that were not
//! trade: a `mercantilism` number on a culture, and a line in `war` that
//! cooled tension a little between two realms that both happened to value
//! it. Nothing was produced anywhere, nothing moved, and geography stopped
//! mattering the moment a map was finished being settled. A city on a strait
//! was worth no more than a city in a bog.
//!
//! Three things come out of putting real goods on the ground:
//!
//! * **Places differ, permanently.** A good is a fact about terrain, so the
//!   salt pans and the ore country are still the salt pans and the ore
//!   country a thousand years later, whoever rules them.
//! * **Wealth is *positional*.** A city grows rich by sitting between
//!   places that want what each other has, which is why a realm astride a
//!   route is worth attacking and why a route is worth a war of its own.
//! * **Cutting a route hurts both ends.** Interdependence is what makes a
//!   collapse propagate: when a realm falls or a war closes a road, the
//!   wealth drains out of cities that never saw the fighting.
//!
//! Routes also carry ideas. A city at the far end of a trade route learns
//! what the other end knows far sooner than its own neighbours do, which is
//! how a technique crosses a sea before it crosses a mountain range.

use super::chronicle::{EventKind, Ref};
use super::prose;
use super::World;
use crate::geo::{Biome, Terrain};
use std::collections::BTreeMap;

/// What a stretch of country is worth sending somewhere else.
///
/// Deliberately few. The point is not a commodity model but a reason for one
/// place to want what another has, and a dozen kinds gives every city a
/// handful its neighbours lack.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Good {
    Grain,
    Livestock,
    Fish,
    Timber,
    Stone,
    Metal,
    Salt,
    Furs,
    Spice,
    Wine,
    Horses,
    Leystone,
}

pub const GOODS: [Good; 12] = [
    Good::Grain,
    Good::Livestock,
    Good::Fish,
    Good::Timber,
    Good::Stone,
    Good::Metal,
    Good::Salt,
    Good::Furs,
    Good::Spice,
    Good::Wine,
    Good::Horses,
    Good::Leystone,
];

impl Good {
    pub fn name(self) -> &'static str {
        match self {
            Good::Grain => "grain",
            Good::Livestock => "cattle",
            Good::Fish => "salt fish",
            Good::Timber => "timber",
            Good::Stone => "stone",
            Good::Metal => "ore",
            Good::Salt => "salt",
            Good::Furs => "furs",
            Good::Spice => "spice",
            Good::Wine => "wine",
            Good::Horses => "horses",
            Good::Leystone => "leystone",
        }
    }
    /// Its place in the bitset of what a hinterland offers.
    pub fn bit(self) -> u16 {
        1u16 << (self as u8)
    }
    /// What a load of it is worth. The dear ones are the ones worth carrying
    /// a long way, which is what makes a long route worth having at all.
    pub fn worth(self) -> f32 {
        match self {
            Good::Grain | Good::Fish | Good::Timber | Good::Stone => 1.0,
            Good::Livestock | Good::Salt | Good::Furs => 1.4,
            Good::Metal | Good::Horses | Good::Wine => 1.8,
            Good::Spice => 2.4,
            Good::Leystone => 2.8,
        }
    }
}

/// What each cell of a world produces, if anything.
///
/// A pure reading of the terrain, so it is worked out once and rebuilt on
/// load rather than stored — the same bargain as the sea routes.
pub fn goods_of(t: &Terrain) -> Vec<Option<Good>> {
    // Rebuilt on load, so it meets whatever a save file happens to hold.
    // Every array it reads has to cover the area the terrain claims.
    let n = t.w.saturating_mul(t.h);
    if n == 0
        || t.biome.len() < n
        || t.mana.len() < n
        || t.minerals.len() < n
        || t.coast.len() < n
        || t.moist.len() < n
        || t.temp.len() < n
        || t.fertility.len() < n
    {
        return Vec::new();
    }
    (0..n)
        .map(|i| {
            if !t.is_land(i) {
                return None;
            }
            // Ranked by how strongly the ground says so, so a cell with ore
            // *and* forest is ore country: the rarer thing wins.
            if t.mana[i] > 0.72 {
                return Some(Good::Leystone);
            }
            if t.minerals[i] > 0.55 {
                return Some(Good::Metal);
            }
            // Salt where the sea meets dry country, or in a desert basin.
            if (t.coast[i] && t.moist[i] < 0.35)
                || (t.biome[i] == Biome::Desert && t.moist[i] < 0.2)
            {
                return Some(Good::Salt);
            }
            match t.biome[i] {
                Biome::Jungle if t.temp[i] > 0.7 => Some(Good::Spice),
                Biome::Taiga | Biome::Tundra => Some(Good::Furs),
                Biome::Mountain | Biome::Peak | Biome::Hills if t.minerals[i] > 0.3 => {
                    Some(Good::Metal)
                }
                Biome::Mountain | Biome::Peak => Some(Good::Stone),
                Biome::Forest => Some(Good::Timber),
                Biome::Steppe | Biome::Savanna => Some(Good::Horses),
                Biome::Hills if t.temp[i] > 0.55 && t.moist[i] > 0.3 => Some(Good::Wine),
                Biome::Grassland if t.fertility[i] > 0.55 => Some(Good::Grain),
                Biome::Grassland => Some(Good::Livestock),
                _ if t.coast[i] => Some(Good::Fish),
                _ if t.fertility[i] > 0.5 => Some(Good::Grain),
                _ => None,
            }
        })
        .collect()
}

/// A standing trade between two cities.
#[derive(Clone, Copy, Debug)]
pub struct Route {
    pub a: usize,
    pub b: usize,
    /// What it is worth a year to each end, before either end's troubles.
    pub value: f32,
    /// Whether it goes by water.
    pub by_sea: bool,
    /// Whether anything is moving along it now. A route between realms at
    /// war is a road with nobody on it.
    pub open: bool,
    /// The year the road last changed state. Wars are short and frequent,
    /// so without this the same pair of cities closed and reopened every
    /// three years and the chronicle filled with caravans being mourned and
    /// then not mourned.
    pub since: i32,
}

/// How far a cart will go overland to trade, before roads.
const LAND_RANGE: usize = 14;
/// How far a hull will go, which is further, because water is cheap.
const SEA_RANGE: usize = 30;
/// How wide a hinterland a city draws its goods from.
const HINTERLAND: i32 = 4;
/// How often the routes are worked out again. Cities are founded and fall
/// slowly, so this does not need to be yearly — and it is the one sweep in
/// this module that costs anything.
pub const REFRESH: i32 = 20;

/// What a city can offer: the goods of its own hinterland, as a bitset.
fn hinterland(w: &World, city: usize) -> u16 {
    let cell = w.cities[city].cell;
    let (cx, cy) = w.terrain.xy(cell);
    let mut have = 0u16;
    for dy in -HINTERLAND..=HINTERLAND {
        for dx in -HINTERLAND..=HINTERLAND {
            let x = cx as i32 + dx;
            let y = cy as i32 + dy;
            if x < 0 || y < 0 || x >= w.terrain.w as i32 || y >= w.terrain.h as i32 {
                continue;
            }
            let i = w.terrain.idx(x as usize, y as usize);
            if let Some(Some(g)) = w.goods.get(i) {
                have |= g.bit();
            }
        }
    }
    have
}

/// Work out the world's trade routes again.
///
/// A route is worth having when each end has something the other lacks. Its
/// value falls with distance and rises with what is being carried, so a long
/// route only pays for something dear — which is why spice travels further
/// than grain, and why the cities that grow rich are the ones in between.
pub fn refresh(w: &mut World) {
    let tn = w.tuning;
    // What the roads that already exist have been doing, so that rebuilding
    // the network does not make them forget it. Every twentieth year this
    // function replaced the whole list with fresh routes marked open as of
    // this year — which meant a road shut by a war came back open until
    // `open_and_close` corrected it later the same tick, and, worse, that
    // `since` was reset for every road in the world. `since` is what stops
    // the chronicle reporting the same two cities every three years, so
    // resetting it silenced the reports for a decade after every rebuild,
    // and it is what the Roads page shows, where it read as though every
    // road in the world had been laid in the same year.
    let mut history: BTreeMap<(usize, usize), (bool, i32)> = BTreeMap::new();
    for r in &w.routes {
        history.insert((r.a.min(r.b), r.a.max(r.b)), (r.open, r.since));
    }
    let live: Vec<usize> = (0..w.cities.len())
        .filter(|&c| w.cities[c].destroyed.is_none())
        .collect();
    if live.is_empty() {
        w.routes.clear();
        return;
    }
    let have: Vec<u16> = live.iter().map(|&c| hinterland(w, c)).collect();
    let mut routes: Vec<Route> = Vec::new();
    for (ia, &a) in live.iter().enumerate() {
        for (ib, &b) in live.iter().enumerate().skip(ia + 1) {
            let (ca, cb) = (w.cities[a].cell, w.cities[b].cell);
            let d = w.terrain.dist(ca, cb);
            // By land if near enough; otherwise by water, if both are on it
            // and a crossing joins them within anybody's reach.
            let by_sea = d > LAND_RANGE;
            if by_sea && (d > SEA_RANGE || !w.terrain.coast[ca] || !w.terrain.coast[cb]) {
                continue;
            }
            // What each end wants that the other has.
            let a_offers = have[ia] & !have[ib];
            let b_offers = have[ib] & !have[ia];
            if a_offers == 0 || b_offers == 0 {
                continue;
            }
            let worth = |bits: u16| -> f32 {
                GOODS
                    .into_iter()
                    .filter(|g| bits & g.bit() != 0)
                    .map(Good::worth)
                    .sum()
            };
            let trade = worth(a_offers).min(worth(b_offers));
            // Distance is dear, and dearer by land.
            let far = d as f32 / if by_sea { SEA_RANGE } else { LAND_RANGE } as f32;
            // A road is worth what the two ends can buy from each other, and
            // the ends were not being weighed at all: a route's value came
            // from the goods on the ground and the distance between them,
            // both of which are fixed for ever, so the whole trade of a
            // world was a constant while its cities grew. Trade's share of
            // a realm's income fell from a quarter in the second century to
            // a fortieth by the twenty-eighth, purely by being left behind.
            //
            // This is the other half of a gravity model, whose distance term
            // was already here. Bounded at both ends, because a term that
            // grows with the square of a city is exactly the kind of thing
            // that has to be stopped from running away.
            let mass = ((w.cities[a].pop * w.cities[b].pop).sqrt() / tn.trade_mass_ref.max(0.01))
                .clamp(0.2, 8.0);
            // What the two ends know about carrying goods, taken from the
            // better of them: a road is a thing two places share, and the
            // more advanced partner is the one who organises the trade.
            let craft = [w.cities[a].polity, w.cities[b].polity]
                .into_iter()
                .flatten()
                .filter(|&q| w.polities[q].alive())
                .map(|q| w.tech_trade_mult(q))
                .fold(1.0f32, f32::max);
            let value =
                trade * mass * craft * (1.0 - far * 0.7).max(0.1) * if by_sea { 1.2 } else { 1.0 };
            if value < 0.35 {
                continue;
            }
            let (open, since) = history
                .get(&(a.min(b), a.max(b)))
                .copied()
                .unwrap_or((true, w.year));
            routes.push(Route {
                a,
                b,
                value,
                by_sea,
                open,
                since,
            });
        }
    }
    // A city can only work so many roads at once. Keeping the best few per
    // city stops a dense cluster of towns generating thousands of routes
    // that each carry nothing.
    routes.sort_by(|x, y| y.value.total_cmp(&x.value).then(x.a.cmp(&y.a)));
    let mut per_city = vec![0u8; w.cities.len()];
    routes.retain(|r| {
        if per_city[r.a] >= MAX_ROUTES_PER_CITY || per_city[r.b] >= MAX_ROUTES_PER_CITY {
            return false;
        }
        per_city[r.a] += 1;
        per_city[r.b] += 1;
        true
    });
    w.routes = routes;
}

/// How long a road must have been in its previous state before a change is
/// worth reporting.
const MIN_REPORT: i32 = 15;

/// Most standing trades one city keeps up.
const MAX_ROUTES_PER_CITY: u8 = 5;

/// One year of trade.
pub fn tick(w: &mut World) {
    if w.year % REFRESH == 0 || w.routes.is_empty() {
        refresh(w);
    }
    open_and_close(w);
    reckon(w);
}

/// Total up what trade is worth, once, for everybody who will ask.
///
/// `city_trade` and `trade_between` used to walk the whole route list on
/// every call, and `economy` asks once per realm while `diplomacy` asks once
/// per pair of neighbours. On a large map that is a thousand routes times a
/// thousand cities times every year — the cost of a year climbed from half a
/// millisecond to sixteen by year eighteen hundred, and kept climbing.
///
/// Reckoned here instead: one pass over the routes, and every later question
/// is a lookup.
///
/// Also called at load. `economy` reads these totals in an earlier phase
/// than the one that fills them, so a world reloaded without them governs
/// its first year on no trade at all and diverges from the one that wrote
/// the file.
pub fn reckon(w: &mut World) {
    w.city_takings.clear();
    w.city_takings.resize(w.cities.len(), 0.0);
    w.pair_trade.clear();
    for r in &w.routes {
        if !r.open {
            continue;
        }
        if let Some(slot) = w.city_takings.get_mut(r.a) {
            *slot += r.value;
        }
        if let Some(slot) = w.city_takings.get_mut(r.b) {
            *slot += r.value;
        }
        if let (Some(pa), Some(pb)) = (w.cities[r.a].polity, w.cities[r.b].polity) {
            if pa != pb {
                *w.pair_trade.entry((pa.min(pb), pa.max(pb))).or_insert(0.0) += r.value;
            }
        }
    }
}

/// Decide which routes are running, and tell the world when a great one
/// stops.
///
/// This is where interdependence bites: a war between two realms closes the
/// roads between them, and the wealth drains out of cities that never saw a
/// soldier.
fn open_and_close(w: &mut World) {
    for i in 0..w.routes.len() {
        let r = w.routes[i];
        let (pa, pb) = (w.cities[r.a].polity, w.cities[r.b].polity);
        let ruined = w.cities[r.a].destroyed.is_some() || w.cities[r.b].destroyed.is_some();
        let at_war = match (pa, pb) {
            (Some(x), Some(y)) => x != y && w.war_between(x, y).is_some(),
            _ => false,
        };
        let open = !ruined && !at_war;
        if open == r.open {
            continue;
        }
        let held = w.year - r.since;
        w.routes[i].open = open;
        w.routes[i].since = w.year;
        // Only the great roads are worth a line, and only when the change
        // has any weight to it: a road that shuts for two years while a
        // border skirmish plays out is not news, and saying so every time
        // filled the chronicle with the same two cities.
        if r.value >= 4.0 && held >= MIN_REPORT {
            let text = if open {
                prose::route_reopened(w, r.a, r.b)
            } else {
                prose::route_closed(w, r.a, r.b, at_war)
            };
            let refs = vec![Ref::City(r.a), Ref::City(r.b)];
            let loc = Some(w.cities[r.a].cell);
            w.log(1, EventKind::Politics, &refs, loc, text);
        }
    }
}

impl World {
    /// What trade is worth to a city this year. A lookup; see `reckon`.
    pub fn city_trade(&self, city: usize) -> f32 {
        self.city_takings.get(city).copied().unwrap_or(0.0)
    }

    /// What trade is worth to a realm: the tolls on every road that touches
    /// its cities.
    pub fn realm_trade(&self, p: usize) -> f32 {
        self.polities[p]
            .cities
            .iter()
            .map(|&c| self.city_trade(c))
            .sum()
    }

    /// Cities this one trades with, for the interface.
    pub fn trade_partners(&self, city: usize) -> Vec<(usize, f32, bool)> {
        let mut out: Vec<(usize, f32, bool)> = self
            .routes
            .iter()
            .filter(|r| r.open && (r.a == city || r.b == city))
            .map(|r| (if r.a == city { r.b } else { r.a }, r.value, r.by_sea))
            .collect();
        out.sort_by(|x, y| y.1.total_cmp(&x.1));
        out
    }

    /// Whether two realms have any road between them worth the name. Trade
    /// is what makes two realms unwilling to fight, and it has to be actual
    /// trade rather than two cultures that both like the idea of it.
    pub fn trade_between(&self, p: usize, q: usize) -> f32 {
        self.pair_trade
            .get(&(p.min(q), p.max(q)))
            .copied()
            .unwrap_or(0.0)
    }

    /// The goods a city's own country produces, named.
    pub fn city_goods(&self, city: usize) -> Vec<Good> {
        let bits = hinterland(self, city);
        GOODS.into_iter().filter(|g| bits & g.bit() != 0).collect()
    }
}
