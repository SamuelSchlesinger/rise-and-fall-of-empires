//! The simulation's tuning constants, gathered in one place.
//!
//! Every balance-relevant number the simulation uses lives here rather than
//! as a literal at its use site, so that the shape of the world can be
//! changed without hunting through the code. The `Default` impl holds the
//! values the simulation was written with; changing any of them changes the
//! history a seed produces.
//!
//! Fields are `f32` where the surrounding arithmetic is `f32`, `f64` where a
//! probability is passed to `Rng::chance`, and `i32` for counts of years and
//! cells. Purely cosmetic or structural numbers (text choices, list sizes,
//! clamps) are deliberately left where they are.

/// A `tune.<field>` setting named a field that does not exist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownField(
    /// The name that was asked for.
    pub String,
);

impl std::fmt::Display for UnknownField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown tuning field '{}'", self.0)
    }
}

impl std::error::Error for UnknownField {}

/// Every balance-relevant number the simulation uses. See the module header.
#[derive(Clone, Copy, Debug)]
pub struct Tuning {
    // -- population and migration -----------------------------------------
    /// Base yearly growth rate of a cell's population.
    pub pop_growth_rate: f32,
    /// Floor of the fecundity multiplier on population growth.
    pub pop_growth_fecund_base: f32,
    /// How much a race's fecundity adds to its growth rate.
    pub pop_growth_fecund_weight: f32,
    /// Yearly share of the population lost where the land cannot feed anyone.
    pub pop_starve_rate: f32,
    /// Yearly share of the excess lost where a cell is over its capacity.
    pub pop_overshoot_rate: f32,
    /// Fertility-to-carrying-capacity multiplier for a cell.
    pub cell_capacity_factor: f32,
    /// How much a realm's development raises the capacity of its land.
    pub cell_capacity_dev_weight: f32,
    /// Fill ratio above which people start to look for somewhere else.
    pub migration_pressure: f32,
    /// A cell needs at least this many people before any of them leave.
    pub migration_min_pop: f32,
    /// Share of a cell's population that moves in one year.
    pub migration_share: f32,
    /// Migrants only move somewhere emptier than this share of their pressure.
    pub migration_fill_ratio: f32,
    /// Yearly chance that a crowded coastal community takes to the water
    /// instead of staying put, when a crossing is within its people's reach.
    pub migration_sea_chance: f64,
    /// Base yearly growth rate of a city.
    pub city_growth_rate: f32,
    /// Local fertility multiplier for a city's carrying capacity.
    pub city_capacity_factor: f32,
    /// How much a realm's development raises its cities' capacity.
    pub city_capacity_dev_weight: f32,
    /// Yearly share of the excess lost where a city is over its capacity.
    pub city_overshoot_rate: f32,
    /// Base yearly chance a subject cell adopts its ruler's culture.
    pub assimilation_rate: f32,
    /// Yearly chance a realm whose land has gone foreign changes its own culture.
    pub culture_shift_chance: f64,
    /// Base chance an isolated community becomes a people of its own.
    pub divergence_chance: f64,
    /// Added to that chance when the community lives under a different crown.
    pub divergence_foreign_ruler: f64,

    // -- expansion ---------------------------------------------------------
    /// Base yearly chance that a dense, stateless people raises a chief.
    pub polity_form_chance: f64,
    /// A cell needs at least this many people to become the seat of a new state.
    pub polity_form_min_pop: f32,
    /// Half-width of the box searched for an existing state before founding one.
    pub polity_form_radius_x: i32,
    /// Half-height of that box.
    pub polity_form_radius_y: i32,
    /// Base yearly expansion budget of a realm.
    pub expand_budget_base: f32,
    /// How much the square root of a realm's population adds to that budget.
    pub expand_budget_pop_factor: f32,
    /// Floor of the ruler's-ambition multiplier on the budget.
    pub expand_ambition_base: f32,
    /// Floor of the stability multiplier on the budget.
    pub expand_stability_base: f32,
    /// Multiplier on the budget while the realm is at war.
    pub expand_war_penalty: f32,
    /// Most cells a realm may claim peacefully in one year.
    pub expand_max_claims: i32,
    /// How heavily rough terrain counts against claiming a cell.
    pub expand_terrain_cost_weight: f32,
    /// How heavily distance from the capital counts against claiming a cell.
    pub expand_distance_penalty: f32,
    /// How much of the distance penalty is added to the cell's cost.
    pub expand_distance_cost_weight: f32,
    /// Reach from the capital, in cells, before development and kind.
    pub expand_reach_base: f32,
    /// How much a realm's development extends its reach.
    pub expand_reach_dev_weight: f32,
    /// How much a cell's fertility attracts a claim.
    pub expand_fertility_weight: f32,

    // -- cities ------------------------------------------------------------
    /// Base yearly chance a realm founds a town.
    pub city_found_chance: f64,
    /// A realm may hold one city per this many cells (plus development).
    pub city_cells_per_city: i32,
    /// Minimum horizontal gap between cities, in cells.
    pub city_spacing_x: i32,
    /// Minimum vertical gap between cities, in cells.
    pub city_spacing_y: i32,
    /// Site score a candidate must beat before a town is built there.
    pub city_site_min_score: f32,
    /// How much the people already living on a cell recommend it as a site,
    /// so that a crowded interior can raise a town and not only the coast.
    pub city_site_pop_weight: f32,

