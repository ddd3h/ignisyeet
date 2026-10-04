# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
"""Run the ignisyeet binary in a pseudo terminal and render its screen as PNG (pyte + Pillow).

Usage: python scripts/make_screens.py <repo root> <output dir>

Writes screen_sim_start.png and screen_sim_result.png (the final screen of sim, split in two), screen_dispersion.png (mid-run frame with progress bar and ETA),
screen_aero_panel.png (mid-run frame of the panel method), screen_install.png (install.sh dry run) and
screen_install_*.png (a real interactive user-role install.sh run, its check and its uninstall; see install_screens()).  The output is the program's real
terminal output: the byte stream is replayed into a virtual terminal and drawn cell by cell
with DejaVu Sans Mono (braille spinner glyphs fall back to DejaVu Sans).
"""

import copy
import fcntl
import http.server
import os
import pty
import re
import secrets
import select
import shutil
import struct
import subprocess
import sys
import tempfile
import termios
import threading
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


def drive(cmd, cwd, env_extra, answers, timeout=600, watch=None):
    """Runs cmd in a pty with a controlling terminal (install.sh reads /dev/tty) and answers its prompts like expect.

    answers: list of (prompt text, answer, tag).  When the cursor line contains the prompt text, the answer is typed
    (without Enter), the screen is saved under tag (prompt and answer visible), and Enter is sent.
    watch: optional (tag, regex): the first frame whose screen matches the regex (e.g. a half-filled progress bar) is saved too.
    Returns (final screen, {tag: screen}).
    """
    env = dict(os.environ, TERM="xterm-256color", LANG="C.UTF-8", LC_ALL="C.UTF-8", COLUMNS=str(COLS), LINES=str(ROWS))
    env.pop("NO_COLOR", None)
    env.update(env_extra)
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))

    def ctty():
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)

    p = subprocess.Popen(cmd, cwd=cwd, env=env, stdin=slave, stdout=slave, stderr=slave, close_fds=True, preexec_fn=ctty)
    os.close(slave)
    screen = pyte.Screen(COLS, ROWS)
    stream = pyte.ByteStream(screen)
    frames, pending, t0 = {}, list(answers), time.time()

    def pump(seconds):
        end = time.time() + seconds
        while time.time() < end:
            if select.select([master], [], [], 0.05)[0]:
                try:
                    data = os.read(master, 65536)
                except OSError:
                    return False
                if not data:
                    return False
                stream.feed(data)
                if watch and watch[0] not in frames and any(re.search(watch[1], l) for l in lines_of(screen)):
                    frames[watch[0]] = copy.deepcopy(screen)
        return True

    alive = True
    while alive and time.time() - t0 < timeout:
        alive = pump(0.1)
        if pending and pending[0][0] in "".join(screen.buffer[screen.cursor.y][c].data for c in range(COLS)):
            text, ans, tag = pending.pop(0)
            os.write(master, ans.encode())
            pump(0.4)
            frames[tag] = copy.deepcopy(screen)
            os.write(master, b"\n")
    if p.poll() is None:
        p.kill()
    p.wait()
    os.close(master)
    if pending:
        raise SystemExit(f"prompt not seen: {pending[0][0]}")
    return screen, frames


FAKE_HOME = "/home/user"


def anonymise(screen, real):
    """Overwrites every occurrence of the (equally long) temporary home path in the screen with FAKE_HOME."""
    assert len(real) == len(FAKE_HOME)
    for r in range(screen.lines):
        text = "".join(screen.buffer[r][c].data for c in range(COLS))
        k = text.find(real)
        while k >= 0:
            for j, ch in enumerate(FAKE_HOME):
                screen.buffer[r][k + j] = screen.buffer[r][k + j]._replace(data=ch)
            k = text.find(real, k + 1)
    return screen


def row_of(screen, text, start=0):
    """Index of the first row at or after start that contains text."""
    return next(i for i, l in enumerate(lines_of(screen)) if i >= start and text in l)


class SlowHandler(http.server.SimpleHTTPRequestHandler):
    """Serves files slowly so that the installer's download progress bar can be captured half full."""

    def copyfile(self, source, outputfile):
        while True:
            buf = source.read(65536)
            if not buf:
                break
            outputfile.write(buf)
            outputfile.flush()
            time.sleep(0.06)

    def log_message(self, *args):
        pass


