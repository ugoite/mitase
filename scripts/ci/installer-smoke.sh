#!/usr/bin/env bash
# FEAT-INSTALL-001

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
mock_server_pid=""

cleanup() {
  if [[ -n "${mock_server_pid:-}" ]]; then
    kill "$mock_server_pid" >/dev/null 2>&1 || true
  fi
}

resolve_repo_release_tag() {
  python3 - "$repo_root/Cargo.toml" <<'PY'
import sys
from pathlib import Path
import tomllib

with Path(sys.argv[1]).open("rb") as handle:
    data = tomllib.load(handle)
package = data["package"]
if "version" in package and isinstance(package["version"], str):
    version = package["version"]
elif package.get("version", {}).get("workspace") is True:
    version = data["workspace"]["package"]["version"]
else:
    raise SystemExit("failed to resolve version from Cargo.toml")
print(f"v{version}")
PY
}

resolve_target_triple() {
  local os_name arch_name

  if [[ -n "${MITASE_INSTALL_SMOKE_TARGET:-}" ]]; then
    printf '%s\n' "$MITASE_INSTALL_SMOKE_TARGET"
    return 0
  fi

  os_name="$(uname -s)"
  arch_name="$(uname -m)"

  case "$arch_name" in
    x86_64 | amd64) arch_name="x86_64" ;;
    arm64 | aarch64) arch_name="aarch64" ;;
    *)
      echo "unsupported architecture: $arch_name" >&2
      exit 1
      ;;
  esac

  case "$os_name" in
    Darwin) printf '%s\n' "${arch_name}-apple-darwin" ;;
    Linux)
      if [[ "$arch_name" == "aarch64" ]]; then
        printf '%s\n' "x86_64-unknown-linux-gnu"
      else
        printf '%s\n' "${arch_name}-unknown-linux-gnu"
      fi
      ;;
    MINGW* | MSYS* | CYGWIN*) printf '%s\n' "${arch_name}-pc-windows-msvc" ;;
    *)
      echo "unsupported operating system: $os_name" >&2
      exit 1
      ;;
  esac
}

resolve_binary_name() {
  local target="$1"
  if [[ "$target" == *windows* ]]; then
    printf 'mitase.exe\n'
  else
    printf 'mitase\n'
  fi
}

start_registry() {
  local mode="$1"
  local port="$2"
  local target="$3"
  local default_version="$4"
  local server_log="$5"

  python3 -u "$repo_root/scripts/ci/mock_package_registry.py" \
    --mode "$mode" \
    --port "$port" \
    --target "$target" \
    --default-version "$default_version" \
    --package-repository "test/mitase" \
    >"$server_log" 2>&1 &
  mock_server_pid="$!"

  for _ in $(seq 1 250); do
    if curl --silent --show-error --fail "http://127.0.0.1:${port}/token?scope=repository:test/mitase:pull&service=127.0.0.1" >/dev/null 2>&1; then
      return 0
    fi
    if ! kill -0 "$mock_server_pid" >/dev/null 2>&1; then
      break
    fi
    sleep 0.1
  done

  if [[ -s "$server_log" ]]; then
    cat "$server_log" >&2
  fi
  echo "mock registry did not start" >&2
  exit 1
}

run_install_case() {
  local mode="$1"
  local selector="$2"
  local expected_version="$3"
  local target="$4"
  local binary_name="$5"
  local temp_root port install_dir installed_binary server_log
  local selector_env=(MITASE_VERSION="")

  temp_root="$(mktemp -d)"
  server_log="${temp_root}/registry.log"
  port="$(python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()')"

  start_registry "$mode" "$port" "$target" "$expected_version" "$server_log"
  install_dir="${temp_root}/bin"
  installed_binary="${install_dir}/${binary_name}"

  if [[ -n "$selector" ]]; then
    selector_env=(MITASE_VERSION="$selector")
  fi

  env \
    MITASE_PACKAGE_SCHEME="http" \
    MITASE_PACKAGE_HOST="127.0.0.1:${port}" \
    MITASE_PACKAGE_REPOSITORY="test/mitase" \
    MITASE_INSTALL_DIR="$install_dir" \
    MITASE_TARGET_TRIPLE="$target" \
    "${selector_env[@]}" \
    bash "$repo_root/scripts/install-mitase.sh"

  grep -F "mock mitase ${expected_version} ${target}" "$installed_binary" >/dev/null

  kill "$mock_server_pid" >/dev/null 2>&1 || true
  wait "$mock_server_pid" 2>/dev/null || true
  mock_server_pid=""
  rm -rf "$temp_root"
}

main() {
  local target binary_name
  local default_version

  trap cleanup EXIT

  target="$(resolve_target_triple)"
  binary_name="$(resolve_binary_name "$target")"
  default_version="$(resolve_repo_release_tag)"

  run_install_case "prerelease" "latest" "v0.0.2-beta.1" "$target" "$binary_name"
  run_install_case "prerelease" "alpha" "v0.0.1-alpha.3" "$target" "$binary_name"
  run_install_case "prerelease" "v0.0.1-alpha.2" "v0.0.1-alpha.2" "$target" "$binary_name"
  run_install_case "mixed" "stable" "v0.0.2" "$target" "$binary_name"
  run_install_case "mixed" "" "$default_version" "$target" "$binary_name"
}

main "$@"
