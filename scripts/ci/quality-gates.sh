#!/usr/bin/env bash
set -euo pipefail

mode="${1:-full}"
if (($# > 0)); then
  shift
fi

case "$mode" in
  full)
    cargo fmt --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    cargo run -- validate workspace .
    ;;
  fast)
    rust_changed=false
    spec_changed=false
    range=""
    declare -a paths=()

    paths_file=""
    cleanup_paths_file() {
      if [[ -n "$paths_file" ]]; then
        rm -f "$paths_file"
      fi
    }
    trap cleanup_paths_file EXIT

    if (($# > 0)); then
      paths=("$@")
    elif [[ -n "${PRE_COMMIT_FROM_REF:-}" && -n "${PRE_COMMIT_TO_REF:-}" ]]; then
      from_ref="$PRE_COMMIT_FROM_REF"
      to_ref="$PRE_COMMIT_TO_REF"
      if [[ "$to_ref" =~ ^0+$ ]]; then
        echo "Deleting a remote ref; skipping fast quality gates."
        exit 0
      fi

      if [[ -n "$(git status --porcelain --untracked-files=all)" ]]; then
        echo "pre-push quality gates require a clean worktree; commit or stash local changes first." >&2
        exit 1
      fi
      if [[ "$(git rev-parse HEAD)" != "$to_ref" ]]; then
        echo "pre-push quality gates require the pushed revision to be checked out at HEAD." >&2
        exit 1
      fi

      paths_file="$(mktemp)"
      if [[ "$from_ref" =~ ^0+$ ]]; then
        empty_tree="$(git hash-object -t tree /dev/null)"
        range="$empty_tree..$to_ref"
        git ls-tree -r --name-only -z "$to_ref" >"$paths_file"
      else
        # A branch may merge origin/main before it is pushed. Comparing it to
        # its stale remote tip would then treat main's already-governed files
        # as new branch changes. The repository's change-validation baseline
        # is origin/main, so use that merge-base whenever it is available.
        if git rev-parse --verify --quiet origin/main >/dev/null; then
          baseline="$(git merge-base "$to_ref" origin/main)"
          range="$baseline..$to_ref"
          git diff --name-only -z "$baseline" "$to_ref" >"$paths_file"
        else
          range="$from_ref..$to_ref"
          git diff --name-only -z "$from_ref" "$to_ref" >"$paths_file"
        fi
      fi
      while IFS= read -r -d '' path; do
        paths+=("$path")
      done <"$paths_file"
      if [[ "$from_ref" =~ ^0+$ ]]; then
        range="$empty_tree..$to_ref"
      fi
    else
      # CI and manual invocations without a comparison range intentionally run
      # every fast gate rather than silently accepting an unknown change set.
      rust_changed=true
      spec_changed=true
    fi

    for path in "${paths[@]}"; do
      case "$path" in
        *.rs|Cargo.toml|*/Cargo.toml|Cargo.lock|*/Cargo.lock|rust-toolchain|rust-toolchain.toml|.cargo/*)
          rust_changed=true
          ;;
      esac
      case "$path" in
        syu.yaml|docs/syu/*)
          spec_changed=true
          ;;
      esac
    done

    if [[ "$rust_changed" == true ]]; then
      cargo fmt --check
      cargo clippy --workspace --all-targets -- -D warnings
    fi
    if [[ "$spec_changed" == true ]]; then
      if [[ -n "$range" ]]; then
        cargo run -- validate change . --range "$range"
      else
        cargo run -- validate change . --staged
      fi
    fi
    if [[ "$rust_changed" == false && "$spec_changed" == false ]]; then
      echo "No Rust or syu specification files changed; skipping fast quality gates."
    fi
    ;;
  *)
    echo "usage: $0 [fast|full]" >&2
    exit 2
    ;;
esac
