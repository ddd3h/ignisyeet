# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
"""Turn the English paper of technical report IY-TR-2026-01 into its R&D web page.

The outputs are committed, so the documentation build does not need this script.
Run it by hand after the paper changes (needs pandoc, LuaLaTeX and pdftocairo):

    uv run --project . --with pypandoc_binary python scripts/make_tr_page.py \\
        [--paper ../research/IY-TR-2026-01/paper/en] [--figures ../research/IY-TR-2026-01/nasa_vv/figures]

It writes
  source/research/IY-TR-2026-01/paper.html        the table of contents and the paper body (HTML fragment, MathML, numbered)
  extra/research/IY-TR-2026-01/fig/*.svg           the figures, cropped as in the paper
  extra/research/IY-TR-2026-01/IY-TR-2026-01.pdf   the built paper, when main.pdf exists
"""

import argparse
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

DOC = Path(__file__).resolve().parent.parent
REPORT = "IY-TR-2026-01"
OUT_HTML = DOC / "source" / "research" / REPORT / "paper.html"
OUT_EXTRA = DOC / "extra" / "research" / REPORT

# The paper cuts the bottom of these figures (their own notes) with \includegraphics[trim=0 <pt> 0 0].
CROP_BOTTOM_PT = {"layer1_cp": 80, "validation_bands": 112}

# The concept figure is TikZ inside the paper; it is rendered on its own with the colours of labreport.cls.
CONCEPT_PREAMBLE = r"""\documentclass[border=4pt]{standalone}
\usepackage{fontspec}\setmainfont{TeX Gyre Heros}
\usepackage{tikz}\usetikzlibrary{arrows.meta}
\definecolor{lraccent}{HTML}{1F5F8B}\definecolor{lrdark}{HTML}{0F3B5C}
\begin{document}
"""

TIKZ = re.compile(r"\\begin\{tikzpicture\}.*?\\end\{tikzpicture\}", re.S)
TOKEN = re.compile(
    r"\\appendix\b"
    r"|\\(section|subsection|subsubsection)(\*?)\{"
    r"|\\begin\{(figure|table|equation)\}"
    r"|\\end\{(?:figure|table|equation)\}"
    r"|\\label\{([^}]*)\}"
)


def find_pandoc():
    exe = shutil.which("pandoc")
    if exe:
        return exe
    import pypandoc

    return pypandoc.get_pandoc_path()


def crop_svg(src, dst, bottom_pt):
    svg = src.read_text()
    m = re.search(r'height="([0-9.]+)pt" viewBox="0 0 ([0-9.]+) ([0-9.]+)"', svg)
    h = float(m.group(3)) - bottom_pt
    dst.write_text(svg.replace(m.group(0), f'height="{h:g}pt" viewBox="0 0 {m.group(2)} {h:g}"', 1))


def concept_svg(tex, dst, work):
    (work / "concept.tex").write_text(CONCEPT_PREAMBLE + TIKZ.search(tex).group(0) + "\n\\end{document}\n")
    subprocess.run(["lualatex", "-interaction=nonstopmode", "concept.tex"], cwd=work, check=True, capture_output=True)
    subprocess.run(["pdftocairo", "-svg", str(work / "concept.pdf"), str(dst)], check=True)


def prepare(tex):
    """Rewrite the LaTeX into what pandoc reads well: the abstract as the first section, SVG figures."""
    abstract = re.search(r"\\begin\{abstract\}(.*?)\\end\{abstract\}", tex, re.S)
    tex = tex.replace(abstract.group(0), "")
    tex = tex.replace(r"\maketitle", "\\section*{Abstract}\\label{sec:abstract}\n" + abstract.group(1))
    tex = TIKZ.sub(lambda _: r"\includegraphics{fig/concept.svg}", tex)
    tex = re.sub(r"\\includegraphics\[[^\]]*\]\{fig/([^}]*)\.pdf\}", r"\\includegraphics{fig/\1.svg}", tex)
    tex = tex.replace(r"\path{", r"\texttt{")
    return re.sub(r"\\(begin|end)\{(figure|table)\*\}", r"\\\1{\2}", tex)


def numbering(tex):
    """Number sections (letters after \\appendix), figures, tables and equations as LaTeX does.

    Returns the number of every label, an alias from each further label of an object to its first
    label (the id pandoc gives it), and the numbers of the headings in source order (None if starred).
    """
    labels, alias, headings = {}, {}, []
    sec, appendix, counts = [0, 0, 0], False, {"figure": 0, "table": 0, "equation": 0}
    owner, first = None, None
    for m in TOKEN.finditer(tex[tex.index(r"\begin{document}") :]):
        if m.group(0) == r"\appendix":
            appendix, sec = True, [0, 0, 0]
        elif m.group(1):
            if m.group(2):
                owner, first = None, None
                headings.append(None)
                continue
            level = ["section", "subsection", "subsubsection"].index(m.group(1))
            sec[level] += 1
            sec[level + 1 :] = [0] * (2 - level)
            top = chr(64 + sec[0]) if appendix else str(sec[0])
            owner, first = ".".join([top] + [str(n) for n in sec[1 : level + 1]]), None
            headings.append(owner)
        elif m.group(3):
            counts[m.group(3)] += 1
            env = (str(counts[m.group(3)]), None)
            saved = (owner, first)
            owner, first = env
        elif m.group(0).startswith(r"\end"):
            owner, first = saved
        else:
            label = m.group(4)
            labels[label] = owner
            if first is None:
                first = label
            else:
                alias[label] = first
    return labels, alias, headings


