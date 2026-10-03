# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
"""Run the ignisyeet binary in a pseudo terminal and render its screen as PNG (pyte + Pillow).

Usage: python scripts/make_screens.py <repo root> <output dir>

Writes screen_sim_start.png and screen_sim_result.png (the final screen of sim, split in two), screen_dispersion.png (mid-run frame with progress bar and ETA)
and screen_aero_panel.png (mid-run frame of the panel method).  The output is the program's real
terminal output: the byte stream is replayed into a virtual terminal and drawn cell by cell
with DejaVu Sans Mono (braille spinner glyphs fall back to DejaVu Sans).
"""

import fcntl
import os
import pty
import re
import struct
import subprocess
import sys
import termios
import time
from pathlib import Path

import pyte
from PIL import Image, ImageDraw, ImageFont

FONTS = "/usr/share/fonts/truetype/dejavu/"
CJK_FONTS = ["/usr/share/fonts/opentype/ipafont-gothic/ipag.ttf", "/usr/share/fonts/truetype/fonts-japanese-gothic.ttf", "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf"]
COLS, ROWS = 100, 80
SCALE = 2
SIZE = 15 * SCALE
BG, FG = (30, 31, 34), (216, 216, 212)
ANSI = {
    "black": (60, 62, 66), "red": (237, 94, 94), "green": (89, 201, 140), "brown": (232, 190, 80), "blue": (92, 150, 237),
    "magenta": (200, 130, 220), "cyan": (90, 200, 210), "white": (216, 216, 212),
    "brightblack": (130, 132, 136), "brightred": (255, 120, 120), "brightgreen": (120, 225, 165), "brightbrown": (245, 210, 110),
    "brightblue": (125, 175, 255), "brightmagenta": (225, 160, 240), "brightcyan": (120, 225, 235), "brightwhite": (245, 245, 240),
}


def capture(cmd, cwd, env_extra=None, timeout=120):
    """Runs cmd in a pty; returns a list of (time, bytes) chunks."""
    env = dict(os.environ, TERM="xterm-256color", LANG="C.UTF-8", LC_ALL="C.UTF-8", COLUMNS=str(COLS), LINES=str(ROWS))
    env.pop("NO_COLOR", None)
    env.update(env_extra or {})
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
    p = subprocess.Popen(cmd, cwd=cwd, env=env, stdin=slave, stdout=slave, stderr=slave, close_fds=True)
    os.close(slave)
    chunks, t0 = [], time.time()
    while True:
        try:
            data = os.read(master, 65536)
        except OSError:
            break
        if not data:
            break
        chunks.append((time.time() - t0, data))
        if time.time() - t0 > timeout:
            p.kill()
            break
    p.wait()
    os.close(master)
    return chunks


def replay(chunks):
    screen = pyte.Screen(COLS, ROWS)
    stream = pyte.ByteStream(screen)
    for _, data in chunks:
        stream.feed(data)
    return screen


def colour(c, default):
    if c == "default":
        return default
    if c in ANSI:
        return ANSI[c]
    if re.fullmatch(r"[0-9a-fA-F]{6}", c):
        return tuple(int(c[i:i + 2], 16) for i in (0, 2, 4))
    return default


LINES = set("│─╭╮╰╯━╸")


def draw_line_char(d, g, x, y, cw, lh, fg):
    """Box-drawing and bar glyphs drawn as exact cell-aligned shapes (seamless, and thick enough to survive scaling)."""
    t = 2 * SCALE
    cx, cy = x + cw / 2, y + lh / 2
    if g == "│":
        d.rectangle([cx - t / 2, y, cx + t / 2, y + lh], fill=fg)
    elif g == "─":
        d.rectangle([x, cy - t / 2, x + cw, cy + t / 2], fill=fg)
    elif g in "━╸":  # the half-cell head of the bar is drawn as a full cell: no gap between the filled and empty parts
        d.rectangle([x, cy - t * 0.75, x + cw, cy + t * 0.75], fill=fg)
    else:  # rounded corners: an arc joining the horizontal and vertical lines
        r = min(cw, lh) / 2
        right, down = g in "╭╰", g in "╭╮"
        ax = cx if right else cx - 2 * r
        ay = cy if down else cy - 2 * r
        ang = {"╭": (180, 270), "╮": (270, 360), "╰": (90, 180), "╯": (0, 90)}[g]
        d.arc([ax, ay, ax + 2 * r, ay + 2 * r], ang[0], ang[1], fill=fg, width=t)
        hx0, hx1 = (cx + r, x + cw) if right else (x, cx - r)
        vy0, vy1 = (cy + r, y + lh) if down else (y, cy - r)
        d.rectangle([hx0, cy - t / 2, hx1, cy + t / 2], fill=fg)
        d.rectangle([cx - t / 2, vy0, cx + t / 2, vy1], fill=fg)