    // -- economy -----------------------------------------------------------
    /// Tax taken from a city's people and prosperity each year.
    pub city_income_factor: f32,
    /// Tax taken from each cell of countryside each year.
    pub cell_income_factor: f32,
    /// Yearly cost of each point of army.
    pub army_upkeep_factor: f32,
    /// Yearly cost of each cell of territory.
    pub cell_upkeep_factor: f32,
    /// Share of a realm's population it keeps under arms.
    pub army_pop_factor: f32,
    /// How much a militaristic culture raises that share.
    pub army_militarism_weight: f32,
    /// How fast the army moves toward its target size in peace.
    pub army_build_rate_peace: f32,
    /// How fast it does so at war.
    pub army_build_rate_war: f32,
    /// Base yearly gain in development.
    pub dev_growth_rate: f32,
    /// Development past which further gains slow to a crawl.
    pub dev_soft_cap: f32,
    /// Development a realm can never pass, however long it lives.
    pub dev_hard_cap: f32,
    /// What development growth is multiplied by above the soft cap, so that
    /// a long-settled realm keeps improving without running away.
    pub dev_overflow_rate: f32,
    /// How fast a city's prosperity moves toward its target.
    pub prosperity_adjust_rate: f32,

    // -- stability and decadence -------------------------------------------
    /// Administrative capacity of a realm before kind, development and cities.
    pub admin_capacity_base: f32,
    /// How many more cells each point of development lets a realm govern.
    pub admin_capacity_dev_weight: f32,
    /// How many more cells each city lets a realm govern.
    pub admin_capacity_city_weight: f32,
    /// The stability a plain, well-ruled realm tends toward.
    pub stability_base: f32,
    /// How much a wise ruler adds to that target.
    pub stability_wisdom_weight: f32,
    /// How much a charismatic ruler adds to it.
    pub stability_charisma_weight: f32,
    /// How much being stretched past administrative capacity costs.
    pub stability_overextension_weight: f32,
    /// Share of the settled world past which a realm begins to pay for its
    /// own size in stability, whatever it knows and however good its roads.
    pub hegemony_free_share: f32,
    /// How steeply it pays beyond that.
    ///
    /// The other brakes on size are all *relative* — capacity, distance,
    /// foreign subjects — and knowledge raises every one of their ceilings,
    /// so a realm far enough ahead eventually escaped all of them together
    /// and took the world. This one is absolute: ruling half of everything
    /// is hard because it is half of everything.
    pub hegemony_weight: f32,
    /// How far a realm can govern from its seat before development, in
    /// cells. The denominator of [`crate::sim::Polity::sprawl`].
    ///
    /// Deliberately *not* the expansion reach. Expansion reach is generous
    /// and scales hard with development, so measuring sprawl against it
    /// meant a developed realm always scored below 1.0 and the distance
    /// brake contributed nothing at all — which is exactly what happened
    /// once knowledge began pushing development to its ceiling. Governing
    /// from far away has to stay hard even for an advanced realm.
    pub admin_reach_base: f32,
    /// How much further each point of development lets a realm govern.
    pub admin_reach_dev_weight: f32,
    /// How heavily distance counts against stability: the penalty per unit
    /// of [`crate::sim::Polity::sprawl`] over 1.0.
    ///
    /// This is the brake on size that administrative capacity could not be.
    /// Capacity rises with the cities a realm holds, and a conqueror takes
    /// cities, so conquest paid for its own administration and a realm that
    /// got ahead could not be stopped. Distance does not work that way: a
    /// province two months' ride from the capital makes the next one harder
    /// to hold, not easier, and an empire breaks along its far edge first.
    pub stability_sprawl_weight: f32,
    /// How much ruling foreign subjects costs.
    pub stability_foreign_weight: f32,
    /// How much war-weariness costs.
    pub stability_exhaustion_weight: f32,
    /// How much decadence costs.
    pub stability_decadence_weight: f32,
    /// How fast stability moves toward its target.
    pub stability_adjust_rate: f32,
    /// Yearly war-weariness gained while fighting.
    pub exhaustion_war_growth: f32,
    /// Yearly war-weariness shed in peace.
    pub exhaustion_peace_decay: f32,
    /// Base yearly gain in decadence for an established realm.
    pub decadence_growth: f32,
    /// How much a wise ruler holds decadence back.
    pub decadence_wisdom_relief: f32,
    /// Multiplier on decadence for an empire.
    pub decadence_empire_mult: f32,
    /// A realm must be this old before it can grow decadent.
    pub decadence_min_age: i32,

    // -- rulers ------------------------------------------------------------
    /// Yearly chance a ruler dies of something other than old age.
    pub ruler_death_base: f32,
    /// How steeply the chance of dying rises with age.
    pub ruler_death_age_weight: f32,
    /// Base yearly chance a ruler is murdered.
    pub ruler_assassination_base: f32,
    /// How much a cruel ruler multiplies that chance.
    pub ruler_assassination_cruelty_weight: f32,
    /// How much an unstable realm adds to it.
    pub ruler_assassination_unrest_weight: f32,
    /// Base chance a succession passes off quietly.
    pub succession_smooth_base: f64,
    /// How much a stable realm adds to that chance.
    pub succession_smooth_stability_weight: f64,
    /// Yearly chance a realm throws up a notable person.
    pub notable_chance: f64,
    /// Base yearly chance a notable dies.
    pub notable_death_base: f32,
    /// How steeply that rises with age.
    pub notable_death_age_weight: f32,
    /// Yearly chance an over-ambitious general seizes a failing throne.
    pub general_usurp_chance: f64,

