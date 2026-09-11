// FEAT-LSP-001
// REQ-CORE-001

use mitase_diagnostics::{Diagnostic as CanonicalDiagnostic, Location, Severity};
use mitase_spec_model::{BoundTargetRef, LocalAnchorKind, SpecAnchor, SpecDocument, SpecId};
use mitase_workspace::{FrontendDiagnosticError, SpecIndex, SpecWorkspace};
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
    DiagnosticLocation, DiagnosticRelatedInformation, DidChangeTextDocumentParams,
    DidChangeWatchedFilesParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, Hover, InitializeParams, InitializeResult, Location as LspLocation,
    LspDiagnostic, LspError, MarkupContent, Notification, Position, PublishDiagnosticsParams,
    Range, ServerCapabilities, TextDocumentPositionParams, TextDocumentSaveOptions,
    TextDocumentSyncOptions, WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
};

pub(crate) struct LspHandlers {
    workspace: Option<SpecWorkspace>,
    workspace_root: Option<PathBuf>,
    workspace_folders: Vec<PathBuf>,
    open_documents: BTreeMap<PathBuf, String>,
    published_uris: BTreeSet<String>,
    startup_diagnostics: Vec<CanonicalDiagnostic>,
    initialized: bool,
}

impl LspHandlers {
    pub(crate) fn new() -> Self {
        Self {
            workspace: None,
            workspace_root: None,
            workspace_folders: Vec::new(),
            open_documents: BTreeMap::new(),
            published_uris: BTreeSet::new(),
            startup_diagnostics: Vec::new(),
            initialized: false,
        }
    }

