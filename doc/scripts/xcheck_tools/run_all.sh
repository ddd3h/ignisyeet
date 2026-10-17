#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
# Regenerate everything in doc/data/xcheck_tools/ (see README.md for the one-time setup).
#   usage: run_all.sh <work dir>
# Needs: $OPENROCKET_JAR (the OpenRocket jar), $PY (python of the venv with rocketpy, JPype1, pandas),
#        $IGNISYEET (the release binary; default: <repo>/target/release/ignisyeet).
set -euo pipefail
WORK=$(realpath -m "$1")
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../../.." && pwd)
PY=${PY:-python3}
IGNISYEET=${IGNISYEET:-$ROOT/target/release/ignisyeet}
: "${OPENROCKET_JAR:?set OPENROCKET_JAR to the OpenRocket jar}"
mkdir -p "$WORK/or" "$WORK/rp"

bash "$HERE/run_ignisyeet.sh" "$IGNISYEET" "$WORK/ign"
GEOM="$WORK/ign/geom/geometry.json"

for fins in rounded square airfoil; do
  "$PY" "$HERE/run_openrocket.py" "$GEOM" "$WORK/or" --fins "$fins" $([ "$fins" = rounded ] && echo "--ork $ROOT/doc/data/xcheck_tools/sample_rocket.ork") >/dev/null
done
"$PY" "$HERE/run_openrocket.py" "$GEOM" "$WORK/or" --fins rounded --dt 0.002 >/dev/null

"$PY" "$HERE/make_drag_inputs.py" "$WORK"
D="$ROOT/doc/data/xcheck_tools"
"$PY" "$HERE/run_rocketpy.py" "$GEOM" "$WORK/rp" --drag "or=$D/drag_or.csv" --drag "ign=$D/drag_ign.csv" >/dev/null

"$PY" "$HERE/make_comparison.py" "$WORK"
"$PY" "$HERE/check_environment.py" "$WORK"
"$PY" "$HERE/fit_nose.py" "$WORK/ign/geom/profile.csv" >/dev/null
"$PY" "$HERE/run_basic_finner.py" >/dev/null
"$PY" "$HERE/write_provenance.py" "$IGNISYEET" >/dev/null
