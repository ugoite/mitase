#!/usr/bin/env bash
# FEAT-CONTRIB-004

set -euo pipefail

usage() {
  cat >&2 <<'EOF'
Usage: scripts/ci/bootstrap-contributor-tooling.sh [--website] [--vscode] [--all]

Without flags, install the surfaces whose checked-in `.nvmrc` matches the
current shell's Node major:
  Node 20 => docs site + VS Code extension

Additional opt-in flags:
  --vscode      Install VS Code extension npm dependencies
  --all         Install website and vscode tooling
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
  local current_major website_major vscode_major

  current_major="$(current_node_major)"
  website_major="$(read_nvm_major website/.nvmrc)"
  vscode_major="$(read_nvm_major editors/vscode/.nvmrc)"

  if [[ "$current_major" == "$website_major" ]]; then
    install_website=1
  fi
  if [[ "$current_major" == "$vscode_major" ]]; then
    install_vscode=1
  fi

  if [[ "$install_website" -eq 0 && "$install_vscode" -eq 0 ]]; then
    cat >&2 <<EOF
[bootstrap] Current Node major ${current_major} does not match any checked-in optional surface.
[bootstrap] Switch to Node ${website_major} for website/.nvmrc or editors/vscode/.nvmrc.
[bootstrap] Alternatively, rerun with explicit flags after switching shells for each surface you need.
EOF
    exit 1
  fi
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

print_next_steps() {
  local installed_website=$1
  local installed_vscode=$2

  printf '\n[bootstrap] Ready.\n'
  if [[ "$installed_website" -eq 1 ]]; then
    printf '[bootstrap] Next docs-site check: npm --prefix website run build\n'
  fi
  if [[ "$installed_vscode" -eq 1 ]]; then
    printf '[bootstrap] Next VS Code extension check: npm --prefix editors/vscode test\n'
  fi
}

main() {
  local root
  local install_website=0
  local install_vscode=0

  root="$(repo_root)"
  cd "$root"

  if [[ $# -eq 0 ]]; then
    select_default_surfaces
  fi

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --website)
        install_website=1
        ;;
      --vscode)
        install_vscode=1
        ;;
      --all)
        install_website=1
        install_vscode=1
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

  if [[ "$install_website" -eq 1 ]]; then
    install_website_deps
  fi
  if [[ "$install_vscode" -eq 1 ]]; then
    install_vscode_deps
  fi

  print_next_steps "$install_website" "$install_vscode"
}

main "$@"
