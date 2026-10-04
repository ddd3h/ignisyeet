#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
#
# Print Markdown release notes for <tag>: the commits since the previous v* tag, grouped by
# Conventional Commit type.   scripts/release-notes.sh v0.3.0 [repo-slug]
set -euo pipefail

tag=${1:?usage: release-notes.sh <tag> [owner/repo]}
repo=${2:-${GITHUB_REPOSITORY:-ddd3h/ignisyeet}}
prev=$(git describe --tags --abbrev=0 --match 'v*' "$tag^" 2>/dev/null || true)
range=${prev:+$prev..}$tag

echo "## IgnisYeet ${tag}"
echo
if [ -n "$prev" ]; then echo "Changes since ${prev}."; else echo "First release."; fi

section() { # TITLE REGEX
  local lines
  lines=$(git log --no-merges --format='%s|%h' "$range" | grep -E "^($2)(\([^)]*\))?!?: " |
    sed -E 's/^[a-z]+(\(([^)]*)\))?!?: (.*)\|([0-9a-f]+)$/- \3 (`\4`)/' || true)
  if [ -n "$lines" ]; then printf '\n### %s\n\n%s\n' "$1" "$lines"; fi
}
section "Features" "feat"
section "Bug fixes" "fix"
section "Performance" "perf"
section "Documentation" "docs"
section "Refactoring" "refactor"
section "Tests, build and CI" "test|build|ci|chore|style"

cat <<NOTES

### Install

\`\`\`sh
curl -fsSL https://github.com/${repo}/releases/download/${tag}/install.sh | bash
\`\`\`

Each archive has a matching \`.sha256\` file; \`install.sh\` verifies it before installing.
NOTES
