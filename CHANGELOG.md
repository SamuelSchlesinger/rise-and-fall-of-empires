# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

While the version is below 1.0 the save format and the shape of a generated
world may change between releases. A change that makes a given seed produce a
different history is noted here under **Changed** as *world-changing*, because
it invalidates saved worlds' futures and every seed anyone has written down.

## [0.1.2] - 2026-09-16

### Added

- Persistent navigation rows with highlighted keys for inspecting, browsing,
  recaps, history, stories and interventions. Each screen shows its own controls;
  narrow windows wrap complete key/action pairs instead of hiding them behind
  diagnostics. Very short terminals retain a compact help/back/quit prompt.
- Visible speed and event-follow status, contextual sidebar and chronicle
  prompts, and an expanded welcome card and guide to the richer screens.

### Changed

- Start at 2 years per second instead of 5, with event following off by default.
  Existing explicit configuration values still apply; `+`/`-` change speed and
  `f` toggles event following on the map.
- Reserve space for navigation so page content and mouse hit targets stay above
  it. Show command feedback separately from the main screen shortcuts.
- Simulation rules and save-file format are unchanged.

### Fixed

- Clicking a visible map tile no longer pans the map near the viewport edge.
- List categories stay visible and clickable as you move through them in a
  narrow terminal; filter counts no longer cover the category tabs.

## [0.1.1] - 2026-09-16

### Added

- macOS support on Apple Silicon and Intel, including native release archives.
- An in-app player guide (`:guide`, or `p` from help) and an optional interactive
  tutorial (`:tutorial`, or `t` from help or the welcome card). Practice time,
  movement, map layers, zoom and browsing; skip with Esc or Ctrl-g.
- CI builds and interface tests on both Mac architectures, plus terminal tests
  for raw mode, resizing and restoration after normal exit or a signal.

### Fixed

- Delayed or oversized bracketed pastes remain text until their closing
  marker, so a pause between chunks cannot turn pasted text into commands.
- Linux-only libc bindings prevented macOS builds. The terminal backend now
  selects the correct termios layout, constants, signal set, poll signature
  and errno accessor for each platform.
- Match the C read/write pointer types to keep builds warning-free on newer
  Rust toolchains, and report failure when entering raw mode fails.

### Changed

- Shorten the README and replace its dense options paragraph with tables.
  The complete controls, configuration and simulation guide live in
  `docs/GUIDE.md` and are included in release archives.
- Simulation rules and save-file format are unchanged.

## [0.1.0] - 2026-09-16

The first release. Everything below is the work that led to it.

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

- *World-changing.* **A pass over balance, names and the loudest sentences.**
  Retuned fragmentation, decadence, expansion, sieges, truces, prophecy
  deadlines and the two disaster chances against the `--stats` harness; the
  empire threshold now uses the world's settled land as a *cap* rather than a
  floor, so the chiefdom-kingdom-empire ladder is climbable again; a waning
  school of thought is absorbed into a larger school of its own kind instead
  of lingering for ever; and development keeps raising the capacity of land
  and cities past its old ceiling, so the late centuries mean denser realms
  rather than a world that stops at year 900. Two living realms can no longer
  share a name, a republic goes by its capital's name as its own prose
  already assumed, daughter languages drift further from their parents, and
  hopeless wars now end. Realm shatterings, city fires, arcane catastrophes
  and the conquest epitaph have four phrasings each, and a world never calls
  two of its centuries by the same name.
- *World-changing.* **`--detail` no longer changes the simulation.** It is a
  verbosity setting: low, medium and high produce exactly the same world from
  the same seed and differ only in the least important event the chronicle
  keeps and in the extra line of colour high adds. Every draw that detail used
  to gate now happens at every level.
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
- `ui::detail::detail_lines` became one function per kind of page,
  `ui::render::render_map` one per band of the map, and `sim::magic::tick`
  one per phase of a magical year. No behaviour changed; the same seeds
  produce the same worlds.

### Fixed

- **The map now shows who holds what.** The political layer used to be the
  terrain layer in other colours: identical glyphs, a different palette, and
  nothing at all in a monochrome terminal or a black-and-white screenshot. It
  and the culture layer now draw a neutral field of `·` under their colours
  and rule the frontiers off with `│ ─ ┼` (`| - +` under `--ascii`)
  wherever the realm or the people changes, leaving unheld land blank, with
  rivers, cities, capitals, ruins and event marks still on top. Terrain,
  biomes, mana and population keep their own glyphs.
- **A key to the colours.** The sidebar's *Great powers* block, which at 44
  rows was often down to a single realm, is now an **On screen** block: the
  largest realms in view, each with its swatch, its size and its trend. The
  sidebar shares its rows out by what each block actually has to say instead
  of by fixed reserves, and the key keeps a floor of rows of its own.
- **Nothing in the sidebar stops mid-sentence.** Every block — the era, the
  storyteller, Here, Selected — is cut with an ellipsis (`...` under
  `--ascii`, where a lone `…` flattened to a full stop) at 150, 120 and 100
  columns alike.
- **The event log defaults to the notable.** Level 1 printed about seven
  lines a year, unreadable at five years a second; the default is now 2, `v`
  cycles the same 1-3 that `:log` takes, and both the feed's title and the
  message say what the level means in words. `--min-importance` on the
  command line is unchanged for scripts.
- **A world smaller than the map pane is centred in it** at every zoom
  rather than pinned to the top-left corner, in the renderer, the map labels
  and the mouse alike.
