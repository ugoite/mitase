#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
temp_root=""
app_pid=""

cleanup() {
  if [[ -n "${app_pid:-}" ]]; then
    kill "$app_pid" >/dev/null 2>&1 || true
    wait "$app_pid" 2>/dev/null || true
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
print(data["package"]["version"])
PY
}

print_app_diagnostics() {
  local app_log="$1"

  echo "installed-binary smoke failed" >&2
  if [[ -n "${app_pid:-}" ]]; then
    if kill -0 "$app_pid" >/dev/null 2>&1; then
      echo "app process is still running (pid=${app_pid})" >&2
    else
      echo "app process exited before readiness completed (pid=${app_pid})" >&2
    fi
  fi
  if [[ -f "$app_log" ]]; then
    echo "--- syu app log ---" >&2
    cat "$app_log" >&2
    echo "-------------------" >&2
  fi
}

wait_for_app_url() {
  local app_log="$1"

  APP_LOG="$app_log" python3 - <<'PY'
import os
import pathlib
import re
import sys
import time

app_log = pathlib.Path(os.environ["APP_LOG"])
pattern = re.compile(r"syu app listening on (http://\S+)")

for _ in range(80):
    if app_log.exists():
        content = app_log.read_text(encoding="utf-8", errors="replace")
        match = pattern.search(content)
        if match:
            print(match.group(1))
            sys.exit(0)
    time.sleep(0.1)

sys.exit(1)
PY
}

wait_for_app_payload() {
  local app_url="$1"

  APP_URL="$app_url" python3 - <<'PY'
import json
import os
import sys
import time
import urllib.request

url = f"{os.environ['APP_URL']}/api/app-data.json"
last_error = None

for _ in range(80):
    try:
        with urllib.request.urlopen(url, timeout=1) as response:
            payload = json.load(response)
        if payload.get("workspace_root") and payload.get("spec_root"):
            sys.exit(0)
        last_error = f"incomplete payload keys from {url}: {sorted(payload.keys())}"
    except Exception as error:
        last_error = f"{type(error).__name__}: {error}"
    time.sleep(0.1)

if last_error:
    print(last_error, file=sys.stderr)
sys.exit(1)
PY
}

main() {
  local install_root binary_name installed_binary expected_version actual_version workspace app_log app_url

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

  app_log="${temp_root}/app.log"
  "${installed_binary}" app "$workspace" --bind 127.0.0.1 --port 0 >"$app_log" 2>&1 &
  app_pid="$!"

  if ! app_url="$(wait_for_app_url "$app_log")"; then
    print_app_diagnostics "$app_log"
    exit 1
  fi

  if ! wait_for_app_payload "$app_url"; then
    print_app_diagnostics "$app_log"
    exit 1
  fi
}

main "$@"
