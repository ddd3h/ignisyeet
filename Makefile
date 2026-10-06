# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
# Convenience wrapper. CONFIG selects the configuration file.
CONFIG ?= examples/sample.toml
BIN = target/release/ignisyeet
# Output directory: [output] dir of the configuration, relative to the configuration file.
OUT_RAW = $(shell awk '/^\[/{s=($$0 ~ /^\[output\]/)} s && /^dir[ \t]*=/{match($$0,/"[^"]*"/); print substr($$0,RSTART+1,RLENGTH-2); exit}' $(CONFIG))
OUT ?= $(if $(filter /%,$(OUT_RAW)),$(OUT_RAW),$(dir $(CONFIG))$(if $(OUT_RAW),$(OUT_RAW),out))
# Plot command; without uv: make plot PLOT="python python/plot.py" (inside a venv with python/requirements.txt)
PLOT ?= uv run --project python python/plot.py

.PHONY: all build test geom aero sim dispersion plot clean

all: aero sim dispersion plot

build:
	cargo build --release

test:
	cargo test --release

geom: build
	$(BIN) geom $(CONFIG)

aero: build
	$(BIN) aero $(CONFIG)

sim: build
	$(BIN) sim $(CONFIG) || [ $$? -eq 3 ]

dispersion: build
	$(BIN) dispersion $(CONFIG) || [ $$? -eq 3 ]

plot:
	$(PLOT) all $(OUT)

clean:
	cargo clean
	rm -rf $(OUT)
