#!/usr/bin/env bash
# Screenshots of the HTML report of examples/sample.toml for the usage chapter and the README.
# Usage: doc/scripts/make_report_screens.sh   (needs target/release/ignisyeet, uv, python3 with PIL,
# and a Chromium headless shell in CHROME)
set -euo pipefail
cd "$(dirname "$0")/../.."
CHROME=${CHROME:-$(ls -d ~/.cache/ms-playwright/chromium_headless_shell-*/*/chrome-headless-shell | tail -1)}
OUT=${OUT:-.claude/tmp/report-shot}
rm -rf "$OUT"
for c in geom aero sim dispersion; do target/release/ignisyeet "$c" examples/sample.toml --out "$OUT" -q >/dev/null; done
make plot OUT="$OUT" CONFIG=examples/sample.toml >/dev/null
SOURCE_DATE_EPOCH=1760000000 target/release/ignisyeet report examples/sample.toml --out "$OUT" >/dev/null
sed -i "s#$(cd "$OUT" && pwd)#examples/out#g" "$OUT/report.html"
"$CHROME" --no-sandbox --disable-gpu --hide-scrollbars --window-size=1100,7300 --screenshot="$OUT/full.png" "file://$(cd "$OUT" && pwd)/report.html" >/dev/null 2>&1
python3 - "$OUT/full.png" <<'P'
import sys
from PIL import Image
im = Image.open(sys.argv[1]).convert("RGB")
# crop rows chosen by looking at the page; adjust when the report layout changes
for name, (a, b) in {"report_summary": (0, 1000), "report_flight": (1900, 3870), "report_dispersion": (3880, 5065)}.items():
    im.crop((0, a, 1100, b)).save(f"doc/figures/report/{name}.png", optimize=True)
P