    // -- revolts and fragmentation -----------------------------------------
    /// Stability below which a realm risks revolt.
    pub revolt_stability_threshold: f32,
    /// How sharply the chance of revolt rises below that threshold.
    pub revolt_chance_factor: f64,
    /// Multiplier on the chance of revolt for an empire.
    pub revolt_empire_mult: f64,
    /// A realm must hold more cells than this to be worth revolting against.
    pub revolt_min_cells: i32,
    /// Years of quiet required after a revolt before the next one.
    pub revolt_cooldown: i32,
    /// Smallest share of a realm that breaks away in a revolt.
    pub split_min_share: f32,
    /// Largest such share.
    pub split_max_share: f32,
    /// Stability below which a great realm shatters outright.
    pub fragment_stability_threshold: f32,
    /// A realm must hold more cells than this to shatter rather than fall.
    pub fragment_min_cells: i32,
    /// Yearly chance a realm in that state shatters.
    pub fragment_chance: f64,
    /// Years of quiet required before a realm can shatter.
    pub fragment_cooldown: i32,
    /// Cells a kingdom needs before it can call itself an empire.
    pub empire_min_cells: i32,
    /// The settled world divided by this is a *cap* on that threshold, not a
    /// floor: where nobody could ever reach [`Tuning::empire_min_cells`] the
    /// bar comes down to whoever is genuinely dominant, so the
    /// chiefdom-kingdom-empire ladder stays climbable at every map size. A
    /// third of `empire_min_cells` is the floor, so an almost empty young
    /// world does not crown empires over a handful of villages.
    pub empire_share_divisor: i32,

    // -- diplomacy and war -------------------------------------------------
    /// Base yearly growth of tension along a border.
    pub tension_base_growth: f32,
    /// Added when the neighbour is of another culture.
    pub tension_foreign_culture: f32,
    /// Added again when it is of another race.
    pub tension_foreign_race: f32,
    /// Added when the neighbour rules our people.
    pub tension_claims: f32,
    /// How much rival faiths inflame a border.
    pub tension_faith_weight: f32,
    /// How much shared trade calms it.
    pub tension_trade_relief: f32,
    /// How much a weak neighbour tempts an ambitious ruler.
    pub tension_weak_neighbor: f32,
    /// Tension shed each year regardless.
    pub tension_decay: f32,
    /// Yearly decay applied to stale tension.
    pub tension_decay_mult: f32,
    /// Tension at which a realm will consider declaring war.
    pub war_declare_threshold: f32,
    /// Base yearly chance it goes through with it.
    pub war_declare_chance: f64,
    /// How much an ambitious ruler adds to that chance.
    pub war_declare_ambition_weight: f64,
    /// How large an army a realm wants relative to its target's before attacking.
    pub war_boldness_base: f32,
    /// Shortest truce after a peace, in years.
    pub truce_years_min: i32,
    /// Extra random years added to a truce.
    pub truce_years_random: i32,
    /// How far one battle moves the score of a war.
    pub battle_swing_base: f32,
    /// How much a lopsided battle adds to that swing.
    pub battle_swing_margin_weight: f32,
    /// Share of the loser's army destroyed in a battle.
    pub battle_loser_casualties: f32,
    /// Share of the winner's army destroyed.
    pub battle_winner_casualties: f32,
    /// Chance a besieging army strong enough to storm a city does so.
    pub siege_capture_chance: f64,
    /// Base chance a captured city is sacked.
    pub city_sack_chance: f64,
    /// How much a cruel conqueror raises that chance.
    pub city_sack_cruelty_weight: f64,
    /// Chance a ruler who leads from the front falls with a lost battle.
    pub ruler_battle_death_loser: f64,
    /// Chance one falls in a battle they win.
    pub ruler_battle_death_winner: f64,
    /// Base yearly chance a war ends in peace.
    pub peace_base_chance: f64,
    /// How much shared exhaustion raises that chance.
    pub peace_exhaustion_weight: f64,
    /// A war runs at least this many years before peace is possible.
    pub peace_min_years: i32,

    // -- schools of thought ------------------------------------------------
    /// Yearly chance per city that high mana raises a new school.
    pub school_found_mana_rate: f64,
    /// Yearly chance that an open, curious city does so anyway.
    pub school_found_open_rate: f64,
    /// Base yearly gain in a school's influence over a realm.
    pub school_influence_growth: f32,
    /// Extra influence gained where the school is the state doctrine.
    pub school_state_bonus: f32,
    /// Influence shed each year regardless.
    pub school_influence_decay: f32,
    /// Base yearly influence carried to a neighbouring realm.
    pub school_spread_rate: f32,
    /// Yearly chance a realm without a doctrine adopts a popular school.
    pub school_adopt_chance: f64,
    /// Yearly chance a realm forsakes its doctrine for a more popular one.
    pub school_convert_chance: f64,
    /// Yearly chance a hostile state persecutes a rival school.
    pub school_persecution_chance: f64,
    /// Yearly chance an old, widespread school splits in schism.
    pub school_schism_chance: f64,
    /// A school must be this old before it can split.
    pub school_schism_min_age: i32,
    /// A school needs this many realms following it before a schism is worth
    /// the name. Without a floor, every new splinter is immediately large
    /// enough to splinter again and the world fills with sects.
    pub school_schism_min_adherents: usize,
    /// Chance an adherent realm follows the dissenters in a schism.
    pub school_schism_share: f64,
    /// Influence below which a school is considered to be fading.
    pub school_extinction_threshold: f32,
    /// Years a school may fade before its teachings are forgotten.
    pub school_fading_years: i32,
    /// Highest influence anywhere at which a school counts as waning, and so
    /// may be taken into a larger school of its own kind.
    pub school_absorb_threshold: f32,
    /// Years a school must have been waning before it can be absorbed.
    pub school_absorb_years: i32,
    /// Influence a larger school of the same kind needs over the waning
    /// school's home realm before it can absorb it.
    pub school_absorb_dominance: f32,
    /// Yearly chance an arcane state doctrine unmakes the city that raised it.
    pub arcane_catastrophe_chance: f64,

