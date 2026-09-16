//! World geography: elevation, climate, rivers, biomes, mana, minerals,
//! and named geographical features. Geography is the stage everything
//! else plays out on, so it is generated once and mostly read afterwards.

use crate::lang::Language;
use crate::noise::Noise;
use crate::rng::Rng;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum Biome {
    DeepOcean,
    Ocean,
    Shallows,
    Lake,
    Ice,
    Tundra,
    Taiga,
    Steppe,
    Grassland,
    Forest,
    Jungle,
    Savanna,
    Desert,
    Swamp,
    Hills,
    Mountain,
    Peak,
    Wastes,
}

/// Coarse groupings used for race affinities and prose.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum BiomeGroup {
    Cold = 0,
    Temperate = 1,
    Hot = 2,
    Highland = 3,
    Wetland = 4,
    Arid = 5,
}
pub const GROUP_COUNT: usize = 6;

impl BiomeGroup {
    pub fn all() -> [BiomeGroup; GROUP_COUNT] {
        [
            BiomeGroup::Cold,
            BiomeGroup::Temperate,
            BiomeGroup::Hot,
            BiomeGroup::Highland,
            BiomeGroup::Wetland,
            BiomeGroup::Arid,
        ]
    }
    pub fn phrase(self) -> &'static str {
        match self {
            BiomeGroup::Cold => "the cold north and the snow-forests",
            BiomeGroup::Temperate => "the green lowlands and river valleys",
            BiomeGroup::Hot => "the sun-drenched jungles and savannas",
            BiomeGroup::Highland => "the high hills and mountain fastnesses",
            BiomeGroup::Wetland => "the marshes and deltas",
            BiomeGroup::Arid => "the deserts and dry steppes",
        }
    }
}

