#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
# One-time setup of the tools of the cross-comparison (needs the network).
#   usage: setup.sh <tool dir>        e.g. setup.sh ~/xcheck-tools
# Creates <tool dir>/venv (Python 3.12 with RocketPy and JPype) and downloads the OpenRocket jar.
# Neither the venv nor the jar belongs to the repository.
set -euo pipefail
TOOLS=$(realpath -m "$1")
mkdir -p "$TOOLS/jar"
uv venv --python 3.12 "$TOOLS/venv"
uv pip install --python "$TOOLS/venv/bin/python" rocketpy==1.13.0 JPype1==1.7.1 numpy==2.5.4 scipy==1.18.1 pandas==3.0.6
JAR="$TOOLS/jar/OpenRocket-24.12.jar"
if [ ! -f "$JAR" ]; then
  curl -L -o "$JAR" "https://github.com/openrocket/openrocket/releases/download/release-24.12/OpenRocket-24.12.jar"
fi
sha256sum "$JAR"   # 4959b72f52f5f607941e9722abbb7b7f0c4a38ebbbf84204a329db9f31c4f897
echo "export OPENROCKET_JAR=$JAR PY=$TOOLS/venv/bin/python"
