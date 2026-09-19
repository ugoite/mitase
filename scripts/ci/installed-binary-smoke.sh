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
    MINGW* | MSYS* | CYGWIN*) printf 'mitase.exe\n' ;;
    *) printf 'mitase\n' ;;
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

setup_workspace() {
  local fixture="$1"
  local workspace="$2"

  cp -R "$fixture" "$workspace"
  git -C "$workspace" init >/dev/null
  git -C "$workspace" config user.email "ci@example.invalid"
  git -C "$workspace" config user.name "CI"
  git -C "$workspace" add -A
  git -C "$workspace" commit --quiet -m "fixture snapshot"
  git -C "$workspace" update-ref refs/remotes/origin/main HEAD
}

assert_v1_rejected() {
  local command_name="$1"
  local output_path="$2"
  local error_path="$3"
  shift 3

  if "$installed_binary" "$@" >"$output_path" 2>"$error_path"; then
    echo "$command_name unexpectedly accepted a v1 source" >&2
    return 1
  fi

  OUTPUT_PATH="$output_path" COMMAND_NAME="$command_name" python3 - <<'PY'
import json
import os
from pathlib import Path

payload = json.loads(Path(os.environ["OUTPUT_PATH"]).read_text(encoding="utf-8"))
diagnostics = payload.get("diagnostics", [])
matching = [item for item in diagnostics if item.get("code") == "MITASE-SOURCE-001"]
if len(matching) != 1:
    raise SystemExit(
        f"{os.environ['COMMAND_NAME']} did not report exactly one MITASE-SOURCE-001"
    )
diagnostic = matching[0]
if not diagnostic.get("suggested_action") or "migrate" not in diagnostic["suggested_action"]:
    raise SystemExit(f"{os.environ['COMMAND_NAME']} rejection has no migration action")
if not diagnostic.get("primary", {}).get("path"):
    raise SystemExit(f"{os.environ['COMMAND_NAME']} rejection has no source path")
PY
}

main() {
  local install_root binary_name installed_binary expected_version actual_version workspace fixture
  local v1_fixture v1_workspace validation_report

  trap cleanup EXIT
  cd "$repo_root"

  temp_root="$(mktemp -d)"
  install_root="${temp_root}/install"
  binary_name="$(resolve_binary_name)"
  installed_binary="${install_root}/bin/${binary_name}"
  expected_version="$(resolve_package_version)"
  workspace="${temp_root}/workspace"
  case "$expected_version" in
    0.1.*)
      fixture="${repo_root}/fixtures/v1/valid-web-app"
      ;;
    0.2.*)
      fixture="${repo_root}/fixtures/acceptance/ugoite-current-ops-v2"
      v1_fixture="${repo_root}/fixtures/v1/valid-web-app"
      ;;
    *)
      echo "unsupported release line for installed-binary smoke: $expected_version" >&2
      exit 1
      ;;
  esac
  cargo install --path "$repo_root" --root "$install_root" --force --locked

  actual_version="$("${installed_binary}" --version)"
  test "${actual_version}" = "mitase ${expected_version}"
  setup_workspace "$fixture" "$workspace"

  validation_report="${temp_root}/validation.json"
  if ! "${installed_binary}" validate workspace "$workspace" --format json >"$validation_report"; then
    cat "$validation_report" >&2
    exit 1
  fi

  if [[ "$expected_version" == 0.2.* ]]; then
    "${installed_binary}" check "$workspace" >/dev/null
    v1_workspace="${temp_root}/v1-workspace"
    setup_workspace "$v1_fixture" "$v1_workspace"
    assert_v1_rejected \
      check \
      "${temp_root}/v1-check.json" \
      "${temp_root}/v1-check.stderr" \
      check "$v1_workspace" --format json
    assert_v1_rejected \
      validate \
      "${temp_root}/v1-validate.json" \
      "${temp_root}/v1-validate.stderr" \
      validate workspace "$v1_workspace" --format json
    "${installed_binary}" migrate "$v1_workspace/spec/feature.yaml" --stdout \
      >"${temp_root}/migrated.yaml"
    grep -F "schema: mitase/authoring/v2" "${temp_root}/migrated.yaml" >/dev/null
  fi
}

main "$@"
