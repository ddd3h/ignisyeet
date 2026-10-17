"""Render report.md (this directory) into doc/extra/research/nasa-vv/index.html.

Usage: python3 build_page.py   (needs markdown-it-py)
"""
import re, html, sys
from markdown_it import MarkdownIt
import pathlib
ROOT = str(pathlib.Path(__file__).resolve().parents[4]) + "/"
src = open(ROOT + "research/nasa_vv/report/ja/report.md", encoding="utf-8").read()

md = MarkdownIt("commonmark", {"html": True}).enable("table")

def add_ids(tokens):
    for i, t in enumerate(tokens):
        if t.type == "heading_open":
            inline = tokens[i + 1]
            m = re.search(r"\s*\{#([\w-]+)\}\s*$", inline.content)
            if m:
                t.attrSet("id", m.group(1))
                inline.content = inline.content[: m.start()]
                last = inline.children[-1]
                last.content = re.sub(r"\s*\{#[\w-]+\}\s*$", "", last.content)

def rules(md):
    def table_open(tokens, idx, options, env): return '<div class="tablewrap" tabindex="0" role="region" aria-label="表"><table>\n'
    def table_close(tokens, idx, options, env): return "</table></div>\n"
    md.renderer.rules["table_open"] = table_open
    md.renderer.rules["table_close"] = table_close
    def link_open(tokens, idx, options, env):
        t = tokens[idx]
        h = t.attrGet("href") or ""
        if h.startswith("http"): t.attrSet("rel", "noopener")
        return md.renderer.renderToken(tokens, idx, options, env)
    md.renderer.rules["link_open"] = link_open
    def image(tokens, idx, options, env):
        t = tokens[idx]
        title = t.attrGet("title") or ""
        desc = t.content
        short = re.sub(r"[　 ].*", "", title) if False else title
        return ('<figure class="fig"><img src="%s" alt="%s">'
                '<figcaption><strong>%s</strong> %s</figcaption></figure>'
                % (html.escape(t.attrGet("src")), html.escape(title), html.escape(title), html.escape(desc)))
    md.renderer.rules["image"] = image
rules(md)

tokens = md.parse(src)
add_ids(tokens)
body = md.renderer.render(tokens, md.options, {})
body = re.sub(r"<p>(<figure.*?</figure>)</p>", r"\1", body, flags=re.S)

# split: h1, lead paragraph, then sections by h2
m = re.match(r"\s*<h1>(.*?)</h1>\s*<p>(.*?)</p>(.*)", body, flags=re.S)
title, lead, rest = m.group(1), m.group(2), m.group(3)
parts = re.split(r"(?=<h2 id=)", rest)
sections, toc = [], []
for p in parts:
    p = p.strip()
    if not p: continue
    hm = re.match(r'<h2 id="([\w-]+)">(.*?)</h2>', p, flags=re.S)
    sid, htext = hm.group(1), hm.group(2)
    toc.append((sid, htext))
    sections.append('<section id="%s">\n%s\n</section>' % (sid, p))
# toc numbering: use native numbering only for numbered sections; keep plain list with the original text
toc_html = '<nav class="toc" aria-label="目次">\n<h2>目次</h2>\n<ul style="list-style:none;padding-left:0;columns:2 280px;column-gap:2rem;margin:0">\n' + "\n".join(
    '<li style="margin:0.15rem 0;break-inside:avoid"><a href="#%s">%s</a></li>' % (s, t) for s, t in toc) + "\n</ul>\n</nav>"

page = f'''<!doctype html>
<html lang="ja">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>NASA の検証と妥当性確認の手順を IgnisYeet に適用する - IgnisYeet</title>
<meta name="description" content="NASA WIND/NPARC の検証と妥当性確認の手順を IgnisYeet の空力計算に当てはめた結果の報告。">
<link rel="icon" href="../../_static/landing/logo-mark.svg">
<link rel="stylesheet" href="../research.css">
<script>
  try {{
    var t = localStorage.getItem("theme");
    if (t === "light" || t === "dark") document.documentElement.dataset.theme = t;
  }} catch (e) {{}}
</script>
</head>
<body>

<nav class="nav" aria-label="ナビゲーション">
  <a class="brand" href="../../index.html"><img src="../../_static/landing/logo-mark.svg" alt="" width="32" height="32"><span>IgnisYeet</span></a>
  <ul>
    <li><a href="../../index.html">トップ</a></li>
    <li><a href="../../intro.html">ドキュメント</a></li>
    <li><a href="../" aria-current="true">研究</a></li>
    <li><a href="https://github.com/ddd3h/ignisyeet" rel="noopener">GitHub</a></li>
  </ul>
  <button type="button" class="theme" id="theme" aria-label="配色を切り替える">配色</button>
</nav>

<header class="hero">
  <div class="wrap">
    <p class="crumb"><a href="../">研究</a> ／ NASA の検証と妥当性確認</p>
    <h1>{title}</h1>
    <p class="lead">{lead}</p>
  </div>
</header>

<main class="wrap">
{toc_html}
{chr(10).join(sections)}
</main>

<footer>
  <div class="wrap">
    <p>IgnisYeet by 西濱大将 (NISHIHAMA Daisuke)。GPL-3.0-or-later。</p>
    <p>引用：<a href="https://github.com/ddd3h/ignisyeet/blob/main/CITATION.cff" rel="noopener">CITATION.cff</a></p>
  </div>
</footer>

<script>
  document.getElementById("theme").addEventListener("click", function () {{
    var root = document.documentElement;
    var dark = root.dataset.theme
      ? root.dataset.theme === "dark"
      : matchMedia("(prefers-color-scheme: dark)").matches;
    var next = dark ? "light" : "dark";
    root.dataset.theme = next;
    try {{ localStorage.setItem("theme", next); }} catch (e) {{}}
  }});
</script>
</body>
</html>
'''
open(ROOT + "doc/extra/research/nasa-vv/index.html", "w", encoding="utf-8").write(page)
print("ok", len(page))
