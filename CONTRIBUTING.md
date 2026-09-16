# Contributing

Thanks for looking. This is a small, deliberately self-contained project, so
most of what follows is about keeping it that way.

## Building and running

```sh
cargo build --release
./target/release/empires                       # a new random world
./target/release/empires --seed 42             # replay a particular world
./target/release/empires --headless 800        # no UI, print 800 years
```

`cargo build` must be warning-free. See `README.md` for the full flag list, or
`./target/release/empires --help`.

## The two rules

### 1. Zero dependencies

`[dependencies]` is empty and stays empty. The terminal layer, the noise, the
random numbers, the save format and the language generator are all in the tree,
and the payoff is that the whole game is one `cargo build` from a bare
toolchain, with nothing to audit and nothing to break. If you need something,
write the small version of it here.

This rules out dev-dependencies and build-dependencies too: the tests are
`#[cfg(test)]` modules under `src/`, driven by plain `assert!`.

The only concession is libc, reached through hand-written FFI declarations in
`src/term.rs`, which documents the platform assumptions it makes.

### 2. Determinism

**The same seed must always produce the same history**, and a world reloaded
from a save must continue exactly as it would have. `src/tests.rs` checks both
and will fail loudly if you break them.

In practice:

- Never iterate a `HashMap` or `HashSet` anywhere the simulation can see the
  order. Use `BTreeMap` / `BTreeSet`, or a `Vec` you sorted yourself.
- Keep the order in which cells, realms and people are visited stable wherever
  it feeds a random draw or a tie-break. Sorting by a float needs an explicit,
  total tie-break, not just the float.
- Draw from the RNG the same number of times on every path. An early `return`
  that skips a draw changes every subsequent number in the stream.
- `Rng` clones share one stream, so `w.rng.clone()` is a handle, not a fork.
  That is what makes split borrows of the world safe; do not "fix" it.
- Never use wall-clock time, the environment or the terminal size as an input
  to the simulation. The UI may read them; `sim` may not.

Any change that alters the random draws — a new `rng` call, a reordered loop, a
retuned constant — **changes every seed's history**. That is allowed, and
sometimes necessary, but say so in the commit message and add a
*world-changing* note to `CHANGELOG.md` under **Changed**. Saved worlds will
still load; their futures will simply differ.

## Testing

```sh
cargo test                                     # unit and save-format tests
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo doc --no-deps                            # with RUSTDOCFLAGS=-D warnings in CI
```

Beyond `cargo test`:

```sh
# Drive the real interface in a pseudo-terminal: a few hundred keystrokes,
# every mode, mouse events, save and load. Exits non-zero on a panic.
python3 tools/ptytest.py                       # defaults to target/release/empires

# One rendered frame to <path>.txt and <path>.html, without a terminal.
./target/release/empires --seed 3 --headless 300 --snapshot /tmp/frame --layer political

# Balance metrics per century: population, realms, empires, wars, schools,
# cities, cultures, relics, prophecies, and the largest realm's share.
./target/release/empires --seed 3 --headless 1500 --stats

# Where a year of simulation actually goes, phase by phase.
./target/release/empires --seed 3 --headless 1500 --bench
```

To check determinism by hand, run the same seed twice and diff:

```sh
./target/release/empires --seed 11 --headless 300 --min-importance 0 > /tmp/a
./target/release/empires --seed 11 --headless 300 --min-importance 0 > /tmp/b
diff /tmp/a /tmp/b
```

CI runs all of the above on every push.

## Minimum supported Rust version

`rust-version` in `Cargo.toml` is the MSRV, and CI builds against it. It is
raised only deliberately, in a commit that says so.

Nothing in the tree uses a library or language feature newer than
`std::sync::OnceLock` (Rust 1.70, `src/term.rs`). The MSRV is higher than that
because of type inference, not features: `src/lang.rs` picks from a
`&[&[&str]]` with a nested `rng.pick(rng.pick(VOWEL_SETS))`, and older
compilers cannot infer through that deref coercion (`E0282`, `E0308`). Binding
the inner pick to a `let` first is enough to build on 1.70.

When you touch that line, or the MSRV, re-measure rather than guess:

```sh
rustup toolchain install 1.70.0 --profile minimal
cargo +1.70.0 check --all-targets
```

## Code style

- `cargo fmt` with stock settings; `rustfmt.toml` sets only the edition.
- `cargo clippy --all-targets -- -D warnings` must be clean. Prefer fixing the
  code; where a lint is genuinely wrong, a narrow `#[allow]` with a comment
  saying why beats loosening a threshold in `clippy.toml` for everyone.
- Every module opens with a `//!` block saying what it is for and what rule the
  rest of the tree has to respect when using it. Keep them current — they are
  the map, and `docs/ARCHITECTURE.md` assumes they exist.
