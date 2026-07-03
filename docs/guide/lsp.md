# syu LSP guide

<!-- FEAT-VSCODE-002 -->

Use this guide when you want to run `syu lsp` directly from an installed binary,
from `cargo run`, or from another editor client that speaks the Language Server
Protocol.

Today `syu lsp` is intentionally small and explicit:

- transport is **JSON-RPC 2.0 over stdio**
- the server supports the standard lifecycle requests and notifications
- the only editor feature exposed today is **hover for `PHIL-*`, `POL-*`,
  `REQ-*`, and `FEAT-*` IDs**

That narrow scope is deliberate. The server is already usable for editor
experiments, while the CLI and checked-in YAML remain the broader integration
surface for diagnostics and richer repository workflows.

## Start the server

From an installed binary:

```bash
syu lsp
```

From a source checkout:

```bash
cargo run -- lsp
```

The server reads LSP messages from stdin and writes responses to stdout. It does
not open a TCP port or a socket on its own.

## Transport and initialization

`syu lsp` follows the standard LSP framing:

- `Content-Length: <bytes>`
- blank line
- JSON payload

The expected startup flow is:

1. send `initialize`
2. send `initialized`
3. send requests such as `textDocument/hover`
4. send `shutdown`
5. send `exit`

If `initialize` includes `rootUri`, `syu` loads the workspace from that path.
If `rootUri` is omitted, it falls back to the current working directory of the
server process.

Today the server reads only `rootUri` from the initialize payload. It does not
yet consume `workspaceFolders`, `rootPath`, or other alternate workspace-root
fields, so clients should send `rootUri` explicitly when they do not want the
server process working directory to decide the workspace.

## Current capabilities

At the moment the server advertises:

- `hoverProvider: true`

The current request / notification surface is:

| Method | Support | Notes |
| --- | --- | --- |
| `initialize` | yes | loads the `syu` workspace from `rootUri` or the current directory |
| `initialized` | yes | marks the session ready for later requests |
| `textDocument/hover` | yes | returns Markdown hover content for spec IDs under the cursor |
| `shutdown` | yes | resets server state and returns `null` |
| `exit` | yes | terminates the process cleanly |

Hover content currently resolves only checked-in spec IDs:

- `PHIL-*`
- `POL-*`
- `REQ-*`
- `FEAT-*`

The server does not yet publish validation diagnostics, go-to-definition,
completion, document symbols, or workspace symbols.

## Minimal client example

Any client that can launch a stdio language server can start with a command like
this:

```json
{
  "command": ["syu", "lsp"]
}
```

If you are testing from a repository checkout instead of an installed binary,
the equivalent command is:

```json
{
  "command": ["cargo", "run", "--", "lsp"]
}
```

Use the repository root as the workspace folder so `rootUri` points at the
directory that contains `syu.yaml`. If your client only exposes
`workspaceFolders`, add an explicit `rootUri` override until the server learns
that alternate shape too.

## Relationship to the VS Code extension

The checked-in VS Code extension still uses the CLI and checked-in YAML directly
for its current diagnostics and navigation behavior. The LSP server is the
editor-agnostic foundation for future shared editor capabilities, not a full
replacement for the extension's current architecture yet.

For the current extension story and contributor workflow, read the
[VS Code extension guide](./vscode-extension.md).

## Troubleshooting

### `syu lsp` starts but the client cannot initialize

Check that the client is speaking stdio LSP with `Content-Length` framing. A raw
JSON stream without LSP headers will not work.

### The server says the workspace is not initialized

The client tried to call hover before completing `initialize`. Make sure the
normal LSP lifecycle runs in order and that `rootUri` points at a `syu`
workspace.

### Hover returns nothing

Hover only resolves spec IDs that are literally present under the cursor. It
does not yet provide symbol hover for arbitrary source-language identifiers.

### Initialization fails on one machine but not another

Verify that the process is starting from the intended repository and that the
workspace root actually contains `syu.yaml`. If you are using `cargo run -- lsp`
from a checkout, also confirm the repository builds locally before wiring it
into an editor client.

## Keep going

- Use the [VS Code extension guide](./vscode-extension.md) for the current
  extension architecture and commands.
- Use [getting started](./getting-started.md) when you still need the normal
  install / init / validate path before wiring editor integrations.
- Use [troubleshooting](./troubleshooting.md) for broader validation and
  workspace-repair problems outside the LSP transport itself.
