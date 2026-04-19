#!/usr/bin/env bash
# FEAT-CONTRIB-001 FEAT-CONTRIB-002 FEAT-DOCS-002 FEAT-QUALITY-001

set -euo pipefail

usage() {
  echo "Usage: scripts/ci/pinned-npm.sh <check|install> <package-dir>" >&2
}

repo_root() {
  cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd
}

package_manager_for() {
  local package_json=$1

  node -p "JSON.parse(require('node:fs').readFileSync(process.argv[1], 'utf8')).packageManager ?? ''" "$package_json"
}

required_npm_version() {
  local package_dir=$1
  local package_json=$2
  local package_manager

  package_manager="$(package_manager_for "$package_json")"
  if [[ ! $package_manager =~ ^npm@([0-9][0-9A-Za-z.+-]*)$ ]]; then
    echo "Expected ${package_dir}/package.json to declare packageManager: npm@<version>." >&2
    exit 1
  fi

  printf '%s\n' "${BASH_REMATCH[1]}"
}

main() {
  if [[ $# -ne 2 ]]; then
    usage
    exit 1
  fi

  local mode=$1
  local package_dir=$2
  local root
  local package_json
  local required
  local current

  if [[ $mode != "check" && $mode != "install" ]]; then
    usage
    exit 1
  fi

  root="$(repo_root)"
  package_json="${root}/${package_dir}/package.json"

  if [[ ! -f $package_json ]]; then
    echo "Missing package.json at ${package_dir}/package.json." >&2
    exit 1
  fi

  required="$(required_npm_version "$package_dir" "$package_json")"
  current="$(npm --version)"

  if [[ $mode == "install" && $current != "$required" ]]; then
    echo "Installing npm@${required} to match ${package_dir}/package.json." >&2
    npm install --global "npm@${required}"
    current="$(npm --version)"
  fi

  if [[ $current != "$required" ]]; then
    echo "Expected npm ${required} for ${package_dir}/package.json, found ${current}." >&2
    echo "Run 'scripts/ci/pinned-npm.sh install ${package_dir}' before invoking npm for ${package_dir}/." >&2
    exit 1
  fi
}

main "$@"
