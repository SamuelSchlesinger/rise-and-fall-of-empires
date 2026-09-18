# Rise and Fall of Empires

A world simulator for your terminal, and one god watching it. Peoples settle,
kingdoms rise, wars reshape borders and empires fall on a procedurally
generated map, none of it waiting for you.

Then you bind yourself to a people, and their lives become your strength.
The more of them there are, the further you can reach into the world — a
harvest, a fire, a wonder, a knife — and every act has a price. Help them
and you stay able to help anybody. Spend on a stranger and you have given
something up. Lose them all and the altars go cold.

Pause to explore a city, follow a dynasty across nine centuries, or read the
last fifty years of history. Or bind yourself to nobody, leave it running,
and see what survives.

Written in Rust with **zero crate dependencies**. Runs on **Linux and macOS**.

![The political map, sidebar and chronicle in year 420](docs/screenshot.png)

*Seed 7, year 420.*

## Install

Download an archive from [Releases](https://github.com/SamuelSchlesinger/rise-and-fall-of-empires/releases/latest)
for your machine:

| Platform | Archive suffix |
| --- | --- |
| macOS, Apple Silicon | `aarch64-apple-darwin` |
| macOS, Intel | `x86_64-apple-darwin` |
| Linux, x86_64 | `x86_64-unknown-linux-gnu` or `x86_64-unknown-linux-musl` (static) |
| Linux, ARM64 | `aarch64-unknown-linux-gnu` |

For example, on an Apple Silicon Mac:

```sh
tar xzf empires-v0.3.0-aarch64-apple-darwin.tar.gz
./empires-v0.3.0-aarch64-apple-darwin/empires
```

Or install from a checkout with Rust 1.70 or newer:

```sh
cargo install --path .       # installs empires in ~/.cargo/bin
```

Use a terminal with 24-bit colour, such as kitty, WezTerm or Ghostty.
A window of at least 100×30 shows the sidebar; smaller windows still work.
Windows and the BSDs are not yet supported.

## Start a world

```sh
empires                         # a new random world
empires --seed 42                # choose a world seed
empires --save world.rfe         # autosave every 100 years and on quit
empires --load world.rfe         # continue a saved world
empires --seed 42 --headless 800  # print 800 years of history without the UI
```

On first launch the world asks whose god you will be: press a number to bind
yourself to a people, or **0** to watch and nothing more. Press **t** instead
for a short, optional tutorial or **p** for the player guide; reopen them any
time with `:tutorial` or `:guide`, and change who you serve with `:bind`.

Time starts at **2 years per second**, with event following off so the map
stays where you put it. Press **Space** to pause and **?** for help.
The bottom rows always show the current screen’s main controls.
The map legend explains its symbols; the sidebar describes the selected
place and the world's current stories.

## Essential controls

| Key | Action |
| --- | --- |
| Space / `.` | Pause or resume / advance one year |
| `+` / `-` | Speed up / slow down |
| Arrows or `h j k l` | Move the cursor |
| Enter | Inspect the place under the cursor |
| Tab / Shift+Tab | Change map layer |
| `zi` / `zo` | Zoom in / out |
| `e` / `c` / `r` | Entity lists (realms, houses, figures, …) / chronicle / recap |
| `x` | Reach into the world where the cursor is, at a price |
| `/` | Search by name |
| `:` | Enter a command, such as `:w`, `:speed 25` or `:theme paper` |
| Esc | Go back |
| `?`, then `p` or `t` | Read the in-app guide or start the tutorial |
| `q` twice or `ZZ` | Quit, saving if a save file is set |

Click to select a place, click again to inspect it, and scroll with the wheel.
The [player guide](docs/GUIDE.md#playing) covers every key and command.

## Common options

| Option | Purpose |
| --- | --- |
| `-s, --seed N` | Choose a world seed. |
| `-w, --width W` | Map width: 40–600; default 288. |
| `--height H` | Map height: 20–300; default 144. |
| `-d, --detail LEVEL` | Chronicle detail: `low`, `medium` (default), or `high`. |
| `-o, --save FILE` | Enable saving and autosaving. |
| `-l, --load FILE` | Continue a saved world. |
| `--headless N` | Run N years and print the chronicle. |
| `--bind PEOPLE` | Start bound to a people (`auto` for the largest). |
| `--at PLACE` | Centre a snapshot on a named place. |
| `--plain` | Snapshot the map alone, as an illustration. |
| `--ascii` | Use plain ASCII glyphs. |
| `--no-mouse` | Keep the terminal's text selection. |
| `--mkconfig` | Write a commented config template. |

Detail changes how much history is recorded, not what happens in the world.
Run `empires --help` or see the [full option reference](docs/GUIDE.md#command-line-options)
for filters, statistics, benchmarks and frame exports.

## Learn more

- [Player guide](docs/GUIDE.md): controls, saves, configuration and how the world works.
- [Changelog](CHANGELOG.md): release notes and changes to simulation rules.
- [Architecture](docs/ARCHITECTURE.md): source layout and the simulation pipeline.
- [Contributing](CONTRIBUTING.md): development rules and testing.

This is pre-1.0 software. Older saves are tested for compatibility, but the
format may evolve. A seed's history can also change between versions.

## Development

```sh
cargo build --release
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
python3 tools/ptytest.py
python3 tools/termtest.py
```

See [tools](tools/README.md) for interface testing and screenshots.

## License

Licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
Contributions use the same dual license unless explicitly stated otherwise.
