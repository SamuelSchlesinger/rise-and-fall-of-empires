# Player guide

Complete options, controls, configuration and simulation reference for
Rise and Fall of Empires. Start with the [README](../README.md) for installation
and a quick introduction. In the game, press `?` for the full key list.

## Getting started

The world starts at **2 years per second**, with event following **off**.
Press **Space** to pause, then move with the arrow
keys or `h j k l`. Each colour on the political map is a realm, borders mark
its territory, `@` marks a capital and `#` a city. The legend under the map
explains the current layer; the sidebar describes the place under your cursor.

Press **Enter** to inspect a place, **Tab** to change the map layer, and **r**
for a recap of the last fifty years. Use `e` to browse lists of realms, cities,
people and more. **Esc** takes you back. To keep this world, type `:w` and
press Enter; later, `:e NAME` loads a save. Quit with `q` twice on the map.

The bottom rows show the keys for the current screen, with shortcuts in
yellow. Time and follow status stay on the last line. Press `-` to slow down
further, or `f` on the map to enable automatic jumps to major events.

### Explore the world

| Screen | How to open it | What to look for |
|---|---|---|
| Entity details | `e`, choose an item, Enter | Realm stability and its causes, rulers, families, doctrines and histories; bracketed letters open related pages |
| Recap | `r` on the map | The last fifty years, for the selected realm or city, or the whole world if nothing is selected |
| Chronicle | `c` on the map | Search history with `/`, change importance with `f`, or click an event to visit its location |
| Current stories | `t` on the map, or click a story under Now | The realm, person or place behind a current headline |
| Hand of Fate | Select a realm with `s`, then `x` | Six interventions; press 1–6 to choose, or Esc to cancel |

In lists, Tab changes category; the selected category stays visible even in
a narrow window. On detail pages, Backspace retraces links and Esc returns
to the previous screen. `?` opens all controls; `:guide` reopens this guide.

### Learn in the app

The complete guide is available offline with `:guide`, or **p** from the `?`
help screen. Scroll with arrows, `j`/`k`, PageUp/PageDown or the mouse wheel;
Esc returns to your previous screen.

For practice, run `:tutorial` or press **t** from help or the welcome card.
The tutorial walks through the real controls for time, movement, layers, zoom
and lists. Press **Esc** to skip (on the return-to-map lesson, Esc performs
that action), or **Ctrl-g** to skip at any time. When you finish or skip, your
previous pause and follow settings return. The tutorial does not create a
save; use `:w` when you want to keep your world.

## Playing

Keys follow vim conventions: a count before a motion repeats it (`12j`,
`3]`, `5.`), `gg`/`G` jump, `zz` centres, `zi`/`zo` zoom, `:` opens a
command line, `/` searches, `ZZ` quits and `ZQ` quits without saving. Arrow
keys, Shift/Ctrl+arrows, PageUp/PageDown and the mouse work too. Press `?`
in the game for the full key list — it is the authority, and it scrolls.

| key | action |
|---|---|
| Space, `+`/`-`, `.` | pause, change speed (0.5 to 100 years/second), step a year |
| `h j k l` / arrows | move the cursor; `H`/`L` or Shift+arrows 8 cells, `K`/`J` 4 rows, Ctrl+arrows 20 and 10 |
| `0` `$` Home `zz` `zt` | left edge, right edge, centre of the world, centre on cursor, cursor to the top |
| `zi` `zo` | zoom in / out (1 to 4 world cells per character; the whole world fits at 2, centred in the pane) |
| Tab / Shift+Tab | map layer: political, terrain, culture, mana, population, biomes |
| Enter / `s` / Esc | open / select what is under the cursor / clear the selection |
| `gg` or `G` | jump to the selected thing |
| `]` `[` and `}` `{` | next / previous realm, next / previous city |
| `t` | go to the top story in the storyteller panel |
| `r` | a recap of the last fifty years (of the selected realm or city, if one is selected) |
| `/name`, `n`, `N` | search realms, cities, peoples, people, schools, wars, places, relics; cycle matches |
| `f` | follow: the cursor jumps to each major event as it happens |
| `e` | lists: realms, cities, peoples, schools, persons, wars, places, relics, prophecies |
| `c` | the full chronicle (`f` or `v` cycles importance 0–3, `/` filters by text) |
| `x` | the Hand of Fate: intervene in the selected realm |
| `D`, `v` | cycle simulation detail; the event log's level (1 everything, 2 the notable, 3 only the great) |
| `?` or F1 | help |
| `q` twice, `ZZ`, Ctrl-C | quit, saving if a save file is set; `ZQ` quits without saving |

