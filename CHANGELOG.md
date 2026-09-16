# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

While the version is below 1.0 the save format and the shape of a generated
world may change between releases. A change that makes a given seed produce a
different history is noted here under **Changed** as *world-changing*, because
it invalidates saved worlds' futures and every seed anyone has written down.

## [Unreleased]

Everything so far. The project has not been released yet; this section is the
history of the work that led to the first release.

### Added

- **The simulation.** A procedurally generated world — fractal elevation
  shaped into continents, latitude and altitude for temperature, prevailing
  winds for rainfall and rain shadows, filled depressions, rivers to the sea,
  biomes from climate, and a mana field with ley nexuses. Named mountain
  ranges, forests, deserts, seas, rivers and islands.
- **Peoples** with homelands, favoured biomes, lifespans and temperaments;
  population growth toward carrying capacity, migration into empty country,
  assimilation under a foreign crown, and cultural drift into new peoples with
  daughter languages.
- **Realms**: tribes forming where people are dense, expansion priced by
  terrain and distance from the capital, cities founded on rivers and coasts,
  ranks from chiefdom to empire, and stability driven by the ruler,
  overextension, foreign subjects, war-weariness and decadence — with revolts,
  civil wars and shattering when it fails.
- **War**: border tension from ambition, culture, claims and rival faiths;
  battles shaped by terrain, generals and rulers' valour; sieges, sackings and
  peace by exhaustion.
- **Schools of thought**: arcane orders, faiths and philosophies that are
  founded, spread along trade and conquest, adopted or persecuted, split in
  schisms, and occasionally unmake the city that raised them.
- **Stories**: dynasties and rival claimants, tyrants, legends, prophecies with
  deadlines that come true or fail, and relics forged, carried off as spoils,
  lost and dug up centuries later.
- **Procedural languages**: a phonology and orthographic habits per culture,
  from which every name is coined, with sound shifts producing daughter tongues.
- **The chronicle**: every notable event stored as structured data with prose
  already rendered, indexed by the entities involved so every detail page
  carries its own history.
- **A vim-style terminal interface**: counts before motions, `gg`/`G`, `zz`,
  `zi`/`zo`, `:` commands, `/` search with `n`/`N`, `ZZ` to quit, plus arrow
  keys, modified arrows, PageUp/PageDown and the mouse. A scrollable map with
  political, terrain, culture, mana, population and biome layers, a sidebar, a
  live event log, browsable lists, detail pages and the full chronicle.
- **Save and load**: `:w`, `:e`, `:saveas`, `:saves`, `:autosave N`, and
  `--save` / `--load` on the command line. Saved worlds continue exactly as
  they would have.
- **Config and themes**: `~/.config/empires/config` with `key = value`
  settings and vim-style `map <from> <to>` remaps, `--mkconfig` to write a
  commented template, and the `default`, `phosphor`, `amber`, `paper` and
  `dusk` themes.
- **The Hand of Fate** (`x`, `:fate N`): six ways to interfere with the
  selected realm.
- **Headless and analysis modes**: `--headless N` to print a chronicle with no
  UI, `--min-importance` to filter it, `--stats` for balance metrics per
  century, `--bench` for ms/year and per-phase timings, and `--snapshot PATH`
  to render one frame to text and HTML.
- **`--version` / `-V`.**
- **Tests** (`cargo test`): determinism, save-file round trips, the version-1
  fixture, damaged and future files, configuration parsing, name generation,
  geography invariants and the plain-words mappings. `tools/ptytest.py` drives
  the real interface in a pty.
- **Packaging and project scaffolding**: dual MIT / Apache-2.0 licensing,
  crate metadata, this changelog, `CONTRIBUTING.md`, `docs/ARCHITECTURE.md`,
  GitHub Actions for CI and releases, and `rustfmt.toml` / `clippy.toml`.

### Changed

- *World-changing.* **Balance and tuning.** Every balance-relevant number now
  lives in `sim::tuning::Tuning` instead of as a literal at its use site, and
  the values were retuned against the `--stats` harness so that great powers
  live a plausibly long time without the map freezing into one empire.
- *World-changing.* **One RNG stream.** `Rng` clones now share a stream rather
  than forking an identical one, so a handle taken with `w.rng.clone()` and the
  world's own generator can no longer draw the same numbers.
- **The save format is now chunked (version 2).** A file is a header (magic,
  format version, body length, FNV-1a checksum) followed by one tagged,
  length-prefixed chunk per section, each carrying its own record version. A
  reader skips chunks it does not know and defaults sections that are missing,
  so saves survive new versions of the game. Version-1 files still load, and
  damage is reported as a checksum error instead of a mangled world.
  `SaveError` replaced stringly-typed failures.
- **Plain language throughout.** Stability, treasuries, armies, realm size and
  a school's reach read as words (*on the brink*, *bankrupt*, *formidable*, *a
  great power*, *a local cult*) with the numbers kept beside them in lists,
  where they are there to compare.
- **Causes sit next to effects.** `sim::explain` gives realms, wars and people
  a **Why** block: what pulls stability up or down, ranked and in words, each
  contribution in points; who is winning a war and why, and what a peace signed
  this year would look like. Its weights mirror `politics::economy` and
  `war::strength`, so the explanation and the simulation agree.
- **The map says what it is.** Realm names beside their capitals at zoom 1, a
  one-line legend under the map (`:legend`), and a **Here** block that
  describes whatever the cursor sits on as a sentence.
- **Digests.** `r` and `:recap [N]` summarise the last fifty years for the
  world or for the selected realm, and coming back to the map says in one line
  what you missed.
- **A first-run card** explaining the map, that time is already running, and
  the four keys that matter. Any key dismisses it for good; `--tour` or
  `:tour` brings it back.
- **The prose layer.** Every sentence the chronicle prints moved out of the
  simulation into `sim::prose`, where articles, plurals, pronouns and lists are
  handled once in tested helpers, and a realm is introduced by its full name
  and referred to by its short one thereafter.
- **A year of simulation now costs the map, not the map times the realms.**
  `World::owner_cells` indexes each realm's own land in map order, maintained
  by `claim` and `fall`, so no phase walks the whole map once per realm. At
  high detail over 1500 years a year costs about 0.44 ms at 160x64 and about
  2.1 ms at 400x160 — roughly proportional to the number of cells rather than
  climbing with the number of realms that have ever lived.
- **The chronicle no longer grows without bound.** Past `chronicle_cap`
  (60,000 events by default) the oldest small entries are dropped, trivia
  first; importance 2 and 3 are always kept.
- **The interface was split** from one 2953-line module into `ui::input`,
  `ui::commands` and `ui::render` (map, sidebar, panels).

### Fixed

- **Terminal restoration.** The saved termios state is kept in sound statics,
  the panic hook can read it without allocating or blocking, and SIGINT,
  SIGTERM and SIGHUP restore the terminal instead of leaving it in raw mode on
  the alternate screen.
- **Bracketed paste** is enabled and understood, so pasted text can no longer
  be run as a burst of commands.
