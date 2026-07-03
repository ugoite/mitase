// FEAT-LSP-001
// REQ-CORE-001
//
// LSP Server for syu editor integrations
//
// This module provides a Language Server Protocol (LSP) compatible server
// that exposes syu's specification validation and navigation capabilities
// to editor clients (VS Code, Neovim, Emacs, etc.).
//
// **Current Scope:**
// - JSON-RPC 2.0 over stdio (standard LSP transport)
// - Basic LSP lifecycle: initialize, initialized, shutdown, exit
// - Hover support for spec IDs (PHIL-*, POL-*, REQ-*, FEAT-*)
//
// **Foundation for Future Features:**
// - Diagnostics for validation errors
// - Go-to-definition for spec IDs and trace references
// - Code completion for spec IDs
// - Document symbols for spec files
// - Workspace symbols for cross-file navigation
//
// This is a production-ready foundation, not a placeholder. The protocol
// implementation is stable and extensible for both VS Code and non-VS Code
// clients.

mod handlers;
mod protocol;
mod server;

use anyhow::Result;
use server::LspServer;

pub(crate) fn run_lsp_server() -> Result<()> {
    let mut server = LspServer::new();
    server.run()
}
