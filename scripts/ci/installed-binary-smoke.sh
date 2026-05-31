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
print(data["package"]["version"])
PY
}

main() {
  local install_root binary_name installed_binary expected_version actual_version workspace port

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

  port="$(python3 - <<'PY'
import socket

with socket.socket() as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
)"

  "${installed_binary}" workbench "$workspace" --bind 127.0.0.1 --port "$port" >"${temp_root}/workbench.log" 2>&1 &
  server_pid="$!"

  python3 - "$port" <<'PY'
import json
import sys
import time
import urllib.error
import urllib.request

port = sys.argv[1]
base = f"http://127.0.0.1:{port}"

def get(path):
    with urllib.request.urlopen(f"{base}{path}", timeout=1) as response:
        return response.read().decode("utf-8")

deadline = time.time() + 20
last_error = None
while time.time() < deadline:
    try:
        health = json.loads(get("/api/health"))
        assert health["ok"] is True
        actions = json.loads(get("/api/actions"))
        assert any(action["id"] == "request.scope" for action in actions["actions"])
        shell = get("/")
        assert "Syu Workbench" in shell
        assert "/assets/tailwind.css" in shell
        css = get("/assets/tailwind.css")
        assert "--color-command-active" in css
        snapshot = json.loads(get("/api/workspace/snapshot"))
        assert "state" in snapshot
        break
    except (AssertionError, OSError, urllib.error.URLError, TimeoutError) as exc:
        last_error = exc
        time.sleep(0.25)
else:
    raise SystemExit(f"workbench API smoke failed: {last_error}")
PY

  kill "$server_pid"
  wait "$server_pid" 2>/dev/null || true
  server_pid=""
}

main "$@"