Uppercase aliases work on the map where the lowercase key is taken by a
motion: `C` chronicle, `R` recap, `F` follow, `X` fate, `T` top story.

Everywhere off the map: `j`/`k` scroll, Ctrl-d/Ctrl-u half a page,
Ctrl-f/Ctrl-b a page, `gg`/`G` top and bottom, `m` shows the thing on the
map, `q` or Esc goes back.

In a list, Tab or `]`/`[` change tab, `n`/`N` move the cursor, Enter opens
and `x` opens the Hand of Fate for the highlighted realm. On a detail page,
the letter in brackets opens that link — `[r]` ruler, `[c]` people, `[K]`
capital, `[f]` family and so on, always the key you press — Backspace
retraces, and Enter shows the thing on the map.

Mouse: click to select, click again (or right-click) to open, wheel to
scroll, click a chronicle line or a sidebar entry to jump to it. Pass
`--no-mouse` (or `:mouse`) to leave the mouse to the terminal for selecting
text.

### Commands

| command | effect |
|---|---|
| `:w [name]`, `:e name`, `:saveas name`, `:saves` | save, load, save under a new name, list saves (kept in `~/.local/share/empires`) |
| `:autosave N` | autosave every N years once a save file is set (default 100; quitting also saves) |
| `:speed 25`, `:detail high`, `:layer culture`, `:zoom 2`, `:theme paper` | settings |
| `:find name` | search and jump |
| `:until 900`, `:step 50` | run to a year and pause; advance N years at once |
| `:new [seed]` | start a fresh world without restarting |
| `:story` | go to the top story |
| `:recap [N]` | the digest of the last N years (default 50, and `N` sticks); `r` on the map does the same |
| `:follow on`, `:log 2`, `:filter 2` | jump to major events as they happen; the event log's level (1–3, default 2), and the chronicle's |
| `:legend` | the one-line key under the map (on by default) |
| `:tour` | show the welcome card again |
| `:guide`, `:tutorial` | read the player guide or practice the interface |
| `:mute battle` | hide an event kind from the feeds (toggle); `:mute` lists |
| `:set key value`, `:map <from> <to>`, `:unmap key`, `:maps` | runtime versions of the config file |
| `:mkconfig`, `:config` | write a commented config template; show its path |
| `:fate N` | apply Hand of Fate option N to the selected realm |
| `:q`, `:wq`, `:q!` | quit (saving if a save file is set), save and quit, quit without saving |

Most have the abbreviations and synonyms you would expect — `:write`,
`:load`, `:x` for `:wq`, `:c` for `:chronicle`, `:l` for `:layer`,
`:goto` for `:find`. `:help` opens the same page as `?`, which lists the
rest. `:set width` and `:set height` are refused while a world exists:
the map is raised once and cannot be resized.

### Saving

`empires --save ~/worlds/mine.rfe` starts a world that autosaves every
100 years and on quit; `empires --load ~/worlds/mine.rfe` continues it.
Inside the game `:w` saves to `~/.local/share/empires/world-SEED.rfe` and
`:e NAME` loads. Saves are self-contained and a loaded world continues
exactly as it would have (the simulation is deterministic per seed).

The file is a small header (magic, format version, body length and an
FNV-1a checksum) followed by one tagged, length-prefixed chunk per section
of the world. A reader skips chunks it does not know and defaults the ones
that are missing, so saves survive new versions of the game; damage is
reported as a checksum error rather than a mangled world. Saves written by
older versions still load. See the module docs in `src/ser.rs` for the rule
on adding a field.

### Config file

`empires --mkconfig` writes `~/.config/empires/config` (or
`$XDG_CONFIG_HOME/empires/config`):