    // -- disasters ---------------------------------------------------------
    /// Yearly chance a plague breaks out, scaled by the number of great cities.
    pub plague_chance: f64,
    /// Share of a stricken cell's people carried off each year.
    pub plague_cell_deaths: f32,
    /// Share of a stricken city's people carried off each year.
    pub plague_city_deaths: f32,
    /// Yearly chance a plague crosses into a neighbouring realm.
    pub plague_spread_chance: f64,
    /// Base yearly chance of famine, scaled by how poor the land is.
    pub famine_chance: f64,
    /// Share of the countryside that survives a famine.
    pub famine_cell_survival: f32,
    /// Share of a city that survives a famine.
    pub famine_city_survival: f32,
    /// Yearly chance a city suffers a fire, flood or earthquake.
    pub city_disaster_chance: f64,
    /// Share of a city that survives such a disaster.
    pub city_disaster_survival: f32,
    /// Yearly chance of an omen in the sky.
    pub omen_chance: f64,
    /// Yearly chance a rich, peaceful realm begins a wonder.
    pub wonder_chance: f64,
    /// Treasury a realm needs before it will build one.
    pub wonder_min_treasury: f32,
    /// What a wonder costs.
    pub wonder_cost: f32,

    // -- stories -----------------------------------------------------------
    /// Shortest deadline a prophecy is given, in years.
    pub prophecy_deadline_min: i32,
    /// Extra random years a prophecy may be given.
    pub prophecy_deadline_range: i32,
    /// Yearly chance a lost relic is dug up again.
    pub artifact_found_chance: f64,
    /// Yearly chance a held relic is stolen or lost.
    pub artifact_lost_chance: f64,
    /// Relics one realm may hold before it makes no more.
    pub artifact_per_polity_cap: usize,
    /// Relics in the world before any realm makes more, before the count of realms.
    pub artifact_world_cap_base: usize,
    /// How many living realms the world's relic cap grows by one for.
    pub artifact_world_cap_divisor: usize,
    /// Chance a great ruler's death is marked with a relic.
    pub artifact_ruler_chance: f64,
    /// Chance a new wonder is crowned with one.
    pub artifact_wonder_chance: f64,
    /// Chance a new school is given one.
    pub artifact_school_chance: f64,
    /// Chance a relic simply vanishes in the sack of a city.
    pub artifact_capture_lost_chance: f64,
    /// Chance a fallen realm's relics pass to its conqueror rather than being lost.
    pub artifact_fall_pass_chance: f64,
    /// Yearly chance a long-reigning cruel ruler is named a tyrant.
    pub tyrant_chance: f64,

    // -- houses and figures ------------------------------------------------
    /// Yearly chance an unwed ruler finds a match.
    pub marriage_chance: f64,
    /// Yearly chance a married ruler of childbearing age has a child.
    pub birth_chance: f64,
    /// Smallest realm a faith will crown an emperor of.
    pub coronation_min_cells: usize,
    /// Yearly chance a qualifying ruler is crowned by their faith.
    pub coronation_chance: f64,
    /// Yearly chance a ruler's child dies before reaching majority.
    pub child_death_chance: f64,
    /// How many generals a dead conqueror needs before they divide the
    /// realm between themselves rather than let it pass to an heir.
    pub diadochi_min_generals: usize,
    /// Smallest realm whose generals are worth dividing it over.
    pub diadochi_min_cells: usize,

    // -- alliances, tribute and coalitions ---------------------------------
    /// Yearly chance two realms with a quiet border and a common enemy ally.
    pub alliance_chance: f64,
    /// Yearly chance an ally answers a call to arms.
    pub ally_joins_chance: f64,
    /// Share of a tributary's income that goes to its overlord.
    pub tribute_share: f32,
    /// Yearly chance a tributary tests its overlord's grip.
    pub tributary_revolt_chance: f64,
    /// Share of the settled world above which a realm is a hegemon, and its
    /// neighbours start to combine against it rather than each other.
    pub hegemon_share: f32,
    /// How much a hegemon's share adds to everyone else's tension with it.
    pub containment_tension: f32,

    // -- what the world learns ---------------------------------------------
    /// Yearly chance a city of ordinary size and prosperity works something
    /// out, before its people, its faith and its wealth are counted.
    pub tech_discover_chance: f64,
    /// How much dearer each innovation the *world* already knows makes the
    /// next one. This is what paces a tree across millennia instead of
    /// centuries: the frontier recedes as it is approached. Global rather
    /// than per-realm, because discovery is rolled at every city and it is
    /// the world's rate that has to be held down, not one realm's.
    pub tech_frontier_drag: f64,
    /// How much a city must be able to support, per point of an
    /// innovation's difficulty, before it can work that innovation out.
    pub tech_effort: f32,
    /// Chance per year that a cell takes up something a neighbour knows,
    /// before the innovation's own readiness to travel is counted.
    pub tech_spread_chance: f64,
    /// Chance that emptied ground forgets what it knew, which is what makes
    /// a dark age possible and a rediscovery worth reading.
    pub tech_forget_chance: f64,

    // -- housekeeping ------------------------------------------------------
    /// How many events the chronicle keeps before the oldest small ones are
    /// dropped. Great events (importance 2 and 3) are always kept, so the
    /// real length can sit above this. 0 keeps everything for ever.
    pub chronicle_cap: usize,
}