impl Biome {
    pub fn is_water(self) -> bool {
        matches!(
            self,
            Biome::DeepOcean | Biome::Ocean | Biome::Shallows | Biome::Lake
        )
    }
    pub fn is_sea(self) -> bool {
        matches!(self, Biome::DeepOcean | Biome::Ocean | Biome::Shallows)
    }
    pub fn name(self) -> &'static str {
        match self {
            Biome::DeepOcean => "deep ocean",
            Biome::Ocean => "ocean",
            Biome::Shallows => "shallows",
            Biome::Lake => "lake",
            Biome::Ice => "ice",
            Biome::Tundra => "tundra",
            Biome::Taiga => "taiga",
            Biome::Steppe => "steppe",
            Biome::Grassland => "grassland",
            Biome::Forest => "forest",
            Biome::Jungle => "jungle",
            Biome::Savanna => "savanna",
            Biome::Desert => "desert",
            Biome::Swamp => "swamp",
            Biome::Hills => "hills",
            Biome::Mountain => "mountains",
            Biome::Peak => "peaks",
            Biome::Wastes => "blighted wastes",
        }
    }
    pub fn group(self) -> Option<BiomeGroup> {
        match self {
            Biome::Ice | Biome::Tundra | Biome::Taiga => Some(BiomeGroup::Cold),
            Biome::Grassland | Biome::Forest => Some(BiomeGroup::Temperate),
            Biome::Jungle | Biome::Savanna => Some(BiomeGroup::Hot),
            Biome::Hills | Biome::Mountain | Biome::Peak => Some(BiomeGroup::Highland),
            Biome::Swamp => Some(BiomeGroup::Wetland),
            Biome::Desert | Biome::Steppe | Biome::Wastes => Some(BiomeGroup::Arid),
            _ => None,
        }
    }
    /// Cost to expand into / travel through. `None` is impassable.
    pub fn move_cost(self) -> Option<f32> {
        match self {
            Biome::DeepOcean | Biome::Ocean | Biome::Lake | Biome::Peak => None,
            Biome::Shallows => Some(4.0),
            Biome::Ice => Some(8.0),
            Biome::Tundra => Some(3.0),
            Biome::Taiga => Some(2.4),
            Biome::Steppe => Some(1.2),
            Biome::Grassland => Some(1.0),
            Biome::Forest => Some(1.9),
            Biome::Jungle => Some(3.0),
            Biome::Savanna => Some(1.2),
            Biome::Desert => Some(3.2),
            Biome::Swamp => Some(3.0),
            Biome::Hills => Some(2.0),
            Biome::Mountain => Some(5.5),
            Biome::Wastes => Some(4.0),
        }
    }
    pub fn base_fertility(self) -> f32 {
        match self {
            Biome::Ice => 0.0,
            Biome::Tundra => 0.12,
            Biome::Taiga => 0.3,
            Biome::Steppe => 0.45,
            Biome::Grassland => 1.0,
            Biome::Forest => 0.7,
            Biome::Jungle => 0.55,
            Biome::Savanna => 0.6,
            Biome::Desert => 0.06,
            Biome::Swamp => 0.35,
            Biome::Hills => 0.5,
            Biome::Mountain => 0.15,
            Biome::Peak => 0.0,
            Biome::Wastes => 0.04,
            _ => 0.0,
        }
    }
    /// Defensive terrain multiplier.
    pub fn defense(self) -> f32 {
        match self {
            Biome::Hills => 1.3,
            Biome::Mountain => 1.8,
            Biome::Peak => 2.5,
            Biome::Swamp => 1.4,
            Biome::Jungle => 1.3,
            Biome::Forest => 1.15,
            Biome::Taiga => 1.15,
            _ => 1.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FeatureKind {
    Ocean,
    Sea,
    Lake,
    Range,
    Forest,
    Jungle,
    Desert,
    Marsh,
    Steppe,
    Island,
    Continent,
    River,
    Nexus,
}

impl FeatureKind {
    pub fn label(self) -> &'static str {
        match self {
            FeatureKind::Ocean => "ocean",
            FeatureKind::Sea => "sea",
            FeatureKind::Lake => "lake",
            FeatureKind::Range => "mountain range",
            FeatureKind::Forest => "forest",
            FeatureKind::Jungle => "jungle",
            FeatureKind::Desert => "desert",
            FeatureKind::Marsh => "marsh",
            FeatureKind::Steppe => "steppe",
            FeatureKind::Island => "island",
            FeatureKind::Continent => "continent",
            FeatureKind::River => "river",
            FeatureKind::Nexus => "ley nexus",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Feature {
    pub kind: FeatureKind,
    pub name: Option<String>,
    pub named_by: Option<usize>,
    pub cells: Vec<usize>,
    pub center: (usize, usize),
}

impl Feature {
    pub fn display(&self) -> String {
        match &self.name {
            Some(n) => n.clone(),
            None => format!("an unnamed {}", self.kind.label()),
        }
    }
}

pub struct Terrain {
    pub w: usize,
    pub h: usize,
    pub elev: Vec<f32>,
    pub sea: f32,
    pub temp: Vec<f32>,
    pub moist: Vec<f32>,
    pub biome: Vec<Biome>,
    /// 0 none, 1 stream, 2 river, 3 great river
    pub river: Vec<u8>,
    pub flow: Vec<u32>,
    pub fertility: Vec<f32>,
    pub minerals: Vec<f32>,
    pub mana: Vec<f32>,
    pub coast: Vec<bool>,
    /// Index+1 into `features` for the land/water region a cell belongs to (0 = none).
    pub region: Vec<u16>,
    /// Index+1 into `features` for the river a cell carries (0 = none).
    pub river_feat: Vec<u16>,
    pub landmass: Vec<u16>,
    pub features: Vec<Feature>,
    pub land_count: usize,
}

const NB8: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];
const NB4: [(i32, i32); 4] = [(0, -1), (-1, 0), (1, 0), (0, 1)];

impl Terrain {
    pub fn empty() -> Terrain {
        Terrain {
            w: 0,
            h: 0,
            elev: Vec::new(),
            sea: 0.5,
            temp: Vec::new(),
            moist: Vec::new(),
            biome: Vec::new(),
            river: Vec::new(),
            flow: Vec::new(),
            fertility: Vec::new(),
            minerals: Vec::new(),
            mana: Vec::new(),
            coast: Vec::new(),
            region: Vec::new(),
            river_feat: Vec::new(),
            landmass: Vec::new(),
            features: Vec::new(),
            land_count: 0,
        }
    }

    #[inline]
    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.w + x
    }
    #[inline]
    pub fn xy(&self, i: usize) -> (usize, usize) {
        (i % self.w, i / self.w)
    }
    pub fn n(&self) -> usize {
        self.w * self.h
    }
    pub fn is_land(&self, i: usize) -> bool {
        !self.biome[i].is_water()
    }
    pub fn neighbors8(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        let (x, y) = self.xy(i);
        NB8.iter().filter_map(move |&(dx, dy)| {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= self.w as i32 || ny >= self.h as i32 {
                None
            } else {
                Some(self.idx(nx as usize, ny as usize))
            }
        })
    }
    pub fn neighbors4(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        let (x, y) = self.xy(i);
        NB4.iter().filter_map(move |&(dx, dy)| {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= self.w as i32 || ny >= self.h as i32 {
                None
            } else {
                Some(self.idx(nx as usize, ny as usize))
            }
        })
    }
    /// Chebyshev distance between two cells.
    pub fn dist(&self, a: usize, b: usize) -> usize {
        let (ax, ay) = self.xy(a);
        let (bx, by) = self.xy(b);
        (ax as i32 - bx as i32)
            .unsigned_abs()
            .max((ay as i32 - by as i32).unsigned_abs()) as usize
    }
    pub fn height_norm(&self, i: usize) -> f32 {
        ((self.elev[i] - self.sea) / (1.0 - self.sea)).clamp(0.0, 1.0)
    }
    pub fn feature_at(&self, i: usize) -> Option<usize> {
        if self.region[i] > 0 {
            Some(self.region[i] as usize - 1)
        } else {
            None
        }
    }
    pub fn river_at(&self, i: usize) -> Option<usize> {
        if self.river_feat[i] > 0 {
            Some(self.river_feat[i] as usize - 1)
        } else {
            None
        }
    }

    /// Turn cells into blighted wastes (magical catastrophe).
    pub fn blight(&mut self, cells: &[usize]) {
        for &c in cells {
            if self.is_land(c) && !matches!(self.biome[c], Biome::Peak | Biome::Mountain) {
                self.biome[c] = Biome::Wastes;
                self.fertility[c] = 0.04;
                self.mana[c] = (self.mana[c] + 0.4).min(1.0);
            }
        }
    }

    pub fn describe_cell(&self, i: usize) -> String {
        let b = self.biome[i];
        let mut s = String::from(b.name());
        if self.river[i] >= 2 {
            s.push_str(", on a river");
        } else if self.river[i] == 1 {
            s.push_str(", by a stream");
        }
        if self.coast[i] && !b.is_water() {
            s.push_str(", coastal");
        }
        s
    }
}

fn percentile(vals: &[f32], p: f32) -> f32 {
    let mut v: Vec<f32> = vals.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let k = ((v.len() - 1) as f32 * p) as usize;
    v[k]
}

pub fn generate(rng: &Rng, w: usize, h: usize) -> Terrain {
    let n = w * h;
    let noise_e = Noise::new(rng);
    let noise_c = Noise::new(rng);
    let noise_r = Noise::new(rng);
    let noise_t = Noise::new(rng);
    let noise_m = Noise::new(rng);
    let noise_mana = Noise::new(rng);
    let noise_min = Noise::new(rng);
    let noise_warp = Noise::new(rng);

    // --- Elevation -------------------------------------------------------
    let aspect = w as f32 / h as f32;
    let mut elev = vec![0f32; n];
    let scale = 1.0 / (w.max(h) as f32);
    let ox = rng.range(0.0, 1000.0) as f32;
    let oy = rng.range(0.0, 1000.0) as f32;
    for y in 0..h {
        for x in 0..w {
            let fx = x as f32 * scale * 3.2 + ox;
            // Cells are ~2:1 tall in a terminal, so stretch y.
            let fy = y as f32 * scale * 3.2 * (aspect / 2.0).max(1.0) + oy;
            let wx = noise_warp.fbm(fx * 0.7, fy * 0.7, 2, 2.0, 0.5) * 0.6;
            let wy = noise_warp.fbm(fx * 0.7 + 31.7, fy * 0.7 + 17.3, 2, 2.0, 0.5) * 0.6;
            let base = noise_e.fbm(fx + wx, fy + wy, 6, 2.0, 0.5) * 0.5 + 0.5;
            let cont = noise_c.fbm(fx * 0.45, fy * 0.45, 3, 2.0, 0.5) * 0.5 + 0.5;
            let ridge = noise_r.ridged(fx * 2.8 + wx, fy * 2.8 + wy, 4);
            let ex = (x as f32 / w as f32).min(1.0 - x as f32 / w as f32) * 2.0;
            let ey = (y as f32 / h as f32).min(1.0 - y as f32 / h as f32) * 2.0;
            let edge = (ex.min(ey) * 5.0).clamp(0.0, 1.0);
            let edge = edge * edge * (3.0 - 2.0 * edge);
            let mut e = base * 0.55 + cont * 0.45;
            e = e * (0.25 + 0.75 * edge);
            e += ridge * 0.13 * (cont * 0.6 + 0.4);
            elev[y * w + x] = e;
        }
    }
    let lo = percentile(&elev, 0.01);
    let hi = percentile(&elev, 0.995);
    for e in elev.iter_mut() {
        *e = ((*e - lo) / (hi - lo)).clamp(0.0, 1.0);
    }
    let sea = percentile(&elev, 0.57);

    let mut t = Terrain {
        w,
        h,
        elev,
        sea,
        temp: vec![0.0; n],
        moist: vec![0.0; n],
        biome: vec![Biome::Ocean; n],
        river: vec![0; n],
        flow: vec![0; n],
        fertility: vec![0.0; n],
        minerals: vec![0.0; n],
        mana: vec![0.0; n],
        coast: vec![false; n],
        region: vec![0; n],
        river_feat: vec![0; n],
        landmass: vec![0; n],
        features: Vec::new(),
        land_count: 0,
    };

    // Preliminary water mask.
    let water: Vec<bool> = t.elev.iter().map(|&e| e < sea).collect();

    // --- Temperature -----------------------------------------------------
    // Cooling with altitude uses each cell's rank among land heights, so only
    // the genuinely high country turns cold whatever the continent's shape.
    let mut land_h: Vec<f32> = (0..n).filter(|&i| !water[i]).map(|i| t.elev[i]).collect();
    land_h.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let rank_of = |e: f32| -> f32 {
        if land_h.is_empty() {
            return 0.0;
        }
        let pos = land_h.partition_point(|&v| v < e);
        pos as f32 / land_h.len() as f32
    };
    for y in 0..h {
        let lat = ((y as f32 + 0.5) / h as f32 - 0.5).abs() * 2.0;
        for x in 0..w {
            let i = y * w + x;
            let nz = noise_t.fbm(x as f32 * 0.05, y as f32 * 0.1, 2, 2.0, 0.5) * 0.12;
            let mut temp = 0.98 - lat * 0.98 + nz;
            if !water[i] {
                let rank = rank_of(t.elev[i]);
                temp -= rank.powi(3) * 0.85;
            }
            t.temp[i] = temp.clamp(0.0, 1.0);
        }
    }

    // --- Moisture (prevailing winds with rain shadow) --------------------
    let mut raw = vec![0f32; n];
    for y in 0..h {
        let lat = ((y as f32 + 0.5) / h as f32 - 0.5).abs() * 2.0;
        // Trade winds blow east->west in the tropics, westerlies west->east.
        let east_to_west = lat < 0.33 || lat > 0.8;
        let mut carry = 0.45f32;
        let mut prev_h = 0.0f32;
        for step in 0..w {
            let x = if east_to_west { w - 1 - step } else { step };
            let i = y * w + x;
            if water[i] {
                let warm = 0.5 + 0.5 * t.temp[i];
                carry += (1.0 - carry) * 0.12 * warm;
                raw[i] = carry;
                prev_h = 0.0;
            } else {
                let hn = ((t.elev[i] - sea) / (1.0 - sea)).max(0.0);
                let up = (hn - prev_h).max(0.0);
                let precip = carry * (0.06 + up * 3.0).min(0.9);
                raw[i] = precip * 2.5 + carry * 0.25;
                carry -= precip;
                carry *= 0.985;
                carry = carry.max(0.0);
                prev_h = hn;
            }
        }
    }
    let land_raw: Vec<f32> = (0..n).filter(|&i| !water[i]).map(|i| raw[i]).collect();
    let m_lo = percentile(&land_raw, 0.05);
    let m_hi = percentile(&land_raw, 0.9);
    for i in 0..n {
        let (x, y) = (i % w, i / w);
        let nz = noise_m.fbm(x as f32 * 0.06, y as f32 * 0.12, 3, 2.0, 0.5) * 0.25;
        let v = (raw[i] - m_lo) / (m_hi - m_lo + 1e-6) + nz;
        t.moist[i] = v.clamp(0.0, 1.0);
    }

    // --- Hydrology: priority-flood depression filling, then flow ---------
    let mut filled = t.elev.clone();
    let mut visited = vec![false; n];
    let key = |v: f32| -> i64 { (v * 1.0e6) as i64 };
    let mut heap: BinaryHeap<(Reverse<i64>, usize)> = BinaryHeap::new();
    for i in 0..n {
        if water[i] {
            visited[i] = true;
            heap.push((Reverse(key(filled[i])), i));
        }
    }
    let mut raised = vec![0f32; n];
    while let Some((Reverse(_), i)) = heap.pop() {
        let cur = filled[i];
        for nb in t.neighbors8(i).collect::<Vec<_>>() {
            if visited[nb] {
                continue;
            }
            visited[nb] = true;
            if filled[nb] < cur {
                raised[nb] = cur + 1e-4 - filled[nb];
                filled[nb] = cur + 1e-4;
            }
            heap.push((Reverse(key(filled[nb])), nb));
        }
    }
    // Lakes where depressions were filled substantially.
    let mut lake = vec![false; n];
    for i in 0..n {
        if !water[i] && raised[i] > 0.012 {
            lake[i] = true;
        }
    }
    // Flow direction: steepest descent on filled surface.
    let mut dir: Vec<Option<usize>> = vec![None; n];
    for i in 0..n {
        if water[i] {
            continue;
        }
        let mut best = None;
        let mut best_e = filled[i];
        for nb in t.neighbors8(i) {
            if filled[nb] < best_e {
                best_e = filled[nb];
                best = Some(nb);
            }
        }
        dir[i] = best;
    }
    let mut order: Vec<usize> = (0..n).filter(|&i| !water[i]).collect();
    order.sort_by(|&a, &b| filled[b].partial_cmp(&filled[a]).unwrap());
    let mut acc = vec![0f32; n];
    for &i in &order {
        acc[i] += (t.moist[i] - 0.15).max(0.0) * (1.0 + t.temp[i] * 0.5);
        if let Some(d) = dir[i] {
            acc[d] += acc[i];
        }
    }
    let scale_r = (n as f32 / 16000.0).sqrt();
    let (t1, t2, t3) = (9.0 * scale_r, 26.0 * scale_r, 70.0 * scale_r);
    for i in 0..n {
        if water[i] {
            continue;
        }
        let a = acc[i];
        t.flow[i] = a as u32;
        t.river[i] = if a >= t3 {
            3
        } else if a >= t2 {
            2
        } else if a >= t1 {
            1
        } else {
            0
        };
    }

    // --- Biomes ----------------------------------------------------------
    let ridge_noise = Noise::new(rng);
    // Relief classes by percentile of land height so proportions hold for any world shape.
    let relief: Vec<f32> = (0..n)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let hn = ((t.elev[i] - sea) / (1.0 - sea)).max(0.0);
            hn + ridge_noise.fbm(x as f32 * 0.15, y as f32 * 0.3, 2, 2.0, 0.5) * 0.06
        })
        .collect();
    let land_relief: Vec<f32> = (0..n).filter(|&i| !water[i]).map(|i| relief[i]).collect();
    let hill_t = percentile(&land_relief, 0.72);
    let mount_t = percentile(&land_relief, 0.88);
    let peak_t = percentile(&land_relief, 0.965);
    for i in 0..n {
        let (_x, _y) = (i % w, i / w);
        let e = t.elev[i];
        let b = if water[i] {
            if e < sea - 0.10 {
                Biome::DeepOcean
            } else if e < sea - 0.025 {
                Biome::Ocean
            } else {
                Biome::Shallows
            }
        } else if lake[i] {
            Biome::Lake
        } else {
            let hn = (e - sea) / (1.0 - sea);
            let temp = t.temp[i];
            let moist = t.moist[i];
            let r = relief[i];
            let rn = r - hn;
            if temp < 0.06 && r < mount_t {
                Biome::Ice
            } else if r >= peak_t {
                Biome::Peak
            } else if r >= mount_t {
                Biome::Mountain
            } else if r >= hill_t {
                Biome::Hills
            } else if hn < 0.07 && moist > 0.72 && temp > 0.28 && (t.river[i] > 0 || rn > 0.0) {
                Biome::Swamp
            } else if temp < 0.24 {
                if moist < 0.38 {
                    Biome::Tundra
                } else {
                    Biome::Taiga
                }
            } else if temp < 0.62 {
                if moist < 0.2 {
                    Biome::Steppe
                } else if moist < 0.55 {
                    Biome::Grassland
                } else {
                    Biome::Forest
                }
            } else if moist < 0.2 {
                Biome::Desert
            } else if moist < 0.46 {
                Biome::Savanna
            } else {
                Biome::Jungle
            }
        };
        t.biome[i] = b;
    }
    // Rivers through lakes are fine; rivers should not sit in the sea.
    for i in 0..n {
        if t.biome[i].is_water() {
            t.river[i] = 0;
        }
    }

    // --- Coast, fertility, minerals, mana --------------------------------
    for i in 0..n {
        if t.biome[i].is_water() {
            continue;
        }
        let coastal = t.neighbors8(i).any(|nb| t.biome[nb].is_sea());
        t.coast[i] = coastal;
    }
    for i in 0..n {
        let b = t.biome[i];
        if b.is_water() {
            continue;
        }
        let mut f = b.base_fertility();
        match t.river[i] {
            1 => f += 0.25,
            2 => f = f.max(0.35) + 0.5,
            3 => f = f.max(0.45) + 0.8,
            _ => {}
        }
        if t.coast[i] {
            f += 0.2;
        }
        if t.neighbors8(i).any(|nb| t.biome[nb] == Biome::Lake) {
            f += 0.2;
        }
        t.fertility[i] = f.min(1.8);
        let (x, y) = (i % w, i / w);
        let mn = noise_min.fbm(x as f32 * 0.09, y as f32 * 0.18, 3, 2.0, 0.5) * 0.5 + 0.5;
        let bonus = match b {
            Biome::Hills => 0.25,
            Biome::Mountain => 0.4,
            Biome::Peak => 0.3,
            _ => 0.0,
        };
        t.minerals[i] = (mn * 0.7 + bonus).clamp(0.0, 1.0);
    }
    // Mana: slow field plus nexus points.
    let nexus_count = 3 + rng.below(4);
    let mut nexi: Vec<(f32, f32, f32)> = Vec::new();
    for _ in 0..nexus_count {
        nexi.push((
            rng.range(0.05, 0.95) as f32 * w as f32,
            rng.range(0.05, 0.95) as f32 * h as f32,
            rng.range(4.0, 9.0) as f32,
        ));
    }
    for i in 0..n {
        let (x, y) = (i % w, i / w);
        let base = noise_mana.fbm(x as f32 * 0.03, y as f32 * 0.06, 3, 2.0, 0.5) * 0.5 + 0.5;
        let mut m = base * 0.55;
        for &(nx, ny, r) in &nexi {
            let dx = (x as f32 - nx) / r;
            let dy = (y as f32 - ny) / (r * 0.5);
            let d2 = dx * dx + dy * dy;
            m += 0.7 * (-d2).exp();
        }
        match t.biome[i] {
            Biome::Peak => m += 0.15,
            Biome::Swamp => m += 0.1,
            Biome::Jungle => m += 0.05,
            Biome::Mountain => m += 0.08,
            _ => {}
        }
        t.mana[i] = m.clamp(0.0, 1.0);
    }

    t.land_count = t.biome.iter().filter(|b| !b.is_water()).count();
    label_features(&mut t, &nexi, dir);
    t
}