def install_screens(root, out):
    """A real interactive user-role install (and its uninstall) of install.sh against a local fake release.

    Everything happens in a temporary directory used as HOME, so the images show ~/... paths only.  There is no published
    release to download in the build, so the release tarball is made locally with scripts/package-release.sh and served on
    127.0.0.1.  The tarball is named for the platform that install.sh detects (x86_64-unknown-linux-musl, aarch64-... or the
    macOS triples) although the binary inside is the native build, because the package script only uses the triple in the file name.
    """
    version = re.search(r'^version = "([^"]+)"', (root / "Cargo.toml").read_text(), re.M).group(1)
    tag = f"v{version}"
    machine, system = os.uname().machine, os.uname().sysname
    triple = {("Linux", "x86_64"): "x86_64-unknown-linux-musl", ("Linux", "aarch64"): "aarch64-unknown-linux-musl",
              ("Darwin", "x86_64"): "x86_64-apple-darwin", ("Darwin", "arm64"): "aarch64-apple-darwin"}[(system, machine)]
    # The work directory (also HOME) gets a name exactly as long as FAKE_HOME so that it can be replaced in the images without
    # changing any line length: /tmp/iXXXX shows as /home/user.
    while True:
        work = Path("/tmp") / ("i" + secrets.token_hex(2))
        try:
            work.mkdir()
            break
        except FileExistsError:
            continue
    try:
        home = work
        (home / ".cargo" / "bin").mkdir(parents=True)
        rel = work / "releases" / tag
        rel.mkdir(parents=True)
        subprocess.run([str(root / "scripts" / "package-release.sh"), triple, tag, str(root / "target" / "release" / "ignisyeet"), str(rel)],
                       check=True, stdout=subprocess.DEVNULL)
        uv = shutil.which("uv")
        if uv:  # shown as ~/.cargo/bin/uv; the real location may be under the author's home
            uv_cache = subprocess.run([uv, "cache", "dir"], capture_output=True, text=True).stdout.strip()
            (home / ".cargo" / "bin" / "uv").symlink_to(uv)
        else:
            uv_cache = ""
        handler = lambda *a, **k: SlowHandler(*a, directory=str(work / "releases"), **k)
        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            # a clean PATH: no tools of the machine's owner (cargo, micromamba, ...) and none of the build's virtual environment
            std = [d for d in ("/usr/local/sbin", "/usr/local/bin", "/usr/sbin", "/usr/bin", "/sbin", "/bin") if Path(d).is_dir()]
            env = {"HOME": str(home), "PATH": ":".join([str(home / ".cargo" / "bin")] + std), "SHELL": "/bin/bash", "VIRTUAL_ENV": "",
                   "USER": "user", "LOGNAME": "user", "IGNISYEET_VERSION": tag,
                   "IGNISYEET_DOWNLOAD_BASE": f"http://127.0.0.1:{server.server_address[1]}"}
            if uv_cache:
                env["UV_CACHE_DIR"] = uv_cache  # reuse the download cache: numpy etc. need not be fetched again
            scr, fr = drive(
                ["bash", str(root / "install.sh")], work, env,
                [("Choose", "1", "role"), ("Install the optional CFD tools", "n", "cfd"), ("Proceed?", "y", "plan"), ("to PATH in", "n", "path")],
                watch=("download", r"[█░]{6,}\s+(?:[3-7]\d)%"))
            home_s = str(work)
            fr = {k: anonymise(v, home_s) for k, v in fr.items()}
            scr = anonymise(scr, home_s)
            # role: banner and the two choices
            render(fr["role"], out / "screen_install_role.png", 0, row_of(fr["role"], "Choose") + 1)
            # CFD question: after the environment listing
            s = fr["cfd"]
            render(s, out / "screen_install_cfd.png", row_of(s, "Environment"), row_of(s, "Install the optional CFD") + 1)
            # plan and confirmation
            s = fr["plan"]
            render(s, out / "screen_install_plan.png", row_of(s, "Plan"), row_of(s, "Proceed?") + 1)
            # progress: the download in the middle
            s = fr["download"]
            i = next(k for k, l in enumerate(lines_of(s)) if re.search(r"[█░]{6,}", l))
            render(s, out / "screen_install_progress.png", max(0, i - 4), i + 1)
            # finish: PATH question and summary box
            render(scr, out / "screen_install_done.png", row_of(scr, "to PATH in") - 1)
            assert "ready" in "\n".join(lines_of(scr)), "install did not finish"

            # verification in a shell-like session (the PATH line is what the installer suggested)
            env["PATH"] = ":".join([str(home / ".local" / "bin")] + env["PATH"].split(":"))
            script = (
                'run() { printf "\033[38;5;78m$\033[0m %s\n" "$*"; eval "$*"; }\n'
                'run ignisyeet --version\n'
                'run cp -r ~/.local/share/ignisyeet/current/examples ignisyeet-examples\n'
                'run cd ignisyeet-examples\n'
                'run ignisyeet sim sample.toml\n')
            scr = anonymise(replay(capture(["bash", "-c", script], home, env)), home_s)
            cut = after_config(scr)
            render(scr, out / "screen_install_check.png", 0, row_of(scr, "ignisyeet sim") + 1)  # version and the command
            render(scr, out / "screen_install_check_sim.png", cut)

            # uninstall
            scr, fr = drive(["bash", str(root / "install.sh"), "--uninstall"], home, env,
                            [("Remove the installed binary", "y", "bin"), ("Remove the plotting environment", "y", "venv")])
            render(anonymise(scr, home_s), out / "screen_install_uninstall.png")
        finally:
            server.shutdown()
    finally:
        shutil.rmtree(work, ignore_errors=True)


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

    # 4. install.sh: developer + CFD plan in dry-run mode (detects existing tools, changes nothing).
    home = os.environ.get("HOME", str(root))
    # Hide the documentation build's own virtual environment so the installer reports the system python.
    path = ":".join(d for d in os.environ.get("PATH", "").split(":") if ".venv" not in d)
    env = {"PATH": path, "VIRTUAL_ENV": ""}
    scr = replay(capture(["bash", "install.sh", "--developer", "--cfd", "--dry-run", "-y", "--dir", f"{home}/ignisyeet"], root, env))
    render(scr, out / "screen_install.png")

    # 5. install.sh: a real user-role install, check and uninstall in a temporary HOME.
    install_screens(root, out)


if __name__ == "__main__":
    main()
