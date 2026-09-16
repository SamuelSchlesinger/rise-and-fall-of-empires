# tools

Small scripts that support development. None of them is needed to build or run
the game, and nothing in `src/` depends on them. They are Python 3 and POSIX
`sh` only — the zero-dependency rule applies here too.

These files are excluded from the published crate (see `exclude` in
`Cargo.toml`); they live in the repository, not in the package.

## `ptytest.py` — drive the interface in a pseudo-terminal

`cargo test` covers the simulation, the save format and the plain-words
mappings, but it cannot press a key. This does: it starts the game on a pty,
sizes the window to 160x45, and sends a few hundred keystrokes through every
mode — motions and counts, zoom and layers, search, the `:` commands, lists and
detail pages and their links, the chronicle, help, the Hand of Fate, mouse
clicks, wheel and drag, a bracketed paste, a save and load round trip, `:new`,
the in-app guide and tutorial, and finally an interrupted `ZZ` followed by
the two `q`s that really quit.

```sh
cargo build --release
python3 tools/ptytest.py                  # target/release/empires by default
python3 tools/ptytest.py path/to/empires --seed 9
python3 tools/ptytest.py --verbose        # print the output tail on failure
python3 tools/ptytest.py --keep           # keep the throwaway HOME
```

It exits 0 only if the game panicked nowhere, exited 0, entered *and left* the
alternate screen, quit when asked, and produced a plausible amount of output.
Anything else is exit 1 with a `FAIL:` line saying which, and a panic is
printed with its surrounding context. That makes it usable from CI, which runs
it on every push.

The game is run inside a throwaway `HOME` with `XDG_CONFIG_HOME`,
`XDG_DATA_HOME`, `XDG_STATE_HOME` and `XDG_CACHE_HOME` pointed into it, so the
first-run marker, the config file and the saves it writes cannot touch yours.
The directory is removed afterwards unless you pass `--keep`.

When adding a key or a `:` command, add it to `KEYS` near the feature it
belongs with. Keys that would quit belong in `QUIT_KEYS` at the end, otherwise
the run stops early and the test reports it.

## `termtest.py` — check native terminal behaviour

```sh
python3 tools/termtest.py                  # target/release/empires by default
python3 tools/termtest.py path/to/empires
```

Checks the libc bindings against a real pseudo-terminal on Linux and macOS:
raw-mode flags and control characters, window resizing, input without Enter,
and exact restoration of the original settings after normal exit, SIGHUP,
SIGINT and SIGTERM. Configuration and data stay in a temporary directory.
CI runs this on Linux, Apple Silicon and Intel Macs.

## `screenshot.sh` — regenerate `docs/screenshot.png`

The game can render one frame without a terminal: `--snapshot PATH` writes
`PATH.txt` (plain text) and `PATH.html` (one `<span>` per cell, carrying the
real 24-bit colours). This script does that and points a headless browser at
the HTML to get the PNG the README shows.

```sh
tools/screenshot.sh                # seed 7, year 420, the political layer
tools/screenshot.sh 42 900 culture # seed, years, layer
```

It uses whichever of `firefox`, `chromium` or `google-chrome` it finds. There
is deliberately no image code in the tree, so without a browser you get the
text frame and no PNG:

```sh
./target/release/empires --seed 7 --headless 420 --snapshot /tmp/frame --layer political
cat /tmp/frame.txt
```

`docs/*.html` and `docs/*.txt` are ignored by git; only the PNG is checked in.
