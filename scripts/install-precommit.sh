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

find_precommit_bin() {
  local python_bin="$1"

  if command -v pre-commit >/dev/null 2>&1; then
    command -v pre-commit
    return 0
  fi

  local user_base
  user_base="$("$python_bin" -m site --user-base)"
  if [ -x "$user_base/bin/pre-commit" ]; then
    echo "$user_base/bin/pre-commit"
    return 0
  fi

  if command -v pipx >/dev/null 2>&1; then
    local pipx_bin
    pipx_bin="${PIPX_BIN_DIR:-}"
    if [ -z "$pipx_bin" ]; then
      pipx_bin="$(pipx environment --value PIPX_BIN_DIR 2>/dev/null || true)"
    fi
    if [ -n "$pipx_bin" ] && [ -x "$pipx_bin/pre-commit" ]; then
      echo "$pipx_bin/pre-commit"
      return 0
    fi
  fi

  return 1
}

install_hooks() {
  local python_bin="$1"
  local precommit_bin

  if precommit_bin="$(find_precommit_bin "$python_bin")"; then
    "$precommit_bin" install --hook-type pre-commit --hook-type pre-push
    return 0
  fi

  "$python_bin" -m pre_commit install --hook-type pre-commit --hook-type pre-push
}

main() {
  local python_bin
  python_bin="$(find_python)"

  install_precommit "$python_bin"
  install_hooks "$python_bin"

  echo "pre-commit is installed and hooks are active."
  echo "Run 'pre-commit run --all-files --hook-stage pre-commit' to verify the local setup."
}

main "$@"
