# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
# Convenience wrapper. CONFIG selects the configuration file.
CONFIG ?= examples/sample.toml
BIN = target/release/ignisyeet
OUT ?= $(dir $(CONFIG))out
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
	$(BIN) sim $(CONFIG)

dispersion: build
	$(BIN) dispersion $(CONFIG)

plot:
	$(PLOT) all $(OUT)

clean:
	cargo clean
	rm -rf $(OUT)