impl Default for Tuning {
    fn default() -> Tuning {
        Tuning {
            // population and migration
            pop_growth_rate: 0.04,
            pop_growth_fecund_base: 0.6,
            pop_growth_fecund_weight: 0.8,
            pop_starve_rate: 0.08,
            pop_overshoot_rate: 0.15,
            cell_capacity_factor: 5.0,
            cell_capacity_dev_weight: 1.1,
            migration_pressure: 0.55,
            migration_min_pop: 0.25,
            migration_share: 0.08,
            migration_sea_chance: 0.35,
            migration_fill_ratio: 0.7,
            city_growth_rate: 0.03,
            city_capacity_factor: 1.6,
            city_capacity_dev_weight: 1.7,
            city_overshoot_rate: 0.1,
            assimilation_rate: 0.004,
            culture_shift_chance: 0.03,
            divergence_chance: 0.15,
            divergence_foreign_ruler: 0.2,
            // expansion
            polity_form_chance: 0.2,
            polity_form_min_pop: 0.7,
            polity_form_radius_x: 5,
            polity_form_radius_y: 3,
            expand_budget_base: 0.7,
            expand_budget_pop_factor: 0.19,
            expand_ambition_base: 0.5,
            expand_stability_base: 0.4,
            expand_war_penalty: 0.5,
            expand_max_claims: 6,
            expand_terrain_cost_weight: 0.35,
            expand_distance_penalty: 1.2,
            expand_distance_cost_weight: 0.5,
            expand_reach_base: 9.0,
            expand_reach_dev_weight: 19.0,
            expand_fertility_weight: 1.4,
            // cities
            city_found_chance: 0.2,
            city_cells_per_city: 16,
            city_spacing_x: 5,
            city_spacing_y: 3,
            city_site_min_score: 0.6,
            city_site_pop_weight: 0.75,
            // economy
            city_income_factor: 0.08,
            cell_income_factor: 0.012,
            army_upkeep_factor: 0.05,
            cell_upkeep_factor: 0.004,
            army_pop_factor: 0.015,
            army_militarism_weight: 0.04,
            army_build_rate_peace: 0.12,
            army_build_rate_war: 0.25,
            dev_growth_rate: 0.0022,
            dev_soft_cap: 3.0,
            dev_hard_cap: 5.0,
            dev_overflow_rate: 0.3,
            prosperity_adjust_rate: 0.08,
            // stability and decadence
            admin_capacity_base: 16.0,
            admin_capacity_dev_weight: 45.0,
            admin_capacity_city_weight: 8.0,
            stability_base: 0.5,
            stability_wisdom_weight: 0.2,
            stability_charisma_weight: 0.1,
            stability_overextension_weight: 0.25,
            hegemony_free_share: 0.22,
            hegemony_weight: 1.35,
            admin_reach_base: 7.0,
            admin_reach_dev_weight: 4.5,
            stability_sprawl_weight: 0.28,
            stability_foreign_weight: 0.2,
            stability_exhaustion_weight: 0.3,
            stability_decadence_weight: 0.44,
            stability_adjust_rate: 0.1,
            exhaustion_war_growth: 0.035,
            exhaustion_peace_decay: 0.03,
            decadence_growth: 0.006,
            decadence_wisdom_relief: 0.002,
            decadence_empire_mult: 1.6,
            decadence_min_age: 50,
            // rulers
            ruler_death_base: 0.0015,
            ruler_death_age_weight: 0.09,
            ruler_assassination_base: 0.002,
            ruler_assassination_cruelty_weight: 3.0,
            ruler_assassination_unrest_weight: 0.02,
            succession_smooth_base: 0.6,
            succession_smooth_stability_weight: 0.35,
            notable_chance: 0.008,
            notable_death_base: 0.004,
            notable_death_age_weight: 0.08,
            general_usurp_chance: 0.06,
            // revolts and fragmentation
            revolt_stability_threshold: 0.35,
            revolt_chance_factor: 0.4,
            revolt_empire_mult: 1.4,
            revolt_min_cells: 10,
            revolt_cooldown: 8,
            split_min_share: 0.12,
            split_max_share: 0.35,
            fragment_stability_threshold: 0.2,
            fragment_min_cells: 45,
            fragment_chance: 0.35,
            fragment_cooldown: 5,
            empire_min_cells: 180,
            empire_share_divisor: 8,
            // diplomacy and war
            tension_base_growth: 0.008,
            tension_foreign_culture: 0.02,
            tension_foreign_race: 0.01,
            tension_claims: 0.02,
            tension_faith_weight: 0.025,
            tension_trade_relief: 0.02,
            tension_weak_neighbor: 0.02,
            tension_decay: 0.015,
            tension_decay_mult: 0.97,
            war_declare_threshold: 0.65,
            war_declare_chance: 0.06,
            war_declare_ambition_weight: 0.15,
            war_boldness_base: 0.7,
            truce_years_min: 14,
            truce_years_random: 12,
            battle_swing_base: 0.07,
            battle_swing_margin_weight: 0.2,
            battle_loser_casualties: 0.08,
            battle_winner_casualties: 0.04,
            siege_capture_chance: 0.62,
            city_sack_chance: 0.3,
            city_sack_cruelty_weight: 0.5,
            ruler_battle_death_loser: 0.035,
            ruler_battle_death_winner: 0.008,
            peace_base_chance: 0.03,
            peace_exhaustion_weight: 0.15,
            peace_min_years: 3,
            // schools of thought
            school_found_mana_rate: 0.00035,
            school_found_open_rate: 0.00008,
            school_influence_growth: 0.007,
            school_state_bonus: 0.012,
            school_influence_decay: 0.009,
            school_spread_rate: 0.002,
            school_adopt_chance: 0.05,
            school_convert_chance: 0.05,
            school_persecution_chance: 0.06,
            school_schism_chance: 0.006,
            school_schism_min_age: 60,
            school_schism_min_adherents: 4,
            school_schism_share: 0.45,
            school_extinction_threshold: 0.04,
            school_fading_years: 15,
            school_absorb_threshold: 0.85,
            school_absorb_years: 10,
            school_absorb_dominance: 0.15,
            arcane_catastrophe_chance: 0.0005,
            // disasters
            plague_chance: 0.0035,
            plague_cell_deaths: 0.12,
            plague_city_deaths: 0.15,
            plague_spread_chance: 0.25,
            famine_chance: 0.006,
            famine_cell_survival: 0.88,
            famine_city_survival: 0.9,
            city_disaster_chance: 0.0015,
            city_disaster_survival: 0.8,
            omen_chance: 0.006,
            wonder_chance: 0.03,
            wonder_min_treasury: 120.0,
            wonder_cost: 100.0,
            // stories
            prophecy_deadline_min: 90,
            prophecy_deadline_range: 200,
            artifact_found_chance: 0.004,
            artifact_lost_chance: 0.0015,
            artifact_per_polity_cap: 2,
            artifact_world_cap_base: 3,
            artifact_world_cap_divisor: 2,
            artifact_ruler_chance: 0.25,
            artifact_wonder_chance: 0.3,
            artifact_school_chance: 0.25,
            artifact_capture_lost_chance: 0.25,
            artifact_fall_pass_chance: 0.7,
            tyrant_chance: 0.06,
            marriage_chance: 0.22,
            birth_chance: 0.16,
            coronation_min_cells: 60,
            coronation_chance: 0.05,
            child_death_chance: 0.012,
            diadochi_min_generals: 2,
            diadochi_min_cells: 90,
            alliance_chance: 0.035,
            ally_joins_chance: 0.55,
            tribute_share: 0.2,
            tributary_revolt_chance: 0.02,
            hegemon_share: 0.2,
            containment_tension: 0.05,
            tech_discover_chance: 0.0045,
            tech_frontier_drag: 0.5,
            tech_effort: 14.0,
            tech_spread_chance: 0.02,
            tech_forget_chance: 0.03,
            chronicle_cap: 60_000,
        }
    }
}

