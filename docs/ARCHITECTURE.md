# Architecture

How the game is put together, for anyone about to change it. Each module also
opens with a `//!` block that says more than this page does; treat those as the
authority and this as the map.

The whole thing is one binary crate with no dependencies. The only outside
world it touches is libc, through hand-written FFI declarations in
`src/term.rs`, and the files it reads and writes under `~/.config` and
`~/.local/share`.

## Module map

```
main.rs          argument parsing, then one of: the UI, a headless run,
                 --stats, --bench, --snapshot, --mkconfig, --version

noise.rs         2D gradient noise and fBm: terrain, climate, mana
rng.rs           xoshiro256** with interior mutability; clones share a stream
lang.rs          per-culture phonology; every name in the world is coined here,
                 daughter tongues by sound shift
geo.rs           elevation -> climate -> rivers -> biomes -> mana -> features,
                 and the sea routes between landmasses. Generated once,
                 read afterwards

sim/
  mod.rs         the World: entities, World::tick, recompute, the owner index,
                 and the helpers every subsystem shares
  tuning.rs      every balance-relevant number, in one struct
  genesis.rs     races, first peoples, the opening lines
  people.rs      growth, migration, assimilation, cultural drift
  politics.rs    tribes, expansion, cities, economy, rulers, unrest, collapse
  dynasty.rs     houses, marriage, heirs, mortality, greatness and acclaim,
                 inheritance customs, crowns given by a faith
  blood.rs       two alleles a trait, recessives, kinship and inbreeding
  circles.rs     rivalry and patronage between people who hold no throne
  tech.rs        a tree of innovations grown per world; discovery, diffusion
                 and forgetting. Knowledge is held by cells, not realms
  trade.rs       goods from terrain, routes between cities, what closes them
  climate.rs     bands of wet and dry drifting over centuries
  war.rs         tension, stances, alliances, tribute, war aims, declarations,
                 coalition battles, sieges, peace
  magic.rs       schools of thought: founding, spread, adoption, schism
  events.rs      disasters, notable people, wonders, the naming of ages
  stories.rs     prophecies, relics, tyrants, legends
  chronicle.rs   the event store and its per-entity index
  prose/         every sentence the chronicle prints
  explain.rs     why things are as they are — a pure reading of &World

ser.rs           save files: one symmetric Io trait, chunked format
config.rs        ~/.config/empires/config, settings and key remaps
theme.rs         colour themes as a post-pass over a rendered frame
term.rs          raw mode, key input, bracketed paste, a diff-rendered cell
                 buffer emitting 24-bit ANSI
stats.rs         --stats balance metrics and the --bench profiler

ui/
  mod.rs         the Ui struct, modes, the frame loop, save-directory rules
  input.rs       counts, pending prefixes, per-mode key handlers, prompts, mouse
  commands.rs    the `:` command line
  detail.rs      lists, detail pages, the HELP table, the Hand of Fate
  recap.rs       the digest of the last N years
  words.rs       numbers as plain words
  render/        map, sidebar, panels, and the frame that composes them
```

The dependency direction is one-way: `geo` and `lang` know nothing about
`sim`; `sim` knows nothing about `ui` or `term`; `ui` reads the world and never
changes it except through the few `&mut World` entry points (`tick`, loading, the
Hand of Fate). `ser` is the one module that sees the whole `World` at once.

## The data model

`World` holds a `Vec` per kind of entity — `cells`, `races`, `cultures`,
`cities`, `polities`, `persons`, `houses`, `schools`, `wars`, `eras`,
`plagues`, `artifacts`, `prophecies` — and entities refer to one another by
index into those vectors. A ruler is a `usize` into `persons`; a war names two `usize`
polities; a chronicle line carries `Ref::Polity(7)`.

**Those vectors are append-only.** Nothing is ever removed or reordered,
because the indices are stored in save files, in every chronicle event and in
the UI's selection and history. A realm that falls is marked dead and stays
where it is. This is the single most important invariant in the tree: an
`Vec::remove`, a `retain`, or a sort in the wrong place silently rewrites
centuries of history and every save ever written.

