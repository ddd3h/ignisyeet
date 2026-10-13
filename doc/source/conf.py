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
templates_path = ["_templates"]
# Old page names redirect to the page that now holds the topic (sphinx-reredirects).
redirects = {
    "overview": "intro.html",
    "aero_coefficients": "aerodynamics.html",
    "cfd_internals": "cfd.html",
    "verification_aero": "verification.html",
    "file_formats": "usage.html",
    "errors": "errors/index.html",
    "safety": "workflows.html",
    "about": "intro.html",
    "tutorial/index": "../quickstart.html",
    "howto/index": "../workflows.html",
    "howto/install": "../install.html",
    "howto/own_rocket": "../workflows.html",
    "howto/motor": "../workflows.html",
    "howto/launcher_parachute": "../workflows.html",
    "howto/environment": "../workflows.html",
    "howto/external_table": "../workflows.html",
    "howto/sweeps": "../workflows.html",
    "howto/dispersion_run": "../workflows.html",
    "howto/keepout_zone": "../workflows.html",
    "howto/cfd_setup": "../cfd_usage.html",
    "howto/cfd_table": "../cfd_usage.html",
    "howto/cad_step": "../cfd_usage.html",
    "howto/read_results": "../usage.html",
    "howto/plots_kml": "../usage.html",
    "howto/warnings_errors": "../usage.html",
    "reference/index": "../usage.html",
    "reference/cli": "../usage.html",
    "reference/output_files": "../usage.html",
    "reference/exit_codes": "../usage.html",
    "reference/error_codes": "../usage.html",
    "reference/installer": "../install.html",
    "reference/symbols": "../intro.html",
    "reference/aero_table_format": "../cfd_usage.html",
    "reference/cfd_output_files": "../cfd_usage.html",
    "reference/config/index": "../../config.html",
    "reference/config/general": "../../config.html",
    "reference/config/sim": "../../config.html",
    "reference/config/aero": "../../config.html",
    "reference/config/cfd": "../../config.html",
    "reference/config/dispersion": "../../config.html",
    "reference/config/environment": "../../config.html",
    "theory/index": "../intro.html",
    "theory/overview": "../intro.html",
    "theory/aero_choose": "../aerodynamics.html",
    "theory/aero_coefficients": "../aerodynamics.html",
    "theory/aero_tables": "../aerodynamics.html",
    "theory/barrowman": "../barrowman.html",
    "theory/panel": "../panel.html",
    "theory/cfd_choose": "../cfd.html",
    "theory/cfd_equations": "../cfd.html",
    "theory/cfd_coefficients": "../cfd_methods.html",
    "theory/atmosphere_gravity": "../environment.html",
    "theory/wind": "../environment.html",
    "theory/coordinates": "../coordinates.html",
    "theory/geometry": "../geometry.html",
    "theory/propulsion": "../propulsion.html",
    "theory/dynamics": "../dynamics.html",
    "theory/descent": "../descent.html",
    "theory/dispersion": "../dispersion.html",
    "theory/limitations": "../limitations.html",
    "theory/integrators": "../dynamics.html",
    "vv/index": "../verification.html",
    "vv/verification": "../verification.html",
    "vv/aero_verification": "../verification.html",
    "vv/cfd_verification": "../verification.html",
    "vv/rans_verification": "../verification.html",
    "vv/cross_comparison": "../validation.html",
    "vv/validation_basic_finner": "../validation.html",
    "vv/performance": "../validation.html",
    "dev/index": "../development.html",
    "dev/architecture": "../development.html",
    "dev/build_release": "../development.html",
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
html_favicon = "_static/landing/logo-mark.svg"
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
ogp_image_alt = "IgnisYeet: CAD データからの空力推算と 6 自由度飛翔・落下分散解析"
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
% A definition list in a table cell (the key cells of the configuration chapter) starts on the first line of the
% row: drop the list skips and the empty line that the cell label leaves in front of the list.
\usepackage{etoolbox}
\AtBeginEnvironment{description}{\ifinner\setlength{\topsep}{0pt}\setlength{\partopsep}{0pt}\setlength{\parsep}{0pt}\par\nointerlineskip\vskip-\baselineskip\fi}
% The key names in those cells may break after an underscore (wind.profile_mean_direction_deg is wider than the cell).
\makeatletter\AtBeginEnvironment{description}{\ifinner\let\IY@us\_\def\_{\IY@us\allowbreak}\fi}\makeatother
% :kbd: keys (the Enter key in the quickstart) are drawn as key caps.
\makeatletter
\protected\def\sphinxkeyboard#1{{\setlength{\fboxsep}{1.2pt}\setlength{\fboxrule}{0.5pt}\fcolorbox{black!50}{black!6}{\strut\,\textsf{\footnotesize\bfseries #1}\,}}}
\makeatother
""",
}
