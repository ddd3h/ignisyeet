% labreport.cls — organisation-published research report (LaTeX port of report.typ)
% (Named labreport, because report.cls is a standard LaTeX class.)
% Single-column by default; class option `twocolumn` for two columns.
% Compile with LuaLaTeX (latexmk -lualatex).
\NeedsTeXFormat{LaTeX2e}
\ProvidesClass{labreport}[2026/10/17 Lab / company technical report (port of report.typ)]

\RequirePackage{iftex}
\RequireLuaTeX

\newif\iflr@twocol
\DeclareOption{twocolumn}{\lr@twocoltrue}
\DeclareOption{onecolumn}{\lr@twocolfalse}
\ProcessOptions\relax
\iflr@twocol
  \LoadClass[10pt,twocolumn]{article}
  \def\lr@side{52bp}
\else
  \LoadClass[10pt,onecolumn]{article}
  \def\lr@side{110bp}   % shorter measure (about 90 characters per line) in one-column mode
\fi

% ---------------------------------------------------------------- packages
\RequirePackage[a4paper, top=58.4pt, bottom=54.4pt, hmargin=\lr@side,
  headheight=26pt, headsep=7.25pt, footskip=27.8pt, columnsep=22bp]{geometry}
\RequirePackage{fontspec}
\RequirePackage{amsmath}
\RequirePackage{unicode-math}
\RequirePackage[protrusion=false, expansion=false, tracking=true]{microtype}
\RequirePackage{xcolor}
\RequirePackage{fancyhdr}
\RequirePackage[explicit]{titlesec}
\RequirePackage{caption}
\RequirePackage{booktabs}
\RequirePackage{colortbl}
\RequirePackage{etoolbox}
\RequirePackage{graphicx}
\RequirePackage[authoryear,round]{natbib}
\RequirePackage{environ}
\RequirePackage{ragged2e}
\RequirePackage[most]{tcolorbox}
\RequirePackage{hyperref}

% ---------------------------------------------------------------- fonts & colours
\setmainfont{STIX Two Text}
\setmathfont{STIX Two Math}
% brand colours: redefine in the preamble, e.g. \definecolor{lraccent}{HTML}{8B1F3F}
\definecolor{lraccent}{HTML}{1F5F8B}
\definecolor{lrdark}{HTML}{0F3B5C}
\hypersetup{colorlinks=true, linkcolor=lraccent, citecolor=lraccent, urlcolor=lraccent}
\urlstyle{same}

\renewcommand\normalsize{\fontsize{10}{12.3}\selectfont
  \abovedisplayskip 5.75pt plus 1pt minus 1pt \belowdisplayskip 5.75pt plus 1pt minus 1pt
  \abovedisplayshortskip 6pt plus 1pt \belowdisplayshortskip 6pt plus 1pt}
\normalsize
\setlength\parindent{1.5em}
\setlength\parskip{0pt}
\setlength\jot{0.5pt}
\setlength\emergencystretch{1em}
\raggedbottom
\renewcommand\footnoterule{}

