// FEAT-LSP-001
// REQ-CORE-001

use mitase_diagnostics::{Diagnostic as CanonicalDiagnostic, Location, Severity};
use mitase_spec_model::SpecDocument;
use mitase_workspace::SpecWorkspace;
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::Command,
};
use url::Url;

use super::protocol::{
    DiagnosticLocation, DiagnosticRelatedInformation, Hover, InitializeParams, InitializeResult,
    LspDiagnostic, LspError, MarkupContent, Notification, Position, PublishDiagnosticsParams,
    Range, ServerCapabilities, TextDocumentPositionParams,
};

pub(crate) struct LspHandlers {
    workspace: Option<SpecWorkspace>,
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
            SpecWorkspace::load(&root_path)
                .map_err(|error| LspError::internal(error.to_string()))?,
        );

        let result = InitializeResult {
            capabilities: ServerCapabilities {
                hover_provider: Some(true),
            },
        };

        serde_json::to_value(result).map_err(|error| LspError::internal(error.to_string()))
    }

    pub(crate) fn handle_initialized(&mut self) -> Result<Vec<Notification>, LspError> {
        self.initialized = true;
        match &self.workspace {
            Some(_) => self.publish_diagnostics(),
            None => Ok(Vec::new()),
        }
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

    fn publish_diagnostics(&self) -> Result<Vec<Notification>, LspError> {
        let workspace = self
            .workspace
            .as_ref()
            .ok_or_else(|| LspError::internal("workspace not initialized"))?;
        let index = workspace
            .index()
            .map_err(|error| LspError::internal(error.to_string()))?;
        let revision = current_revision(&workspace.root);
        let context = mitase_validation::ValidationContext {
            config: &workspace.config,
            workspace,
            index: &index,
            changed_files: None,
            reported_changed_files: None,
            preset: workspace.config.validation.preset,
            revision: revision.as_deref(),
            change_base_revision: None,
        };
        let result = mitase_validation::validate_workspace(&context);

        let mut diagnostics_by_uri = BTreeMap::<String, Vec<LspDiagnostic>>::new();
        let mut known_uris = BTreeSet::new();
        known_uris.insert(path_to_uri(&workspace.root.join("mitase.yaml"))?);
        for document in &workspace.documents {
            known_uris.insert(path_to_uri(&document.path)?);
        }

        for diagnostic in &result.diagnostics {
            let path = diagnostic_path(&workspace.root, &diagnostic.primary.path);
            let uri = path_to_uri(&path)?;
            known_uris.insert(uri.clone());
            diagnostics_by_uri
                .entry(uri)
                .or_default()
                .push(to_lsp_diagnostic(&workspace.root, diagnostic)?);
        }
        for diagnostics in diagnostics_by_uri.values_mut() {
            diagnostics.sort_by(|left, right| {
                (
                    &left.code,
                    &left.message,
                    left.range.start.line,
                    left.range.start.character,
                    left.range.end.line,
                    left.range.end.character,
                )
                    .cmp(&(
                        &right.code,
                        &right.message,
                        right.range.start.line,
                        right.range.start.character,
                        right.range.end.line,
                        right.range.end.character,
                    ))
            });
        }

        known_uris
            .into_iter()
            .map(|uri| {
                let diagnostics = diagnostics_by_uri.remove(&uri).unwrap_or_default();
                let params = PublishDiagnosticsParams { uri, diagnostics };
                Ok(Notification {
                    jsonrpc: "2.0".to_string(),
                    method: "textDocument/publishDiagnostics".to_string(),
                    params: Some(
                        serde_json::to_value(params)
                            .map_err(|error| LspError::internal(error.to_string()))?,
                    ),
                })
            })
            .collect()
    }
}

