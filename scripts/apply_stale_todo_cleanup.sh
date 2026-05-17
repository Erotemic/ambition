#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
git -C "$repo_root" apply "$repo_root/docs/patches/stale_todo_cleanup_2026-05-13.patch"
git -C "$repo_root" diff -- TODO.md
