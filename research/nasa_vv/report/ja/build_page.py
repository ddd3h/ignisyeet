"""Render report.md (this directory) into the Sphinx page doc/source/research/nasa-vv/index.rst.

The page is a raw-HTML block inside the research section's iy-* wrappers, so it shares the top bar
and footer with the other pages. The two figures are copied from research/nasa_vv/figures/ to
doc/source/_static/research/nasa-vv/, where Sphinx serves them.

Usage: python3 build_page.py   (needs markdown-it-py)
"""
import html
import pathlib
import re
import unicodedata

from markdown_it import MarkdownIt

ROOT = pathlib.Path(__file__).resolve().parents[4]
SRC = ROOT / "research/nasa_vv/report/ja/report.md"
FIGURES_FROM = ROOT / "research/nasa_vv/figures"
FIGURES_TO = ROOT / "doc/source/_static/research/nasa-vv"
PAGE = ROOT / "doc/source/research/nasa-vv/index.rst"
FIGURE_HREF = "../../_static/research/nasa-vv/"

DESCRIPTION = "NASA WIND/NPARC の検証と妥当性確認の手順を IgnisYeet の空力計算に当てはめた結果の報告。"


def _renderer():
    md = MarkdownIt("commonmark", {"html": True}).enable("table")

    def table_open(tokens, idx, options, env):
        return '<div class="tablewrap" tabindex="0" role="region" aria-label="表"><table>\n'

    def table_close(tokens, idx, options, env):
        return "</table></div>\n"

    def link_open(tokens, idx, options, env):
        t = tokens[idx]
        if (t.attrGet("href") or "").startswith("http"):
            t.attrSet("rel", "noopener")
        return md.renderer.renderToken(tokens, idx, options, env)

    def image(tokens, idx, options, env):
        t = tokens[idx]
        title = t.attrGet("title") or ""
        src = t.attrGet("src")
        if src.startswith("figures/"):
            src = FIGURE_HREF + src[len("figures/"):]
        return ('<figure class="fig"><img src="%s" alt="%s">'
                '<figcaption><strong>%s</strong> %s</figcaption></figure>'
                % (html.escape(src), html.escape(title), html.escape(title), html.escape(t.content)))

    md.renderer.rules["table_open"] = table_open
    md.renderer.rules["table_close"] = table_close
    md.renderer.rules["link_open"] = link_open
    md.renderer.rules["image"] = image
    return md


def _add_ids(tokens):
    """Turn a trailing `{#id}` on a heading into its id attribute."""
    for i, t in enumerate(tokens):
        if t.type == "heading_open":
            inline = tokens[i + 1]
            m = re.search(r"\s*\{#([\w-]+)\}\s*$", inline.content)
            if m:
                t.attrSet("id", m.group(1))
                inline.content = inline.content[: m.start()]
                last = inline.children[-1]
                last.content = re.sub(r"\s*\{#[\w-]+\}\s*$", "", last.content)


def _width(text):
    """Display width in columns; docutils counts wide characters as two."""
    return sum(2 if unicodedata.east_asian_width(c) in "WF" else 1 for c in text)


def _plain(fragment):
    return html.unescape(re.sub(r"<[^>]+>", "", fragment))


def render():
    """Return the text of the reStructuredText page."""
    md = _renderer()
    tokens = md.parse(SRC.read_text(encoding="utf-8"))
    _add_ids(tokens)
    body = md.renderer.render(tokens, md.options, {})
    body = re.sub(r"<p>(<figure.*?</figure>)</p>", r"\1", body, flags=re.S)

    # h1, lead paragraph, then the sections split at each h2
    m = re.match(r"\s*<h1>(.*?)</h1>\s*<p>(.*?)</p>(.*)", body, flags=re.S)
    title, lead, rest = m.group(1), m.group(2), m.group(3)
    sections, toc = [], []
    for p in re.split(r"(?=<h2 id=)", rest):
        p = p.strip()
        if not p:
            continue
        hm = re.match(r'<h2 id="([\w-]+)">(.*?)</h2>', p, flags=re.S)
        toc.append((hm.group(1), hm.group(2)))
        sections.append("<section>\n%s\n</section>" % p)
    toc_html = ('<nav class="iy-toc" aria-label="目次">\n<p class="iy-toc-title">目次</p>\n<ul>\n'
                + "\n".join('<li><a href="#%s">%s</a></li>' % (s, t) for s, t in toc)
                + "\n</ul>\n</nav>")

    block = f'''<div class="iy iy-report">

<header class="iy-hero iy-page-hero">
  <div class="iy-wrap">
    <p class="iy-crumb"><a href="../">研究</a> ／ NASA の検証と妥当性確認</p>
    <p class="iy-page-title" role="presentation">{title}</p>
    <p class="iy-sub">{lead}</p>
  </div>
</header>

<section class="iy-section iy-research iy-report-body">
  <div class="iy-wrap">
{toc_html}
{chr(10).join(sections)}
  </div>
</section>

</div>
'''
    heading = _plain(title)
    rule = "=" * _width(heading)
    indented = "\n".join(("   " + line) if line.strip() else "" for line in block.split("\n"))
    return (f":orphan:\n\n.. meta::\n   :description: {DESCRIPTION}\n\n"
            f"{heading}\n{rule}\n\n.. raw:: html\n\n{indented}")


def main():
    PAGE.parent.mkdir(parents=True, exist_ok=True)
    FIGURES_TO.mkdir(parents=True, exist_ok=True)
    for svg in sorted(FIGURES_FROM.glob("*.svg")):
        (FIGURES_TO / svg.name).write_bytes(svg.read_bytes())
    text = render()
    PAGE.write_text(text, encoding="utf-8")
    print("ok", len(text))


if __name__ == "__main__":
    main()
