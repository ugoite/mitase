#!/usr/bin/env bash
# FEAT-CONTRIB-001

set -euo pipefail

repo_root() {
  cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd
}

log_step() {
  printf "\n[devcontainer] %s\n" "$1"
}

install_coverage_tooling() {
  log_step "Installing cargo-llvm-cov for scripts/ci/coverage.sh summary."
  cargo install cargo-llvm-cov --locked
}

install_wasm_tooling() {
  log_step "Installing wasm-pack for scripts/ci/check-browser-app-freshness.sh."
  cargo install wasm-pack --locked
}

install_precommit_tooling() {
  log_step "Installing local hooks with scripts/install-precommit.sh."
  bash scripts/install-precommit.sh
}

main() {
  local root
  root="$(repo_root)"

  cd "$root"

  log_step "Setting up the contributor toolchain. See CONTRIBUTING.md#local-checks for what this bootstrap installs and which workflows still stay opt-in."
  install_coverage_tooling
  install_wasm_tooling
  install_precommit_tooling
  log_step "Browser-app npm installs and Playwright browsers stay opt-in. Run bash .devcontainer/setup-browser-tooling.sh when you need app builds or end-to-end coverage."
  log_step "Devcontainer setup complete."
}

main "$@"
