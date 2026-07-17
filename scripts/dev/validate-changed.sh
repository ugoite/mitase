#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

is_relevant_path() {
  local path="$1"

  case "$path" in
    syu.yaml|docs/syu/*)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

validate_changed() {
  local repo_root
  local -a files=()
  local path

  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  cd "$repo_root"
  if (($# > 0)); then
    files=("$@")
  else
    local files_file
    files_file="$(mktemp)"
    trap 'rm -f "$files_file"' RETURN
    git diff --name-only --cached --diff-filter=ACMR -z >"$files_file"
    while IFS= read -r -d '' path; do
      files+=("$path")
    done <"$files_file"
  fi

  for path in "${files[@]}"; do
    if is_relevant_path "$path"; then
      cargo run -- validate change . --staged
      return 0
    fi
  done

  echo "No repository-relevant files changed; skipping syu validation."
}

validate_changed "$@"
