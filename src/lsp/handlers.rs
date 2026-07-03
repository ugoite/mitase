// FEAT-LSP-001
// REQ-CORE-001

use regex::Regex;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::LazyLock;
use url::Url;

use crate::workspace::{Workspace, load_workspace};

use super::protocol::{
    Hover, InitializeParams, InitializeResult, LspError, MarkupContent, ServerCapabilities,
    TextDocumentPositionParams,
};

pub(crate) struct LspHandlers {
    workspace: Option<Workspace>,
    initialized: bool,
}

impl LspHandlers {
    pub(crate) fn new() -> Self {
        Self {
            workspace: None,
            initialized: false,
        }
    }

    pub(crate) fn handle_initialize(
        &mut self,
        params: InitializeParams,
    ) -> Result<Value, LspError> {
        let root_path = if let Some(root_uri) = params.root_uri {
            uri_to_path(&root_uri)?
        } else {
            std::env::current_dir().map_err(|error| LspError::internal(error.to_string()))?
        };

        self.workspace = Some(
            load_workspace(&root_path).map_err(|error| LspError::internal(error.to_string()))?,
        );

        let result = InitializeResult {
            capabilities: ServerCapabilities {
                hover_provider: Some(true),
            },
        };

        serde_json::to_value(result).map_err(|error| LspError::internal(error.to_string()))
    }

    pub(crate) fn handle_initialized(&mut self) -> Result<(), LspError> {
        self.initialized = true;
        Ok(())
    }

    pub(crate) fn handle_shutdown(&mut self) -> Result<Value, LspError> {
        self.initialized = false;
        Ok(Value::Null)
    }

    pub(crate) fn handle_hover(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<Hover>, LspError> {
        let workspace = match &self.workspace {
            Some(ws) => ws,
            None => return Err(LspError::internal("workspace not initialized")),
        };

        let file_path = uri_to_path(&params.text_document.uri)?;
        let line = params.position.line as usize;

        let content = std::fs::read_to_string(&file_path)
            .map_err(|error| LspError::internal(error.to_string()))?;
        let lines: Vec<&str> = content.lines().collect();

        if line >= lines.len() {
            return Ok(None);
        }

        let current_line = lines[line];
        let char_pos = params.position.character as usize;

        if let Some(spec_id) = find_spec_id_at_position(current_line, char_pos)
            && let Some(hover_content) = create_hover_for_spec_id(workspace, &spec_id)
        {
            return Ok(Some(hover_content));
        }

        Ok(None)
    }
}

fn uri_to_path(uri: &str) -> Result<PathBuf, LspError> {
    if uri.starts_with("file://") {
        let parsed = Url::parse(uri).map_err(|error| {
            LspError::invalid_params(format!("invalid file URI `{uri}`: {error}"))
        })?;
        parsed
            .to_file_path()
            .map_err(|()| LspError::invalid_params(format!("unsupported file URI path: {uri}")))
    } else {
        Ok(PathBuf::from(uri))
    }
}

fn find_spec_id_at_position(line: &str, char_pos: usize) -> Option<String> {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\b(PHIL-[A-Z0-9-]+|POL-[A-Z0-9-]+|REQ-[A-Z0-9-]+|FEAT-[A-Z0-9-]+)\b").unwrap()
    });

    for cap in RE.captures_iter(line) {
        let matched = cap.get(0)?;
        let start = matched.start();
        let end = matched.end();

        if char_pos >= start && char_pos <= end {
            return Some(matched.as_str().to_string());
        }
    }

    None
}

