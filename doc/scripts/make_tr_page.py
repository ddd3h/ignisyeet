# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
"""Turn a paper of technical report IY-TR-2026-01 (English or Japanese) into its R&D web page.

The outputs are committed, so the documentation build does not need this script.
Run it by hand after a paper changes, once per language (needs pandoc, LuaLaTeX and pdftocairo):

    uv run --project . --with pypandoc_binary python scripts/make_tr_page.py \\
        [--lang en|ja] [--paper ../research/IY-TR-2026-01/paper/<lang>] [--figures ../research/IY-TR-2026-01/nasa_vv/figures]

It writes, for --lang en (the default)
  source/research/IY-TR-2026-01/paper.html           the table of contents and the paper body (HTML fragment, MathML, numbered)
  extra/research/IY-TR-2026-01/fig/concept.svg        the concept figure (TikZ in the paper), rendered on its own
  extra/research/IY-TR-2026-01/IY-TR-2026-01.pdf      the built paper, when main.pdf exists
and for --lang ja
  source/research/IY-TR-2026-01/paper-ja.html        the same for the Japanese paper (the page is ja/index.rst)
  extra/research/IY-TR-2026-01/fig/concept-ja.svg     the concept figure with its Japanese labels
  extra/research/IY-TR-2026-01/IY-TR-2026-01-ja.pdf   the built Japanese paper, when main.pdf exists
Both write extra/research/IY-TR-2026-01/fig/{layer1_cp,validation_bands}.svg, the plots shared by the
two papers, cropped as in the paper.
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
OUT_SOURCE = DOC / "source" / "research" / REPORT
OUT_EXTRA = DOC / "extra" / "research" / REPORT

# The paper cuts the bottom of these figures (their own notes) with \includegraphics[trim=0 <pt> 0 0].
CROP_BOTTOM_PT = {"layer1_cp": 80, "validation_bands": 112}

# The concept figure is TikZ inside the paper; it is rendered on its own with the colours of labreport.cls.
# The Japanese labels are set in Harano Aji Gothic (luatexja), the Latin text in the same font as the English.
CONCEPT_FONTS = {
    "en": r"\usepackage{fontspec}\setmainfont{TeX Gyre Heros}",
    "ja": r"\usepackage[match]{luatexja-fontspec}\setmainfont{TeX Gyre Heros}"
    r"\setmainjfont{HaranoAjiGothic}[Extension=.otf, UprightFont=*-Regular, BoldFont=*-Bold]",
}
CONCEPT_PREAMBLE = r"""\documentclass[border=4pt]{standalone}
%s
\usepackage{tikz}\usetikzlibrary{arrows.meta}
\definecolor{lraccent}{HTML}{1F5F8B}\definecolor{lrdark}{HTML}{0F3B5C}
\begin{document}
"""

# What differs between the two pages. The Japanese page sits one directory lower (ja/), so its
# figures are one level up.
LANGS = {
    "en": {
        "html": "paper.html", "pdf": f"{REPORT}.pdf", "concept": "concept.svg", "fig": "fig/",
        "abstract": "Abstract", "references": "References", "contents": "Contents",
        "figure": "<strong>Figure {}.</strong> ", "table": "<strong>Table {}.</strong> ",
    },
    "ja": {
        "html": "paper-ja.html", "pdf": f"{REPORT}-ja.pdf", "concept": "concept-ja.svg", "fig": "../fig/",
        "abstract": "要旨", "references": "参考文献", "contents": "目次",
        "figure": "<strong>図 {}</strong>\u3000", "table": "<strong>表 {}</strong>\u3000",
    },
}

# Japanese characters and full-width punctuation. A line break after one of them is no space in
# Japanese (LuaTeX-ja drops it), but pandoc would read it as one. Comment lines in between are dropped.
CJK = "\u3000-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uff00-\uffef"
CJK_LINE_END = re.compile(rf"^((?:[^%\\\n]|\\.)*[{CJK}])\n(?:[ \t]*%[^\n]*\n)*(?=[^%\n])", re.M)

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


def concept_svg(tex, dst, work, lang):
    (work / "concept.tex").write_text(CONCEPT_PREAMBLE % CONCEPT_FONTS[lang] + TIKZ.search(tex).group(0) + "\n\\end{document}\n")
    subprocess.run(["lualatex", "-interaction=nonstopmode", "concept.tex"], cwd=work, check=True, capture_output=True)
    subprocess.run(["pdftocairo", "-svg", str(work / "concept.pdf"), str(dst)], check=True)


def prepare(tex, text):
    """Rewrite the LaTeX into what pandoc reads well: the abstract as the first section, SVG figures."""
    abstract = re.search(r"\\begin\{abstract\}(.*?)\\end\{abstract\}", tex, re.S)
    tex = tex.replace(abstract.group(0), "")
    tex = tex.replace(r"\maketitle", "\\section*{%s}\\label{sec:abstract}\n" % text["abstract"] + abstract.group(1))
    tex = TIKZ.sub(lambda _: r"\includegraphics{fig/%s}" % text["concept"], tex)
    tex = re.sub(r"\\includegraphics\[[^\]]*\]\{fig/([^}]*)\.pdf\}", r"\\includegraphics{fig/\1.svg}", tex)
    tex = tex.replace(r"\path{", r"\texttt{")
    tex = CJK_LINE_END.sub(r"\1", tex)
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


def convert(tex, bib, pandoc, work, text):
    tex = prepare(tex, text)
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
        lambda m: m.group(1) + text["figure"].format(labels[m.group(2)]), html, flags=re.S,
    )
    html = re.sub(
        r'<div id="(tab:[^"]*)"[^>]*>\s*<table([^>]*)>\s*<caption>',
        lambda m: f'<div id="{m.group(1)}" class="iy-table"><table{m.group(2)}><caption>' + text["table"].format(labels[m.group(1)]),
        html,
    )
    # The paper lists its references before the appendices; pandoc puts them last.
    refs = re.search(r'<div id="refs".*</div>\s*$', html, re.S).group(0)
    html = html.replace(refs, "")
    appendix = html.index(f'<h2 id="{next(k for k, v in labels.items() if v == "A")}"')
    html = html[:appendix] + f'<h2 id="references">{text["references"]}</h2>\n' + refs + "\n" + html[appendix:]
    # Each figure opens at full size.
    return re.sub(
        r'<img src="fig/([^"]*)"([^>]*)/>',
        lambda m: f'<a href="{text["fig"]}{m.group(1)}"><img loading="lazy" src="{text["fig"]}{m.group(1)}"{m.group(2)}/></a>',
        html,
    )


def contents(html, lang, text):
    """The table of contents: sections and subsections, linked to their headings."""
    items = []
    for m in re.finditer(r'<h([23])[^>]*? id="([^"]*)"[^>]*>(.*?)</h\1>', html, re.S):
        number = re.search(r'<span class="iy-secnum">([^<]*)</span>', m.group(3))
        title = re.sub(r"<[^>]+>", "", re.sub(r'<span class="iy-secnum">[^<]*</span>', "", m.group(3))).strip()
        num = f'<span class="iy-secnum">{number.group(1)}</span>' if number else ""
        items.append(f'<li class="iy-toc-l{int(m.group(1)) - 1}"><a href="#{m.group(2)}">{num}{title}</a></li>')
    return (
        f'<nav class="iy-toc" aria-label="{text["contents"]}" lang="{lang}"><details open><summary>{text["contents"]}</summary>\n<ul>\n'
        + "\n".join(items)
        + "\n</ul></details></nav>\n"
    )


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--lang", choices=sorted(LANGS), default="en")
    ap.add_argument("--paper", type=Path, help="the paper's directory (default: ../research/IY-TR-2026-01/paper/<lang>)")
    ap.add_argument("--figures", type=Path, default=DOC.parent / "research" / REPORT / "nasa_vv" / "figures")
    args = ap.parse_args()
    lang, text = args.lang, LANGS[args.lang]
    paper = args.paper or DOC.parent / "research" / REPORT / "paper" / lang
    tex = (paper / "main.tex").read_text()
    fig = OUT_EXTRA / "fig"
    fig.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        concept_svg(tex, fig / text["concept"], work, lang)
        html = convert(tex, paper / "references.bib", find_pandoc(), work, text)
    for name, pt in CROP_BOTTOM_PT.items():
        crop_svg(args.figures / f"{name}.svg", fig / f"{name}.svg", pt)
    OUT_SOURCE.mkdir(parents=True, exist_ok=True)
    (OUT_SOURCE / text["html"]).write_text(contents(html, lang, text) + f'<article class="iy-paper" lang="{lang}">\n' + html + "</article>\n")
    pdf = paper / "main.pdf"
    if pdf.exists():
        shutil.copyfile(pdf, OUT_EXTRA / text["pdf"])
    else:
        print(f"no {pdf}: the PDF was not copied", file=sys.stderr)


if __name__ == "__main__":
    main()