% tracked text: \lrtrack{<pt>}{<size pt>}{text}  (microtype amount is in 1/1000 em;
% microtype puts half the tracking before the first letter, so pull it back)
\newcommand\lrtrack[3]{{\fontsize{#2}{#2}\selectfont
  \leavevmode\kern-\fpeval{#1/2}pt\textls[\fpeval{round(1000*#1/#2)}]{#3}}}

% ---------------------------------------------------------------- metadata (all optional)
\newcommand\lr@org{Example Labs}
\newcommand\lr@logo{}
\newif\iflr@showname \lr@shownametrue
\newcommand\lr@shorttitle{}
\newcommand\lr@kind{Technical report}
\newcommand\lr@date{}
\newcommand\lr@reportid{}
\newcommand\lr@url{}
\newcommand\lr@notice{}
\newcommand\lr@correspondence{}
\newcommand\lr@keywords{}
\newcommand\lr@abstract{}
\newcommand\organization[1]{\renewcommand\lr@org{#1}}
\newcommand\logo[1]{\renewcommand\lr@logo{#1}}               % e.g. \logo{\includegraphics[height=18pt]{logo.pdf}}
\newcommand\hidename{\lr@shownamefalse}                     % logo only, no name next to it
\newcommand\shorttitle[1]{\renewcommand\lr@shorttitle{#1}}
\newcommand\kind[1]{\renewcommand\lr@kind{#1}}              % "Technical report", "System card", ... ; empty to hide
\renewcommand\date[1]{\renewcommand\lr@date{#1}}            % e.g. October 2026 (not \today by default)
\newcommand\reportid[1]{\renewcommand\lr@reportid{#1}}      % e.g. EL-TR-2026-07
\newcommand\reporturl[1]{\renewcommand\lr@url{#1}}
\newcommand\notice[1]{\renewcommand\lr@notice{#1}}          % page-1 footer; empty = none
\newcommand\correspondence[1]{\renewcommand\lr@correspondence{#1}}
\newcommand\keywords[1]{\renewcommand\lr@keywords{#1}}
\newcommand\amark[1]{\textcolor{lraccent}{\textsuperscript{#1}}}
% \affiliation{...} (numbered) and \authornote{...} (e.g. \authornote{* Equal contribution}) may be repeated
\newcounter{lraffil}
\newcommand\lr@meta{}
\newcommand\affiliation[1]{\stepcounter{lraffil}%
  \xappto\lr@meta{\noexpand\lr@affilline{\thelraffil}}\gappto\lr@meta{{#1}}}
\newcommand\lr@affilline[2]{\textcolor{lraccent}{\upshape\textsuperscript{#1}}\hspace{0.2em}#2\par}
\newcommand\authornote[1]{\gappto\lr@meta{#1\par}}
\RenewEnviron{abstract}{\global\let\lr@abstract\BODY}

% keywords "a, b, c" -> "a · b · c" (each dot glued to the keyword before it)
\newcommand\lr@kwsep{}
\newcommand\lr@kwitem[1]{\lr@kwsep\mbox{#1}\gdef\lr@kwsep{~\textperiodcentered{} }}
\newcommand\lr@printkeywords{\gdef\lr@kwsep{}%
  \expandafter\forcsvlist\expandafter\lr@kwitem\expandafter{\lr@keywords}}

% join non-empty parts with " · "
\newcommand\lr@sep{}
\newcommand\lr@part[1]{\ifblank{#1}{}{\lr@sep#1\gdef\lr@sep{\ \textperiodcentered\ }}}
\newcommand\lr@mastheadright{\gdef\lr@sep{}%
  \ifdefempty\lr@kind{}{\lr@part{\MakeUppercase{\lr@kind}}}%
  \ifdefempty\lr@date{}{\lr@part{\MakeUppercase{\lr@date}}}%
  \ifdefempty\lr@reportid{}{\lr@part{\MakeUppercase{\lr@reportid}}}}
\newcommand\lr@wordmark{{\fontsize{12}{12}\selectfont\bfseries\color{lrdark}\textls[25]{\lr@org}}}
\newcommand\lr@brand{\ifdefempty\lr@logo{\lr@wordmark}%
  {\raisebox{-4.6pt}{\lr@logo}\iflr@showname\hspace{6pt}\lr@wordmark\fi}}

% ---------------------------------------------------------------- page styles
\pagestyle{fancy}
\fancyhf{}
\renewcommand\headrulewidth{0pt}
\renewcommand\footrulewidth{0pt}
\fancyhead[L]{\parbox[b]{\textwidth}{%
  {\color{lrdark}\bfseries\lrtrack{0.9}{7.6}{\MakeUppercase{\lr@org}}}\hfill
  {\fontsize{9}{10}\selectfont\itshape\lr@shorttitle}\par\nointerlineskip\vspace{5.05pt}%
  {\color{lrdark}\rule{\textwidth}{0.4pt}}}}
\fancyfoot[C]{\fontsize{8.6}{10}\selectfont\thepage}

\fancypagestyle{lrfirst}{\fancyhf{}%
  \ifdefempty\lr@notice{}{%
  \fancyfoot[L]{\parbox[t]{\textwidth}{\vspace{-18.85pt}%
    {\color{lraccent}\rule{\textwidth}{0.5pt}}\par\vspace{2.05pt}%
    \fontsize{7.6}{9.6}\selectfont\raggedright\lr@notice}}}}

% ---------------------------------------------------------------- page-1 masthead (absolute)
\newlength\lr@tw
\newcommand\lr@masthead{%
  \setlength\unitlength{1pt}\setlength\lr@tw{\textwidth}%
  \put(\strip@pt\dimexpr\lr@side\relax,-42.2){\makebox[\lr@tw][s]{\lr@brand\hfill
    \raisebox{\ifdefempty\lr@logo{0pt}{1.6pt}}{\lrtrack{0.9}{7.6}{\lr@mastheadright}}}}%
  \put(\strip@pt\dimexpr\lr@side\relax,-52.85){\textcolor{lrdark}{\rule{\lr@tw}{0.6pt}}}}

% ---------------------------------------------------------------- headings
\setcounter{secnumdepth}{3}
\titleformat{\section}{\color{lraccent}\bfseries}
  {\lrtrack{1.4}{10.6}{\thesection}}{1em}{\lrtrack{1.4}{10.6}{\MakeUppercase{#1}}}
\titleformat{name=\section, numberless}{\color{lraccent}\bfseries}
  {}{0pt}{\lrtrack{1.4}{10.6}{\MakeUppercase{#1}}}
\titleformat{\subsection}{\bfseries}{\thesubsection}{1em}{#1}
\titleformat{\subsubsection}{\itshape}{\thesubsubsection}{1em}{#1}
\titlespacing*{\section}{0pt}{16.2pt}{5pt}
\titlespacing*{\subsection}{0pt}{7.8pt}{1.8pt}
\titlespacing*{\subsubsection}{0pt}{5.8pt}{1.2pt}

% titlesec drops a subsection's space-above right after a section heading;
% Typst keeps the larger of the two, so add the difference back
\pretocmd{\subsection}{\if@nobreak\par\nobreak\vspace{3.8pt}\fi}{}{}

% ---------------------------------------------------------------- captions, tables
\DeclareCaptionLabelSeparator{vgap}{.\hspace{0.6em}}
\captionsetup{font={stretch=1}, size=footnotesize, labelfont={bf,color=lraccent},
  labelsep=vgap, justification=justified, singlelinecheck=false, skip=8pt}
\renewcommand\footnotesize{\fontsize{8.8}{10.8}\selectfont}
\captionsetup[table]{position=top}
\setlength\heavyrulewidth{0.7pt}
\setlength\lightrulewidth{0.4pt}
\arrayrulecolor{lrdark}
\AtBeginEnvironment{tabular}{\fontsize{9}{11}\selectfont}
\AtBeginEnvironment{tabular*}{\fontsize{9}{11}\selectfont}
\renewcommand\arraystretch{1.36}
\RequirePackage{tabularx}
\AtBeginEnvironment{tabularx}{\fontsize{9}{11}\selectfont}
% rule spacing as in the Typst version: airy below the top rule and above the bottom rule,
% tight under the header rule
\setlength\belowrulesep{0pt}
\let\lr@toprule\toprule
\let\lr@bottomrule\bottomrule
\renewcommand\toprule{\lr@toprule\addlinespace[7.3pt]}
\renewcommand\bottomrule{\addlinespace[5.9pt]\lr@bottomrule}
% like Typst, the paragraph that follows a table/figure in the source is not indented
\newcommand\lr@noindentnext{\global\everypar{{\setbox\z@\lastbox}\global\everypar{}}}
\AfterEndEnvironment{table}{\lr@noindentnext}
\AfterEndEnvironment{figure}{\lr@noindentnext}
\setlength\textfloatsep{12pt plus 2pt minus 2pt}
\setlength\floatsep{12pt plus 2pt minus 2pt}
\setlength\intextsep{12pt plus 2pt minus 2pt}

% ---------------------------------------------------------------- references
\setlength\bibsep{3pt}
\setlength\bibhang{1.5em}
\renewcommand\bibfont{\fontsize{8.8}{10.8}\selectfont\raggedright}
\renewcommand\bibsection{\section*{References}}

% ---------------------------------------------------------------- title block
\newcommand\lr@front{%
  \raggedright
  \vspace*{\iflr@twocol 25.9pt\else 4.7pt\fi}%
  {\fontsize{22}{27}\selectfont\bfseries\@title\par}
  \vspace{16.8pt}%
  {\fontsize{12.5}{16.5}\selectfont\@author\par}
  \ifboolexpr{test{\ifdefempty\lr@meta} and test{\ifdefempty\lr@correspondence} and test{\ifdefempty\lr@url}}{}{%
    \vspace{10.3pt}%
    {\fontsize{8.8}{10.8}\selectfont\itshape\lr@meta
     \ifdefempty\lr@correspondence{}{Correspondence to: {\upshape\href{mailto:\lr@correspondence}{\lr@correspondence}}\par}%
     \ifdefempty\lr@url{}{{\upshape\url{\lr@url}}\par}}}%
  \ifboolexpr{test{\ifdefempty\lr@abstract} and test{\ifdefempty\lr@keywords}}{}{%
    \vspace{21.4pt}%
    \begin{tcolorbox}[enhanced, sharp corners, boxrule=0pt, frame hidden, colback=lraccent!7,
        borderline west={2pt}{0pt}{lraccent}, left=14pt, right=14pt, top=12pt, bottom=13.2pt,
        boxsep=0pt, before skip=0pt, after skip=0pt]
      \ifdefempty\lr@abstract{}{%
        {\color{lraccent}\bfseries\lrtrack{2}{10}{ABSTRACT}}\par\vspace{5pt}%
        {\noindent\justifying\fontsize{10}{12.4}\selectfont\lr@abstract\par}}%
      \ifdefempty\lr@keywords{}{%
        \ifdefempty\lr@abstract{}{\vspace{8pt}}%
        {\fontsize{8.8}{11.2}\selectfont\noindent
         \sbox0{\color{lraccent}\bfseries\lrtrack{1}{8.8}{KEYWORDS}}%
         \usebox0\hspace{0.7em}%
         \parbox[t]{\dimexpr\linewidth-\wd0-0.7em\relax}{\raggedright\lr@printkeywords\strut}\par}}%
    \end{tcolorbox}}%
  % in two-column mode the first heading starts a fresh column (its space above is dropped)
  \vspace{\iflr@twocol 22.2pt\else 8pt\fi}%
}

\renewcommand\maketitle{%
  \AddToHookNext{shipout/background}{\lr@masthead}%
  \iflr@twocol\twocolumn[\lr@front]\else{\lr@front}\fi
  \thispagestyle{lrfirst}}

\endinput