    pub(crate) fn handle_initialize(
        &mut self,
        params: InitializeParams,
    ) -> Result<Value, LspError> {
        let workspace_folders = params
            .workspace_folders
            .unwrap_or_default()
            .into_iter()
            .map(|folder| uri_to_path(&folder.uri))
            .collect::<Result<Vec<_>, _>>()?;
        let root_path = if let Some(root_uri) = params.root_uri {
            uri_to_path(&root_uri)?
        } else if let Some(folder) = workspace_folders.first() {
            folder.clone()
        } else {
            std::env::current_dir().map_err(|error| LspError::internal(error.to_string()))?
        };

        self.workspace_root = Some(root_path.clone());
        self.workspace_folders = if workspace_folders.is_empty() {
            vec![root_path.clone()]
        } else {
            workspace_folders
        };
        self.open_documents.clear();
        self.published_uris.clear();
        self.reload_workspace()?;

        let result = InitializeResult {
            capabilities: ServerCapabilities {
                hover_provider: Some(true),
                definition_provider: Some(true),
                text_document_sync: Some(TextDocumentSyncOptions {
                    open_close: true,
                    change: 1,
                    save: Some(TextDocumentSaveOptions {
                        include_text: false,
                    }),
                }),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: WorkspaceFoldersServerCapabilities {
                        supported: true,
                        change_notifications: false,
                    },
                }),
            },
        };

        serde_json::to_value(result).map_err(|error| LspError::internal(error.to_string()))
    }

    pub(crate) fn handle_initialized(&mut self) -> Result<Vec<Notification>, LspError> {
        self.initialized = true;
        if self.workspace.is_some() || !self.startup_diagnostics.is_empty() {
            self.publish_diagnostics()
        } else {
            Ok(Vec::new())
        }
    }

    pub(crate) fn handle_shutdown(&mut self) -> Result<Value, LspError> {
        self.initialized = false;
        Ok(Value::Null)
    }

    pub(crate) fn handle_did_open(
        &mut self,
        params: DidOpenTextDocumentParams,
    ) -> Result<Vec<Notification>, LspError> {
        let path = uri_to_path(&params.text_document.uri)?;
        self.open_documents.insert(path, params.text_document.text);
        self.refresh_diagnostics()
    }

    pub(crate) fn handle_did_change(
        &mut self,
        params: DidChangeTextDocumentParams,
    ) -> Result<Vec<Notification>, LspError> {
        let change = params
            .content_changes
            .last()
            .ok_or_else(|| LspError::invalid_params("contentChanges must not be empty"))?;
        if change.range.is_some() {
            return Err(LspError::invalid_params(
                "mitase LSP uses full document synchronization",
            ));
        }
        let path = uri_to_path(&params.text_document.uri)?;
        self.open_documents.insert(path, change.text.clone());
        self.refresh_diagnostics()
    }

    pub(crate) fn handle_did_save(
        &mut self,
        params: DidSaveTextDocumentParams,
    ) -> Result<Vec<Notification>, LspError> {
        let path = uri_to_path(&params.text_document.uri)?;
        if let Some(text) = params.text {
            self.open_documents.insert(path, text);
        }
        self.reload_workspace()?;
        self.refresh_diagnostics()
    }

    pub(crate) fn handle_did_close(
        &mut self,
        params: DidCloseTextDocumentParams,
    ) -> Result<Vec<Notification>, LspError> {
        let path = uri_to_path(&params.text_document.uri)?;
        self.open_documents.remove(&path);
        self.refresh_diagnostics()
    }

    pub(crate) fn handle_did_change_watched_files(
        &mut self,
        params: DidChangeWatchedFilesParams,
    ) -> Result<Vec<Notification>, LspError> {
        let affects_workspace = params.changes.iter().any(|event| {
            uri_to_path(&event.uri).ok().is_some_and(|path| {
                self.workspace_folders
                    .iter()
                    .any(|root| path.starts_with(root))
            })
        });
        if affects_workspace {
            self.reload_workspace()?;
        }
        self.refresh_diagnostics()
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

        let content = self.document_text(&file_path)?;
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

    pub(crate) fn handle_definition(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<LspLocation>, LspError> {
        let workspace = match &self.workspace {
            Some(ws) => ws,
            None => return Err(LspError::internal("workspace not initialized")),
        };

        let file_path = uri_to_path(&params.text_document.uri)?;
        let content = self.document_text(&file_path)?;
        let line = params.position.line as usize;
        let lines: Vec<&str> = content.lines().collect();
        let Some(current_line) = lines.get(line) else {
            return Ok(None);
        };
        let Some(reference) =
            find_reference_at_position(current_line, params.position.character as usize)
        else {
            return Ok(None);
        };

        let index = workspace
            .index()
            .map_err(|error| LspError::internal(error.to_string()))?;
        let Some((item, declaration)) = resolve_definition_reference(&index, &reference) else {
            return Ok(None);
        };
        let Some(path) = index.item_paths.get(&item) else {
            return Ok(None);
        };
        let source = self.document_text(path)?;
        let Some(range) = find_declaration_range(&source, &item, declaration) else {
            return Ok(None);
        };

        Ok(Some(LspLocation {
            uri: path_to_uri(path)?,
            range,
        }))
    }

    fn document_text(&self, path: &Path) -> Result<String, LspError> {
        if let Some(text) = self.open_documents.get(path) {
            return Ok(text.clone());
        }
        std::fs::read_to_string(path).map_err(|error| LspError::internal(error.to_string()))
    }

    fn reload_workspace(&mut self) -> Result<(), LspError> {
        let root = self
            .workspace_root
            .clone()
            .ok_or_else(|| LspError::internal("workspace not initialized"))?;
        match SpecWorkspace::load(&root) {
            Ok(workspace) => {
                self.workspace = Some(workspace);
                self.startup_diagnostics.clear();
            }
            Err(error) => {
                self.workspace = None;
                if let Some(frontend) = error.downcast_ref::<FrontendDiagnosticError>() {
                    self.startup_diagnostics =
                        vec![crate::canonical_frontend_diagnostic(&frontend.diagnostic)];
                } else {
                    self.startup_diagnostics = vec![CanonicalDiagnostic::error(
                        "MITASE-LSP-001",
                        format!("workspace reload failed: {error}"),
                        "workspace",
                    )];
                }
            }
        }
        Ok(())
    }

    fn refresh_diagnostics(&mut self) -> Result<Vec<Notification>, LspError> {
        if self.initialized {
            self.publish_diagnostics()
        } else {
            Ok(Vec::new())
        }
    }

    fn publish_diagnostics(&mut self) -> Result<Vec<Notification>, LspError> {
        let (root, document_paths, diagnostics) = if let Some(workspace) = &self.workspace {
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
            let mut result = mitase_validation::validate_workspace(&context);
            result.diagnostics.splice(
                0..0,
                workspace
                    .frontend_diagnostics
                    .iter()
                    .map(crate::canonical_frontend_diagnostic),
            );
            (
                workspace.root.clone(),
                workspace
                    .documents
                    .iter()
                    .map(|document| document.path.clone())
                    .collect::<Vec<_>>(),
                result.diagnostics,
            )
        } else {
            (
                self.workspace_root
                    .clone()
                    .ok_or_else(|| LspError::internal("workspace not initialized"))?,
                Vec::new(),
                self.startup_diagnostics.clone(),
            )
        };

        let mut diagnostics_by_uri = BTreeMap::<String, Vec<LspDiagnostic>>::new();
        let mut current_uris = BTreeSet::new();
        current_uris.insert(path_to_uri(&root.join("mitase.yaml"))?);
        for document_path in document_paths {
            current_uris.insert(path_to_uri(&document_path)?);
        }

        for diagnostic in &diagnostics {
            let path = diagnostic_path(&root, &diagnostic.primary.path);
            let uri = path_to_uri(&path)?;
            current_uris.insert(uri.clone());
            diagnostics_by_uri
                .entry(uri)
                .or_default()
                .push(to_lsp_diagnostic(&root, diagnostic)?);
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

        let mut published_uris = current_uris.clone();
        published_uris.extend(self.published_uris.iter().cloned());
        let notifications = published_uris
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
            .collect::<Result<Vec<_>, LspError>>()?;
        self.published_uris = current_uris;
        Ok(notifications)
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

#[derive(Debug, Clone)]
enum DeclarationReference {
    Item,
    Anchor(SpecAnchor),
    Target(BoundTargetRef),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceSpan {
    line: usize,
    start: usize,
    end: usize,
}

fn find_reference_at_position(line: &str, char_pos: usize) -> Option<String> {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"[A-Za-z][A-Za-z0-9-]*(?:#[a-z]+\.[a-z0-9-]+(?:/target\.[a-z0-9-]+)?)?")
            .expect("reference regex")
    });

    RE.find_iter(line).find_map(|matched| {
        let start = line[..matched.start()].encode_utf16().count();
        let end = line[..matched.end()].encode_utf16().count();
        (char_pos >= start && char_pos < end).then(|| matched.as_str().to_string())
    })
}

fn resolve_definition_reference(
    index: &SpecIndex,
    candidate: &str,
) -> Option<(SpecId, DeclarationReference)> {
    if let Ok(target) = candidate.parse::<BoundTargetRef>() {
        if index.target(&target).is_some() {
            let item = target.binding.item.clone();
            return Some((item, DeclarationReference::Target(target)));
        }
        return None;
    }
    if let Ok(anchor) = candidate.parse::<SpecAnchor>() {
        if index.anchor(&anchor).is_some() {
            let item = anchor.item.clone();
            return Some((item, DeclarationReference::Anchor(anchor)));
        }
        return None;
    }

    let item = SpecId(candidate.to_string());
    index
        .item_paths
        .contains_key(&item)
        .then_some((item, DeclarationReference::Item))
}

fn find_declaration_range(
    source: &str,
    item: &SpecId,
    declaration: DeclarationReference,
) -> Option<Range> {
    let lines: Vec<&str> = source.lines().collect();
    let span = match declaration {
        DeclarationReference::Item => find_item_declaration_span(&lines, item),
        DeclarationReference::Anchor(anchor) => find_anchor_declaration_span(&lines, item, &anchor),
        DeclarationReference::Target(target) => find_target_declaration_span(&lines, item, &target),
    }?;
    Some(source_span_to_range(lines[span.line], span))
}

fn find_item_declaration_span(lines: &[&str], item: &SpecId) -> Option<SourceSpan> {
    lines.iter().enumerate().find_map(|(line, text)| {
        find_id_value_span(text, &item.0).map(|(start, end)| SourceSpan { line, start, end })
    })
}

fn find_anchor_declaration_span(
    lines: &[&str],
    item: &SpecId,
    anchor: &SpecAnchor,
) -> Option<SourceSpan> {
    let item_span = find_item_declaration_span(lines, item)?;
    let item_end = item_scope_end(lines, item_span.line);
    if anchor.kind == LocalAnchorKind::Binding {
        return find_binding_declaration_span(lines, item_span.line, item_end, anchor);
    }
    let section_names: &[&str] = match anchor.kind {
        LocalAnchorKind::Principle => &["principles", "principle"],
        LocalAnchorKind::Rule => &["rules", "rule"],
        LocalAnchorKind::Criterion => &["criteria", "criterion"],
        LocalAnchorKind::Binding => unreachable!(),
        LocalAnchorKind::Contract => &["contracts", "contract"],
    };
    let section = find_section(lines, item_span.line + 1, item_end, section_names)?;
    let section_end = scope_end(lines, section.line, line_indent(lines[section.line]));
    if let Some((start, end)) = find_id_value_span(lines[section.line], &anchor.local_id.0) {
        return Some(SourceSpan {
            line: section.line,
            start,
            end,
        });
    }
    find_id_value_in_range(
        lines,
        section.line + 1,
        section_end.min(item_end),
        &anchor.local_id.0,
    )
    .or(Some(section))
}

fn find_binding_declaration_span(
    lines: &[&str],
    item_line: usize,
    item_end: usize,
    anchor: &SpecAnchor,
) -> Option<SourceSpan> {
    if let Some(section) = find_section(lines, item_line + 1, item_end, &["bindings"]) {
        let section_end = scope_end(lines, section.line, line_indent(lines[section.line]));
        if let Some(span) = find_id_value_in_range(
            lines,
            section.line + 1,
            section_end.min(item_end),
            &anchor.local_id.0,
        ) {
            return Some(span);
        }
    }

    for section_name in ["implementation", "verification", "binding"] {
        let Some(section) = find_section(lines, item_line + 1, item_end, &[section_name]) else {
            continue;
        };
        let section_end = scope_end(lines, section.line, line_indent(lines[section.line]));
        if let Some((start, end)) = find_id_value_span(lines[section.line], &anchor.local_id.0) {
            return Some(SourceSpan {
                line: section.line,
                start,
                end,
            });
        }
        if let Some(span) = find_id_value_in_range(
            lines,
            section.line + 1,
            section_end.min(item_end),
            &anchor.local_id.0,
        ) {
            return Some(span);
        }
        if anchor.local_id.0 == section_name {
            return Some(section);
        }
    }

    None
}

fn find_target_declaration_span(
    lines: &[&str],
    item: &SpecId,
    target: &BoundTargetRef,
) -> Option<SourceSpan> {
    let binding_span = find_anchor_declaration_span(lines, item, &target.binding)?;
    let binding_start =
        short_binding_section_start(lines, binding_span.line).unwrap_or(binding_span.line);
    let binding_end = scope_end(lines, binding_start, line_indent(lines[binding_start]));
    let section = find_section(
        lines,
        binding_span.line + 1,
        binding_end,
        &["targets", "target"],
    )?;
    let section_end = scope_end(lines, section.line, line_indent(lines[section.line]));
    if let Some((start, end)) = find_id_value_span(lines[section.line], &target.target_id.0) {
        return Some(SourceSpan {
            line: section.line,
            start,
            end,
        });
    }
    find_id_value_in_range(
        lines,
        section.line + 1,
        section_end.min(binding_end),
        &target.target_id.0,
    )
    .or(Some(section))
}

fn short_binding_section_start(lines: &[&str], binding_line: usize) -> Option<usize> {
    let binding_indent = line_indent(lines[binding_line]);
    lines
        .iter()
        .enumerate()
        .take(binding_line)
        .rev()
        .find_map(|(line, text)| {
            (line_indent(text) < binding_indent
                && (yaml_container_key_span(text, "implementation").is_some()
                    || yaml_container_key_span(text, "verification").is_some()))
            .then_some(line)
        })
}

fn find_section(lines: &[&str], start: usize, end: usize, names: &[&str]) -> Option<SourceSpan> {
    lines
        .iter()
        .enumerate()
        .skip(start)
        .take(end.saturating_sub(start))
        .find_map(|(line, text)| {
            names.iter().find_map(|name| {
                yaml_container_key_span(text, name).map(|(start, end)| SourceSpan {
                    line,
                    start,
                    end,
                })
            })
        })
}

fn find_id_value_in_range(
    lines: &[&str],
    start: usize,
    end: usize,
    value: &str,
) -> Option<SourceSpan> {
    lines
        .iter()
        .enumerate()
        .skip(start)
        .take(end.saturating_sub(start))
        .find_map(|(line, text)| {
            find_id_value_span(text, value).map(|(start, end)| SourceSpan { line, start, end })
        })
}

fn scope_end(lines: &[&str], start: usize, base_indent: usize) -> usize {
    lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find_map(|(line, text)| {
            (!text.trim().is_empty() && line_indent(text) <= base_indent).then_some(line)
        })
        .unwrap_or(lines.len())
}

fn item_scope_end(lines: &[&str], start: usize) -> usize {
    let base_indent = line_indent(lines[start]);
    if lines[start].trim_start().starts_with("- ") {
        return scope_end(lines, start, base_indent);
    }
    lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find_map(|(line, text)| {
            (!text.trim().is_empty() && line_indent(text) < base_indent).then_some(line)
        })
        .unwrap_or(lines.len())
}

fn line_indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn yaml_key_span(line: &str, key: &str) -> Option<(usize, usize)> {
    let leading = line.len() - line.trim_start_matches(' ').len();
    let mut start = leading;
    if line[start..].starts_with("- ") {
        start += 2;
    }
    let rest = &line[start..];
    rest.strip_prefix(key)
        .filter(|suffix| suffix.starts_with(':'))
        .map(|_| (start, start + key.len()))
}

fn yaml_container_key_span(line: &str, key: &str) -> Option<(usize, usize)> {
    let span = yaml_key_span(line, key)?;
    let value = line[span.1 + 1..].trim_start();
    (value.is_empty() || value.starts_with('#') || value.starts_with('{') || value.starts_with('['))
        .then_some(span)
}

fn find_id_value_span(line: &str, value: &str) -> Option<(usize, usize)> {
    let key_start = line.match_indices("id:").find_map(|(start, _)| {
        if yaml_comment_before(line, start) {
            return None;
        }
        let previous = line[..start].chars().next_back();
        previous
            .is_none_or(|character| !character.is_ascii_alphanumeric() && character != '_')
            .then_some(start)
    })?;
    let key_end = key_start + 2;
    let value_start = line[key_end + 1..]
        .char_indices()
        .find_map(|(offset, character)| {
            (!character.is_whitespace()).then_some(key_end + 1 + offset)
        })?;
    if let Some(quote @ ('\'' | '"')) = line[value_start..].chars().next() {
        let content_start = value_start + quote.len_utf8();
        let suffix = &line[content_start..];
        if !suffix.starts_with(value) {
            return None;
        }
        let value_end = content_start + value.len();
        if line.as_bytes()[value_end..].first().copied() != Some(quote as u8) {
            return None;
        }
        let boundary = line[value_end + quote.len_utf8()..].chars().next();
        if boundary.is_some_and(|character| {
            !character.is_whitespace() && !matches!(character, ',' | ']' | '}' | '#')
        }) {
            return None;
        }
        return Some((content_start, value_end));
    }

    let suffix = &line[value_start..];
    if !suffix.starts_with(value) {
        return None;
    }
    let boundary = suffix[value.len()..].chars().next();
    if boundary.is_some_and(|character| {
        !character.is_whitespace() && !matches!(character, ',' | ']' | '}' | '#')
    }) {
        return None;
    }
    Some((value_start, value_start + value.len()))
}

fn yaml_comment_before(line: &str, end: usize) -> bool {
    let mut quote = None;
    for (offset, character) in line.char_indices() {
        if offset >= end {
            break;
        }
        match (quote, character) {
            (None, '\'' | '"') => quote = Some(character),
            (Some(current), character) if character == current => quote = None,
            (None, '#')
                if offset == 0
                    || line[..offset]
                        .chars()
                        .next_back()
                        .is_some_and(char::is_whitespace) =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}

fn source_span_to_range(line: &str, span: SourceSpan) -> Range {
    Range {
        start: Position {
            line: span.line as u32,
            character: line[..span.start].encode_utf16().count() as u32,
        },
        end: Position {
            line: span.line as u32,
            character: line[..span.end].encode_utf16().count() as u32,
        },
    }
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
    use crate::lsp::protocol::{
        DidCloseTextDocumentParams, Position, TextDocumentIdentifier, WorkspaceFolder,
    };
    use mitase_diagnostics::DiagnosticSubject;
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

    fn definition_location(
        workspace_root: &Path,
        source_path: &Path,
        reference: &str,
    ) -> Option<LspLocation> {
        let mut handlers = LspHandlers::new();
        handlers
            .handle_initialize(InitializeParams {
                process_id: None,
                root_uri: Some(format!("file://{}", workspace_root.display())),
                workspace_folders: None,
                capabilities: None,
            })
            .expect("initialize should succeed");
        let source = fs::read_to_string(source_path).expect("source should exist");
        let (line, text) = source
            .lines()
            .enumerate()
            .find(|(_, text)| text.contains(reference))
            .expect("reference should exist in source");
        let character = text.find(reference).expect("reference offset") as u32;

        handlers
            .handle_definition(TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: path_to_uri(source_path).expect("source URI"),
                },
                position: Position {
                    line: line as u32,
                    character,
                },
            })
            .expect("definition should resolve")
    }

    #[test]
    fn handle_definition_resolves_ids_anchors_and_targets_from_v1_source() {
        let workspace = fixture_path("valid-web-app");
        let requirement = workspace.join("spec/requirement.yaml");
        let feature = workspace.join("spec/feature.yaml");

        let item =
            definition_location(&workspace, &requirement, "REQ-AUTH-001").expect("item definition");
        assert_eq!(
            item.uri,
            path_to_uri(&requirement).expect("requirement URI")
        );
        assert_eq!(item.range.start.line, 5);
        assert_eq!(item.range.start.character, 8);

        let anchor = definition_location(
            &workspace,
            &requirement,
            "REQ-AUTH-001#criterion.invalid-credentials",
        )
        .expect("anchor definition");
        assert_eq!(
            anchor.uri,
            path_to_uri(&requirement).expect("requirement URI")
        );
        assert_eq!(anchor.range.start.line, 11);
        assert_eq!(anchor.range.start.character, 12);

        let target = definition_location(
            &workspace,
            &requirement,
            "FEAT-AUTH-001#binding.backend/target.handler",
        )
        .expect("target definition");
        assert_eq!(target.uri, path_to_uri(&feature).expect("feature URI"));
        assert_eq!(target.range.start.line, 25);
        assert_eq!(target.range.start.character, 16);

        let inline_target = definition_location(
            &workspace,
            &feature,
            "FEAT-AUTH-001#binding.schema/target.operation",
        )
        .expect("inline target definition");
        assert_eq!(
            inline_target.uri,
            path_to_uri(&feature).expect("feature URI")
        );
        assert_eq!(inline_target.range.start.line, 40);
    }

    #[test]
    fn handle_definition_resolves_references_from_authoring_v2_source() {
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let requirements = workspace.join("docs/mitase/requirements/capability-contracts.yaml");
        let features = workspace.join("docs/mitase/features/capabilities/surfaces.yaml");

        let anchor = definition_location(
            &workspace,
            &requirements,
            "REQ-CAPABILITY-001#criterion.lsp-navigation",
        )
        .expect("v2 anchor definition");
        assert_eq!(
            anchor.uri,
            path_to_uri(&requirements).expect("requirements URI")
        );
        assert_eq!(anchor.range.start.line, 55);

        let target = definition_location(
            &workspace,
            &requirements,
            "FEAT-LSP-001#binding.implementation/target.lsp-server",
        )
        .expect("v2 target definition");
        assert_eq!(target.uri, path_to_uri(&features).expect("features URI"));
        assert_eq!(target.range.start.line, 74);
    }

    #[test]
    fn handle_definition_does_not_guess_unknown_references() {
        let workspace = fixture_path("valid-web-app");
        let tempdir = tempdir().expect("tempdir");
        let reference_path = tempdir.path().join("reference.yaml");
        fs::write(
            &reference_path,
            "criterion: REQ-AUTH-001#criterion.not-authored\n",
        )
        .expect("reference source");

        let mut handlers = LspHandlers::new();
        handlers
            .handle_initialize(InitializeParams {
                process_id: None,
                root_uri: Some(format!("file://{}", workspace.display())),
                workspace_folders: None,
                capabilities: None,
            })
            .expect("initialize should succeed");
        let definition = handlers
            .handle_definition(TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: path_to_uri(&reference_path).expect("reference URI"),
                },
                position: Position {
                    line: 0,
                    character: 12,
                },
            })
            .expect("unknown references should be handled");

        assert!(definition.is_none());
    }

    #[test]
    fn declaration_ranges_follow_yaml_structure_and_quoted_ids() {
        let source = concat!(
            "schema: mitase/spec/v1\n",
            "kind: requirements\n",
            "requirements:\n",
            "  - id: \"REQ-NAV-001\"\n",
            "    bindings:\n",
            "      - id: \"verify\"\n",
            "        role: verification\n",
            "        facet: verification\n",
            "        responsibility: Verify the behavior.\n",
            "        targets:\n",
            "          - id: \"test\"\n",
            "            adapter: rust\n",
            "            path: tests/example.rs\n",
            "            selector: { kind: file }\n",
            "            claims:\n",
            "              - kind: verifies\n",
            "                criterion: REQ-NAV-001#criterion.behavior\n",
            "    criteria:\n",
            "      - id: \"behavior\"\n",
        );
        let item: SpecId = "REQ-NAV-001".into();
        let anchor: SpecAnchor = "REQ-NAV-001#criterion.behavior".parse().expect("anchor");
        let target: BoundTargetRef = "REQ-NAV-001#binding.verify/target.test"
            .parse()
            .expect("target");

        let item_range =
            find_declaration_range(source, &item, DeclarationReference::Item).expect("item range");
        assert_eq!(item_range.start.line, 3);
        let anchor_range =
            find_declaration_range(source, &item, DeclarationReference::Anchor(anchor))
                .expect("anchor range");
        assert_eq!(anchor_range.start.line, 18);
        let target_range =
            find_declaration_range(source, &item, DeclarationReference::Target(target))
                .expect("target range");
        assert_eq!(target_range.start.line, 10);

        let short_source = concat!(
            "schema: mitase/authoring/v2\n",
            "kind: requirement\n",
            "requirement:\n",
            "  id: \"REQ-SHORT-001\"\n",
            "  verification:\n",
            "    facet: verification\n",
            "    responsibility: Verify the behavior.\n",
            "    target:\n",
            "      id: \"test\"\n",
            "      path: tests/example.rs\n",
            "  implementation:\n",
            "    id: \"implementation\"\n",
            "    facet: delivery\n",
            "    responsibility: Implement the behavior.\n",
            "    target:\n",
            "      id: \"source\"\n",
            "      path: src/example.rs\n",
            "  criterion: { id: behavior }\n",
        );
        let short_item: SpecId = "REQ-SHORT-001".into();
        let short_binding: SpecAnchor = "REQ-SHORT-001#binding.implementation"
            .parse()
            .expect("short binding");
        let short_target: BoundTargetRef = "REQ-SHORT-001#binding.implementation/target.source"
            .parse()
            .expect("short target");
        let short_binding_range = find_declaration_range(
            short_source,
            &short_item,
            DeclarationReference::Anchor(short_binding),
        )
        .expect("short binding range");
        assert_eq!(short_binding_range.start.line, 11);
        let short_target_range = find_declaration_range(
            short_source,
            &short_item,
            DeclarationReference::Target(short_target),
        )
        .expect("short target range");
        assert_eq!(short_target_range.start.line, 15);

        let comment_source = "# id: REQ-COMMENT-001\nid: \"REQ-COMMENT-001\"\n";
        let comment_item: SpecId = "REQ-COMMENT-001".into();
        let comment_range =
            find_declaration_range(comment_source, &comment_item, DeclarationReference::Item)
                .expect("comment-safe item range");
        assert_eq!(comment_range.start.line, 1);
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
    fn handle_definition_requires_initialization() {
        let handlers = LspHandlers::new();
        let error = handlers
            .handle_definition(TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: "file:///tmp/example.yaml".to_string(),
                },
                position: Position {
                    line: 0,
                    character: 0,
                },
            })
            .expect_err("definition should require initialization");
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
                workspace_folders: None,
                capabilities: None,
            })
            .expect("initialize should succeed");

        assert_eq!(value["capabilities"]["hoverProvider"], true);
        assert_eq!(value["capabilities"]["definitionProvider"], true);
        assert!(handlers.workspace.is_some());
    }

    #[test]
    fn handle_initialize_uses_workspace_folder_when_root_uri_is_omitted() {
        let mut handlers = LspHandlers::new();
        let workspace = fixture_path("valid-web-app");
        let value = handlers
            .handle_initialize(InitializeParams {
                process_id: None,
                root_uri: None,
                workspace_folders: Some(vec![WorkspaceFolder {
                    uri: format!("file://{}", workspace.display()),
                    name: "valid-web-app".to_string(),
                }]),
                capabilities: None,
            })
            .expect("workspaceFolders should select the workspace");

        assert_eq!(
            handlers.workspace_root.as_deref(),
            Some(workspace.as_path())
        );
        assert_eq!(handlers.workspace_folders, vec![workspace]);
        assert_eq!(value["capabilities"]["textDocumentSync"]["change"], 1);
        assert_eq!(
            value["capabilities"]["workspace"]["workspaceFolders"]["supported"],
            true
        );
    }

    #[test]
    fn handle_initialized_publishes_frontend_load_diagnostic() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("docs/mitase")).expect("spec dir");
        fs::write(
            tempdir.path().join("mitase.yaml"),
            "schema: mitase/config/v1\n",
        )
        .expect("config");
        fs::write(
            tempdir.path().join("docs/mitase/invalid.yaml"),
            concat!(
                "schema: mitase/authoring/v2\n",
                "kind: requirement\n",
                "namespace: test\n",
                "category: Test\n",
                "requirement:\n",
                "  id: REQ-LSP-INVALID-001\n",
                "  title: Invalid frontend input\n",
                "  description: The adapter cannot be inferred.\n",
                "  priority: medium\n",
                "  status: planned\n",
                "  criterion: { id: behavior, kind: behavior, statement: Explicit, governed_by: [] }\n",
                "  implementation:\n",
                "    facet: delivery\n",
                "    responsibility: Own the implementation.\n",
                "    target: { path: src/example.txt, satisfies: behavior }\n",
                "  verification:\n",
                "    facet: verification\n",
                "    responsibility: Verify the implementation.\n",
                "    target:\n",
                "      adapter: rust\n",
                "      path: src/lib.rs\n",
                "      verifies: { criterion: behavior, covers: [source], runner: cargo-test }\n",
            ),
        )
        .expect("invalid authoring document");

        let mut handlers = LspHandlers::new();
        handlers
            .handle_initialize(InitializeParams {
                process_id: None,
                root_uri: Some(format!("file://{}", tempdir.path().display())),
                workspace_folders: None,
                capabilities: None,
            })
            .expect("initialize should return capabilities with a frontend diagnostic");
        let notifications = handlers
            .handle_initialized()
            .expect("frontend diagnostics should publish");
        assert!(handlers.workspace.is_none());
        assert!(notifications.iter().any(|notification| {
            notification.params.as_ref().is_some_and(|params| {
                params["diagnostics"].as_array().is_some_and(|diagnostics| {
                    diagnostics.iter().any(|diagnostic| {
                        diagnostic["code"] == "MITASE-AUTHORING-002"
                            && diagnostic["data"]["reason"].is_string()
                            && diagnostic["data"]["suggested_action"].is_string()
                    })
                })
            })
        }));
    }

    #[test]
    fn handle_hover_returns_none_for_out_of_bounds_positions() {
        let mut handlers = LspHandlers::new();
        let workspace = fixture_path("valid-web-app");
        handlers
            .handle_initialize(InitializeParams {
                process_id: None,
                root_uri: Some(format!("file://{}", workspace.display())),
                workspace_folders: None,
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
                workspace_folders: None,
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
        diagnostic = diagnostic
            .with_relation(
                "verifies",
                DiagnosticSubject {
                    kind: "bound-target".into(),
                    value: "FEAT-LSP-001#binding.verification/target.test".into(),
                },
                DiagnosticSubject {
                    kind: "spec-anchor".into(),
                    value: "REQ-LSP-001#criterion.behavior".into(),
                },
            )
            .with_next_read("show", "REQ-LSP-001");

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
        assert_eq!(
            mapped.data.as_ref().expect("canonical data")["relation"]["relation"],
            "verifies"
        );
        assert_eq!(
            mapped.data.as_ref().expect("canonical data")["next"][0]["value"],
            "REQ-LSP-001"
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
                workspace_folders: None,
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

    #[test]
    fn edit_loop_clears_diagnostics_for_a_document_removed_after_reload() {
        let mut handlers = LspHandlers::new();
        let workspace = fixture_path("valid-web-app");
        handlers
            .handle_initialize(InitializeParams {
                process_id: None,
                root_uri: Some(format!("file://{}", workspace.display())),
                workspace_folders: None,
                capabilities: None,
            })
            .expect("initialize should succeed");
        let initial = handlers
            .handle_initialized()
            .expect("initial diagnostics should publish");
        let removed_path = workspace.join("spec/requirement.yaml");
        let removed_uri = path_to_uri(&removed_path).expect("document URI");
        assert!(initial.iter().any(|notification| {
            notification
                .params
                .as_ref()
                .is_some_and(|params| params["uri"] == removed_uri)
        }));

        handlers.workspace = None;
        handlers.startup_diagnostics.clear();
        let refreshed = handlers
            .handle_did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier {
                    uri: removed_uri.clone(),
                },
            })
            .expect("edit loop should refresh diagnostics");
        assert!(refreshed.iter().any(|notification| {
            notification.params.as_ref().is_some_and(|params| {
                params["uri"] == removed_uri
                    && params["diagnostics"].as_array().is_some_and(Vec::is_empty)
            })
        }));
    }
}