Four derived structures are maintained rather than recomputed. All three of
the index vectors exist for the same reason: the entity vectors hold
everything that has *ever* existed, so a phase that filters one for the
living costs the whole history of the world every year.

- **`owner_cells`** — each polity's own cells, ascending, kept exact by
  `claim` and `fall` and rebuilt by `recompute`. `cells_of_ref` reads it.
  It exists so that no phase has to walk the whole map once per realm; see
  *Cost* below.
- **`alive_polities`** — realms still standing, ascending. Appended by
  `politics::found_polity`, pruned by `politics::fall` and `recompute`.
  `living_polities` clones it, and several phases call that every year.
- **`alive_persons`** — people still living, ascending. Appended by
  `new_person`, pruned by `World::forget_the_dead`. `dynasty::mortality` and
  the yearly scoring of standing walk it.
- **`cell_yield`** — what each cell's knowledge is worth to its harvests.
  `cell_capacity` is called for every cell every year, and working the
  multiplier out from the knowledge bitset meant walking the whole tree each
  time. Refreshed where knowledge changes, which is rare.
- **The chronicle index** — for each `Ref`, the events that mention it, which
  is what lets any detail page show its own history without a scan.

Three more are derived and rebuilt rather than kept: `Terrain::crossings`
(where the sea can be crossed), `World::goods` (what the ground yields) and
`World::climate` (the drifting weather). All three are pure functions of
things that do not change — the terrain, and for the weather the seed and the
epoch — so a section for them in the save could only ever disagree with what
it described. `ser::load` rebuilds them before `recompute`.

`Terrain` (in `geo.rs`) is the immutable half of a cell: elevation,
temperature, rainfall, river, biome, mana, minerals, features. `CellState` (in
`sim`) is the mutable half: population, culture, owner, development. They are
parallel arrays indexed by `y * w + x`.

## The ratchet

Everything else in the simulation cycles: realms rise and break, faiths
spread and fade, and a map at year fifteen hundred is arranged differently
from one at year three hundred without being different *in kind*. Nothing
accumulated — `Polity::dev` was the only thing that went up, it belonged to
the realm rather than to the ground, and it died with it.

`World::known` is a `u128` per cell: a bitset over that world's own tree of
innovations. **Knowledge is held by the land.** A realm that falls leaves its
irrigation, its roads and its writing behind for whoever takes the ground,
and that is the only thing in the world which survives the state that built
it.

The tree itself is **grown from the seed**, the way languages and peoples
are, because a fixed table means every world learns the same things in the
same order — and twenty-four hand-written entries were known everywhere by
year eight hundred, after which the ratchet was inert again. What stays
universal is `tech::Effect`, the vocabulary of *consequences*, because the
simulation has to read a tree it did not write.

Three things gate discovery, and the third is the one that matters for
pacing: the ground must suit it, the prerequisites must be known, and a city
must be able to support it. What gates invention is not time but surplus.
Above that sits a drag that rises with what the *world* already knows —
global, because discovery is rolled at every city and it is the world's rate
that has to be held down.

Culture decides what a people can learn at all. `Culture::learning` gives a
bent per field with real strengths and real blind spots, gating both
discovery and adoption, which is why a technique can stall for ever at a
cultural border that a trade good crosses in a season — and why the world
does not converge on one body of knowledge.

Knowledge is also the thing most likely to unbalance the world, because
every brake on the size of a realm is *relative* and knowledge raises all
their ceilings at once. See `tuning.hegemony_weight`, which is absolute, and
the note on `admin_reach_base`.

## The sea

Because the terrain never changes, **where the sea can be crossed is a fixed
property of the map**. `geo::sea_routes` works it out once — every pair of
coastal cells on *different* landmasses within `MAX_CROSSING` of each other,
joined by a line whose interior is all water — and stores it as
`Terrain::crossings`, sorted so `crossings_from` is a binary search. It is
derived, so `ser` rebuilds it on load rather than storing it.

