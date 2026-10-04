# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
# Sphinx configuration for the IgnisYeet documentation.

project = "IgnisYeet"
author = "西濱大将 (NISHIHAMA Daisuke)"
copyright = "2025-2026, 西濱大将 (NISHIHAMA Daisuke)"
release = "0.3.0"
language = "ja"

extensions = ["sphinx.ext.mathjax", "sphinxcontrib.bibtex", "sphinxext.opengraph"]
bibtex_bibfiles = ["references.bib"]
bibtex_default_style = "plain"
bibtex_reference_style = "author_year"
templates_path = []
exclude_patterns = []

numfig = True
numfig_format = {"figure": "図 %s", "table": "表 %s", "code-block": "リスト %s", "section": "%s 節"}
math_numfig = True
math_number_all = False
math_eqref_format = "式 ({number})"

mathjax3_config = {
    "tex": {
        "macros": {
            "bm": [r"\boldsymbol{#1}", 1],
            "dd": r"\mathrm{d}",
            "sgn": r"\operatorname{sgn}",
        }
    }
}

html_theme = "furo"
html_title = "IgnisYeet ドキュメント"
html_logo = "../IgnisYeet-logo.svg"
html_static_path = ["_static"]
html_css_files = ["custom.css"]
html_theme_options = {
    "source_repository": "https://github.com/ddd3h/ignisyeet",
    "source_branch": "main",
    "source_directory": "doc/source/",
}

# Open Graph / Twitter card metadata for link previews (image: scripts/make_ogp.py)
ogp_site_url = "https://www.ddd3h.com/ignisyeet/"
ogp_site_name = "IgnisYeet ドキュメント"
ogp_image = "_static/ogp.png"
ogp_image_alt = "IgnisYeet: STL からの空力推算と 6 自由度飛翔・落下分散解析"
ogp_type = "website"
ogp_social_cards = {"enable": False}
ogp_custom_meta_tags = ['<meta name="twitter:card" content="summary_large_image" />']

# PDF (make pdf): upLaTeX + jsbook. Figures are generated as both SVG (HTML) and PDF (LaTeX).
latex_engine = "uplatex"
latex_documents = [("index", "ignisyeet.tex", "IgnisYeet ドキュメント", author, "manual")]
latex_elements = {
    "papersize": "a4paper",
    "pointsize": "11pt",
    "preamble": r"""
\usepackage{amsmath,amssymb,bm}
\newcommand{\dd}{\mathrm{d}}
\newcommand{\sgn}{\operatorname{sgn}}
\renewcommand{\bibname}{参考文献}
""",
}
