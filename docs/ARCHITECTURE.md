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
geo.rs           elevation -> climate -> rivers -> biomes -> mana -> features.
                 Generated once, mostly read afterwards

sim/
  mod.rs         the World: entities, World::tick, recompute, the owner index,
                 and the helpers every subsystem shares
  tuning.rs      every balance-relevant number, in one struct
  genesis.rs     races, first peoples, the opening lines
  people.rs      growth, migration, assimilation, cultural drift
  politics.rs    tribes, expansion, cities, economy, rulers, unrest, collapse
  war.rs         tension, declarations, battles, sieges, peace
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
`cities`, `polities`, `persons`, `schools`, `wars`, `eras`, `plagues`,
`artifacts`, `prophecies` — and entities refer to one another by index into
those vectors. A ruler is a `usize` into `persons`; a war names two `usize`
polities; a chronicle line carries `Ref::Polity(7)`.

**Those vectors are append-only.** Nothing is ever removed or reordered,
because the indices are stored in save files, in every chronicle event and in
the UI's selection and history. A realm that falls is marked dead and stays
where it is. This is the single most important invariant in the tree: an
`Vec::remove`, a `retain`, or a sort in the wrong place silently rewrites
centuries of history and every save ever written.

Two derived structures are maintained rather than recomputed:

- **`owner_cells`** — each polity's own cells, ascending, kept exact by
  `claim` and `fall` and rebuilt by `recompute`. `cells_of_ref` reads it.
  It exists so that no phase has to walk the whole map once per realm; see
  *Cost* below.
- **The chronicle index** — for each `Ref`, the events that mention it, which
  is what lets any detail page show its own history without a scan.

`Terrain` (in `geo.rs`) is the immutable half of a cell: elevation,
temperature, rainfall, river, biome, mana, minerals, features. `CellState` (in
`sim`) is the mutable half: population, culture, owner, development. They are
parallel arrays indexed by `y * w + x`.

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
| 5 | `war::diplomacy` | border tension rising and cooling; declarations |
| 6 | `war::resolve_wars` | battles, sieges, sackings, peace |
| 7 | `politics::rulers` | deaths, successions, dynasties, claimants |
| 8 | `politics::unrest` | revolts, civil wars, secession, shattering |
| 9 | `magic::tick` | schools founded, spread, adopted, persecuted, split |
| 10 | `people::culture_drift` | assimilation and drift into new peoples |
| 11 | `events::disasters` | plagues, famines, eruptions, storms |
| 12 | `events::notables` | notable lives |
| 13 | `events::wonders` | wonders begun and finished |
| 14 | `stories::tick_artifacts` | relics forged, taken, lost, found |
| 15 | `stories::tick_prophecies` | prophecies fulfilled or failed at their deadline |
| 16 | `stories::tick_legends` | tyrants, heroes, legends |
| 17 | `World::recompute` | rebuild aggregates: sizes, populations, neighbours, cultures within, foreign share |
| 18 | `events::eras` | name the age if it has turned |

Then the chronicle is compacted to `tuning.chronicle_cap`, and every tenth year
the population is pushed onto a history for the sidebar graph.

The ordering matters and is not arbitrary: people exist before states claim
them, states expand before they fight, wars resolve before rulers die of them,
and everything is aggregated by `recompute` before the era logic reads the
aggregates.

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

`--bench` prints ms/year and the per-phase breakdown. At high detail over 1500
years a year costs roughly 0.44 ms at 160x64 and 2.1 ms at 400x160 — a map 6.25
times larger for about 4.8 times the cost, which is the shape to keep. A
profile where one phase grows with the *number of realms that have ever lived*
is the regression to look for.

The chronicle is the other unbounded thing, and is capped: past
`tuning.chronicle_cap` (60,000 events) the oldest small entries are dropped,
trivia first, with importance 2 and 3 kept whatever happens.

## The save format

`src/ser.rs`, whose module docs are the specification. The shape:

```
magic "RFE\0" | format version u32 | body length u64 | FNV-1a 64 of the body
body: chunk* where chunk = tag u32 | record version u32 | length u64 | payload
```

One chunk per top-level section of the world — `head`, `terr`, `cell`, `race`,
`cult`, `city`, `poly`, `pers`, `schl`, `wars`, `eras`, `plag`, `arti`, `prop`,
`chrn`, `stat`, `ctrs`, `bfnm` — listed in a `sections!` macro that generates
both the writer and the reader from one table.

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
`sim/politics.rs` did.

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

Its weights deliberately mirror `politics::economy` and `war::strength`. That
duplication is the cost of having explanations at all — the simulation cannot
afford to build a justification for every number it computes every year for
every realm — and it comes with an obligation: **change the two together.** An
explanation that disagrees with the simulation is worse than no explanation.

**`ui::words`** is the smallest of the three and the most used: pure functions
mapping numbers to the vocabulary the interface speaks in — *on the brink*,
*bankrupt*, *formidable*, *a great power*, *a local cult*. Because they are
pure they are tested, and because they are shared the sidebar, the lists and
the detail pages cannot disagree about what 12% stability is called.