That one static graph is what makes every question about the sea a lookup:

- `World::sea_reach` is how wide a crossing a realm can manage, from its
  people's seafaring, its own development and what kind of thing it is. A
  realm that has just learned to build sea-going hulls can cross a strait;
  only an old, developed, seafaring power reaches the far isles.
- `politics::expand` walks the realm's own sorties rather than scanning a box
  around every coastal cell, so reach is a capability rather than a constant.
- `recompute` adds a **sea neighbour** for any crossing whose two shores are
  held by different realms and which somebody's reach can make. Without it
  the sea was not a distance but a wall: realms on opposite shores were not
  neighbours, so no tension built, so no war was ever declared.
- `war::resolve_wars` puts both shores of such a crossing on the front, so
  the war can be fought. `battle` notices when the ground could not have been
  walked to and charges `LANDING_PENALTY` for the assault.
- `people::grow_and_migrate` lets a crowded coastal community take a crossing
  within its *people's* own reach — narrow water only, since a people has no
  shipwrights of its own. Islands get peopled, which is what gives a state
  something to colonise or conquer later.
- `recompute` also discounts the distance to land across water by
  `SEA_BINDS_FACTOR × reach`, because for most of history the sea was the
  fast road. Without that the sea could be opened and would then be punished:
  every overseas holding would drive up `sprawl` and fray the realm that took
  it.

`the_sea_joins_the_world` asserts the property all of this exists for — that
every landmass worth settling can be reached from the largest one by a chain
of crossings, so a realm that masters the sea can in principle reach the whole
world.

## Goods and roads

`World::goods` is what each cell yields, read off the terrain and rebuilt on
load. Cities trade when each has something the other's hinterland lacks, by
land within a fortnight's cart or by sea within a month's sail.

The point is that **wealth becomes positional**: the busiest city takes about
four times the median, because it sits between places that want what each
other has. That makes a realm astride the roads worth attacking, and makes
cutting a road an act with consequences — a war closes the roads between the
realms fighting it and the wealth drains out of cities that never saw a
soldier, which is what lets a collapse propagate instead of staying local.

Routes also carry knowledge, in `tech::carried_by_trade`. The far end of a
route is as near as the near end, so a technique crosses a sea years before
it crosses the mountain range behind the port.

## The weather

`World::climate` is a band of wet and dry drifting over the map on a scale of
centuries, a pure function of the seed and the *epoch* — which is what lets
it be rebuilt at load rather than stored, including partway through an epoch.

It does exactly one thing: it moves the carrying capacity of land. Everything
else follows on its own, because the simulation already knows what to do when
land stops feeding people. A wet century pushes farming into the margins; a
dry one pushes the margins back onto the farmers, and the people who live
where the grass fails are the ones with horses.

## The tick pipeline

`World::tick` advances exactly one year and is the only thing that changes the
simulation. Every phase is wrapped in a `phase!` macro that charges its time to
the profiler when `--bench` is on, so the list below is also what the profile
prints.

