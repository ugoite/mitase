#!/usr/bin/env bash
# FEAT-CONTRIB-003

set -euo pipefail

find_python() {
  if command -v python3 >/dev/null 2>&1; then
    command -v python3
    return 0
  fi

  if command -v python >/dev/null 2>&1; then
    command -v python
    return 0
  fi

  echo "python3 or python is required to install pre-commit" >&2
  exit 1
}

install_precommit() {
  local python_bin="$1"

  if command -v pipx >/dev/null 2>&1; then
    pipx install --force pre-commit
    return 0
  fi

  "$python_bin" -m pip install --user --upgrade pip pre-commit
}

install_hooks() {
  pre-commit install --hook-type pre-commit --hook-type pre-push
}

main() {
  local python_bin
  python_bin="$(find_python)"

  install_precommit "$python_bin"
  install_hooks

  echo "pre-commit is installed and hooks are active."
  echo "Run 'pre-commit run --all-files --hook-stage pre-commit' to verify the local setup."
}

main "$@"
