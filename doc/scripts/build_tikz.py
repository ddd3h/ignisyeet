"""Compile every figures/tikz/*.tex body into SVG (HTML) and PDF (LaTeX) with lualatex + dvisvgm.

Each source file contains only a tikzpicture; this script wraps it in a standalone
document with a shared preamble so all figures use the same fonts and colours.
Figures are rebuilt only when the source (or this script) is newer than the SVG.
"""

import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

PREAMBLE = r"""\documentclass[tikz,border=6pt]{standalone}
\usepackage{luatexja}
\usepackage[ipaex]{luatexja-preset}
\usepackage{amsmath,amssymb,bm}
\usetikzlibrary{arrows.meta,calc,positioning,shapes.geometric,decorations.pathreplacing,angles,quotes,patterns,fit}
\definecolor{cblue}{HTML}{2A78D6}
\definecolor{corange}{HTML}{EB6834}
\definecolor{caqua}{HTML}{1BAF7A}
\definecolor{cyellow}{HTML}{EDA100}
\definecolor{cink}{HTML}{0B0B0B}
\definecolor{cink2}{HTML}{52514E}
\definecolor{cgrid}{HTML}{C9C8C3}
\definecolor{cfill}{HTML}{E3EEFB}
\definecolor{cfillo}{HTML}{FCE6DC}
\definecolor{cfillg}{HTML}{F0EFEC}
\tikzset{>=Stealth, every picture/.style={line cap=round, line join=round}}
\begin{document}
"""


def build(src: Path, dst: Path) -> None:
    with tempfile.TemporaryDirectory() as tmp:
        tex = Path(tmp) / "fig.tex"
        tex.write_text(PREAMBLE + src.read_text(encoding="utf-8") + "\n\\end{document}\n", encoding="utf-8")
        r = subprocess.run(["lualatex", "-interaction=nonstopmode", "-halt-on-error", "fig.tex"], cwd=tmp, capture_output=True, text=True)
        if r.returncode != 0:
            log = (Path(tmp) / "fig.log").read_text(errors="replace")
            err = [l for l in log.splitlines() if l.startswith("!")]
            raise SystemExit(f"lualatex failed for {src.name}:\n" + "\n".join(err[:10] or log.splitlines()[-30:]))
        subprocess.run(["dvisvgm", "--pdf", "--no-fonts", "--exact-bbox", "-o", str(dst.resolve()), "fig.pdf"], cwd=tmp, check=True, capture_output=True)
        shutil.copyfile(Path(tmp) / "fig.pdf", dst.with_suffix(".pdf"))


def main() -> None:
    src_dir, out_dir = Path(sys.argv[1]), Path(sys.argv[2])
    out_dir.mkdir(parents=True, exist_ok=True)
    if not shutil.which("lualatex") or not shutil.which("dvisvgm"):
        raise SystemExit("lualatex and dvisvgm are required to build the TikZ figures")
    me = Path(__file__).stat().st_mtime
    for src in sorted(src_dir.glob("*.tex")):
        dst = out_dir / (src.stem + ".svg")
        if dst.exists() and dst.with_suffix(".pdf").exists() and dst.stat().st_mtime > max(src.stat().st_mtime, me):
            continue
        build(src, dst)
        print(f"tikz  {src.name} -> {dst}")


if __name__ == "__main__":
    main()