| # | phase | what it does |
|---|---|---|
| 0 | `people::grow_and_migrate` | population toward carrying capacity; movement into emptier land |
| 1 | `politics::form_polities` | new tribes where people are dense and unruled |
| 2 | `politics::expand` | claim cells, priced by terrain and distance from the capital |
| 3 | `politics::found_cities` | new cities on rivers and coasts |
| 4 | `politics::economy` | treasury, development, and stability drifting toward its target |
| 5 | `war::diplomacy` | border tension rising and cooling; fear of the hegemon; declarations, each with an aim |
| 6 | `war::alliances` | oaths sworn against a common enemy, and lapsed |
| 7 | `war::tribute` | tributaries pay, and test their overlord's grip |
| 8 | `war::resolve_wars` | coalition battles, sieges, sackings, peace that settles the aim |
| 9 | `dynasty::tick` | reigns, mortality, marriages, births, standing, acclaim, coronations |
| 10 | `politics::rulers` | deaths, successions, dynasties, claimants |
| 11 | `politics::unrest` | revolts, civil wars, secession, shattering |
| 12 | `magic::tick` | schools founded, spread, adopted, persecuted, split |
| 13 | `people::culture_drift` | assimilation and drift into new peoples |
| 14 | `climate::tick` | the drifting field of wet and dry, every eighth year |
| 15 | `trade::tick` | routes reckoned afresh every twenty years; roads opened and closed |
| 16 | `tech::tick` | what is worked out, what spreads, what is forgotten |
| 17 | `events::disasters` | plagues, famines, eruptions, storms |
| 18 | `events::notables` | notable lives |
| 19 | `circles::tick` | rivalries and patronage, and what comes of them |
| 20 | `events::wonders` | wonders begun and finished |
| 21 | `stories::tick_artifacts` | relics forged, taken, lost, found |
| 22 | `stories::tick_prophecies` | prophecies fulfilled or failed at their deadline |
| 23 | `stories::tick_legends` | tyrants, heroes, legends |
| 24 | `World::recompute` | rebuild aggregates: sizes, populations, neighbours, sprawl, what each realm knows |
| 25 | `events::eras` | name the age if it has turned |

Then the chronicle is compacted to `tuning.chronicle_cap`, and every tenth year
the population is pushed onto a history for the sidebar graph.

The ordering matters and is not arbitrary: people exist before states claim
them, states expand before they fight, wars resolve before rulers die of them,
houses settle their marriages and their heirs before a succession has to draw
on them, and everything is aggregated by `recompute` before the era logic reads
the aggregates.

`Detail` (low / medium / high) sets a minimum importance below which `log`
drops an event, and also gates the finer-grained simulation itself: extra
rounds in a battle, more notable lives, the smaller magical incidents
(`detail_rate`, `high_detail`). Because those gates sit in front of RNG draws,
**detail is part of a world's identity**: the same seed at low and at high
detail are different histories, not the same history told at two lengths. A
saved world carries its detail level for that reason.

## Determinism

The same seed must always produce the same history, and a world reloaded from a
save must continue exactly as it would have. `src/tests.rs` asserts both.

What that demands of the code:

- One RNG, `World::rng`, with interior mutability so it can be shared across
  split borrows of the world. **Cloning an `Rng` shares the stream** rather
  than forking an identical one — otherwise a handle taken with
  `w.rng.clone()` and the world's own generator would draw the same numbers.
- Ordered containers only where the simulation can see the order: `BTreeMap`
  and `BTreeSet`, or a `Vec` sorted with a total tie-break. A `HashMap`
  iteration in `sim` is a bug even when it looks harmless.
- Stable visit order over cells and entities wherever it feeds a draw or a
  tie-break — which, because entity vectors are append-only, is just their
  natural order.
- No wall-clock time, environment or terminal size as simulation input. The UI
  may read all three; `sim` may not. The one clock in `tick` is the profiler's,
  and it only ever writes to `prof`.
- The same number of draws on every path. An early return that skips a draw
  shifts the whole stream.

The prose layer is bound by the same rule from the other side: a helper that
wants variety either draws *exactly once* per choice from the world RNG
(`Pick::rolled`) or derives its choice from the year and an entity id and draws
nothing (`Pick::stable`).

Consequently any change to the draws — a new call, a reordered loop, a retuned
constant — changes every seed's history. That is allowed; it just has to be
declared, in the commit message and in `CHANGELOG.md`.

## Cost

A year must cost the map, not the map times the number of realms. The rule is
that **no phase may walk all the cells once per realm**: `owner_cells` gives a
realm its own land directly, and `cells_of_ref` is the cheap read (`cells_of`
clones, for callers that go on to mutate). `recompute` is the one full sweep,
and it is a single pass that collects border crossings flat and counts them
afterwards rather than doing a map lookup per cell.

