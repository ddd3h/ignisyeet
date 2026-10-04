# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
"""Draw the 1200 x 630 Open Graph image of the documentation site (source/_static/ogp.png).

The image is committed, so the documentation build does not need this script.
Run it by hand after the logo or the wording changes (needs inkscape and Pillow):

    uv run --project . python scripts/make_ogp.py
"""

import subprocess
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

DOC = Path(__file__).resolve().parent.parent
W, H = 1200, 630
BG, INK, INK2, ACCENT = "#ffffff", "#0b0b0b", "#52514e", "#e8522a"
FONT = "/usr/share/fonts/opentype/ipaexfont-gothic/ipaexg.ttf"


def main() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        logo_png = Path(tmp) / "logo.png"
        subprocess.run(["inkscape", str(DOC / "IgnisYeet-logo.svg"), "--export-type=png", f"--export-filename={logo_png}", "-w", "400", "-h", "400"],
                       check=True, capture_output=True)
        logo = Image.open(logo_png).convert("RGBA")

    img = Image.new("RGB", (W, H), BG)
    img.paste(logo, (70, (H - logo.height) // 2), logo)
    d = ImageDraw.Draw(img)
    x = 520
    d.text((x, 150), "IgnisYeet", font=ImageFont.truetype(FONT, 104), fill=INK)
    d.rectangle((x, 285, x + 120, 293), fill=ACCENT)
    body = ImageFont.truetype(FONT, 38)
    for i, line in enumerate(["STL からの空力推算と", "6 自由度飛翔・落下分散解析"]):
        d.text((x, 325 + 56 * i), line, font=body, fill=INK2)
    d.text((x, 470), "オープンソースのロケット飛翔シミュレータ", font=ImageFont.truetype(FONT, 28), fill=INK2)

    out = DOC / "source" / "_static" / "ogp.png"
    img.save(out, optimize=True)
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
