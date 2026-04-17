#!/usr/bin/env bash
# FEAT-CONTRIB-003

set -euo pipefail

log_step() {
  echo "==> $*"
}

print_troubleshooting_hint() {
  local python_bin="$1"
  echo "Troubleshooting: compare '$python_bin -m site --user-base' with your PATH, then rerun scripts/install-precommit.sh." >&2
  if command -v pipx >/dev/null 2>&1; then
    echo "If you installed pre-commit with pipx, also compare 'pipx environment --value PIPX_BIN_DIR' with your PATH." >&2
  fi
  echo "See CONTRIBUTING.md#local-checks for the expected local bootstrap flow." >&2
}

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
    log_step "Installing pre-commit with pipx."
    if pipx install --force pre-commit; then
      return 0
    fi

    echo "pipx install --force pre-commit failed." >&2
    print_troubleshooting_hint "$python_bin"
    return 1
  fi

  log_step "Installing pre-commit with $python_bin -m pip --user."
  if "$python_bin" -m pip install --user --upgrade pip pre-commit; then
    return 0
  fi

  echo "Python user-base installation failed while running '$python_bin -m pip install --user --upgrade pip pre-commit'." >&2
  print_troubleshooting_hint "$python_bin"
  return 1
}

find_precommit_bin() {
  local python_bin="$1"
  local user_base
  local user_base_output

  if command -v pre-commit >/dev/null 2>&1; then
    command -v pre-commit
    return 0
  fi

  if ! user_base="$("$python_bin" -m site --user-base 2>/dev/null)"; then
    user_base_output="$("$python_bin" -m site --user-base 2>&1 >/dev/null || true)"
    echo "Failed to query the Python user-base with '$python_bin -m site --user-base' while locating pre-commit." >&2
    if [ -n "$user_base_output" ]; then
      echo "$user_base_output" >&2
    fi
    print_troubleshooting_hint "$python_bin"
    return 1
  fi

  if [ -x "$user_base/bin/pre-commit" ]; then
    echo "$user_base/bin/pre-commit"
    return 0
  fi

  echo "Checked Python user-base path: $user_base/bin/pre-commit (not executable)." >&2

  if command -v pipx >/dev/null 2>&1; then
    local pipx_bin
    local pipx_output
    pipx_bin="${PIPX_BIN_DIR:-}"
    if [ -z "$pipx_bin" ]; then
      if ! pipx_bin="$(pipx environment --value PIPX_BIN_DIR 2>/dev/null)"; then
        pipx_output="$(pipx environment --value PIPX_BIN_DIR 2>&1 >/dev/null || true)"
        echo "pipx is installed, but 'pipx environment --value PIPX_BIN_DIR' failed while locating pre-commit." >&2
        if [ -n "$pipx_output" ]; then
          echo "$pipx_output" >&2
        fi
        print_troubleshooting_hint "$python_bin"
        return 1
      fi
    fi
    if [ -n "$pipx_bin" ] && [ -x "$pipx_bin/pre-commit" ]; then
      echo "$pipx_bin/pre-commit"
      return 0
    fi

    if [ -n "$pipx_bin" ]; then
      echo "Checked pipx bin path: $pipx_bin/pre-commit (not executable)." >&2
    else
      echo "pipx is installed, but neither PIPX_BIN_DIR nor 'pipx environment --value PIPX_BIN_DIR' returned a usable bin directory." >&2
    fi
  else
    echo "pipx is not installed, so no pipx bin lookup was available." >&2
  fi

  print_troubleshooting_hint "$python_bin"
  return 1
}

install_hooks() {
  local python_bin="$1"
  local precommit_bin

  if precommit_bin="$(find_precommit_bin "$python_bin")"; then
    log_step "Installing git hooks with $precommit_bin."
    "$precommit_bin" install --hook-type pre-commit --hook-type pre-push
    return 0
  fi

  log_step "Falling back to '$python_bin -m pre_commit install' because no standalone pre-commit binary was found."
  if "$python_bin" -m pre_commit install --hook-type pre-commit --hook-type pre-push; then
    return 0
  fi

  echo "Fallback hook installation via '$python_bin -m pre_commit install' failed." >&2
  print_troubleshooting_hint "$python_bin"
  return 1
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
