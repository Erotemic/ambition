#!/usr/bin/env bash
set -euo pipefail

# Overlay ZIPs cannot delete files, so this script performs the approved
# cleanup after the replacement docs have been extracted.
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ ! -f docs/archive/FEATURES.md ]]; then
  echo "expected docs/archive/FEATURES.md from the overlay; aborting" >&2
  exit 1
fi
if [[ ! -d docs/archive/vertical-slices ]]; then
  echo "expected docs/archive/vertical-slices from the overlay; aborting" >&2
  exit 1
fi

rm -f FEATURES.md
rm -rf docs/history
rm -rf dev/vertical-slices

echo "Removed stale active docs: FEATURES.md, docs/history, dev/vertical-slices"
