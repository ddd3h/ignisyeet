#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
#
# Build the release tarball that install.sh downloads.
#
#   scripts/package-release.sh <target-triple> <version> [binary] [out-dir]
#
#   target-triple  e.g. x86_64-unknown-linux-musl (only used in the file name)
#   version        e.g. v0.2.0
#   binary         path of the built ignisyeet binary
#                  (default: target/<target>/release/ignisyeet, else target/release/ignisyeet)
#   out-dir        default: dist/
#
# Output: <out-dir>/ignisyeet-<version>-<target>.tar.gz and .tar.gz.sha256
set -euo pipefail

target=${1:?usage: package-release.sh <target> <version> [binary] [out-dir]}
version=${2:?usage: package-release.sh <target> <version> [binary] [out-dir]}
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
binary=${3:-}
out=${4:-$root/dist}

if [ -z "$binary" ]; then
  if [ -x "$root/target/$target/release/ignisyeet" ]; then
    binary=$root/target/$target/release/ignisyeet
  else
    binary=$root/target/release/ignisyeet
  fi
fi
[ -x "$binary" ] || { echo "binary not found: $binary" >&2; exit 1; }

name=ignisyeet-${version}-${target}
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
dest=$stage/$name

mkdir -p "$dest/bin" "$dest/python" "$out"
install -m 755 "$binary" "$dest/bin/ignisyeet"
cp -R "$root/examples" "$dest/examples"
rm -rf "$dest/examples/out"
install -m 644 "$root/python/plot.py" "$root/python/requirements.txt" "$root/python/pyproject.toml" "$dest/python/"
install -m 644 "$root/LICENSE" "$root/NOTICE" "$root/CITATION.cff" "$root/README.md" "$dest/"

tarball=$out/$name.tar.gz
# Reproducible-ish: fixed ordering, numeric owner.
tar -C "$stage" --owner=0 --group=0 --numeric-owner -czf "$tarball" "$name" 2>/dev/null \
  || tar -C "$stage" -czf "$tarball" "$name"

if command -v sha256sum >/dev/null 2>&1; then
  (cd "$out" && sha256sum "$name.tar.gz" > "$name.tar.gz.sha256")
else
  (cd "$out" && shasum -a 256 "$name.tar.gz" > "$name.tar.gz.sha256")
fi
echo "$tarball"
echo "$tarball.sha256"
