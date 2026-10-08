# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
# Sphinx configuration for the IgnisYeet documentation.

project = "IgnisYeet"
author = "西濱大将 (NISHIHAMA Daisuke)"
copyright = "2025-2026, 西濱大将 (NISHIHAMA Daisuke)"
release = "0.4.0"
language = "ja"

extensions = ["sphinx.ext.mathjax", "sphinxcontrib.bibtex", "sphinxext.opengraph", "sphinx_reredirects"]
bibtex_bibfiles = ["references.bib"]
bibtex_default_style = "plain"
bibtex_reference_style = "author_year"
templates_path = []
# Pages of the previous layout, kept in the tree until their content is moved into the new parts.
exclude_patterns = [
    "usage.rst",
    "intro.rst",
    "overview.rst",
    "coordinates.rst",
    "geometry.rst",
    "aerodynamics.rst",
    "aero_coefficients.rst",
    "barrowman.rst",
    "panel.rst",
    "cfd.rst",
    "cfd_internals.rst",
    "environment.rst",
    "propulsion.rst",
    "dynamics.rst",
    "dispersion.rst",
    "verification.rst",
    "verification_aero.rst",
    "validation.rst",
    "limitations.rst",
    "file_formats.rst",
    "safety.rst",
]

# Old page names redirect to the page that now holds the topic (sphinx-reredirects).
redirects = {
    "usage": "howto/index.html",
    "intro": "about.html",
    "overview": "theory/overview.html",
    "coordinates": "theory/coordinates.html",
    "geometry": "theory/geometry.html",
    "aerodynamics": "theory/aero_choose.html",
    "aero_coefficients": "theory/aero_coefficients.html",
    "barrowman": "theory/barrowman.html",
    "panel": "theory/panel.html",
    "cfd": "theory/cfd_choose.html",
    "cfd_internals": "theory/cfd_equations.html",
    "environment": "theory/wind.html",
    "propulsion": "theory/propulsion.html",
    "dynamics": "theory/dynamics.html",
    "dispersion": "theory/dispersion.html",
    "verification": "vv/verification.html",
    "verification_aero": "vv/aero_verification.html",
    "validation": "vv/validation_basic_finner.html",
    "limitations": "theory/limitations.html",
    "file_formats": "reference/output_files.html",
    "errors": "reference/error_codes.html",
    "safety": "howto/keepout_zone.html",
}

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
# Cross-references in the PDF carry the section number and the page: Sphinx appends " (\autopageref*{label})"
# to every :ref: and :doc: link, and the preamble below prints it as "(<number>、p. <page>)".
latex_show_pagerefs = True
latex_elements = {
    "papersize": "a4paper",
    "pointsize": "11pt",
    "preamble": r"""
% Embed the Japanese fonts: map the default Mincho and Gothic of upLaTeX/dvipdfmx to the IPAex fonts.
\usepackage[ipaex]{pxchfon}
\usepackage{amsmath,amssymb,bm}
\newcommand{\dd}{\mathrm{d}}
\newcommand{\sgn}{\operatorname{sgn}}
\renewcommand{\bibname}{参考文献}
% :ref: links: "(section number、p. page)" instead of the default "(page N)".
\makeatletter
\DeclareRobustCommand*{\autopageref}{\@ifstar\IY@numpage\IY@numpage}
\newcommand*{\IY@numpage}[1]{\ref*{#1}、p.~\pageref*{#1}}
\makeatother
% The appendix chapter "参考文献" (references.rst) already gives the heading and the contents entry:
% list the entries without the extra \chapter* and contents line of the bibliography environment.
\makeatletter
\renewenvironment{sphinxthebibliography}[1]{%
  \list{\@biblabel{\@arabic\c@enumiv}}{\settowidth\labelwidth{\@biblabel{#1}}%
    \leftmargin\labelwidth\advance\leftmargin\labelsep\usecounter{enumiv}%
    \let\p@enumiv\@empty\renewcommand\theenumiv{\@arabic\c@enumiv}}%
  \sloppy\clubpenalty4000\@clubpenalty\clubpenalty\widowpenalty4000\sfcode`\.\@m}%
  {\def\@noitemerr{\@latex@warning{Empty `thebibliography' environment}}\endlist}
\makeatother
""",
}