def render(screen, path, first=0, stop=None):
    rows = [r for r in range(screen.lines)]
    last = max((r for r in rows if "".join(screen.buffer[r][c].data for c in range(COLS)).strip()), default=0)
    rows = rows[first:(last + 1 if stop is None else stop)]
    mono = ImageFont.truetype(FONTS + "DejaVuSansMono.ttf", SIZE)
    bold = ImageFont.truetype(FONTS + "DejaVuSansMono-Bold.ttf", SIZE)
    sans = ImageFont.truetype(FONTS + "DejaVuSans.ttf", SIZE)
    cjk_path = next((p for p in CJK_FONTS if Path(p).exists()), None)
    cjk = ImageFont.truetype(cjk_path, SIZE) if cjk_path else mono
    cw = round(mono.getlength("M"))
    lh = round(SIZE * 1.17)  # about the font's ascent + descent, so vertical box lines join
    pad = 12 * SCALE
    img = Image.new("RGB", (int(COLS * cw) + 2 * pad, lh * len(rows) + 2 * pad), BG)
    d = ImageDraw.Draw(img)
    for i, r in enumerate(rows):
        for c in range(COLS):
            ch = screen.buffer[r][c]
            fg, bg = colour(ch.fg, FG), colour(ch.bg, BG)
            if ch.reverse:
                fg, bg = bg, fg
            x, y = pad + c * cw, pad + i * lh  # integer cell grid: no seams in box lines
            if bg != BG:
                d.rectangle([x, y, x + cw, y + lh], fill=bg)
            if ch.data in LINES:
                draw_line_char(d, ch.data, x, y, cw, lh, fg)
            elif ch.data.strip():
                wide = ch.data >= "\u2e80"  # CJK: occupies two terminal cells
                f = cjk if wide else (sans if "⠀" <= ch.data <= "⣿" else (bold if ch.bold else mono))
                w = f.getlength(ch.data)
                d.text((x + ((2 * cw if wide else cw) - w) / 2, y), ch.data, font=f, fill=fg)
    path.parent.mkdir(parents=True, exist_ok=True)
    img.save(path)
    print(f"screen {path.name} {img.size}")


def lines_of(screen):
    return ["".join(screen.buffer[r][c].data for c in range(COLS)).rstrip() for r in range(screen.lines)]


def after_config(screen):
    """Index of the first row after the configuration panel."""
    ls = lines_of(screen)
    return next((i + 1 for i, l in enumerate(ls) if l.startswith("╰")), 0)


def pick_frame(chunks, lo, hi):
    """Replays chunks until the first progress line shows a percentage in [lo, hi] with a status line below it."""
    for k in range(len(chunks)):
        scr = replay(chunks[: k + 1])
        ls = lines_of(scr)
        for i, l in enumerate(ls[:-1]):
            m = re.search(r"━.*?(\d+)%", l)
            if m and lo <= int(m.group(1)) <= hi and ls[i + 1].strip():
                return scr
    raise SystemExit("no suitable mid-run frame found")


def main():
    root, out = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()
    bin_ = str(root / "target" / "release" / "ignisyeet")
    work = root / "examples" / "out" / "screens"
    work.mkdir(parents=True, exist_ok=True)
    base = (root / "examples" / "sample.toml").read_text()

    def variant(name, subs):
        s = base.replace('stl = "sample_rocket.stl"', 'stl = "../../sample_rocket.stl"').replace('eng = "sample_motor.eng"', 'eng = "../../sample_motor.eng"')
        s = s.replace('dir = "out"', f'dir = "{name}"')
        for a, b in subs:
            assert a in s, a
            s = s.replace(a, b)
        (work / f"{name}.toml").write_text(s)
        return f"examples/out/screens/{name}.toml"

    # 1. sim: final screen of the sample configuration.
    scr = replay(capture([bin_, "sim", "examples/sample.toml"], root))
    cut = after_config(scr)
    render(scr, out / "screen_sim_start.png", 0, cut)  # banner and configuration panel
    render(scr, out / "screen_sim_result.png", cut)  # steps, result panel, written files

    # 2. dispersion: a Monte Carlo run long enough to catch a frame with the bar around the middle.
    cfg = variant("mc", [('mode = "wind_grid"', 'mode = "monte_carlo"'), ("samples = 1000", "samples = 12000"), ('integrator = "rk4"', 'integrator = "rk45"')])
    chunks = capture([bin_, "dispersion", cfg], root)
    scr = pick_frame(chunks, 35, 65)
    render(scr, out / "screen_dispersion.png", after_config(scr))

    # 3. aero: the panel method (forced rebuild) while it is solving.
    cfg = variant("panel", [('method = "barrowman"', 'method = "panel"')])
    chunks = capture([bin_, "aero", cfg, "--force"], root)
    try:
        scr = pick_frame(chunks, 40, 80)
    except SystemExit:
        scr = replay(chunks[: max(1, len(chunks) // 2)])
    render(scr, out / "screen_aero_panel.png", after_config(scr))


if __name__ == "__main__":
    main()
