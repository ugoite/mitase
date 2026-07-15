# Contributing

Run the v1 release gates before opening a change:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -- validate change . --range origin/main...HEAD
```

Executable changes should be driven by `syu work plan` from exact specification anchors. Keep edits inside the selected slice and use `syu validate --plan` to verify scope.
