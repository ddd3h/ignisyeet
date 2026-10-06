# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
# Convenience wrapper. CONFIG selects the configuration file.
CONFIG ?= examples/sample.toml
BIN ?= $(or $(CARGO_TARGET_DIR),target)/release/ignisyeet
# Output directory: [output] dir of the configuration (basic or literal TOML string; indentation, a quoted key and
# spaces inside the table header are accepted), relative to the configuration file.
# The shell computes it so that directories containing spaces work (make's $(dir) splits on whitespace).
OUT_SHELL := $(shell d=$$(dirname "$(CONFIG)"); r=$$(awk '/^[ \t]*\[/{s=($$0 ~ /^[ \t]*\[[ \t]*("output"|\047output\047|output)[ \t]*\]/)} s && /^[ \t]*("dir"|\047dir\047|dir)[ \t]*=/{sub(/^[^=]*=[ \t]*/,""); if (match($$0,/^("[^"]*"|\047[^\047]*\047)/)) print substr($$0,RSTART+1,RLENGTH-2); exit}' "$(CONFIG)"); if [ -z "$$r" ]; then printf '%s' "$$d/out"; elif [ "$$(printf %.1s "$$r")" = / ]; then printf '%s' "$$r"; else printf '%s' "$$d/$$r"; fi)
OUT ?= $(OUT_SHELL)
# Plot command; without uv: make plot PLOT="python python/plot.py" (inside a venv with python/requirements.txt)
PLOT ?= uv run --project python python/plot.py

# Status 3 (results written but incomplete) from sim/dispersion is recorded here; `make all` still plots, then fails.
INCOMPLETE = $(OUT)/.incomplete

# aero, sim and dispersion share the aero table cache: never run them in parallel (make -j).
.NOTPARALLEL:

.PHONY: all begin finish build test geom aero sim dispersion plot clean

all: begin aero sim dispersion plot finish

begin:
	rm -f "$(INCOMPLETE)"

finish:
	@if [ -f "$(INCOMPLETE)" ]; then echo "make: results are INCOMPLETE or NOT USABLE (exit status 3): see the lines above" >&2; rm -f "$(INCOMPLETE)"; exit 3; fi

build:
	cargo build --release

test:
	cargo test --release

geom: build
	"$(BIN)" geom "$(CONFIG)"

aero: build
	"$(BIN)" aero "$(CONFIG)"

sim: build
	"$(BIN)" sim "$(CONFIG)" || { s=$$?; [ $$s -eq 3 ] || exit $$s; mkdir -p "$(OUT)" && touch "$(INCOMPLETE)"; }

dispersion: build
	"$(BIN)" dispersion "$(CONFIG)" || { s=$$?; [ $$s -eq 3 ] || exit $$s; mkdir -p "$(OUT)" && touch "$(INCOMPLETE)"; }

plot:
	$(PLOT) all "$(OUT)"

clean:
	cargo clean
	rm -rf "$(OUT)"