fn create_hover_for_spec_id(workspace: &Workspace, spec_id: &str) -> Option<Hover> {
    if spec_id.starts_with("PHIL-") {
        workspace
            .philosophies
            .iter()
            .find(|p| p.id == spec_id)
            .map(|phil| {
                let content = format!(
                    "# {}\n\n**{}**\n\n## Product Design Principle\n{}\n\n## Coding Guideline\n{}",
                    phil.id, phil.title, phil.product_design_principle, phil.coding_guideline
                );
                Hover {
                    contents: MarkupContent::markdown(content),
                    range: None,
                }
            })
    } else if spec_id.starts_with("POL-") {
        workspace
            .policies
            .iter()
            .find(|p| p.id == spec_id)
            .map(|pol| {
                let content = format!(
                    "# {}\n\n**{}**\n\n## Summary\n{}\n\n## Description\n{}",
                    pol.id, pol.title, pol.summary, pol.description
                );
                Hover {
                    contents: MarkupContent::markdown(content),
                    range: None,
                }
            })
    } else if spec_id.starts_with("REQ-") {
        workspace
            .requirements
            .iter()
            .find(|r| r.id == spec_id)
            .map(|req| {
                let content = format!(
                    "# {}\n\n**{}**\n\n{}\n\n**Priority:** {} | **Status:** {}",
                    req.id, req.title, req.description, req.priority, req.status
                );
                Hover {
                    contents: MarkupContent::markdown(content),
                    range: None,
                }
            })
    } else if spec_id.starts_with("FEAT-") {
        workspace
            .features
            .iter()
            .find(|f| f.id == spec_id)
            .map(|feat| {
                let content = format!(
                    "# {}\n\n**{}**\n\n{}\n\n**Status:** {}",
                    feat.id, feat.title, feat.summary, feat.status
                );
                Hover {
                    contents: MarkupContent::markdown(content),
                    range: None,
                }
            })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp::protocol::{Position, TextDocumentIdentifier};
    use std::{fs, path::PathBuf};
    use tempfile::tempdir;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/workspaces")
            .join(name)
    }

    #[test]
    fn test_find_spec_id_at_position() {
        let line = "// FEAT-AUTH-001 implements authentication";
        assert_eq!(
            find_spec_id_at_position(line, 5),
            Some("FEAT-AUTH-001".to_string())
        );
        assert_eq!(find_spec_id_at_position(line, 0), None);
    }

    #[test]
    fn test_uri_to_path() {
        let uri = "file:///home/user/file.txt";
        let path = uri_to_path(uri).unwrap();
        assert_eq!(path.to_str().unwrap(), "/home/user/file.txt");
    }

    #[test]
    fn test_uri_to_path_decodes_percent_encoding() {
        let uri = "file:///tmp/space%20name.txt";
        let path = uri_to_path(uri).unwrap();
        assert_eq!(path.to_str().unwrap(), "/tmp/space name.txt");
    }

    #[test]
    fn uri_to_path_accepts_plain_paths() {
        let path = uri_to_path("/tmp/plain.txt").unwrap();
        assert_eq!(path.to_str().unwrap(), "/tmp/plain.txt");
    }

    #[test]
    fn handle_hover_requires_initialization() {
        let handlers = LspHandlers::new();
        let error = handlers
            .handle_hover(TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: "file:///tmp/example.rs".to_string(),
                },
                position: Position {
                    line: 0,
                    character: 0,
                },
            })
            .expect_err("hover should require initialization");
        assert!(error.to_string().contains("workspace not initialized"));
    }

    #[test]
    fn handle_initialize_loads_workspace_from_root_uri() {
        let mut handlers = LspHandlers::new();
        let workspace = fixture_path("passing");
        let value = handlers
            .handle_initialize(InitializeParams {
                process_id: None,
                root_uri: Some(format!("file://{}", workspace.display())),
                capabilities: None,
            })
            .expect("initialize should succeed");

        assert_eq!(value["capabilities"]["hoverProvider"], true);
        assert!(handlers.workspace.is_some());
    }

    #[test]
    fn handle_hover_returns_none_for_out_of_bounds_positions() {
        let mut handlers = LspHandlers::new();
        let workspace = fixture_path("passing");
        handlers
            .handle_initialize(InitializeParams {
                process_id: None,
                root_uri: Some(format!("file://{}", workspace.display())),
                capabilities: None,
            })
            .expect("initialize should succeed");

        let tempdir = tempdir().expect("tempdir");
        let file_path = tempdir.path().join("notes.txt");
        fs::write(&file_path, "plain text\n").expect("write file");

        let hover = handlers
            .handle_hover(TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: format!("file://{}", file_path.display()),
                },
                position: Position {
                    line: 10,
                    character: 0,
                },
            })
            .expect("hover should succeed");
        assert!(hover.is_none());
    }

    #[test]
    fn handle_hover_renders_each_spec_layer() {
        let mut handlers = LspHandlers::new();
        let workspace = fixture_path("passing");
        handlers
            .handle_initialize(InitializeParams {
                process_id: None,
                root_uri: Some(format!("file://{}", workspace.display())),
                capabilities: None,
            })
            .expect("initialize should succeed");

        let tempdir = tempdir().expect("tempdir");
        let file_path = tempdir.path().join("spec-ids.txt");
        fs::write(
            &file_path,
            "PHIL-TRACE-001\nPOL-TRACE-001\nREQ-TRACE-001\nFEAT-TRACE-001\n",
        )
        .expect("write file");

        for (line, expected) in [
            (0, "Product Design Principle"),
            (1, "## Summary"),
            (2, "**Priority:**"),
            (3, "**Status:**"),
        ] {
            let hover = handlers
                .handle_hover(TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: format!("file://{}", file_path.display()),
                    },
                    position: Position { line, character: 2 },
                })
                .expect("hover should succeed")
                .expect("hover should exist");
            assert!(hover.contents.value.contains(expected));
        }
    }

    #[test]
    fn handle_hover_returns_none_when_no_spec_id_matches() {
        let mut handlers = LspHandlers::new();
        let workspace = fixture_path("passing");
        handlers
            .handle_initialize(InitializeParams {
                process_id: None,
                root_uri: Some(format!("file://{}", workspace.display())),
                capabilities: None,
            })
            .expect("initialize should succeed");

        let tempdir = tempdir().expect("tempdir");
        let file_path = tempdir.path().join("notes.txt");
        fs::write(&file_path, "nothing to hover here\n").expect("write file");

        let hover = handlers
            .handle_hover(TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: format!("file://{}", file_path.display()),
                },
                position: Position {
                    line: 0,
                    character: 1,
                },
            })
            .expect("hover should succeed");
        assert!(hover.is_none());
    }

    #[test]
    fn handle_initialized_and_shutdown_flip_state() {
        let mut handlers = LspHandlers::new();
        handlers.handle_initialized().expect("initialized");
        assert!(handlers.initialized);
        assert_eq!(handlers.handle_shutdown().expect("shutdown"), Value::Null);
        assert!(!handlers.initialized);
    }

    #[test]
    fn create_hover_returns_none_for_unknown_ids() {
        let workspace = load_workspace(&fixture_path("passing")).expect("workspace");
        assert!(create_hover_for_spec_id(&workspace, "NOTE-UNKNOWN-001").is_none());
    }
}