The same rule applies to the entity vectors, which hold everything that has
ever existed: **no phase may filter `polities` or `persons` for the living**.
`alive_polities` and `alive_persons` are there for that, and a phase that
walks the raw vector costs the age of the world every year. This is the
regression to look for in a profile — a phase whose cost grows with how long
the world has been running rather than with how much is in it now.

Three things in the simulation would otherwise grow without bound, and each is
deliberately capped:

- **A realm's `tension` map.** Fear of the hegemon reaches beyond a realm's own
  borders, so without a bound a realm ends up weighing every power that ever
  frightened it. Non-neighbours must clear a higher floor to stay on the books
  (`FAR_TENSION_FLOOR`) and the map is capped at `TENSION_CAP` entries, coldest
  forgotten first. Fear itself only reaches the hegemon's neighbours and theirs.
- **A realm's `truce` map**, swept on a rolling schedule (`TRUCE_SWEEP`) rather
  than annually, because a stale entry is harmless and rebuilding the map every
  year for every realm costs more than the entries do.
- **The chronicle**, capped at `tuning.chronicle_cap` (60,000 events): the
  oldest trivia goes first, with importance 2 and 3 kept whatever happens.

`--bench` prints ms/year and the per-phase breakdown. At high detail a year
costs roughly 0.6 ms at 160x64 over 2000 years — ten thousand years of a
default world runs in about seven seconds. The larger
map is 6.25 times the cells but supports about six times as many *living*
realms, and `diplomacy` is priced per realm-pair rather than per cell, so it
dominates a crowded map and the cost grows faster than the area. That is the
shape to expect; what it must not do is grow with the years.

## The save format

`src/ser.rs`, whose module docs are the specification. The shape:

```
magic "RFE\0" | format version u32 | body length u64 | FNV-1a 64 of the body
body: chunk* where chunk = tag u32 | record version u32 | length u64 | payload
```

One chunk per top-level section of the world — `head`, `terr`, `cell`, `race`,
`cult`, `city`, `poly`, `pers`, `hous`, `schl`, `wars`, `eras`, `plag`, `arti`,
`prop`, `chrn`, `stat`, `ctrs`, `bfnm` — listed in a `sections!` macro that
generates both the writer and the reader from one table.

Derived indexes are **not** sections. `alive_polities` and `alive_persons` are
rebuilt as their entity sections are read, because a stored index could
disagree with the entities it indexes. A field that is derived but read a tick
*before* it is next rebuilt has to be stored all the same — `Polity::sprawl` is
computed by `recompute` at the end of a tick and read by `economy` at the start
of the next, so a world reloaded without it governs its first year on a zero
and diverges from the world that wrote the file.

Three ideas carry the design:

1. **One symmetric `Io` trait drives reading and writing.** Each record is
   described once, as a function that visits its fields; `Writer` and `Reader`
   both implement the trait, so the two directions cannot drift apart.
2. **Chunks are tagged and length-prefixed, records are positional.** The
   framing is what makes the file survivable — a reader takes the tags it
   knows, skips the rest, and leaves unseen sections at their blank default —
   while positional records keep the file small and the loop fast.
3. **Per-section record versions.** A section that gains a field bumps its
   version in the table; the field is written unconditionally and read behind
   `if s.ver() >= N`. A reader meeting a *newer* version of a section it knows
   cannot parse it positionally, so it skips that chunk too and keeps defaults.

The format version at the top changes only when the framing changes. Version 1
(a bare positional body with no chunks) is still read, by a frozen `read_v1`,
and `tests/fixtures/v1.rfe` exists to keep it that way. Damage is caught by the
checksum and reported as a `SaveError` rather than becoming a mangled world.

Because entity vectors are append-only and indices are stable, a save is very
nearly just those vectors written out: there is no id remapping on load. The
`head` chunk adds the seed, the year, the detail level and **the RNG's state**,
which is what lets a loaded world resume bit-identically rather than merely
plausibly.

## The interface

`Ui` owns the `World`, a `Screen` (the cell buffer from `term.rs`), and the
view state: mode, layer, cursor, viewport, zoom, selection, scroll offsets per
panel, search and filter strings, the keymap and the theme.