def convert(tex, bib, pandoc, work):
    tex = prepare(tex)
    labels, alias, headings = numbering(tex)
    (work / "pre.tex").write_text(tex)
    html = subprocess.run(
        [pandoc, str(work / "pre.tex"), "-f", "latex", "-t", "html5", "--mathml", "--citeproc",
         "--bibliography", str(bib), "--wrap=none"],
        capture_output=True, text=True, check=True,
    ).stdout

    def ref(m):
        label = m.group(1)
        if label not in labels:
            sys.exit(f"unresolved reference: {label}")
        return f'<a href="#{alias.get(label, label)}" class="iy-xref">{labels[label]}</a>'

    html = re.sub(r'<a href="#[^"]*"[^>]*data-reference="([^"]*)"[^>]*>.*?</a>', ref, html)
    # \paragraph comes out as h4; the page title is h1, so every level moves one down.
    html = re.sub(r"<h4([^>]*)>(.*?)</h4>", r"<h5\1>\2</h5>", html, flags=re.S)
    numbers = iter(headings)

    def heading(m):
        level, number = int(m.group(1)) + 1, next(numbers)
        prefix = f'<span class="iy-secnum">{number}</span> ' if number else ""
        return f"<h{level}{m.group(2)}>{prefix}{m.group(3)}</h{level}>"

    html = re.sub(r"<h([1-3])([^>]*)>(.*?)</h\1>", heading, html, flags=re.S)
    html = re.sub(
        r'(<figure id="([^"]*)"[^>]*>.*?<figcaption>)',
        lambda m: f"{m.group(1)}<strong>Figure {labels[m.group(2)]}.</strong> ", html, flags=re.S,
    )
    html = re.sub(
        r'<div id="(tab:[^"]*)"[^>]*>\s*<table([^>]*)>\s*<caption>',
        lambda m: f'<div id="{m.group(1)}" class="iy-table"><table{m.group(2)}><caption><strong>Table {labels[m.group(1)]}.</strong> ',
        html,
    )
    # The paper lists its references before the appendices; pandoc puts them last.
    refs = re.search(r'<div id="refs".*</div>\s*$', html, re.S).group(0)
    html = html.replace(refs, "")
    appendix = html.index(f'<h2 id="{next(k for k, v in labels.items() if v == "A")}"')
    html = html[:appendix] + '<h2 id="references">References</h2>\n' + refs + "\n" + html[appendix:]
    # Each figure opens at full size.
    return re.sub(r'<img src="(fig/[^"]*)"([^>]*)/>', r'<a href="\1"><img loading="lazy" src="\1"\2/></a>', html)


def contents(html):
    """The table of contents: sections and subsections, linked to their headings."""
    items = []
    for m in re.finditer(r'<h([23])[^>]*? id="([^"]*)"[^>]*>(.*?)</h\1>', html, re.S):
        number = re.search(r'<span class="iy-secnum">([^<]*)</span>', m.group(3))
        title = re.sub(r"<[^>]+>", "", re.sub(r'<span class="iy-secnum">[^<]*</span>', "", m.group(3))).strip()
        num = f'<span class="iy-secnum">{number.group(1)}</span>' if number else ""
        items.append(f'<li class="iy-toc-l{int(m.group(1)) - 1}"><a href="#{m.group(2)}">{num}{title}</a></li>')
    return (
        '<nav class="iy-toc" aria-label="Contents" lang="en"><details open><summary>Contents</summary>\n<ul>\n'
        + "\n".join(items)
        + "\n</ul></details></nav>\n"
    )


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--paper", type=Path, default=DOC.parent / "research" / REPORT / "paper" / "en")
    ap.add_argument("--figures", type=Path, default=DOC.parent / "research" / REPORT / "nasa_vv" / "figures")
    args = ap.parse_args()
    tex = (args.paper / "main.tex").read_text()
    fig = OUT_EXTRA / "fig"
    fig.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        concept_svg(tex, fig / "concept.svg", work)
        html = convert(tex, args.paper / "references.bib", find_pandoc(), work)
    for name, pt in CROP_BOTTOM_PT.items():
        crop_svg(args.figures / f"{name}.svg", fig / f"{name}.svg", pt)
    OUT_HTML.parent.mkdir(parents=True, exist_ok=True)
    OUT_HTML.write_text(contents(html) + '<article class="iy-paper" lang="en">\n' + html + "</article>\n")
    pdf = args.paper / "main.pdf"
    if pdf.exists():
        shutil.copyfile(pdf, OUT_EXTRA / f"{REPORT}.pdf")
    else:
        print(f"no {pdf}: the PDF was not copied", file=sys.stderr)


if __name__ == "__main__":
    main()