```
# detail = medium        # low | medium | high
# speed = 2              # years per second at start; snapped to the nearest of
#                        # 0.5 1 2 5 10 25 50 100
# theme = default        # default | phosphor | amber | paper | dusk
# mouse = on
# ascii = off
# autosave = 100         # years between autosaves when a save file is set (0 = off)
# width = 160            # 40-600
# height = 64            # 20-300
# log = 2                # least importance shown in the event log: 1 everything,
#                        # 2 the notable (the default), 3 only the great
# follow = off           # jump the cursor to major events when enabled
# zoom = 1               # 1-4, how many world cells per character

# tune.decadence_growth = 0.0045   # override any field of sim::tuning::Tuning

# map w k                # examples: map <S-Up> K, map <C-p> :, map ; :
```

Every line is optional, and the template is written commented out: uncomment
what you want. `on`/`off`, `yes`/`no`, `true`/`false` and `1`/`0` are all
accepted for the switches, a line may be written `key value` as well as
`key = value`, and a leading `set ` is ignored, so lines can be pasted
between the file and the `:set` command.

Nothing here is silently ignored: an unknown setting, an unknown theme, a
value outside the range in the comment or a `tune.<field>` that does not
exist is reported with its line number the first time the game draws a
frame. `tune.<field>` reaches any of the simulation's balance constants by
name, so the shape of the world is adjustable without rebuilding.

### The storyteller

The sidebar's **Now** panel picks the three most interesting things in the
world at the moment: the biggest war and who is winning it, a great realm
about to break, a new empire, a plague, a prophecy about to run out. `t`
jumps to the top one.

### Reading the world

Nothing asks you to decode a percentage. Stability reads *steady*,
*restless*, *troubled* or *on the brink*; a treasury is *bankrupt*, *poor*,
*solvent* or *rich*; an army is *outmatched*, *matched* or *formidable*
against its neighbours; a realm is *a small realm* or *a great power*; a
school of thought is *a local cult*, *spreading* or *a great faith*. The
numbers stay beside the words in lists, where they are there to compare.

**Causes sit next to effects.** A realm's page carries a **Why** block: what
is pulling its stability up or down, ranked and in words — *overextended:
383 lands, but the crown can govern about 250*, *the court has rotted
through with luxury*, *the ruler is beloved* — with each contribution in
points, adding up to the value stability is drifting towards. A war's page
says who is ahead and why (hosts, generals, terrain, trouble at home) and
what a peace signed this year would look like. A person's page opens with
one sentence of standing: *a beloved ruler of twenty years*.

**The map says what it is.** At zoom 1 each realm's name is written beside
its capital, and a one-line legend under the map says what the current
layer's colours and glyphs mean (`:legend` turns it off). The sidebar's
**Here** block is a plain sentence about whatever the cursor sits on:
*Steppe on the coast, where Fil stands, ruled by the Lystzylese
Commonwealth, home to 4,470 Whendan folk.*

**Ownership is drawn, not only coloured.** The political and culture layers
put a plain field of `·` under their colours and rule the frontiers off
with `│ ─ ┼` wherever the realm — or the people — changes, leaving land
nobody holds blank; rivers, cities, capitals, ruins and event marks stay on
top of it. So the same frame read on a monochrome terminal, in a black-and-
white screenshot, or under `--ascii` (where the rules come out as `| - +`)
still shows where one realm stops and the next begins. The terrain, biome,
mana and population layers keep their own glyphs, and their legends name
every one of them. Beside the map, the sidebar's **On screen** block is the
key to the colours: the largest realms in view, each with its swatch, its
size and whether it is rising or falling.

**Digests.** `r` or `:recap` summarises the last fifty years — realms risen
and fallen, wars begun and ended, rulers dead, cities founded and sacked,
schools founded, who grew and who shrank — for the world, or for the
selected realm. `:recap 200` looks further back. Coming back to the map
after a while on another screen, the sidebar says in one line what you
missed.

On a first run (no config file and no `~/.local/share/empires`), a welcome
card offers the tutorial with `t` and the guide with `p`. Any other key
dismisses it; `--tour` or `:tour` brings it back.

## How the world works

**Geography.** Fractal elevation shaped into continents; latitude and
altitude set temperature; prevailing winds carry moisture off the sea and
drop it on windward slopes, leaving rain shadows behind. Depressions are
filled, rivers flow downhill to the sea, biomes follow from climate, and a
mana field with a few ley nexuses marks where magic runs strong. Mountain
ranges, forests, deserts, seas, rivers and islands are labelled as features
and named by the first people to reach them.