fn components(t: &Terrain, pred: &dyn Fn(usize) -> bool, four: bool) -> Vec<Vec<usize>> {
    let n = t.n();
    let mut seen = vec![false; n];
    let mut out = Vec::new();
    for s in 0..n {
        if seen[s] || !pred(s) {
            continue;
        }
        let mut comp = Vec::new();
        let mut stack = vec![s];
        seen[s] = true;
        while let Some(i) = stack.pop() {
            comp.push(i);
            let nbs: Vec<usize> = if four {
                t.neighbors4(i).collect()
            } else {
                t.neighbors8(i).collect()
            };
            for nb in nbs {
                if !seen[nb] && pred(nb) {
                    seen[nb] = true;
                    stack.push(nb);
                }
            }
        }
        out.push(comp);
    }
    out
}

fn centroid(t: &Terrain, cells: &[usize]) -> (usize, usize) {
    let mut sx = 0usize;
    let mut sy = 0usize;
    for &c in cells {
        let (x, y) = t.xy(c);
        sx += x;
        sy += y;
    }
    (sx / cells.len().max(1), sy / cells.len().max(1))
}

fn label_features(t: &mut Terrain, nexi: &[(f32, f32, f32)], dir: Vec<Option<usize>>) {
    let n = t.n();
    let water_total = n - t.land_count;
    let mut features: Vec<Feature> = Vec::new();
    let mut region = vec![0u16; n];
    let push = |features: &mut Vec<Feature>,
                region: &mut Vec<u16>,
                kind: FeatureKind,
                cells: Vec<usize>,
                t: &Terrain| {
        let id = features.len() + 1;
        for &c in &cells {
            if region[c] == 0 {
                region[c] = id as u16;
            }
        }
        let center = centroid(t, &cells);
        features.push(Feature {
            kind,
            name: None,
            named_by: None,
            cells,
            center,
        });
    };

    // Seas and oceans.
    for comp in components(t, &|i| t.biome[i].is_sea(), true) {
        let kind = if comp.len() >= water_total / 4 {
            FeatureKind::Ocean
        } else if comp.len() >= 25 {
            FeatureKind::Sea
        } else {
            continue;
        };
        push(&mut features, &mut region, kind, comp, t);
    }
    for comp in components(t, &|i| t.biome[i] == Biome::Lake, false) {
        if comp.len() >= 3 {
            push(&mut features, &mut region, FeatureKind::Lake, comp, t);
        }
    }
    // Landmasses.
    let mut landmass = vec![0u16; n];
    let mut lm_id = 0u16;
    for comp in components(t, &|i| t.is_land(i), false) {
        lm_id += 1;
        for &c in &comp {
            landmass[c] = lm_id;
        }
        let kind = if comp.len() < 260 {
            FeatureKind::Island
        } else {
            FeatureKind::Continent
        };
        if comp.len() >= 3 {
            let cells = comp.clone();
            let id = features.len() + 1;
            let center = centroid(t, &cells);
            features.push(Feature {
                kind,
                name: None,
                named_by: None,
                cells,
                center,
            });
            // Islands label their cells as a region; continents only when nothing finer applies.
            if kind == FeatureKind::Island {
                for &c in &comp {
                    region[c] = id as u16;
                }
            }
        }
    }
    t.landmass = landmass;
    // Ranges, forests, deserts, marshes, steppes.
    let specs: Vec<(FeatureKind, Box<dyn Fn(usize, &Terrain) -> bool>, usize)> = vec![
        (
            FeatureKind::Range,
            Box::new(|i, t| matches!(t.biome[i], Biome::Mountain | Biome::Peak)),
            6,
        ),
        (
            FeatureKind::Jungle,
            Box::new(|i, t| t.biome[i] == Biome::Jungle),
            18,
        ),
        (
            FeatureKind::Forest,
            Box::new(|i, t| matches!(t.biome[i], Biome::Forest | Biome::Taiga)),
            18,
        ),
        (
            FeatureKind::Desert,
            Box::new(|i, t| t.biome[i] == Biome::Desert),
            16,
        ),
        (
            FeatureKind::Marsh,
            Box::new(|i, t| t.biome[i] == Biome::Swamp),
            6,
        ),
        (
            FeatureKind::Steppe,
            Box::new(|i, t| matches!(t.biome[i], Biome::Steppe | Biome::Savanna)),
            30,
        ),
    ];
    for (kind, pred, min) in specs {
        for comp in components(t, &|i| pred(i, t), false) {
            if comp.len() >= min {
                push(&mut features, &mut region, kind, comp, t);
            }
        }
    }
    // Rivers: trace from mouths upstream along the largest tributary.
    let mut river_feat = vec![0u16; n];
    let mut mouths: Vec<usize> = (0..n)
        .filter(|&i| t.river[i] >= 2 && dir[i].map(|d| t.biome[d].is_water()).unwrap_or(true))
        .collect();
    mouths.sort_by_key(|&i| std::cmp::Reverse(t.flow[i]));
    for m in mouths {
        if river_feat[m] != 0 {
            continue;
        }
        let mut cells = vec![m];
        let mut cur = m;
        loop {
            // Find the upstream neighbour with the largest flow that drains into cur.
            let mut best: Option<usize> = None;
            let mut best_flow = 0u32;
            for nb in t.neighbors8(cur) {
                if dir[nb] == Some(cur)
                    && t.river[nb] >= 1
                    && river_feat[nb] == 0
                    && t.flow[nb] > best_flow
                {
                    best_flow = t.flow[nb];
                    best = Some(nb);
                }
            }
            match best {
                Some(b) => {
                    cells.push(b);
                    cur = b;
                }
                None => break,
            }
        }
        if cells.len() >= 6 {
            let id = features.len() + 1;
            for &c in &cells {
                river_feat[c] = id as u16;
            }
            let center = t.xy(cells[cells.len() / 2]);
            features.push(Feature {
                kind: FeatureKind::River,
                name: None,
                named_by: None,
                cells,
                center,
            });
        }
    }
    // Ley nexus points (the strongest mana cells near each nexus centre).
    for &(nx, ny, _) in nexi {
        let x = (nx as usize).min(t.w - 1);
        let y = (ny as usize).min(t.h - 1);
        let mut best = t.idx(x, y);
        let mut best_m = -1.0;
        for dy in -3i32..=3 {
            for dx in -6i32..=6 {
                let cx = x as i32 + dx;
                let cy = y as i32 + dy;
                if cx < 0 || cy < 0 || cx >= t.w as i32 || cy >= t.h as i32 {
                    continue;
                }
                let i = t.idx(cx as usize, cy as usize);
                if t.is_land(i) && t.mana[i] > best_m {
                    best_m = t.mana[i];
                    best = i;
                }
            }
        }
        if t.is_land(best) {
            let center = t.xy(best);
            features.push(Feature {
                kind: FeatureKind::Nexus,
                name: None,
                named_by: None,
                cells: vec![best],
                center,
            });
        }
    }
    t.features = features;
    t.region = region;
    t.river_feat = river_feat;
}