fn diagnostic_path(root: &Path, path: &str) -> PathBuf {
    if path.is_empty() || path == "workspace" {
        return root.join("mitase.yaml");
    }
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn path_to_uri(path: &Path) -> Result<String, LspError> {
    Url::from_file_path(path)
        .map(|uri| uri.to_string())
        .map_err(|()| {
            LspError::internal(format!(
                "cannot convert path to file URI: {}",
                path.display()
            ))
        })
}

fn current_revision(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let revision = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (!revision.is_empty()).then_some(revision)
}

fn to_lsp_diagnostic(
    root: &Path,
    diagnostic: &CanonicalDiagnostic,
) -> Result<LspDiagnostic, LspError> {
    let related_information = if diagnostic.related.is_empty() {
        None
    } else {
        Some(
            diagnostic
                .related
                .iter()
                .map(|related| {
                    Ok(DiagnosticRelatedInformation {
                        location: DiagnosticLocation {
                            uri: path_to_uri(&diagnostic_path(root, &related.location.path))?,
                            range: lsp_range(&related.location),
                        },
                        message: related.message.clone(),
                    })
                })
                .collect::<Result<Vec<_>, LspError>>()?,
        )
    };

    Ok(LspDiagnostic {
        range: lsp_range(&diagnostic.primary),
        severity: Some(match diagnostic.severity {
            Severity::Error => 1,
            Severity::Warning => 2,
            Severity::Info => 3,
        }),
        code: Some(diagnostic.rule_id.clone()),
        source: Some("mitase".to_string()),
        message: diagnostic.message.clone(),
        related_information,
        data: Some(
            serde_json::to_value(diagnostic)
                .map_err(|error| LspError::internal(error.to_string()))?,
        ),
    })
}

fn lsp_range(location: &Location) -> Range {
    Range {
        start: lsp_position(location.line, location.column),
        end: lsp_position(
            location.end_line.or(location.line),
            location.end_column.or(location.column),
        ),
    }
}

fn lsp_position(line: Option<u32>, character: Option<u32>) -> Position {
    Position {
        line: line.map_or(0, |line| line.saturating_sub(1)),
        character: character.map_or(0, |character| character.saturating_sub(1)),
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

fn create_hover_for_spec_id(workspace: &SpecWorkspace, spec_id: &str) -> Option<Hover> {
    for loaded in &workspace.documents {
        match &loaded.document {
            SpecDocument::Philosophies { philosophies, .. } => {
                if let Some(philosophy) = philosophies.iter().find(|item| item.id.0 == spec_id) {
                    return Some(Hover {
                        contents: MarkupContent::markdown(format!(
                            "# {}\n\n**{}**\n\n{}\n\n**Principles:** {}",
                            philosophy.id,
                            philosophy.title,
                            philosophy.summary,
                            philosophy.principles.len()
                        )),
                        range: None,
                    });
                }
            }
            SpecDocument::Policies { policies, .. } => {
                if let Some(policy) = policies.iter().find(|item| item.id.0 == spec_id) {
                    return Some(Hover {
                        contents: MarkupContent::markdown(format!(
                            "# {}\n\n**{}**\n\n## Summary\n{}\n\n## Description\n{}",
                            policy.id, policy.title, policy.summary, policy.description
                        )),
                        range: None,
                    });
                }
            }
            SpecDocument::Requirements { requirements, .. } => {
                if let Some(requirement) = requirements.iter().find(|item| item.id.0 == spec_id) {
                    return Some(Hover {
                        contents: MarkupContent::markdown(format!(
                            "# {}\n\n**{}**\n\n{}\n\n**Priority:** {:?} | **Status:** {:?}",
                            requirement.id,
                            requirement.title,
                            requirement.description,
                            requirement.priority,
                            requirement.status
                        )),
                        range: None,
                    });
                }
            }
            SpecDocument::Features { features, .. } => {
                if let Some(feature) = features.iter().find(|item| item.id.0 == spec_id) {
                    return Some(Hover {
                        contents: MarkupContent::markdown(format!(
                            "# {}\n\n**{}**\n\n{}\n\n**Status:** {:?}",
                            feature.id, feature.title, feature.summary, feature.status
                        )),
                        range: None,
                    });
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp::protocol::{Position, TextDocumentIdentifier};
    use std::{fs, path::PathBuf};
    use tempfile::tempdir;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/v1")
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
        let workspace = fixture_path("valid-web-app");
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
        let workspace = fixture_path("valid-web-app");
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
        let workspace = SpecWorkspace::load(fixture_path("valid-web-app")).expect("workspace");

        for (spec_id, expected) in [
            ("PHIL-AUTH-001", "**Safe authentication**"),
            ("POL-AUTH-001", "## Summary"),
            ("REQ-AUTH-001", "**Priority:**"),
            ("FEAT-AUTH-001", "**Status:**"),
        ] {
            let hover = create_hover_for_spec_id(&workspace, spec_id).expect("hover should exist");
            assert!(hover.contents.value.contains(expected));
        }
    }

    #[test]
    fn handle_hover_returns_none_when_no_spec_id_matches() {
        let mut handlers = LspHandlers::new();
        let workspace = fixture_path("valid-web-app");
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
        let workspace = SpecWorkspace::load(fixture_path("valid-web-app")).expect("workspace");
        assert!(create_hover_for_spec_id(&workspace, "NOTE-UNKNOWN-001").is_none());
    }

    #[test]
    fn canonical_diagnostics_map_to_lsp_fields_and_zero_based_ranges() {
        let mut diagnostic = CanonicalDiagnostic::error(
            "MITASE-TARGET-002",
            "target resolution is ambiguous",
            "spec/feature.yaml",
        );
        diagnostic.primary.line = Some(12);
        diagnostic.primary.column = Some(5);
        diagnostic.primary.end_line = Some(12);
        diagnostic.primary.end_column = Some(19);

        let mapped =
            to_lsp_diagnostic(Path::new("/workspace"), &diagnostic).expect("diagnostic should map");

        assert_eq!(mapped.range.start.line, 11);
        assert_eq!(mapped.range.start.character, 4);
        assert_eq!(mapped.range.end.line, 11);
        assert_eq!(mapped.range.end.character, 18);
        assert_eq!(mapped.code.as_deref(), Some("MITASE-TARGET-002"));
        assert_eq!(mapped.source.as_deref(), Some("mitase"));
        assert_eq!(mapped.severity, Some(1));
        assert_eq!(mapped.message, "target resolution is ambiguous");
        assert_eq!(
            mapped.data.as_ref().expect("canonical data")["code"],
            "MITASE-TARGET-002"
        );
    }

    #[test]
    fn handle_initialized_publishes_deterministic_notifications_for_known_documents() {
        let mut handlers = LspHandlers::new();
        let workspace = fixture_path("valid-web-app");
        handlers
            .handle_initialize(InitializeParams {
                process_id: None,
                root_uri: Some(format!("file://{}", workspace.display())),
                capabilities: None,
            })
            .expect("initialize should succeed");

        let first = handlers
            .handle_initialized()
            .expect("diagnostics should publish");
        let second = handlers
            .handle_initialized()
            .expect("diagnostics should publish again");

        assert!(!first.is_empty());
        assert_eq!(
            serde_json::to_value(&first).expect("serialize notifications"),
            serde_json::to_value(&second).expect("serialize notifications")
        );
        assert!(first.iter().all(|notification| {
            notification.method == "textDocument/publishDiagnostics"
                && notification.params.as_ref().is_some_and(|params| {
                    params["uri"].as_str().is_some() && params["diagnostics"].is_array()
                })
        }));
        assert!(first.iter().any(|notification| {
            notification.params.as_ref().is_some_and(|params| {
                params["diagnostics"].as_array().is_some_and(|diagnostics| {
                    diagnostics.iter().any(|diagnostic| {
                        diagnostic["source"] == "mitase"
                            && diagnostic["code"].is_string()
                            && diagnostic["data"]["reason"].is_string()
                    })
                })
            })
        }));
        assert!(first.iter().any(|notification| {
            notification
                .params
                .as_ref()
                .is_some_and(|params| params["diagnostics"].as_array().is_some_and(Vec::is_empty))
        }));
    }
}
