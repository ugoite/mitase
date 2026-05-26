#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
temp_root=""

cleanup() {
  if [[ -n "${temp_root:-}" && -d "${temp_root}" ]]; then
    rm -rf "${temp_root}"
  fi
}

resolve_binary_name() {
  case "$(uname -s)" in
    MINGW* | MSYS* | CYGWIN*) printf 'syu.exe\n' ;;
    *) printf 'syu\n' ;;
  esac
}

resolve_package_version() {
  python3 - <<'PY'
from pathlib import Path
import tomllib

cargo_toml = Path("Cargo.toml")
with cargo_toml.open("rb") as handle:
    data = tomllib.load(handle)
print(data["package"]["version"])
PY
}

main() {
  local install_root binary_name installed_binary expected_version actual_version workspace

  trap cleanup EXIT
  cd "$repo_root"

  temp_root="$(mktemp -d)"
  install_root="${temp_root}/install"
  binary_name="$(resolve_binary_name)"
  installed_binary="${install_root}/bin/${binary_name}"
  expected_version="$(resolve_package_version)"
  workspace="${temp_root}/workspace"

  cargo install --path "$repo_root" --root "$install_root" --force --locked

  actual_version="$("${installed_binary}" --version)"
  test "${actual_version}" = "syu ${expected_version}"

  "${installed_binary}" init "$workspace" >/dev/null
  test -f "${workspace}/syu.yaml"
  test -d "${workspace}/docs/syu"

  "${installed_binary}" validate "$workspace" >/dev/null
  "${installed_binary}" browse "$workspace" --non-interactive >/dev/null
}

main "$@"
