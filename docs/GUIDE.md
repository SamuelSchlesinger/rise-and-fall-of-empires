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
| `e` | lists: realms, cities, peoples, schools, persons, wars, places, relics, prophecies, figures, houses |
| `c` | the full chronicle (`f` or `v` cycles importance 0–3, `/` filters by text) |
| `x` | the Hand of Fate: intervene in the selected realm |
| `D`, `v` | cycle simulation detail; the event log's level (1 everything, 2 the notable, 3 only the great) |
| `?` or F1 | help |
| `q` twice, `ZZ`, Ctrl-C | quit, saving if a save file is set; `ZQ` quits without saving |

Uppercase aliases work on the map where the lowercase key is taken by a
motion: `C` chronicle, `R` recap, `F` follow, `X` fate, `T` top story.

**Figures** lists everyone the world has called great, the living first, with
the deed each is remembered for. **Houses** lists the ruling families; opening
one shows its whole line of succession, century by century, with every throne
it has held.

On a detail page the letter in brackets follows that link: `[r]` ruler,
`[c]` people, `[K]` capital, `[f]` family, `[H]` house, `[m]` spouse,
`[v]` whoever they served under, `[O]` overlord, `[s]` school, `[p]` parent
realm or school, `[b]` the house this one branched from. Backspace retraces.

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
# width = 288            # 40-600
# height = 144           # 20-300
# log = 2                # least importance shown in the event log: 1 everything,
#                        # 2 the notable (the default), 3 only the great
# follow = off           # jump the cursor to major events when enabled
# zoom = 1               # 1-4, how many world cells per character

# tune.decadence_growth = 0.0045   # override any field of sim::tuning::Tuning
#
# Dials worth turning, with the shape of the world they change:
#   tune.stability_sprawl_weight = 0.28   # how hard distance punishes a big realm
#   tune.hegemon_share = 0.2              # share of the world that provokes a coalition
#   tune.alliance_chance = 0.035          # how readily realms swear to each other
#   tune.tribute_share = 0.2              # what a tributary owes its overlord
#   tune.marriage_chance = 0.22           # how often a ruler finds a match
#   tune.birth_chance = 0.16              # how many heirs a house produces
#   tune.diadochi_min_generals = 2        # generals a dead conqueror needs to be
#                                         # carved up between them
#   tune.migration_sea_chance = 0.35      # how readily a crowded coast takes to boats
#   tune.tech_discover_chance = 0.0045    # how readily a city works something out
#   tune.tech_frontier_drag = 0.5         # how much dearer each thing already known makes the next
#   tune.climate_amplitude = 0.28         # how far a good or bad century moves harvests
#   tune.trade_toll_factor = 0.05         # what the crown takes from trade passing through
#   tune.hegemony_weight = 1.35           # what ruling too much of the world costs in stability
#   tune.rivalry_chance = 0.12            # how often two notables become rivals
#   tune.schools_per_realm = 0.5          # how many living traditions the world carries

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

**What the world works out.** Every world grows its own tree of inventions,
named in its own idiom: no two worlds learn the same things in the same
order. An innovation appears where it makes sense — irrigation on watered
land, smelting where there is ore — needs whatever came before it, and needs
a city rich and large enough to support it, which is what spreads a tree
across ten thousand years rather than four hundred.

Knowledge belongs to the **ground**, not to the realm. A kingdom that falls
leaves its roads and its writing behind for whoever takes its land, and that
is the only thing in the world that outlives the state which built it.
Ground that empties of people forgets what it knew, which is how a dark age
happens and how something comes to be worked out a second time.

What a people can learn depends on who they are. Each has a bent — two fields
they take to and two that never interested them — drawn from what they value
and inherited by their daughters, so a region keeps a recognisable character
across the rise and fall of its realms. It is why a technique can stall for
ever at a border that a trade good crosses in a season.

**Goods and roads.** The land yields ore, spice, salt, horses and the rest
where the terrain puts them. Cities trade when each has something the other's
country lacks. Wealth becomes positional: a city between places that want
what each other has grows rich on what passes through rather than on what
grows around it. A war closes the roads between the realms fighting it, and
the markets feel it at both ends — including in cities that never saw a
soldier.

**The weather.** Bands of wet and dry drift across the map over centuries. A
good stretch pushes farming out into the margins; a dry one pushes the
margins back onto the farmers, and the people who live where the grass fails
are the ones with horses.

**Blood.** Everyone carries two copies of each trait, one from each parent. A
rare strain can be carried unseen for generations and surface in a child
whose parents showed nothing of the kind. A house that marries its own to
keep a claim concentrates what it carries and loses vigour — its children are
frailer, die younger and breed less readily; one that marries out gets it
back.

**Rivalries and patronage.** People who hold no throne have ties to each
other. Two of comparable standing in the same trade — in one realm or across
a border they share — find themselves measured against one another, and every
later achievement by either is an event about both. Somebody of standing
takes up somebody young, who rises faster for it and sometimes outgrows them.

**The sea.** Where the water can be crossed is a property of the map, fixed
when the world is made. A crowded people on a coast will take a narrow strait
on its own and settle the island beyond it. A realm has to learn to build
sea-going hulls first, and then how wide a crossing it can manage depends on
its people's seafaring, its own development, and what kind of realm it is — a
republic on the water is a thalassocracy and reaches furthest. A strait is
within reach of almost anyone; only an old and developed naval power reaches
the far isles.

Once two realms face each other across water they are neighbours: quarrels
build between them, slowly, and armies can be sent. Landing on a hostile shore
is the hardest thing an army does and about a third to a half of landings are
thrown back into the sea. But a realm that controls the water is bound
together by it rather than stretched across it, so an overseas province is
counted as nearer to the capital than the map says — which is what makes an
empire on both sides of an ocean possible at all.

**Houses and figures.** Rulers marry — at home, or into a neighbouring house,
which quiets that border for a generation — and their children are born, named
and grow up in the chronicle. When a throne falls vacant the realm's own
inheritance custom decides what happens: the eldest takes the whole, or every
adult child takes a share and a great realm becomes several, or the strongest
kinsman takes it and a passed-over sibling raises the provinces against them.
A conqueror the world acclaimed who leaves no grown heir may have the realm
divided between the generals who marched with him. A consort who is the
nearest heir of a realm with none of its own can bring two crowns under one
head without a battle.

Standing is scored every year from what somebody has actually done, weighted
towards the *rate* of conquest rather than the total, and when a life stands
clear of its contemporaries the world names it great **at the time** rather
than in an obituary. A faith with reach may crown a pious and successful
ruler, which is the one road to an empire that does not run through conquest.

**Alliances, tribute and coalitions.** Realms take standing positions, not
just temperatures: oaths sworn against a common enemy, marriages between
houses, tributaries that keep their crown and their customs and send money
every year. Every war is declared *for* something — a border, a named city,
tribute, a relic, a conversion, a claimant's throne, plunder — and the peace
reports whether that was achieved. Allies are called into wars worth their
blood, and their armies count on the field. When one realm grows past a fifth
of the settled world, its neighbours and theirs begin to fear it more than
they fear each other, and combine.

Size is its own punishment: a realm is judged on how far its provinces lie
from its seat, so an empire frays at its edge first.

**Stories.** Cruel rulers become tyrants, renowned generals and poets become
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
| `-w, --width W` | Map width: 40–600 cells; default 288. |
| `--height H` | Map height: 20–300 cells; default 144. |
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
