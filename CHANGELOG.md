# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

While the version is below 1.0 the save format and the shape of a generated
world may change between releases. A change that makes a given seed produce a
different history is noted here under **Changed** as *world-changing*, because
it invalidates saved worlds' futures and every seed anyone has written down.

## [Unreleased]

Understanding a world, and reaching into it.

### Added

- **Eight new map layers, in two groups.** Four say how people live on the
  ground, from data the simulation already kept and never drew: **goods**
  (what each cell yields — the only way to learn a stretch of country was
  salt land was to open every city on it one at a time), **trade** (what the
  roads carry, drawn as a network, bright where busy and dark where a war
  has shut them), **harvest** (fertility, this century's weather and what is
  known, multiplied — the number that decides where anybody can live) and
  **knowledge** (the local technology multiplier, which is where a dark age
  finally looks like one).

  Four more say what is *moving*: **settling** (where people are arriving
  and leaving), **fighting** (where the war actually is, rather than who is
  nominally at war), **frontier** (where the map is being redrawn) and
  **drift** (where the weather has turned since living memory). Every layer
  of the map used to be a photograph — true of one year and silent about the
  year before — so a steppe emptying out and a steppe that had always been
  empty looked the same.

  Fourteen layers is too many to cycle one at a time, so Tab steps between
  families (power, peoples, land, living, motion) and `\` steps the maps
  within one.

- **A carrying layer**, showing where the trade has just been won and lost:
  gold along a road newly opened, dark red along one a war has shut, and the
  whole corridor rather than the two towns at its ends. The network already
  knew when a road changed state — it is what stops the chronicle reporting
  the same two cities every three years — and the map had no way to show it.

- **A memory layer and a settled layer.** Memory shows where in the world
  history actually happened, from every event the chronicle still holds,
  with great events counting for more than small ones — and it is named for
  what it is, since compaction means it shows the world's *memory* rather
  than its whole past. Settled shows how long each stretch of country has
  been held by whoever holds it: dark earth newly taken, pale ground settled
  for centuries. `:layer memory`, `:layer settled`.

- **A relations layer.** The political layer says who owns what and nothing
  about who is sworn to whom, so the whole of diplomacy — tension, stances,
  tributaries, hegemony — was readable one realm page at a time and nowhere
  else, though it is among the largest systems here. With a realm selected
  the layer shows the diplomatic world *from there*: who it is fighting, who
  is sworn to it, who has married in, who pays it tribute and whom it pays,
  with the neutrals shaded by how angry they are, since those are the next
  war if there is going to be one. With nothing selected, each realm's own
  standing. `:layer relations`, or `diplomacy`, `alliances`, `tension`.

- **The world keeps a record of itself**, and the recap draws it. Eleven
  series sampled every ten years — population, realms, cities, wars,
  schools, trade, development, stability, treasury, income, decadence — as
  sparklines, each scaled to itself because they share no units. `:chart`,
  or `:recap`. `:export series` writes them as CSV.

  It is stored in a save chunk of its own rather than added to the existing
  one, because a build that meets a newer version of a section it knows
  drops that whole section: bumping `stat` would have cost an older binary
  `peak_pop` and `pop_history` too, while a tag it has never heard of costs
  it nothing. `src/sim/flows.rs` argues against keeping history and is right
  about what it describes — a per-cell window is megabytes — but a sample is
  one *world*, eleven numbers a decade, about eight kilobytes for the
  longest game anybody will play, and unlike a flow field it cannot heal: no
  amount of running forward recovers what the population was in the eighth
  century.

  The count of chronicle entries compaction has thrown away rides in the
  same chunk. The chronicle is rebuilt from its events on load, through
  `Chronicle::default`, so a world that had forgotten half a million events
  came back claiming to have forgotten none.

- **A rise-and-fall timeline**, which is the view the game is named for and
  the one it could not draw. Every realm as a bar from its founding to its
  fall across the whole of history, the great ones first, with successor
  states indented under the realm they broke away from — so a kingdom that
  shattered into four reads as a family rather than four unrelated rows.
  `e` then Tab to **Timeline**, or `:list timeline`; `:export timeline`
  writes it as Markdown. It needed no new state: `founded`, `fell`,
  `parent`, `peak_cells` and `peak_year` have all been recorded since long
  before there was anything to draw them with.

- **A world remembers its own motion.** Four signed fields, one value per
  cell, each a decaying running total of something that happened rather than
  something that is. A window over the last N years would have meant N
  snapshots of a forty-thousand-cell map to carry and to save; a running
  total with a half-life carries one number per cell, and needs no place in
  a save file — these fields are not derived, but they heal, so a loaded
  world has them back within a lifetime of play.

- **A Money block on every realm's page.** Where the money comes from and
  where it goes, with the figures adding up to the line printed under them
  and, for a realm running down, how many years of treasury it has left.
  Stability has had an explanation since the beginning; money never did,
  which left the two economic entries in that block as the only visible
  economics in the game, both of them effects with no stated cause.

- **A list can be asked a question.** The filter matched the text of a
  rendered row; a term is now a word to look for *or* a comparison against a
  named quantity — `lands>200`, `income<0`, `prosperity>=150`. Each page
  advertises what may be asked of it.

- **Two new list pages.** **Wealth** ranks realms by what they hold with
  what the year will bring beside it, because a full treasury with a deficit
  is a different thing from the same treasury with a surplus. **Roads** is
  the trade network as a list that can be sorted and filtered, which is the
  only way to find the one road a war has shut.

- **The Hand of Fate reaches more than realms.** Twenty-four interventions
  across four kinds of thing: a realm, a town, a person, or a region. A town
  can boom, burn, be walled, raise a wonder, take a sickness off a ship or
  find the carrying trade. A person can be made renowned or disgraced,
  brilliant or ambitious, can be killed, or can be given a child who will be
  remarkable. A region can be made fertile or exhausted, blighted, struck
  with ore, quickened with ley — or emptied, which is the only way to start
  a dark age on purpose, because knowledge belongs to the ground and ground
  that empties of people forgets.

- **`:export`** writes a world out for somebody who does not have the game:
  the chronicle as Markdown grouped by century, the map as HTML, or any list
  page as CSV with a column for every quantity its filter advertises.

### Changed

- **A treasury has no ceiling, and three drains instead.** *World-changing.*
  Gold was pinned at six hundred, and the ceiling destroyed every fact about
  wealth above it: eight of the ten richest realms sat at exactly the cap,
  each drawing the identical bonus to its stability. What a cap was standing
  in for is a drain — a treasury's only cost was upkeep, which scales with
  the army and the land and not at all with how full the coffers are. Now a
  **court spends what it has**, buying prestige and *decadence*; **corruption
  skims the tax roll**, from a rotten court and from the distance taxes must
  travel; and **gold raises soldiers**, who then cost upkeep for as long as
  they stand. Confidence in a full treasury saturates rather than dividing
  by a ceiling, so there is a gradient all the way up.

  The effect is that the richest realms are now the ones about to have
  trouble, which is the story this game has always been about and could not
  previously tell.

- **Trade grows with the world.** *World-changing.* A route's worth came
  from the goods on the ground and the distance between two cities, both
  fixed for ever, while city income compounds — so trade's share of what
  realms earned fell from a quarter in the second century to a fortieth by
  the twenty-eighth, purely by standing still. Added the mass half of a
  gravity model, whose distance term was already there. Trade now holds near
  a fifth of realm income, flat across the ages.

- **A world can learn to carry goods.** *World-changing.* Twelve kinds of
  effect a generated tree could hand out, and not one touched a road: a
  realm could invent writing, coinage and the ocean-going ship and its
  caravans carried what they carried in year one. There is a thirteenth now,
  and five of the ten fields can produce it.

- **A world carries only so many traditions.** *World-changing.* See below.

- **Stability is bent towards its ceiling too.** *World-changing.* Its
  target sums thirteen terms, and a settled, rich, well-ruled kingdom at
  peace clears 1.0 on them without difficulty — so the top tenth of realms
  in a mature world all sat at exactly 1.0, reading identically: the same
  drift word, the same "pulls towards 100%", for a realm at 1.02 and one at
  1.6. Realms at the ceiling fall from the whole top decile to under one in
  a hundred, and the ninetieth percentile from 1.00 to 0.92.

  `explain` also clamped the *sum* while the simulation clamped the
  *value*, so above 1.0 the explanation and the world disagreed and the
  page understated the real pull. Both go through the same bend now, and
  when it bites the page says so: "everything together adds to 128%, which
  settles at 96%".

- **Prosperity is bent towards its ceiling rather than clipped against it.**
  *World-changing.* Everything that makes a city rich was added up and the
  sum then clamped, so by the twenty-first century the *median* city in the
  world sat at exactly the maximum: more than half of them had their income
  decided by population alone, and prosperity had stopped telling cities
  apart. It also made the saturating trade term pointless for the cities it
  mattered most to, since their target was already over the cap. The
  pile-up at the top falls from 56% of cities to 7%.

  That is the fourth place this simulation had made the same mistake — sum
  every advantage a thing has, then clamp the sum — after the treasury, the
  confidence a treasury buys, and the prosperity a city draws from its
  traffic. There is one shape for it now, `sim::soft_ceiling`, and a test
  that holds prosperity to it.

- The prosperity a city draws from its traffic saturates rather than
  stopping at a hard ceiling, so the difference between a good position on
  the roads and a commanding one no longer disappears.

- A city is taxed on the year's opening prosperity and its towns grow after
  the assessment rather than before it. *World-changing*, and what makes the
  Money block checkable against the treasury it describes.

### Fixed

- **Schools compounded without limit**, and a teaching with nowhere to go
  lived for ever. Every school rolled a flat yearly chance to schism, so the
  count grew by a fixed fraction per century — seventeen hundred living
  schools against four hundred realms by the thirty-fourth century — and the
  ordinary way a school ends needs a larger cousin holding ground where it
  still stands, which one that had retreated somewhere no cousin reached met
  never. Since every school rolls against every realm it touches, and each
  persecution makes a martyr — a person created already dead and appended to
  the person list for ever — the person list reached three and a half
  million while barely a thousand people were alive, and a large map went
  from two milliseconds a year to unbounded. Schism now answers to how
  crowded the ground is, and a tradition that has been fading for
  generations with nothing to be folded into is forgotten.

- **The stability breakdown had drifted from the simulation.** Its treasury
  term was a second copy of the old ratio, and went on dividing by a ceiling
  that no longer existed — so a realm holding three thousand was told its
  full treasury was worth a hundred and four points of stability out of a
  hundred, printed at the top of the page as the reason for everything.

- **A road forgot that a war had closed it.** Every twentieth year the
  network was rebuilt with every road marked open as of that year, which
  also reset the clock that stops the chronicle reporting the same two
  cities every three years — so a rebuild silenced those reports for a
  decade.

- Sacking a city was the one way into a treasury that did not enforce the
  ceiling the other three did.

- Prosperity is no longer printed as a percentage. It runs to two and a
  half and is not a share of anything, so the city page read "194%
  prosperity", which invites a reader to wonder 194% of what. It is an index
  now, with "ordinary is 100" said beside it. And a city sacked once is
  sacked "once" rather than "1 times".

- A sentence may begin with a figure. "26% of the world's settled land now
  lay under the Lafulannic Kingdom" is good English and the prose check
  rejected it.

## [0.3.1] - 2026-09-17

A world that ran for five thousand years slowed to a crawl, and the default
map was smaller than it needed to be. Both are fixed, and the second is only
possible because of the first.

### Changed

- **The default map is now 288x144 rather than 160x64** — four times the
  world. Two cells wide for every one tall, which is what draws square: at
  zoom 1 a world cell is one terminal character, and a character is about
  twice as tall as it is wide. Steady cost is about 2.5 ms a year, so the
  fastest speed still keeps up with room to spare. *World-changing*: a seed
  gives a different world than it did at the old size.

- **The chronicle's cap is now real.** Events of importance 2 used to be
  undroppable, which meant that on a long game the cap was a fiction — a
  large map at the eightieth century held four hundred thousand events
  against a cap of sixty thousand, because by then almost nothing left could
  be dropped. Importance 2 is trimmed oldest-first when it must be; only
  importance 3, the naming of an age and the rise and fall of a hegemony, is
  kept whatever happens. A compaction that cannot reach its target now waits
  proportionally longer before trying again, so a futile scan costs a
  constant amount of work per event rather than a growing one.

- **A world carries only so many traditions at once.** Every school rolled
  the same yearly chance to schism whatever the state of the world, so the
  count grew by a fixed fraction per century for ever: seventeen hundred
  living schools against four hundred realms by the thirty-fourth century.
  Schism now answers to how crowded the ground already is, through the new
  `schools_per_realm` dial. *World-changing.*

### Fixed

- **The simulation no longer gets slower the longer it runs.** A large map
  went from two milliseconds a year in its first century to twenty-three in
  its fiftieth and grew without bound after that. It is now flat from about
  year fifteen hundred to year ten thousand, the length of the longest game
  anyone is likely to play. The causes were all the same shape — work
  proportional to the length of a world's history rather than to what is
  alive in it:

  - Naming a war counted the wars that already carried the name by walking
    every war ever fought, building a string per war to compare against.
    Sixteen thousand of them by year three thousand, on every declaration.
    Kept as a tally instead.
  - The phases that age and kill notables, that remember the dead as
    legends, that resolve wars, that work out every realm's reach over water
    and that sweep the magical schools all walked the whole of a vector to
    find the few thousand entries that were alive. They read the living
    indexes now, and wars have an index of their own.
  - **A teaching with nowhere to go lived for ever.** The ordinary way a
    school ends is absorption into a larger school of its own kind holding
    ground where it still stands. One that had retreated somewhere no cousin
    reached met that condition never, and never fell below the extinction
    floor either, so it survived holding a tenth of one realm and went on
    rolling against every realm it touched. Six hundred of those had
    accumulated by the hundredth century, and since each one could persecute
    and make martyrs — people created already dead, appended to the person
    list for ever — the person list reached three and a half million while
    barely a thousand people were alive. A school that has been fading for
    generations with nothing to be folded into is now forgotten.
  - Joining a house compared the newcomer against every name the house had
    ever carried, and closing a spent house walked the same list on every
    death. Both are searches now.

- **The interface no longer gets slower the longer it runs.** The
  storyteller that fills the sidebar walked every person who had ever lived
  and every war ever fought, sixty times a second. The list pages sorted the
  whole of history and rendered all of it to text on every frame, which cost
  the Wars page nine milliseconds a frame by the eightieth century. Both
  read the living indexes or a bounded best-of now, and a page that is a
  window on more than it shows says so in its footer.

- **The map cursor is visible again.** It was drawn with reverse video
  alone, which exchanges a cell's own two colours — and on a shaded map
  those are often nearly the same. Deep water is drawn in (5, 17, 41) on
  (4, 13, 31), eighteen apart out of seven hundred and sixty-five, and
  better than a quarter of the map is within a fifth of that. Swapping them
  changed nothing anyone could see, so the cursor vanished over open sea and
  over any flat stretch of country. It is now forced to a contrasting pair
  rather than an exchanged one.

## [0.3.0] - 2026-09-17

History acquires a direction. Until now everything in the world cycled:
realms rose and broke, faiths spread and faded, and a map at year fifteen
hundred was arranged differently from one at year three hundred without being
different in kind. Nothing accumulated. This release gives the world five
things that do.

### Added

- **A tree of knowledge, grown per world.** Every world works out its own
  hundred and twenty innovations, named in its own idiom, arranged in tiers,
  each needing what came before it. No two worlds learn the same things in
  the same order. What stays universal is the vocabulary of *consequences*,
  because the simulation has to read a tree it did not write — so every world
  eventually works out something that makes walls stop mattering, and none of
  them call it the same thing.

  **Knowledge belongs to the ground, not the realm.** A kingdom that falls
  leaves its irrigation, its roads and its writing behind for whoever takes
  its land. It is the only thing in the world that outlives the state which
  built it. Ground that empties of people forgets, which is how a dark age
  happens and how something comes to be worked out a second time.

  Three things gate discovery: the ground must suit it, the prerequisites
  must be known, and a city must be able to support it. The third is what
  paces a tree across ten thousand years rather than four hundred — what
  gates invention is not time but people fed well enough that some of them
  can do something other than farm.

- **Culture decides what a people can learn.** Each has a bent per field with
  real strengths and real blind spots, drawn from what it values and
  inherited with drift by its daughters. It gates discovery and adoption
  both, which is why a technique can stall for ever at a cultural border that
  a trade good crosses in a season — and why a region keeps a recognisable
  character across the rise and fall of its realms. Without it the world
  converged on one body of knowledge by year two thousand; with it, realms
  still hold 21, 111 and 115 innovations at year ninety-seven hundred.

- **Goods, and roads between them.** Twelve goods sit where the terrain puts
  them — ore in the ore country, spice in the hot jungle, salt where the sea
  meets dry land. Cities trade when each has something the other's hinterland
  lacks, by land or by sea. Wealth becomes **positional**: the busiest city
  takes four times the median because it sits between places that want what
  each other has. A war closes the roads between the realms fighting it and
  the wealth drains out of cities that never saw a soldier, which is what
  lets a collapse propagate instead of staying local. Routes carry ideas as
  well as cargo, so a technique crosses a sea years before it crosses the
  mountain range behind the port.

- **Blood.** Two alleles a trait, one from each parent, kept whole rather
  than averaged. A rare strain can be carried unseen for generations and
  surface in a child whose parents showed nothing of the kind. A house that
  marries its own to keep a claim concentrates what it carries and loses
  vigour — frailer children, shorter lives, fewer of them; one that marries
  out gets it back. The person page shows what somebody carries without
  showing it.

- **Rivalry and patronage.** People who hold no throne now have ties to each
  other. Two of comparable standing in the same trade, in one realm or across
  a border they share, find themselves measured against one another — and
  every later achievement by either is an event about both, up to and
  including the one who is losing deciding that the shorter road past a rival
  runs through them. Somebody of standing takes up somebody young, who begins
  with a quarter of their patron's renown and sometimes outgrows them.

- **Weather that lasts longer than a lifetime.** Bands of wet and dry drift
  across the map over centuries. It does one thing — moves the carrying
  capacity of land — and everything else follows, because the simulation
  already knows what to do when land stops feeding people. A wet century
  pushes farming into the margins; a dry one pushes the margins back onto the
  farmers, and the people who live where the grass fails are the ones with
  horses.

- Ages name themselves after the innovation that turned them, where one did.

### Changed

- *World-changing.* Every seed produces a different history.
- **The brakes on the size of a realm were all relative, and knowledge raised
  every ceiling at once**, so a realm far enough ahead escaped administrative
  capacity, distance and foreign subjects together and took the world by year
  two thousand. Three fixes: diminishing returns on every accumulated
  advantage; governing reach separated from expansion reach, since sprawl was
  measured against a generous reach that scales with development and a
  developed realm therefore always scored below the threshold; and an
  absolute cost for sheer size, because ruling half of everything is hard
  because it is half of everything. Four seeds now hold between 9% and 28% of
  the world across six thousand years.
- Tension relief from trade fires on goods actually moving between two
  realms, rather than on two cultures both merely valuing commerce.
- Plagues spare a realm that knows how to keep water clean.

### Fixed

- The prose well-formedness test ran a hundred years of a small world, which
  reached none of the events this release adds. It runs five hundred now,
  which is how a lost line continuation printing ten spaces mid-sentence came
  to light.
- A realm founded in year two, when five cells in all were settled, held every
  one of them and recorded a peak share of 100% for ever after.

### Performance

- A year costs about 0.6 ms at 160x64; ten thousand years of a default world
  runs in seven seconds.
- The per-cell yield multiplier is cached, since working it out from the
  knowledge bitset meant walking the whole tree for every cell every year.

## [0.2.0] - 2026-09-17

The world now has people in it, and a sea. Rulers marry, have children the
chronicle watches grow up, and are called great while they are alive to hear
it; realms take standing positions towards one another instead of only
temperatures; an empire that grows too large manufactures the coalition that
pulls it down; and the water between two shores is a distance rather than a
wall.

### Added

- **The sea.** Where the water can be crossed is worked out once, when the
  world is made: every pair of coastal cells on different landmasses within
  reach of each other over open water. How wide a crossing a realm can manage
  then depends on its people's seafaring, its own development and what kind of
  realm it is — a republic on the water reaches furthest. A strait is within
  reach of almost anyone; only an old, developed naval power reaches the far
  isles.

  Two realms facing each other across water are now **neighbours**: quarrels
  build between them and armies can be sent. Before this the sea was a wall
  rather than a distance — realms on opposite shores were not neighbours at
  all, so no tension ever built, so no war was ever declared, and an army
  could see a coast it could never be sent to. Over fifteen centuries nought
  to two realms in an entire world ever held land on two landmasses.

  Landing on a hostile shore is the hardest thing an army does and a third to
  a half of landings are thrown back into the sea. A realm that controls the
  water is bound together by it rather than stretched across it, so an
  overseas province counts as nearer to its capital than the map says — which
  is what makes an empire on both sides of an ocean possible at all rather
  than merely reachable and then immediately punished for it.

  Crowded coastal peoples also take narrow straits on their own, without any
  state's shipwrights, so islands get peopled and there is something out there
  to colonise or to conquer. A test asserts the property the whole system
  exists for: every landmass worth settling can be reached from the largest
  one by a chain of crossings, so a realm that masters the sea can in
  principle reach the whole world.

- **Houses.** A dynasty is an entity rather than a string on a realm, with its
  own page: the whole line of succession down the centuries, every throne it
  has held, where it forked into cadet branches, and who of it never ruled. The
  tree is drawn down *time* rather than down descent, so a thousand-year family
  stays one column wide and fits a terminal. New `Houses` and `Figures` lists
  (`e`, or `:list houses`), and `[H]`, `[m]`, `[v]`, `[O]`, `[b]` links between
  people, houses, spouses and overlords.
- **Kin that exist before they matter.** Rulers marry at home or into a
  neighbouring house; children are born, named, and grow up with traits of
  their own; some do not survive childhood. A succession draws on people the
  reader has already met.
- **Inheritance customs** drawn from a culture's values: primogeniture,
  partition, tanistry, or election. Partition divides a great realm among the
  heirs, which is how a conqueror's work is undone by his own law. A passed-over
  sibling may take the provinces with them.
- **The Diadochi.** An acclaimed conqueror who leaves no grown heir may have
  the realm divided among the generals who marched with him, each founding a
  line. Realms that shatter now name whoever crowned themselves in each
  fragment.
- **Personal union.** A consort who is the nearest heir of a realm with none of
  its own brings both crowns under one head, with no battle.
- **Standing, scored every year.** What a life amounts to, in points, itemised
  on the person's page. Weighted towards the rate of conquest rather than the
  total, so a figure is acclaimed young with a lifetime left to be followed.
  The world names them great in a line of its own, naming the deed that earned
  it, rather than in their obituary.
- **Coronation by a faith**: a state faith with reach may crown a pious and
  successful ruler, granting a title they keep and the one road to an empire
  that does not run through conquest.
- **Alliances, rivalries and dynastic ties** as standing arrangements, formed
  against a common enemy and allowed to lapse when the reason for them goes.
- **Tribute.** A defeated realm can be made a tributary: it keeps its crown and
  its customs, pays a share of its income, and tests its overlord's grip when
  that overlord looks weak.
- **Coalitions.** A realm holding more than a fifth of the settled world is
  recognised as the power of the age, and its neighbours and theirs come to fear
  it more than they fear each other. Allies are called into wars worth their
  blood and their armies count on the field.
- **War aims.** Every declaration states what it is for — a border, a named
  city, tribute, a relic, a conversion, a claimant's throne, plunder, or
  breaking the hegemon — and every peace returns a verdict on it, including the
  victory that did not get what it came for.
- Figures and spent houses in the fifty-year recap; the great houses and a
  count of figures per century in `--stats`, with the largest realm's stability
  broken down by `explain` so the two cannot silently disagree.

### Changed

- *World-changing.* Every seed produces a different history. Nearly every draw
  in the simulation has moved.
- **Size is its own punishment.** A realm is judged on how far its provinces lie
  from its seat (`stability_sprawl_weight`). Administrative capacity rises with
  the cities a conqueror takes, so conquest used to pay for its own
  administration and nothing could stop a realm that got ahead; distance does
  not work that way, and an empire now frays at its edge first. Great realms
  reach 37–63% of the settled world at their height and then break, where they
  previously plateaued around 10%.
- **Wars are fought by sides, not realms.** Front lines, field strength,
  casualties and captured ground all account for everyone present, with a
  penalty for the divided command of a coalition.
- Battles, rulers, cities and wars are credited to the *person* as well as the
  realm, so the chronicle still knows four centuries later who took the land.
  Land taken in war is counted apart from land settled.
- Everyone who is not on a throne now grows old and dies. Only rulers had a
  death roll, so a king's younger children never died: a house accumulated
  immortal claimants for a thousand years.
- Relic regalia raise the stability a realm *tends toward* instead of being
  added to its stored stability every year. As a yearly addition it was a
  ratchet rather than a bonus, and a realm with a few crowns climbed to total
  stability however badly it was governed.
- Wars are numbered by name rather than by the pair fighting them, so three
  different realms no longer each fight a "First War of the Ngerrilm Plain",
  and a war named for its aim says so.
- Non-neighbour tension is bounded and expired truces are swept, so a realm
  weighs the powers that matter now rather than every power that ever
  frightened it.
- Config errors are reported in headless and snapshot runs, which have no
  status line to show them on.

### Fixed

- **Lost relics could never be found again.** Every caller that loses a relic
  records where it fell, and `artifact_passes` then cleared that record — so
  relics sacked, buried with their owner or lost in a realm's ruin were gone for
  good, and only relics *born* lost could ever be dug up. Rediscoveries over
  three thousand years go from 1–24 to 143–161, and the world's stock of relics
  grows instead of draining away.
- `Polity::house` and `Polity::sprawl` are saved. Without them a reloaded world
  had no dynasties and governed its first year on a zero, and diverged from the
  world that wrote the file.
- A fifty-year recap can no longer overrun the page it was given.
- The check for "1 lands" no longer fires on the tail of a decimal.
- Sea routes are rebuilt defensively, so a corrupt save cannot index a
  terrain past the area it claims to cover.

### Performance

- Living realms and living people are maintained as indexes beside their
  append-only vectors, the way `owner_cells` already was. Phases that filtered
  the raw vectors cost the whole history of the world every year.
- The sea costs about 3% of a year. Because the terrain never changes, the set
  of possible crossings is static: it is built once at worldgen (6–30 ms) and
  rebuilt on load rather than stored, and every question about the sea after
  that is a lookup in a sorted list of a few dozen routes.
- Greatness is scored without building the sentences that explain it; the prose
  is written only when somebody is going to read it.
- A year costs about 0.40 ms at 160x64 over 1500 years, against 0.26 ms before
  this release, and no phase grows with the age of the world.

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

[0.3.1]: https://github.com/SamuelSchlesinger/rise-and-fall-of-empires/releases/tag/v0.3.1
[0.3.0]: https://github.com/SamuelSchlesinger/rise-and-fall-of-empires/releases/tag/v0.3.0
[0.2.0]: https://github.com/SamuelSchlesinger/rise-and-fall-of-empires/releases/tag/v0.2.0
[0.1.0]: https://github.com/SamuelSchlesinger/rise-and-fall-of-empires/releases/tag/v0.1.0

[0.1.2]: https://github.com/SamuelSchlesinger/rise-and-fall-of-empires/releases/tag/v0.1.2
[0.1.1]: https://github.com/SamuelSchlesinger/rise-and-fall-of-empires/releases/tag/v0.1.1