**Peoples.** Each race has a homeland, favoured biomes, a lifespan and a
temperament. Population grows toward the land's carrying capacity and
migrates into empty country. Cultures under a foreign crown slowly
assimilate; communities cut off from their kin by sea or mountains drift
into new peoples with daughter languages.

**Realms.** Tribes form where people are dense, expand cell by cell with
costs set by terrain and distance from the capital, found cities on rivers
and coasts, and climb from chiefdom to kingdom to empire. Stability depends
on the ruler, overextension, foreign subjects, war-weariness and decadence;
when it fails there are revolts, civil wars and, for the great, shattering.

**War.** Tension builds along borders from ambition, culture, claims and
rival faiths, and cools with trade and time. Battles are shaped by the
defender's terrain, generals and rulers' valour; cities fall to siege and
are sacked or spared; peace comes with exhaustion.

**Schools of thought.** Arcane orders, faiths and philosophies are founded
where mana is high or a people is mystical. They spread along trade and
conquest, are adopted as state doctrine or persecuted, split in schisms, and
an arcane order that reaches too far can unmake the city that raised it.

**Stories.** Rulers found dynasties whose heirs and rival claimants carry
their blood; cruel rulers become tyrants, renowned generals and poets become
legends. Seers utter prophecies with deadlines that later come true or fail.
Relics are forged for great rulers, wonders and schools, carried off as
spoils, lost in sacks and dug up centuries later, and give their holders a
little strength or standing.

**Chronicle.** Everything notable is written as prose and indexed by the
realms, cities, people and schools involved, so every detail page carries
its own history. It does not grow for ever: past `chronicle_cap` events
(60,000 by default) the oldest small entries are dropped, trivia first, and
events of importance 2 and 3 are kept whatever happens.

## Detail levels

`--detail low|medium|high` (and `:detail`, and the `D` key) decides **how much
of the history gets written down, and nothing else.** All three levels produce
exactly the same world from the same seed: the same realms rise, the same wars
are fought, the same people are born. What changes is the chronicle. Low keeps
only what an age would remember (importance 2 and 3), medium keeps the ordinary
run of events as well, and high keeps the trivia too and adds a closing line of
colour to a battle or a ruler's death. A 500-year world at seed 11 is 830
entries at low, 2,634 at medium and 2,744 at high — with identical populations,
realms, cities and cultures at the end of it.

Medium is the default. Low is worth having when you want the chronicle to read
as a list of turning points rather than a diary.

## Command-line options

### World and saves

| Option | Purpose |
| --- | --- |
| `-s, --seed N` | Choose the world seed; defaults to the current time. |
| `-w, --width W` | Map width: 40–600 cells; default 160. |
| `--height H` | Map height: 20–300 cells; default 64. |
| `-d, --detail LEVEL` | Chronicle detail: `low`, `medium` (default), or `high`. |
| `-l, --load FILE` | Continue a saved world. |
| `-o, --save FILE` | Save every 100 years and on quit; in headless mode, save at the end. |

### Terminal and configuration

| Option | Purpose |
| --- | --- |
| `--ascii` | Use plain ASCII glyphs. |
| `--no-mouse` | Keep the terminal's own text selection. |
| `--tour` | Show the introductory card again. |
| `--mkconfig` | Write a commented configuration template. |
| `-V, --version` | Print the version. |
| `-h, --help` | List all options. |

### Headless runs and snapshots

| Option | Purpose |
| --- | --- |
| `--headless N` | Simulate N years without the UI and print the chronicle. |
| `-i, --min-importance N` | Headless event threshold: 0–3, default 1. Above 3 prints only the summary. |
| `--stats` | Print balance metrics per century; default 1,500 years. |
| `--bench` | Print time per year and simulation phase; default 300 years. |
| `--snapshot PATH` | Export one frame to `PATH.txt` and `PATH.html`; default year 300. |
| `--layer LAYER` | Snapshot layer: `political`, `terrain`, `culture`, `mana`, `population`, or `biomes`. |
| `--cols C` | Snapshot width: 20–1,000 columns; default 160. |
| `--rows R` | Snapshot height: 5–1,000 rows; default 45. |

Use `--headless N` to choose the duration for `--stats`, `--bench`, or
`--snapshot`. Width, height and detail defaults can also be set in the config
file. An unknown option or invalid value exits with a message and status 2.
