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

    if (($# > 0)); then
      paths=("$@")
    elif [[ -n "${PRE_COMMIT_FROM_REF:-}" && -n "${PRE_COMMIT_TO_REF:-}" ]]; then
      if [[ "${PRE_COMMIT_FROM_REF}" =~ ^0+$ ]]; then
        while IFS= read -r -d '' path; do
          paths+=("$path")
        done < <(git ls-files -z)
      else
        range="${PRE_COMMIT_FROM_REF}..${PRE_COMMIT_TO_REF}"
        while IFS= read -r -d '' path; do
          paths+=("$path")
        done < <(git diff --name-only -z "$range")
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
      cargo run -- validate change . --staged
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