impl Tuning {
    /// Set one field by name, as the config file's `tune.<field> = <value>`
    /// lines do. Returns an error message for an unknown field.
    /// Set one field by name, as `tune.<name> = <v>` in the config file does.
    ///
    /// # Errors
    ///
    /// [`UnknownField`] if no field goes by that name.
    pub fn set(&mut self, name: &str, v: f64) -> Result<(), UnknownField> {
        match name {
            // population and migration
            "pop_growth_rate" => self.pop_growth_rate = v as f32,
            "pop_growth_fecund_base" => self.pop_growth_fecund_base = v as f32,
            "pop_growth_fecund_weight" => self.pop_growth_fecund_weight = v as f32,
            "pop_starve_rate" => self.pop_starve_rate = v as f32,
            "pop_overshoot_rate" => self.pop_overshoot_rate = v as f32,
            "cell_capacity_factor" => self.cell_capacity_factor = v as f32,
            "cell_capacity_dev_weight" => self.cell_capacity_dev_weight = v as f32,
            "migration_pressure" => self.migration_pressure = v as f32,
            "migration_min_pop" => self.migration_min_pop = v as f32,
            "migration_share" => self.migration_share = v as f32,
            "migration_fill_ratio" => self.migration_fill_ratio = v as f32,
            "migration_sea_chance" => self.migration_sea_chance = v,
            "city_growth_rate" => self.city_growth_rate = v as f32,
            "city_capacity_factor" => self.city_capacity_factor = v as f32,
            "city_capacity_dev_weight" => self.city_capacity_dev_weight = v as f32,
            "city_overshoot_rate" => self.city_overshoot_rate = v as f32,
            "assimilation_rate" => self.assimilation_rate = v as f32,
            "culture_shift_chance" => self.culture_shift_chance = v,
            "divergence_chance" => self.divergence_chance = v,
            "divergence_foreign_ruler" => self.divergence_foreign_ruler = v,
            // expansion
            "polity_form_chance" => self.polity_form_chance = v,
            "polity_form_min_pop" => self.polity_form_min_pop = v as f32,
            "polity_form_radius_x" => self.polity_form_radius_x = v as i32,
            "polity_form_radius_y" => self.polity_form_radius_y = v as i32,
            "expand_budget_base" => self.expand_budget_base = v as f32,
            "expand_budget_pop_factor" => self.expand_budget_pop_factor = v as f32,
            "expand_ambition_base" => self.expand_ambition_base = v as f32,
            "expand_stability_base" => self.expand_stability_base = v as f32,
            "expand_war_penalty" => self.expand_war_penalty = v as f32,
            "expand_max_claims" => self.expand_max_claims = v as i32,
            "expand_terrain_cost_weight" => self.expand_terrain_cost_weight = v as f32,
            "expand_distance_penalty" => self.expand_distance_penalty = v as f32,
            "expand_distance_cost_weight" => self.expand_distance_cost_weight = v as f32,
            "expand_reach_base" => self.expand_reach_base = v as f32,
            "expand_reach_dev_weight" => self.expand_reach_dev_weight = v as f32,
            "expand_fertility_weight" => self.expand_fertility_weight = v as f32,
            // cities
            "city_found_chance" => self.city_found_chance = v,
            "city_cells_per_city" => self.city_cells_per_city = v as i32,
            "city_spacing_x" => self.city_spacing_x = v as i32,
            "city_spacing_y" => self.city_spacing_y = v as i32,
            "city_site_min_score" => self.city_site_min_score = v as f32,
            "city_site_pop_weight" => self.city_site_pop_weight = v as f32,
            // economy
            "city_income_factor" => self.city_income_factor = v as f32,
            "cell_income_factor" => self.cell_income_factor = v as f32,
            "army_upkeep_factor" => self.army_upkeep_factor = v as f32,
            "cell_upkeep_factor" => self.cell_upkeep_factor = v as f32,
            "army_pop_factor" => self.army_pop_factor = v as f32,
            "army_militarism_weight" => self.army_militarism_weight = v as f32,
            "army_build_rate_peace" => self.army_build_rate_peace = v as f32,
            "army_build_rate_war" => self.army_build_rate_war = v as f32,
            "dev_growth_rate" => self.dev_growth_rate = v as f32,
            "dev_soft_cap" => self.dev_soft_cap = v as f32,
            "dev_hard_cap" => self.dev_hard_cap = v as f32,
            "dev_overflow_rate" => self.dev_overflow_rate = v as f32,
            "prosperity_adjust_rate" => self.prosperity_adjust_rate = v as f32,
            // stability and decadence
            "admin_capacity_base" => self.admin_capacity_base = v as f32,
            "admin_capacity_dev_weight" => self.admin_capacity_dev_weight = v as f32,
            "admin_capacity_city_weight" => self.admin_capacity_city_weight = v as f32,
            "stability_base" => self.stability_base = v as f32,
            "stability_wisdom_weight" => self.stability_wisdom_weight = v as f32,
            "stability_charisma_weight" => self.stability_charisma_weight = v as f32,
            "stability_overextension_weight" => self.stability_overextension_weight = v as f32,
            "hegemony_free_share" => self.hegemony_free_share = v as f32,
            "hegemony_weight" => self.hegemony_weight = v as f32,
            "admin_reach_base" => self.admin_reach_base = v as f32,
            "admin_reach_dev_weight" => self.admin_reach_dev_weight = v as f32,
            "stability_sprawl_weight" => self.stability_sprawl_weight = v as f32,
            "stability_foreign_weight" => self.stability_foreign_weight = v as f32,
            "stability_exhaustion_weight" => self.stability_exhaustion_weight = v as f32,
            "stability_decadence_weight" => self.stability_decadence_weight = v as f32,
            "stability_adjust_rate" => self.stability_adjust_rate = v as f32,
            "exhaustion_war_growth" => self.exhaustion_war_growth = v as f32,
            "exhaustion_peace_decay" => self.exhaustion_peace_decay = v as f32,
            "decadence_growth" => self.decadence_growth = v as f32,
            "decadence_wisdom_relief" => self.decadence_wisdom_relief = v as f32,
            "decadence_empire_mult" => self.decadence_empire_mult = v as f32,
            "decadence_min_age" => self.decadence_min_age = v as i32,
            // rulers
            "ruler_death_base" => self.ruler_death_base = v as f32,
            "ruler_death_age_weight" => self.ruler_death_age_weight = v as f32,
            "ruler_assassination_base" => self.ruler_assassination_base = v as f32,
            "ruler_assassination_cruelty_weight" => {
                self.ruler_assassination_cruelty_weight = v as f32;
            }
            "ruler_assassination_unrest_weight" => {
                self.ruler_assassination_unrest_weight = v as f32;
            }
            "succession_smooth_base" => self.succession_smooth_base = v,
            "succession_smooth_stability_weight" => self.succession_smooth_stability_weight = v,
            "notable_chance" => self.notable_chance = v,
            "notable_death_base" => self.notable_death_base = v as f32,
            "notable_death_age_weight" => self.notable_death_age_weight = v as f32,
            "general_usurp_chance" => self.general_usurp_chance = v,
            // revolts and fragmentation
            "revolt_stability_threshold" => self.revolt_stability_threshold = v as f32,
            "revolt_chance_factor" => self.revolt_chance_factor = v,
            "revolt_empire_mult" => self.revolt_empire_mult = v,
            "revolt_min_cells" => self.revolt_min_cells = v as i32,
            "revolt_cooldown" => self.revolt_cooldown = v as i32,
            "split_min_share" => self.split_min_share = v as f32,
            "split_max_share" => self.split_max_share = v as f32,
            "fragment_stability_threshold" => self.fragment_stability_threshold = v as f32,
            "fragment_min_cells" => self.fragment_min_cells = v as i32,
            "fragment_chance" => self.fragment_chance = v,
            "fragment_cooldown" => self.fragment_cooldown = v as i32,
            "empire_min_cells" => self.empire_min_cells = v as i32,
            "empire_share_divisor" => self.empire_share_divisor = (v.max(1.0)) as i32,
            // diplomacy and war
            "tension_base_growth" => self.tension_base_growth = v as f32,
            "tension_foreign_culture" => self.tension_foreign_culture = v as f32,
            "tension_foreign_race" => self.tension_foreign_race = v as f32,
            "tension_claims" => self.tension_claims = v as f32,
            "tension_faith_weight" => self.tension_faith_weight = v as f32,
            "tension_trade_relief" => self.tension_trade_relief = v as f32,
            "tension_weak_neighbor" => self.tension_weak_neighbor = v as f32,
            "tension_decay" => self.tension_decay = v as f32,
            "tension_decay_mult" => self.tension_decay_mult = v as f32,
            "war_declare_threshold" => self.war_declare_threshold = v as f32,
            "war_declare_chance" => self.war_declare_chance = v,
            "war_declare_ambition_weight" => self.war_declare_ambition_weight = v,
            "war_boldness_base" => self.war_boldness_base = v as f32,
            "truce_years_min" => self.truce_years_min = v as i32,
            "truce_years_random" => self.truce_years_random = v as i32,
            "battle_swing_base" => self.battle_swing_base = v as f32,
            "battle_swing_margin_weight" => self.battle_swing_margin_weight = v as f32,
            "battle_loser_casualties" => self.battle_loser_casualties = v as f32,
            "battle_winner_casualties" => self.battle_winner_casualties = v as f32,
            "siege_capture_chance" => self.siege_capture_chance = v,
            "city_sack_chance" => self.city_sack_chance = v,
            "city_sack_cruelty_weight" => self.city_sack_cruelty_weight = v,
            "ruler_battle_death_loser" => self.ruler_battle_death_loser = v,
            "ruler_battle_death_winner" => self.ruler_battle_death_winner = v,
            "peace_base_chance" => self.peace_base_chance = v,
            "peace_exhaustion_weight" => self.peace_exhaustion_weight = v,
            "peace_min_years" => self.peace_min_years = v as i32,
            // schools of thought
            "school_found_mana_rate" => self.school_found_mana_rate = v,
            "school_found_open_rate" => self.school_found_open_rate = v,
            "school_influence_growth" => self.school_influence_growth = v as f32,
            "school_state_bonus" => self.school_state_bonus = v as f32,
            "school_influence_decay" => self.school_influence_decay = v as f32,
            "school_spread_rate" => self.school_spread_rate = v as f32,
            "school_adopt_chance" => self.school_adopt_chance = v,
            "school_convert_chance" => self.school_convert_chance = v,
            "school_persecution_chance" => self.school_persecution_chance = v,
            "school_schism_chance" => self.school_schism_chance = v,
            "school_schism_min_age" => self.school_schism_min_age = v as i32,
            "school_schism_min_adherents" => {
                self.school_schism_min_adherents = v.max(2.0) as usize;
            }
            "school_schism_share" => self.school_schism_share = v,
            "school_extinction_threshold" => self.school_extinction_threshold = v as f32,
            "school_fading_years" => self.school_fading_years = v as i32,
            "school_absorb_threshold" => self.school_absorb_threshold = v as f32,
            "school_absorb_years" => self.school_absorb_years = v as i32,
            "school_absorb_dominance" => self.school_absorb_dominance = v as f32,
            "arcane_catastrophe_chance" => self.arcane_catastrophe_chance = v,
            // disasters
            "plague_chance" => self.plague_chance = v,
            "plague_cell_deaths" => self.plague_cell_deaths = v as f32,
            "plague_city_deaths" => self.plague_city_deaths = v as f32,
            "plague_spread_chance" => self.plague_spread_chance = v,
            "famine_chance" => self.famine_chance = v,
            "famine_cell_survival" => self.famine_cell_survival = v as f32,
            "famine_city_survival" => self.famine_city_survival = v as f32,
            "city_disaster_chance" => self.city_disaster_chance = v,
            "city_disaster_survival" => self.city_disaster_survival = v as f32,
            "omen_chance" => self.omen_chance = v,
            "wonder_chance" => self.wonder_chance = v,
            "wonder_min_treasury" => self.wonder_min_treasury = v as f32,
            "wonder_cost" => self.wonder_cost = v as f32,
            // stories
            "prophecy_deadline_min" => self.prophecy_deadline_min = v as i32,
            "prophecy_deadline_range" => self.prophecy_deadline_range = v as i32,
            "artifact_found_chance" => self.artifact_found_chance = v,
            "artifact_lost_chance" => self.artifact_lost_chance = v,
            "artifact_per_polity_cap" => self.artifact_per_polity_cap = v.max(0.0) as usize,
            "artifact_world_cap_base" => self.artifact_world_cap_base = v.max(0.0) as usize,
            "artifact_world_cap_divisor" => self.artifact_world_cap_divisor = (v.max(1.0)) as usize,
            "artifact_ruler_chance" => self.artifact_ruler_chance = v,
            "artifact_wonder_chance" => self.artifact_wonder_chance = v,
            "artifact_school_chance" => self.artifact_school_chance = v,
            "artifact_capture_lost_chance" => self.artifact_capture_lost_chance = v,
            "artifact_fall_pass_chance" => self.artifact_fall_pass_chance = v,
            "tyrant_chance" => self.tyrant_chance = v,
            "marriage_chance" => self.marriage_chance = v,
            "birth_chance" => self.birth_chance = v,
            "coronation_min_cells" => self.coronation_min_cells = v.max(0.0) as usize,
            "coronation_chance" => self.coronation_chance = v,
            "child_death_chance" => self.child_death_chance = v,
            "diadochi_min_generals" => self.diadochi_min_generals = v.max(0.0) as usize,
            "diadochi_min_cells" => self.diadochi_min_cells = v.max(0.0) as usize,
            "alliance_chance" => self.alliance_chance = v,
            "ally_joins_chance" => self.ally_joins_chance = v,
            "tribute_share" => self.tribute_share = v as f32,
            "tributary_revolt_chance" => self.tributary_revolt_chance = v,
            "hegemon_share" => self.hegemon_share = v as f32,
            "containment_tension" => self.containment_tension = v as f32,
            "tech_discover_chance" => self.tech_discover_chance = v,
            "tech_frontier_drag" => self.tech_frontier_drag = v,
            "tech_effort" => self.tech_effort = v as f32,
            "tech_spread_chance" => self.tech_spread_chance = v,
            "tech_forget_chance" => self.tech_forget_chance = v,
            "chronicle_cap" => self.chronicle_cap = v.max(0.0) as usize,
            _ => return Err(UnknownField(name.to_string())),
        }
        Ok(())
    }
}
