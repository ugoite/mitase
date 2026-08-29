#!/usr/bin/env bash
# FEAT-QUALITY-001
#
# Compatibility wrapper for existing local scripts and transitional workflow
# references. The canonical command definitions live in root mise.toml.
set -euo pipefail

mode="${1:-full}"
case "$mode" in
  full)
    exec mise run ci
    ;;
  fast)
    mise run ci:lane:rust-check
    exec mise run ci:lane:repo
    ;;
  *)
    echo "usage: $0 [fast|full]" >&2
    exit 2
    ;;
esac