- Comments explain *why*. The code already says what.
- Entities live in `Vec`s and refer to each other by index. Those `Vec`s are
  append-only: nothing is removed or reordered, because indices are stored in
  saves, in the chronicle and in the UI. A dead realm is marked dead.
- Player-facing text is written in `sim::prose` or `ui::words`, not inline.

## How to make the common changes

### Add a tuning constant

Balance numbers live in `sim::tuning::Tuning` (`src/sim/tuning.rs`), never as
literals at the use site.

1. Add a documented field to the `Tuning` struct — `f32` where the surrounding
   arithmetic is `f32`, `f64` for a probability passed to `Rng::chance`, `i32`
   for counts of years or cells.
2. Give it the current literal's value in the `Default` impl, so the change
   starts as a pure refactor.
3. Replace the literal with `w.tuning.your_field`.
4. Only then change the value, and check it with `--stats` over 1500 years and
   several seeds. Changing a default is *world-changing*: note it in the
   changelog.

Purely cosmetic or structural numbers — list sizes, display clamps, text
choices — stay where they are.

### Add a save-file field

`src/ser.rs` carries the full rule in its module docs. The short version:

1. Bump that section's version in the `sections!` table. Never reuse or
   renumber a tag.
2. Write the field unconditionally; read it conditionally on `s.ver()`:

   ```rust
   fn city<S: Io>(s: &mut S, c: &mut City) {
       s.string(&mut c.name);
       if s.ver() >= 2 {
           s.f32(&mut c.harbour); // absent from version-1 files: stays default
       }
   }
   ```

3. A whole new section is a new tag in the table, not an addition to
   `read_v1`, which is frozen.
4. Add a round-trip case to `src/ser_tests.rs`. Do not regenerate
   `tests/fixtures/v1.rfe` — it exists precisely to prove old files still load.

The format version at the top of the file changes only when the *framing*
changes, which is a far bigger deal than a field.

### Add a `:` command

`src/ui/commands.rs`. `run_command` splits the line into a command and an
argument and offers it to each group in turn — `cmd_files`, `cmd_sim`,
`cmd_view`, `cmd_search`, `cmd_config`, `cmd_misc`. A group returns `Cmd::Done`
or `Cmd::Quit` if it handled the line, and `Cmd::Unknown` to pass it along.

Add an arm to the one `match` where it belongs, list every abbreviation in the
pattern (`"recap" | "digest" | "lately"`), report the outcome with
`self.say(...)`, and add it to the `HELP` table in `src/ui/detail.rs` and to
the command table in `README.md`. If it is a setting that also belongs in the
config file, wire it through `src/config.rs` as well.

### Add a chronicle event

Events are structured data with the prose already rendered, so there are two
halves.

1. **The sentence** goes in `src/sim/prose/`, in the submodule matching the
   subsystem (`politics`, `war`, `magic`, `people`, `events`, `stories`,
   `genesis`). Use the helpers there for articles, plurals, pronouns and lists
   instead of writing grammar by hand, and follow the naming rule: a realm is
   introduced by its full name ("the Kingdom of Velen") the first time it
   appears in an event and referred to by its short name ("Velen") afterwards
   (`realm_full` and `realm`).
2. **The event** is logged from the simulation:

   ```rust
   w.log(importance, EventKind::Battle, &[Ref::Polity(a), Ref::Polity(b)], loc, text);
   ```

   `importance` runs from 0 (trivia) to 3 (an age turns); it decides what the
   feeds show and what survives the chronicle cap. List *every* entity involved
   in `refs` — that is what puts the line on each of their detail pages — and
   pass the cell it happened in as `loc` so the map can mark it.

If a prose helper needs variety it must respect determinism: either take the
world RNG and draw *exactly once* per choice (`Pick::rolled`), or derive the
choice from the year and an entity id and draw nothing at all (`Pick::stable`).

### Add an explanation

`src/sim/explain.rs` is a pure reading of `&World`: it may not mutate anything
and may not touch the RNG. Its weights mirror `politics::economy` and
`war::strength`, so if you change how stability or a battle works, change the
explanation in the same commit. An explanation that disagrees with the
simulation is worse than none.

## Commits

- One logical change per commit, with the tree building and the tests passing
  at each one.
- Subject line: imperative mood, no trailing full stop, under about 72
  characters. A module prefix where it helps (`ser: a chunked save format that
  can grow`), a plain sentence where it does not (`Make a year of simulation
  cost the map, not the map times the realms`).
- The body says *why*, and must call out anything *world-changing* (see
  Determinism) and anything that changes the save format.
- Update `CHANGELOG.md` under `[Unreleased]` in the same commit as the change
  it describes.
