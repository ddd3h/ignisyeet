#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
# Run IgnisYeet on examples/sample.toml for the cross-comparison.
#   usage: run_ignisyeet.sh <ignisyeet binary> <work dir>
# Cases (one output directory each under <work dir>):
#   nowind     no wind, extra_cd = 0
#   wind       constant 4 m/s from 270 deg, extra_cd = 0
#   nowind_cd  no wind, extra_cd = 0.02 (the sample as it is)
#   wind_cd    constant 4 m/s from 270 deg, extra_cd = 0.02
# extra_cd is the lumped drag of launch lugs; the other two programs have no such component,
# so the cases without it are the ones compared.
set -euo pipefail
BIN=$(realpath "$1")
WORK=$(realpath -m "$2")
HERE=$(cd "$(dirname "$0")" && pwd)
SAMPLE="$HERE/../../../examples/sample.toml"
mkdir -p "$WORK"
run() { # name extra_cd wind_speed
  "$BIN" -q --no-progress sim "$SAMPLE" \
    --set "output.dir=$WORK/$1" --set "output.kml=false" --set "sim.output_interval=0.01" \
    --set "rocket.extra_cd=$2" \
    --set "wind.model=constant" --set "wind.speed=$3" --set "wind.direction_deg=270" >/dev/null || [ $? -eq 3 ]
}
run nowind 0.0 0.0
run wind 0.0 4.0
run nowind_cd 0.02 0.0
run wind_cd 0.02 4.0
"$BIN" -q --no-progress geom "$SAMPLE" --set "output.dir=$WORK/geom" >/dev/null
"$BIN" --version | head -1 > "$WORK/version.txt"
echo "IgnisYeet cases written to $WORK"