/// Coin a name for a feature in the given language.
pub fn name_feature(kind: FeatureKind, lang: &Language, rng: &Rng) -> String {
    let w = lang.name(rng);
    let pick = |opts: &[&str]| -> String { rng.pick(opts).replace("{}", &w) };
    match kind {
        FeatureKind::Ocean => pick(&["the {} Ocean", "the Great {} Sea", "the {} Deep"]),
        FeatureKind::Sea => pick(&[
            "the Sea of {}",
            "the {} Sea",
            "the {} Gulf",
            "the Bay of {}",
        ]),
        FeatureKind::Lake => pick(&["Lake {}", "the {} Mere", "{} Lake", "the Waters of {}"]),
        FeatureKind::Range => pick(&[
            "the {} Mountains",
            "the {} Peaks",
            "the {} Spine",
            "the {} Teeth",
            "the {} Heights",
            "the Wall of {}",
        ]),
        FeatureKind::Forest => pick(&[
            "the {} Forest",
            "the {} Wood",
            "the {}wood",
            "the Wilds of {}",
            "the {} Deepwood",
        ]),
        FeatureKind::Jungle => pick(&[
            "the {} Jungle",
            "the Green {}",
            "the {} Tangle",
            "the Jungles of {}",
        ]),
        FeatureKind::Desert => pick(&[
            "the {} Desert",
            "the {} Sands",
            "the {} Waste",
            "the Red {}",
            "the {} Emptiness",
        ]),
        FeatureKind::Marsh => pick(&[
            "the {} Marsh",
            "the {} Fens",
            "the {} Mire",
            "the Drowned {}",
        ]),
        FeatureKind::Steppe => pick(&[
            "the {} Steppe",
            "the {} Plain",
            "the Plains of {}",
            "the {} Reach",
        ]),
        FeatureKind::Island => pick(&["the Isle of {}", "{} Island", "{}", "the {} Isle"]),
        FeatureKind::Continent => pick(&["{}", "{}", "Greater {}", "the {} Land"]),
        FeatureKind::River => pick(&[
            "the {}",
            "the River {}",
            "the {} River",
            "the {}water",
            "the {}flow",
        ]),
        FeatureKind::Nexus => pick(&[
            "the {} Stone",
            "the Eye of {}",
            "the {} Well",
            "the {} Font",
            "the Wound of {}",
        ]),
    }
}