The loop is: poll for input with a timeout, handle whatever arrived, advance
the world if enough real time has passed for the current speed, render a frame,
repeat. Rendering is a full redraw into the cell buffer; `term.rs` diffs it
against the last frame and emits only what changed, so a still world costs
almost nothing.

The split is by *stage*, not by screen:

- **`input.rs`** turns keys into intent. It holds the vim machinery — a pending
  count, a pending prefix character (`g`, `z`, `Z`), the per-mode handlers —
  plus the `:` and `/` prompts and the mouse. Key remaps from the config are
  applied here, once, before dispatch.
- **`commands.rs`** is the `:` line. `run_command` offers the command to each
  group in turn (`cmd_files`, `cmd_sim`, `cmd_view`, `cmd_search`,
  `cmd_config`, `cmd_misc`); a group returns `Done`, `Quit`, or `Unknown` to
  pass it along. Adding a command touches one `match`.
- **`render/`** draws. `render/mod.rs` decides which panels the current `Mode`
  wants and composes the frame; `map.rs`, `sidebar.rs` and `panels.rs` each own
  their region. Rendering is also where click targets are recorded — the rows
  a chronicle line, a list entry or a sidebar power occupies are pushed into
  the `Ui` as they are drawn, and the mouse handler looks them up next frame.
- **`detail.rs`** builds the content of lists, detail pages, the help screen
  and the Hand of Fate as lines of text, independently of where they land.

`Mode` (`Map`, `List`, `Detail`, `Chronicle`, `Help`, `Fate`, `Recap`) selects
both the key handler and the panel set. `theme.rs` is a post-pass: the frame is
drawn once in its native palette and the theme re-maps every cell's colours, so
a new theme costs nothing but a function.

## Prose and explanation

These are the two layers between the simulation and the reader, and both are
strictly one-directional: they read the world, and never change it.

**`sim::prose`** writes every sentence the chronicle prints. The simulation
decides *what* happens and immediately asks prose *how to say it*, then stores
the finished string in the event. Keeping grammar — articles, plurals,
pronouns, lists — in one tested place is the point; so is the naming rule, that
a realm is introduced by its full name ("the Kingdom of Velen") the first time
it appears in an event and referred to by its short name afterwards. The
submodules mirror the simulation's: `prose/politics.rs` says what
`sim/politics.rs` did, `prose/dynasty.rs` what `sim/dynasty.rs` did, and
`prose/diplomacy.rs` carries the war aims and the peaces that settle them.

Events are rendered eagerly, at the moment they happen, because the world that
explains them is gone by the time anyone reads the line: the realm has fallen,
the ruler is dead, the city is ruins. The event therefore carries its text plus
the structured `refs` and `loc` that index it.

**`sim::explain`** answers the other question — not what happened, but why
things are as they are now. It is computed lazily, on demand, when a page is
opened: why a realm holds together or does not, ranked contributions in points
and in words; who is winning a war and why, and what a peace signed this year
would look like; where a person stands. Every function is a pure reading of
`&World`.

Its weights deliberately mirror `politics::economy` and `war::strength` — the
sprawl and regalia terms in particular are written to be read side by side
with the simulation's. `dynasty::standing` avoids the problem a different way:
the scoring and the wording both come from one `for_each_deed`, so the points
and the sentence explaining them cannot disagree. That duplication is the cost
of having explanations at all — the simulation cannot
afford to build a justification for every number it computes every year for
every realm — and it comes with an obligation: **change the two together.** An
explanation that disagrees with the simulation is worse than no explanation.

**`ui::words`** is the smallest of the three and the most used: pure functions
mapping numbers to the vocabulary the interface speaks in — *on the brink*,
*bankrupt*, *formidable*, *a great power*, *a local cult*. Because they are
pure they are tested, and because they are shared the sidebar, the lists and
the detail pages cannot disagree about what 12% stability is called.
