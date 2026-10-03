# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
# Build the IgnisYeet documentation.
#   make html      figures + HTML (doc/build/html/index.html)
#   make pdf       figures + PDF (doc/build/latex/ignisyeet.pdf, upLaTeX)
#   make figures   TikZ (lualatex + dvisvgm) and matplotlib figures only
# Requires: cargo, uv, lualatex with luatexja, dvisvgm, inkscape, IPAex fonts (PDF: uplatex).

PY      ?= uv run --project .
ROOT    = ..
BIN     = $(ROOT)/target/release/ignisyeet
SAMPLE  = $(ROOT)/examples/sample.toml
GEN     = source/_generated
MC_DIR  = build/mc
MC_OUT  = $(abspath $(ROOT))/examples/out/mc
PN_DIR  = build/panel
PN_OUT  = $(abspath $(ROOT))/examples/out/panel

.PHONY: html pdf figures logo tikz plots screens data clean

html: figures
	$(PY) sphinx-build -b html -W --keep-going source build/html

pdf: figures
	LATEXOPTS="-interaction=nonstopmode -halt-on-error" $(PY) sphinx-build -M latexpdf source build

figures: logo tikz plots screens

# The logo is used as SVG in HTML and as PDF in LaTeX.
logo:
	mkdir -p $(GEN)/logo
	cp IgnisYeet-logo.svg $(GEN)/logo/
	inkscape IgnisYeet-logo.svg --export-type=pdf --export-filename=$(GEN)/logo/IgnisYeet-logo.pdf 2>/dev/null

tikz:
	$(PY) python scripts/build_tikz.py figures/tikz $(GEN)/tikz

data:
	cargo build --release --manifest-path $(ROOT)/Cargo.toml
	$(BIN) aero $(SAMPLE)
	$(BIN) sim $(SAMPLE)
	$(BIN) dispersion $(SAMPLE)
	# Monte Carlo case: sample.toml with mode = "monte_carlo", rk45 and absolute paths (output in examples/out/mc).
	mkdir -p $(MC_DIR)
	sed -e 's|^stl = "|stl = "$(abspath $(ROOT))/examples/|' \
	    -e 's|^eng = "|eng = "$(abspath $(ROOT))/examples/|' \
	    -e 's|^dir = .*|dir = "$(MC_OUT)"|' \
	    -e 's|^mode = "wind_grid".*|mode = "monte_carlo"|' \
	    -e 's|^integrator = "rk4".*|integrator = "rk45"|' \
	    $(SAMPLE) > $(MC_DIR)/mc.toml
	$(BIN) dispersion $(MC_DIR)/mc.toml
	# Panel-method case: sample.toml with method = "panel" and absolute paths (output in examples/out/panel).
	mkdir -p $(PN_DIR)
	sed -e 's|^stl = "|stl = "$(abspath $(ROOT))/examples/|' \
	    -e 's|^eng = "|eng = "$(abspath $(ROOT))/examples/|' \
	    -e 's|^dir = .*|dir = "$(PN_OUT)"|' \
	    -e 's|^method = "barrowman".*|method = "panel"|' \
	    $(SAMPLE) > $(PN_DIR)/panel.toml
	$(BIN) aero $(PN_DIR)/panel.toml
	$(BIN) sim $(PN_DIR)/panel.toml
	$(BIN) dispersion $(PN_DIR)/panel.toml

# Terminal screenshots: the binary runs in a pseudo terminal and its ANSI output is drawn with pyte + Pillow.
screens:
	cargo build --release --manifest-path $(ROOT)/Cargo.toml
	$(PY) python scripts/make_screens.py $(ROOT) $(GEN)/screens

plots: data
	$(PY) python scripts/make_plots.py $(ROOT)/examples/out $(GEN)/plots $(MC_OUT) $(PN_OUT)

clean:
	rm -rf build $(GEN)
