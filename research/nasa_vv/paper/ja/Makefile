# SPDX-License-Identifier: GPL-3.0-or-later
# Builds main.pdf with LuaLaTeX (luatexja). The figures are the committed SVGs of
# ../../figures; they are converted to PDF here, with rsvg-convert when it exists and
# inkscape otherwise. The converted files are not committed. labreport.cls and the
# logo are shared with the English paper and are found through TEXINPUTS.
FIGDIR := ../../figures
SVGS   := $(wildcard $(FIGDIR)/*.svg)
PDFS   := $(patsubst $(FIGDIR)/%.svg,fig/%.pdf,$(SVGS))
RSVG   := $(shell command -v rsvg-convert 2>/dev/null)
INKSC  := $(shell command -v inkscape 2>/dev/null)

main.pdf: main.tex references.bib $(PDFS)
	TEXINPUTS=..: latexmk -lualatex -interaction=nonstopmode main.tex

fig/%.pdf: $(FIGDIR)/%.svg
	mkdir -p fig
ifneq ($(RSVG),)
	rsvg-convert -f pdf -o $@ $<
else ifneq ($(INKSC),)
	inkscape $< --export-type=pdf --export-filename=$@
else
	@echo "neither rsvg-convert nor inkscape is installed" >&2; exit 1
endif

clean:
	TEXINPUTS=..: latexmk -C
	rm -rf fig
.PHONY: clean
