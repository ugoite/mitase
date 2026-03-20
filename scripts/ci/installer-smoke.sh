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

resolve_target_triple() {
  local os_name arch_name

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
    Linux) printf '%s\n' "${arch_name}-unknown-linux-gnu" ;;
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
    printf 'syu.exe\n'
  else
    printf 'syu\n'
  fi
}

start_registry() {
  local mode="$1"
  local port="$2"
  local target="$3"
  local server_log="$4"

  python3 -u "$repo_root/scripts/ci/mock_package_registry.py" \
    --mode "$mode" \
    --port "$port" \
    --target "$target" \
    --package-repository "test/syu" \
    >"$server_log" 2>&1 &
  mock_server_pid="$!"

  for _ in $(seq 1 250); do
    if curl --silent --show-error --fail "http://127.0.0.1:${port}/token?scope=repository:test/syu:pull&service=127.0.0.1" >/dev/null 2>&1; then
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

  temp_root="$(mktemp -d)"
  server_log="${temp_root}/registry.log"
  port="$(python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()')"

  start_registry "$mode" "$port" "$target" "$server_log"
  install_dir="${temp_root}/bin"
  installed_binary="${install_dir}/${binary_name}"

  env \
    SYU_PACKAGE_SCHEME="http" \
    SYU_PACKAGE_HOST="127.0.0.1:${port}" \
    SYU_PACKAGE_REPOSITORY="test/syu" \
    SYU_INSTALL_DIR="$install_dir" \
    SYU_VERSION="$selector" \
    bash "$repo_root/scripts/install-syu.sh"

  grep -F "mock syu ${expected_version} ${target}" "$installed_binary" >/dev/null

  kill "$mock_server_pid" >/dev/null 2>&1 || true
  wait "$mock_server_pid" 2>/dev/null || true
  mock_server_pid=""
  rm -rf "$temp_root"
}

main() {
  local target binary_name

  trap cleanup EXIT

  target="$(resolve_target_triple)"
  binary_name="$(resolve_binary_name "$target")"

  run_install_case "prerelease" "latest" "v0.0.2-beta.1" "$target" "$binary_name"
  run_install_case "prerelease" "alpha" "v0.0.1-alpha.3" "$target" "$binary_name"
  run_install_case "prerelease" "v0.0.1-alpha.2" "v0.0.1-alpha.2" "$target" "$binary_name"
  run_install_case "mixed" "stable" "v0.0.2" "$target" "$binary_name"
}

main "$@"
