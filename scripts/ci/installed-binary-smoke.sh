#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
temp_root=""
server_pid=""

cleanup() {
  if [[ -n "${server_pid:-}" ]]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
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
package = data["package"]
if "version" in package and isinstance(package["version"], str):
    print(package["version"])
elif package.get("version", {}).get("workspace") is True:
    print(data["workspace"]["package"]["version"])
else:
    raise SystemExit("failed to resolve package version")
PY
}

main() {
  local install_root binary_name installed_binary expected_version actual_version workspace fixture plan projection

  trap cleanup EXIT
  cd "$repo_root"

  temp_root="$(mktemp -d)"
  install_root="${temp_root}/install"
  binary_name="$(resolve_binary_name)"
  installed_binary="${install_root}/bin/${binary_name}"
  expected_version="$(resolve_package_version)"
  workspace="${temp_root}/workspace"
  fixture="${repo_root}/fixtures/v1/valid-web-app"
  plan="${temp_root}/plan.yaml"
  projection="${temp_root}/projection.json"

  cargo install --path "$repo_root" --root "$install_root" --force --locked

  actual_version="$("${installed_binary}" --version)"
  test "${actual_version}" = "syu ${expected_version}"
  cp -R "$fixture" "$workspace"
  git -C "$workspace" init >/dev/null
  git -C "$workspace" config user.email "ci@example.invalid"
  git -C "$workspace" config user.name "CI"
  git -C "$workspace" add -A
  git -C "$workspace" commit --quiet -m "fixture snapshot"
  git -C "$workspace" update-ref refs/remotes/origin/main HEAD

  "${installed_binary}" validate workspace "$workspace"
  "${installed_binary}" work plan \
    --request "${workspace}/work.yaml" \
    --out "$plan" \
    --workspace "$workspace"
  test -f "$plan"

  "${installed_binary}" workbench project \
    --workspace "$workspace" \
    --request "${workspace}/work.yaml" \
    --format json >"$projection"

  python3 - "$projection" <<'PY'
import json
import sys
from pathlib import Path

projection = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
assert projection["work"]["request"]["summary"] == "Keep login failures generic."
assert projection["work"]["plan"] is None
assert projection["diagnostics"]["validation"]["state"] == "not_run"
PY
}

main "$@"