- **Every glyph the map draws is in a legend.** The terrain and biome keys
  name hills, taiga, grassland, steppe, tundra, ice and wastes as well, and
  each key is fitted to the width it is given rather than clipped.
- **A realm's stability trend and its Why block agree.** The page said
  "restless (65%), and holding there" three lines above "pulls stability
  towards 62%"; the trend now carries that very number. The *Strain* bar,
  which drew from overextension alone and stood full above three other
  percentages, is gone; the four pressures are named instead.
- **A great realm's epitaph undercounts its cities** ("ruled 599 lands and one
  city"): the peak land figure was paired with a death-time city count.
  `Polity` now tracks `peak_cities`, saved in version 2 of the `poly` section.
- **A queen is not a king.** A kingdom whose language calls its ruler an
  emperor used to title a woman "King".
- **"1 years of fighting", "1 years".** Every count in the simulation's prose
  and explanations goes through `prose::count` / `prose::years`.
- **A realm known by two names.** A republic was named after its capital while
  its short name stayed a coined word, so "Consul X of Tessek" never matched
  "the Republic of Ilmen". A realm's short name is now whatever proper noun
  its full name is built around, and no two living realms share one.
- **Hopeless wars.** A realm with an army of two could prosecute a seven-year
  war against sixty-seven. Once one side has been outmatched in the field for
  a couple of years, or the attacker's army is negligible, peace comes
  quickly — and "the rising" is now said only of a rising, not of a civil war
  or a disputed succession.
- **"A Ighoi kingdom".** The realm, people, relic and place pages wrote a bare
  "A" in front of a word coined by a language that knows nothing about
  English; they go through `prose::a` / `prose::cap_a` now.
- **A lost relic** no longer offers `[h]` to open a holder that is not there.
- **Terminal restoration.** The saved termios state is kept in sound statics,
  the panic hook can read it without allocating or blocking, and SIGINT,
  SIGTERM and SIGHUP restore the terminal instead of leaving it in raw mode on
  the alternate screen.
- **Bracketed paste** is enabled and understood, so pasted text can no longer
  be run as a burst of commands.
- **A damaged or hand-made save can no longer crash the game.** Every index a
  save file carries — into cells, realms, cities, peoples, persons, schools,
  wars, relics, prophecies and the chronicle — is checked against the vector
  it points into before the world is used, and a file that fails is reported
  as inconsistent rather than panicking somewhere far away in `recompute`.
- **A save file can no longer ask for unbounded memory.** A record count is
  believed only as far as the bytes left in its chunk allow, so a forty-byte
  file claiming fifty million people is a truncation error instead of a
  multi-gigabyte allocation. `src/ser_tests.rs` mutates a real save two
  hundred ways and asserts that loading never panics.
- **Ctrl+Space no longer panics** a debug build: the ASCII control block is
  decoded by setting bit 6 rather than by an arithmetic that underflowed on
  NUL. `src/term.rs` now has tests for the whole input parser.
- **Bad command-line arguments are errors.** An unrecognised option, a
  `--seed` that is not a number, a `--detail` that is not a level, a missing
  value or a size outside the allowed range now print a message and exit 2,
  where they used to be silently ignored or clamped. `-h` is `--help`; it
  used to be `--height`.
- **The help page scrolls and wraps.** It is longer than a small terminal, and
  used to be cut off at the bottom edge and mid-word at the right edge, with
  no way to see the rest.
- **`--ascii` now applies to the whole frame.** Box rules, separators, bars,
  arrows and the help page's own symbol key were still being drawn in Unicode;
  a final pass over the cell buffer converts anything left, so the symbols the
  help explains are the ones the map draws.
- **The status line always says how to leave.** Below about ninety columns it
  used to drop every key hint, including on the help page.
- **The chronicle no longer opens mid-sentence.** Both the strip under the map
  and the full page start on the beginning of an entry, and when a single
  entry is taller than the panel they show its first lines rather than its
  last.
- **Sidebar text is elided, not cut.** The era's description, the "since you
  last looked" note and the storyteller's headlines end in an ellipsis when
  they do not fit, instead of stopping in the middle of a word.
- **The `[k]` links worked nowhere**, because `k` scrolls. A realm's capital,
  a school's home city and a relic's maker are `[K]`.
- **`<C-Up>` and friends could not be parsed** in a config file or `:map`,
  though `:maps` printed them: the ctrl-arrows were being read as `<C-u>`.
- **`ZQ` now quits without saving**, as in vim and as `:q!` already did.
- **The config file reports what it cannot honour.** An unknown theme, a
  `zoom` or `log` outside the range its own comment gives, and `:set width`
  or `:set height` on a world that already exists were all accepted and then
  quietly ignored.
- **`:layer` with no argument** switched to the political layer instead of
  printing its usage, and **`:new` with a seed that is not a number** threw
  the world away and started a random one.
- **A realm on the brink is called that everywhere.** The storyteller had a
  word of its own, "teeters", that appeared in no other panel.
- **Recap arithmetic reads correctly.** "53 wars began, 51 ended, 4 still
  burning" invited a subtraction that does not hold, and the digest stopped at
  thirty lines whatever the height of the terminal, leaving a third of a tall
  window blank.
- Person traits and a school's following line up in columns; the headless
  summary says "1 school", not "1 schools".

[0.1.0]: https://github.com/SamuelSchlesinger/rise-and-fall-of-empires/releases/tag/v0.1.0

[0.1.2]: https://github.com/SamuelSchlesinger/rise-and-fall-of-empires/releases/tag/v0.1.2
[0.1.1]: https://github.com/SamuelSchlesinger/rise-and-fall-of-empires/releases/tag/v0.1.1
