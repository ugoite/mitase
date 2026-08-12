# Go example

This example is maintained against the active `mitase` v1 model.

It demonstrates one requirement and one feature connected through explicit bindings.

Current commands:

- `cargo run --quiet -- validate workspace examples/go-only`
- `cargo run --quiet -- workbench project --workspace examples/go-only`
- `(cd examples/go-only && go test -json -run '^TestGoRequirement$' ./go)`

Primary implementation targets:
- `go/app.go` (go)

Primary verification targets:
- `go/app_test.go` (go; exact claim `TestGoRequirement`)

The example is written for the active v1 CLI and spec model.
