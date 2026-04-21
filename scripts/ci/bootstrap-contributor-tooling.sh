#!/usr/bin/env bash
# FEAT-CONTRIB-004

set -euo pipefail

usage() {
  cat >&2 <<'EOF'
Usage: scripts/ci/bootstrap-contributor-tooling.sh [--app] [--website] [--vscode] [--playwright] [--all]

Without flags, install the surfaces whose checked-in `.nvmrc` matches the
current shell's Node major:
  Node 25 => browser app
  Node 20 => docs site + VS Code extension

Additional opt-in flags:
  --vscode      Install VS Code extension npm dependencies
  --playwright  Install Playwright Chromium after app dependencies are ready
  --all         Install app, website, vscode, and Playwright tooling
EOF
}

repo_root() {
  cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd
}

log_step() {
  printf '\n[bootstrap] %s\n' "$1"
}

read_nvm_major() {
  local path="$1"

  head -n 1 "$path" | tr -dc '0-9'
}

current_node_major() {
  node -p 'process.versions.node.split(".")[0]'
}

select_default_surfaces() {
  local current_major app_major website_major vscode_major

  current_major="$(current_node_major)"
  app_major="$(read_nvm_major app/.nvmrc)"
  website_major="$(read_nvm_major website/.nvmrc)"
  vscode_major="$(read_nvm_major editors/vscode/.nvmrc)"

  if [[ "$current_major" == "$app_major" ]]; then
    install_app=1
  fi
  if [[ "$current_major" == "$website_major" ]]; then
    install_website=1
  fi
  if [[ "$current_major" == "$vscode_major" ]]; then
    install_vscode=1
  fi

  if [[ "$install_app" -eq 0 && "$install_website" -eq 0 && "$install_vscode" -eq 0 ]]; then
    cat >&2 <<EOF
[bootstrap] Current Node major ${current_major} does not match any checked-in optional surface.
[bootstrap] Switch to Node ${app_major} for app/.nvmrc, or Node ${website_major} for website/.nvmrc and editors/vscode/.nvmrc.
[bootstrap] Alternatively, rerun with explicit flags after switching shells for each surface you need.
EOF
    exit 1
  fi
}

install_app_deps() {
  log_step "Installing browser-app dependencies."
  scripts/ci/pinned-npm.sh install app
  npm --prefix app ci
}

install_website_deps() {
  log_step "Installing docs-site dependencies."
  scripts/ci/pinned-npm.sh install website
  bash scripts/ci/install-docs-site-deps.sh
}

install_vscode_deps() {
  log_step "Installing VS Code extension dependencies."
  scripts/ci/pinned-npm.sh install editors/vscode
  npm --prefix editors/vscode ci
}

install_playwright_browser() {
  log_step "Installing Playwright Chromium for local browser-app end-to-end coverage."
  npx --prefix app playwright install --with-deps chromium
}

print_next_steps() {
  local installed_app=$1
  local installed_website=$2
  local installed_vscode=$3
  local installed_playwright=$4

  printf '\n[bootstrap] Ready.\n'
  if [[ "$installed_app" -eq 1 ]]; then
    printf '[bootstrap] Next app checks: scripts/ci/check-browser-app-freshness.sh'
    if [[ "$installed_playwright" -eq 1 ]]; then
      printf ' && npm --prefix app run test:e2e'
    fi
    printf '\n'
  fi
  if [[ "$installed_website" -eq 1 ]]; then
    printf '[bootstrap] Next docs-site check: npm --prefix website run build\n'
  fi
  if [[ "$installed_vscode" -eq 1 ]]; then
    printf '[bootstrap] Next VS Code extension check: npm --prefix editors/vscode test\n'
  fi
}

main() {
  local root
  local install_app=0
  local install_website=0
  local install_vscode=0
  local install_playwright=0

  root="$(repo_root)"
  cd "$root"

  if [[ $# -eq 0 ]]; then
    select_default_surfaces
  fi

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --app)
        install_app=1
        ;;
      --website)
        install_website=1
        ;;
      --vscode)
        install_vscode=1
        ;;
      --playwright)
        install_playwright=1
        install_app=1
        ;;
      --all)
        install_app=1
        install_website=1
        install_vscode=1
        install_playwright=1
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        usage
        exit 1
        ;;
    esac
    shift
  done

  if [[ "$install_app" -eq 1 ]]; then
    install_app_deps
  fi
  if [[ "$install_website" -eq 1 ]]; then
    install_website_deps
  fi
  if [[ "$install_vscode" -eq 1 ]]; then
    install_vscode_deps
  fi
  if [[ "$install_playwright" -eq 1 ]]; then
    install_playwright_browser
  fi

  print_next_steps "$install_app" "$install_website" "$install_vscode" "$install_playwright"
}

main "$@"
