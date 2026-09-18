#!/bin/sh
# Regenerate docs/screenshot.png for the README.
#
# The game renders a frame to HTML with --snapshot (one <span> per cell, with
# the real 24-bit colours), and a headless browser turns that into a PNG. There
# is no image code in the tree and there is not going to be any.
#
# Usage: tools/screenshot.sh [SEED] [YEARS] [LAYER]
set -eu

seed=${1:-7}
years=${2:-420}
layer=${3:-political}
cols=150
rows=42

root=$(cd "$(dirname "$0")/.." && pwd)
bin="$root/target/release/empires"
out="$root/docs/screenshot.png"

[ -x "$bin" ] || { echo "build it first: cargo build --release" >&2; exit 1; }

# On a Mac the browsers are app bundles and are not on PATH, which meant a
# project that ships macOS binaries could not regenerate its own README image
# on a Mac.
browser=
for b in firefox chromium chromium-browser google-chrome \
         "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
         "/Applications/Chromium.app/Contents/MacOS/Chromium" \
         "/Applications/Firefox.app/Contents/MacOS/firefox"; do
    command -v "$b" >/dev/null 2>&1 && { browser=$b; break; }
done
[ -n "$browser" ] || {
    echo "no headless browser found (firefox or chromium)." >&2
    echo "The text frame is still useful:" >&2
    echo "  $bin --seed $seed --headless $years --snapshot /tmp/frame --layer $layer" >&2
    exit 1
}

tmp=$(mktemp -d)
mkdir -p "$tmp/profile"
trap 'rm -rf "$tmp"' EXIT

# Bound, because the covenant is what the game is about and an image of the
# status bar reading "unbound" sells the screensaver it used to be.
"$bin" --seed "$seed" --headless "$years" --min-importance 3 --bind auto \
       --snapshot "$tmp/frame" --layer "$layer" --cols "$cols" --rows "$rows" \
       >/dev/null

mkdir -p "$root/docs"
case ${browser##*/} in
    firefox)
        "$browser" --headless --profile "$tmp/profile" \
                   --window-size=1350,660 --screenshot "$out" \
                   "file://$tmp/frame.html" >/dev/null 2>&1
        ;;
    *)
        "$browser" --headless --disable-gpu --hide-scrollbars \
                   --user-data-dir="$tmp/profile" \
                   --window-size=1350,660 --screenshot="$out" \
                   "file://$tmp/frame.html" >/dev/null 2>&1
        ;;
esac

[ -s "$out" ] || { echo "the browser produced no image" >&2; exit 1; }
echo "wrote $out  (seed $seed, year $years, $layer layer)"
