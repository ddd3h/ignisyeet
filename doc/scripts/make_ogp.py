# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
"""Draw the 1200 x 630 Open Graph image of the documentation site (source/_static/ogp.png).

The image is committed, so the documentation build does not need this script.
Run it by hand after the logo, the wording or the screenshots change (needs inkscape and Pillow):

    uv run --project . python scripts/make_ogp.py

It uses the logo mark and the screenshots of the top page (source/_static/landing/).
"""

import subprocess
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

DOC = Path(__file__).resolve().parent.parent
LANDING = DOC / "source" / "_static" / "landing"
W, H = 1200, 630
BG, INK, INK2, ACCENT = (20, 21, 25), (246, 243, 236), (170, 168, 160), (255, 122, 69)
JP = "/usr/share/fonts/opentype/ipaexfont-gothic/ipaexg.ttf"
LATIN = "/usr/share/fonts/opentype/inter/Inter-Bold.otf"


def rounded(im: Image.Image, r: int) -> Image.Image:
    im = im.convert("RGBA")
    mask = Image.new("L", im.size, 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, im.width - 1, im.height - 1), r, fill=255)
    im.putalpha(mask)
    return im


def shadow(canvas: Image.Image, im: Image.Image, xy: tuple[int, int], blur: int = 22, dy: int = 14) -> None:
    sh = Image.new("RGBA", canvas.size, (0, 0, 0, 0))
    box = Image.new("RGBA", im.size, (0, 0, 0, 170))
    box.putalpha(im.getchannel("A").point(lambda a: 170 if a else 0))
    sh.paste(box, (xy[0], xy[1] + dy))
    canvas.alpha_composite(sh.filter(ImageFilter.GaussianBlur(blur)))


def window(shot: Path, width: int, title: str, dark: bool) -> Image.Image:
    """A screenshot in a window frame with a title bar."""
    im = Image.open(shot).convert("RGB")
    im = im.resize((width, round(im.height * width / im.width)), Image.LANCZOS)
    bar = 30
    out = Image.new("RGBA", (width, im.height + bar), (0, 0, 0, 0))
    d = ImageDraw.Draw(out)
    d.rectangle((0, 0, width, bar), fill=(52, 54, 60) if dark else (232, 229, 238))
    for i, c in enumerate(((255, 95, 86), (255, 189, 46), (39, 201, 63))):
        d.ellipse((12 + 20 * i, 9, 24 + 20 * i, 21), fill=c)
    d.text((82, 7), title, font=ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", 13), fill=(200, 200, 200) if dark else (90, 90, 90))
    out.paste(im, (0, bar))
    return rounded(out, 12)


def main() -> None:
    img = Image.new("RGBA", (W, H), BG + (255,))
    # soft orange glow behind the screenshots
    glow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    ImageDraw.Draw(glow).ellipse((560, 60, 1260, 700), fill=ACCENT + (70,))
    img.alpha_composite(glow.filter(ImageFilter.GaussianBlur(120)))

    with tempfile.TemporaryDirectory() as tmp:
        mark = Path(tmp) / "mark.png"
        subprocess.run(["inkscape", str(LANDING / "logo-mark.svg"), "--export-type=png", f"--export-filename={mark}", "-w", "260", "-h", "260"],
                       check=True, capture_output=True)
        logo = rounded(Image.open(mark), 54)
    shadow(img, logo, (64, 92))
    img.alpha_composite(logo, (64, 92))

    d = ImageDraw.Draw(img)
    d.text((64, 380), "IgnisYeet", font=ImageFont.truetype(LATIN, 84), fill=INK)
    d.rectangle((68, 490, 188, 496), fill=ACCENT)
    body = ImageFont.truetype(JP, 31)
    for i, line in enumerate(["CAD データから空力推算、", "6 自由度飛翔、落下分散まで"]):
        d.text((66, 516 + 42 * i), line, font=body, fill=INK2)

    rep = window(DOC / "figures" / "report" / "report_summary.png", 640, "report.html", dark=False)
    shadow(img, rep, (500, 70))
    img.alpha_composite(rep, (500, 70))
    term_src = DOC / "images" / "readme" / "screen_sim_result.png"
    term = window(term_src, 560, "ignisyeet sim examples/sample.toml", dark=True)
    shadow(img, term, (590, 330))
    img.alpha_composite(term, (590, 330))

    out = DOC / "source" / "_static" / "ogp.png"
    img.convert("RGB").save(out, optimize=True)
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
