// FEAT-APP-001
// REQ-CORE-017

use std::{fs, io::Write, net::IpAddr, path::Path, sync::Arc};

use anyhow::{Context, Result};
use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{HeaderValue, StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::get,
};
use include_dir::{Dir, include_dir};
use syu_core::{
    AppPayload, DefinitionCounts, ReferencedRule, SectionKind, Severity, SourceDocument,
    TraceCount, TraceSummary, ValidationIssue, ValidationSnapshot,
};

use crate::{
    cli::AppArgs,
    command::check::collect_check_result,
    config::{load_config, resolve_spec_root},
    model::{CheckResult, FeatureRegistryDocument},
};

static APP_DIST: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/app/dist");

#[derive(Clone)]
struct AppState {
    payload: Arc<AppPayload>,
}

pub fn run_app_command(args: &AppArgs) -> Result<i32> {
    let bind = args
        .bind
        .parse::<IpAddr>()
        .with_context(|| format!("invalid bind address `{}`", args.bind))?;
    let payload = build_app_payload(&args.workspace)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to create runtime for `syu app`")?;

    runtime.block_on(async move {
        let router = app_router(AppState {
            payload: Arc::new(payload),
        });
        let listener = tokio::net::TcpListener::bind((bind, args.port))
            .await
            .with_context(|| format!("failed to bind `{bind}:{}`", args.port))?;
        let local_addr = listener
            .local_addr()
            .context("failed to inspect bind address")?;
        println!("syu app listening on http://{local_addr}");
        std::io::stdout()
            .flush()
            .context("failed to flush stdout")?;

        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .context("local app server exited unexpectedly")
    })?;

    Ok(0)
}

fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/api/app-data.json", get(app_data))
        .route("/healthz", get(healthz))
        .fallback(get(serve_static))
        .with_state(state)
}

async fn app_data(State(state): State<AppState>) -> Json<AppPayload> {
    Json((*state.payload).clone())
}

async fn healthz() -> &'static str {
    "ok"
}

async fn serve_static(uri: Uri) -> Response {
    let Some(path) = normalized_asset_path(uri.path()) else {
        return (StatusCode::NOT_FOUND, "asset not found").into_response();
    };

    if let Some(file) = APP_DIST.get_file(&path) {
        return asset_response(&path, file.contents());
    }

    if !is_asset_like(&path)
        && let Some(index) = APP_DIST.get_file("index.html")
    {
        return asset_response("index.html", index.contents());
    }

    (StatusCode::NOT_FOUND, "asset not found").into_response()
}

fn normalized_asset_path(path: &str) -> Option<String> {
    if path.starts_with("/api/") || path.contains("..") {
        return None;
    }

    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        Some("index.html".to_string())
    } else {
        Some(trimmed.to_string())
    }
}

fn is_asset_like(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|segment| segment.contains('.'))
}

fn asset_response(path: &str, bytes: &'static [u8]) -> Response {
    let mut response = Response::new(Body::from(bytes.to_vec()));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type_for_path(path)),
    );
    response
}

fn content_type_for_path(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".js") {
        "application/javascript; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".json") {
        "application/json; charset=utf-8"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".wasm") {
        "application/wasm"
    } else {
        "application/octet-stream"
    }
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

fn build_app_payload(workspace_root: &Path) -> Result<AppPayload> {
    let workspace_root = workspace_root.canonicalize().with_context(|| {
        format!(
            "failed to resolve workspace root `{}`",
            workspace_root.display()
        )
    })?;
    let loaded = load_config(&workspace_root)?;
    let spec_root = resolve_spec_root(&workspace_root, &loaded.config);

    let mut source_documents = Vec::new();
    source_documents.extend(collect_yaml_sources_recursive(
        &spec_root.join("philosophy"),
        SectionKind::Philosophy,
    )?);
    source_documents.extend(collect_yaml_sources_recursive(
        &spec_root.join("policies"),
        SectionKind::Policies,
    )?);
    source_documents.extend(collect_yaml_sources_recursive(
        &spec_root.join("requirements"),
        SectionKind::Requirements,
    )?);
    source_documents.extend(collect_feature_sources(&spec_root.join("features"))?);
    source_documents.sort_by(|left, right| {
        (left.section.label(), left.path.as_str())
            .cmp(&(right.section.label(), right.path.as_str()))
    });

    Ok(AppPayload {
        workspace_root: workspace_root.display().to_string(),
        spec_root: spec_root.display().to_string(),
        source_documents,
        validation: validation_snapshot(collect_check_result(&workspace_root)),
    })
}

fn collect_feature_sources(feature_root: &Path) -> Result<Vec<SourceDocument>> {
    if !feature_root.is_dir() {
        return Ok(Vec::new());
    }

    let registry_path = feature_root.join("features.yaml");
    if registry_path.is_file() {
        let raw = fs::read_to_string(&registry_path).with_context(|| {
            format!(
                "failed to read feature registry `{}`",
                registry_path.display()
            )
        })?;
        if let Ok(registry) = serde_yaml::from_str::<FeatureRegistryDocument>(&raw) {
            let mut sources = Vec::new();
            for entry in registry.files {
                let path = feature_root.join(entry.file);
                if path.is_file()
                    && let Ok(source) =
                        read_source_document(feature_root, &path, SectionKind::Features)
                {
                    sources.push(source);
                }
            }
            sources.sort_by(|left, right| left.path.cmp(&right.path));
            return Ok(sources);
        }
    }

    let mut sources = collect_yaml_sources_recursive(feature_root, SectionKind::Features)?;
    sources.retain(|source| source.path != "features.yaml");
    Ok(sources)
}

fn collect_yaml_sources_recursive(
    directory: &Path,
    section: SectionKind,
) -> Result<Vec<SourceDocument>> {
    let mut sources = Vec::new();
    collect_yaml_sources_into(directory, directory, section, &mut sources)?;
    sources.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(sources)
}

fn collect_yaml_sources_into(
    root: &Path,
    current: &Path,
    section: SectionKind,
    sources: &mut Vec<SourceDocument>,
) -> Result<()> {
    if !current.is_dir() {
        return Ok(());
    }

    let mut entries = fs::read_dir(current)
        .with_context(|| format!("failed to read directory `{}`", current.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("failed to enumerate directory `{}`", current.display()))?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_yaml_sources_into(root, &path, section, sources)?;
        } else if is_yaml_file(&path) {
            sources.push(read_source_document(root, &path, section)?);
        }
    }

    Ok(())
}

fn is_yaml_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("yaml" | "yml")
    )
}

fn read_source_document(root: &Path, path: &Path, section: SectionKind) -> Result<SourceDocument> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read source document `{}`", path.display()))?;
    let relative = relative_display(root, path)?;

    Ok(SourceDocument {
        section,
        path: relative,
        content,
    })
}

fn relative_display(root: &Path, path: &Path) -> Result<String> {
    let relative = path.strip_prefix(root).with_context(|| {
        format!(
            "failed to make `{}` relative to `{}`",
            path.display(),
            root.display()
        )
    })?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn validation_snapshot(result: CheckResult) -> ValidationSnapshot {
    ValidationSnapshot {
        definition_counts: DefinitionCounts {
            philosophies: result.definition_counts.philosophies,
            policies: result.definition_counts.policies,
            requirements: result.definition_counts.requirements,
            features: result.definition_counts.features,
        },
        trace_summary: TraceSummary {
            requirement_traces: TraceCount {
                declared: result.trace_summary.requirement_traces.declared,
                validated: result.trace_summary.requirement_traces.validated,
            },
            feature_traces: TraceCount {
                declared: result.trace_summary.feature_traces.declared,
                validated: result.trace_summary.feature_traces.validated,
            },
        },
        issues: result
            .issues
            .into_iter()
            .map(|issue| ValidationIssue {
                code: issue.code,
                severity: match issue.severity {
                    crate::model::Severity::Error => Severity::Error,
                    crate::model::Severity::Warning => Severity::Warning,
                },
                subject: issue.subject,
                location: issue.location,
                message: issue.message,
                suggestion: issue.suggestion,
            })
            .collect(),
        referenced_rules: result
            .referenced_rules
            .into_iter()
            .map(|rule| ReferencedRule {
                genre: rule.genre,
                code: rule.code,
                severity: rule.severity,
                title: rule.title,
                summary: rule.summary,
                description: rule.description,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::Arc,
    };

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use axum::{
        body::{Body, to_bytes},
        http::{HeaderValue, Request, StatusCode, header},
    };
    use tempfile::tempdir;
    use tower::util::ServiceExt;

    use super::{
        AppPayload, AppState, SectionKind, Severity, app_router, build_app_payload,
        collect_feature_sources, collect_yaml_sources_recursive, content_type_for_path,
        is_asset_like, normalized_asset_path, relative_display, validation_snapshot,
    };

    fn fixture_root(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/workspaces")
            .join(name)
    }

    fn create_workspace_skeleton(root: &Path) -> PathBuf {
        let spec_root = root.join("docs/syu");
        fs::create_dir_all(spec_root.join("philosophy")).expect("philosophy dir");
        fs::create_dir_all(spec_root.join("policies")).expect("policies dir");
        fs::create_dir_all(spec_root.join("requirements")).expect("requirements dir");
        fs::create_dir_all(spec_root.join("features")).expect("features dir");
        spec_root
    }

    #[cfg(unix)]
    fn set_mode(path: &Path, mode: u32) {
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("permissions");
    }

    #[test]
    fn normalized_asset_path_blocks_api_and_parent_segments() {
        assert_eq!(normalized_asset_path("/"), Some("index.html".to_string()));
        assert_eq!(
            normalized_asset_path("/assets/index.js"),
            Some("assets/index.js".to_string())
        );
        assert_eq!(normalized_asset_path("/api/app-data.json"), None);
        assert_eq!(normalized_asset_path("/../../secret"), None);
    }

    #[test]
    fn asset_like_detection_matches_expected_paths() {
        assert!(is_asset_like("assets/index.js"));
        assert!(is_asset_like("favicon.svg"));
        assert!(!is_asset_like("requirements/REQ-001"));
    }

    #[test]
    fn content_type_detection_covers_known_assets() {
        assert_eq!(
            content_type_for_path("index.html"),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            content_type_for_path("app.js"),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(
            content_type_for_path("styles.css"),
            "text/css; charset=utf-8"
        );
        assert_eq!(
            content_type_for_path("report.json"),
            "application/json; charset=utf-8"
        );
        assert_eq!(content_type_for_path("icon.svg"), "image/svg+xml");
        assert_eq!(content_type_for_path("image.png"), "image/png");
        assert_eq!(content_type_for_path("engine.wasm"), "application/wasm");
        assert_eq!(
            content_type_for_path("blob.bin"),
            "application/octet-stream"
        );
    }

    #[test]
    fn build_app_payload_collects_current_workspace_sources() {
        let payload = build_app_payload(&fixture_root("passing")).expect("payload should build");
        assert!(payload.workspace_root.contains("passing"));
        assert!(payload.spec_root.contains("docs/syu"));
        assert!(
            payload
                .source_documents
                .iter()
                .any(|source| source.section == SectionKind::Philosophy)
        );
        assert!(
            payload
                .source_documents
                .iter()
                .any(|source| source.section == SectionKind::Features)
        );
        assert_eq!(payload.validation.definition_counts.features, 3);
    }

    #[test]
    fn build_app_payload_surfaces_validation_snapshot_details() {
        let payload = build_app_payload(&fixture_root("failing")).expect("payload should build");

        assert!(!payload.validation.issues.is_empty());
        assert!(!payload.validation.referenced_rules.is_empty());
        assert!(
            payload
                .validation
                .issues
                .iter()
                .all(|issue| !issue.message.is_empty())
        );
        assert!(
            payload
                .validation
                .referenced_rules
                .iter()
                .all(|rule| !rule.title.is_empty() && !rule.description.is_empty())
        );
    }

    #[test]
    fn validation_snapshot_maps_warning_issues() {
        let snapshot = validation_snapshot(crate::model::CheckResult {
            workspace_root: PathBuf::from("/repo"),
            definition_counts: crate::model::DefinitionCounts::default(),
            trace_summary: crate::model::TraceSummary::default(),
            issues: vec![crate::model::Issue::warning(
                "SYU-graph-orphan-001",
                "policy",
                None,
                "warning message",
                Some("warning suggestion".to_string()),
            )],
            referenced_rules: Vec::new(),
        });

        assert_eq!(snapshot.issues.len(), 1);
        assert_eq!(snapshot.issues[0].severity, Severity::Warning);
    }

    #[test]
    fn build_app_payload_reports_missing_workspace_roots() {
        let missing = fixture_root("passing").join("missing-workspace-root");
        let error = build_app_payload(&missing).expect_err("missing roots should error");
        assert!(
            error
                .to_string()
                .contains("failed to resolve workspace root")
        );
    }

    #[cfg(unix)]
    #[test]
    fn build_app_payload_propagates_philosophy_directory_errors() {
        let tempdir = tempdir().expect("tempdir should exist");
        let spec_root = create_workspace_skeleton(tempdir.path());
        let philosophy_root = spec_root.join("philosophy");
        set_mode(&philosophy_root, 0o000);

        let result = build_app_payload(tempdir.path());
        set_mode(&philosophy_root, 0o755);

        let error = result.expect_err("unreadable philosophy directory should fail");
        assert!(error.to_string().contains("failed to read directory"));
    }

    #[cfg(unix)]
    #[test]
    fn build_app_payload_propagates_policies_directory_errors() {
        let tempdir = tempdir().expect("tempdir should exist");
        let spec_root = create_workspace_skeleton(tempdir.path());
        let policies_root = spec_root.join("policies");
        set_mode(&policies_root, 0o000);

        let result = build_app_payload(tempdir.path());
        set_mode(&policies_root, 0o755);

        let error = result.expect_err("unreadable policies directory should fail");
        assert!(error.to_string().contains("failed to read directory"));
    }

    #[cfg(unix)]
    #[test]
    fn build_app_payload_propagates_requirements_directory_errors() {
        let tempdir = tempdir().expect("tempdir should exist");
        let spec_root = create_workspace_skeleton(tempdir.path());
        let requirements_root = spec_root.join("requirements");
        set_mode(&requirements_root, 0o000);

        let result = build_app_payload(tempdir.path());
        set_mode(&requirements_root, 0o755);

        let error = result.expect_err("unreadable requirements directory should fail");
        assert!(error.to_string().contains("failed to read directory"));
    }

    #[test]
    fn feature_source_collection_returns_empty_when_feature_root_is_missing() {
        let tempdir = tempdir().expect("tempdir should exist");
        let sources = collect_feature_sources(&tempdir.path().join("docs/syu/features"))
            .expect("missing roots should be empty");
        assert!(sources.is_empty());
    }

    #[test]
    fn feature_source_collection_falls_back_when_registry_is_missing() {
        let tempdir = tempdir().expect("tempdir should exist");
        let feature_root = tempdir.path().join("docs/syu/features");
        fs::create_dir_all(&feature_root).expect("dir");
        fs::write(
            feature_root.join("browser.yaml"),
            "category: Browser\nversion: 1\nfeatures:\n  - id: FEAT-001\n    title: Browser\n    summary: Summary\n    status: implemented\n",
        )
        .expect("feature");

        let sources = collect_feature_sources(&feature_root).expect("fallback should work");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].path, "browser.yaml");
    }

    #[cfg(unix)]
    #[test]
    fn feature_source_collection_reports_unreadable_registry_files() {
        let tempdir = tempdir().expect("tempdir should exist");
        let feature_root = tempdir.path().join("docs/syu/features");
        fs::create_dir_all(&feature_root).expect("dir");
        let registry_path = feature_root.join("features.yaml");
        fs::write(&registry_path, "version: \"1\"\nfiles: []\n").expect("registry");
        set_mode(&registry_path, 0o000);

        let result = collect_feature_sources(&feature_root);
        set_mode(&registry_path, 0o644);

        let error = result.expect_err("unreadable registries should fail");
        assert!(
            error
                .to_string()
                .contains("failed to read feature registry")
        );
    }

    #[test]
    fn recursive_yaml_collection_ignores_non_directories() {
        let tempdir = tempdir().expect("tempdir should exist");
        let file_path = tempdir.path().join("feature.yaml");
        fs::write(
            &file_path,
            "category: Placeholder\nversion: 1\nfeatures: []\n",
        )
        .expect("file");

        let sources = collect_yaml_sources_recursive(&file_path, SectionKind::Features)
            .expect("non-directories should be ignored");
        assert!(sources.is_empty());
    }

    #[test]
    fn recursive_yaml_collection_reads_nested_yaml_files() {
        let tempdir = tempdir().expect("tempdir should exist");
        let source_root = tempdir.path().join("nested");
        fs::create_dir_all(source_root.join("inner")).expect("dirs");
        fs::write(
            source_root.join("top.yaml"),
            "category: Top\nversion: 1\nfeatures: []\n",
        )
        .expect("top yaml");
        fs::write(
            source_root.join("inner/leaf.yml"),
            "category: Leaf\nversion: 1\nfeatures: []\n",
        )
        .expect("leaf yaml");
        fs::write(source_root.join("inner/readme.txt"), "ignore").expect("txt");

        let sources = collect_yaml_sources_recursive(&source_root, SectionKind::Features)
            .expect("recursive collection should work");
        let paths: Vec<_> = sources.into_iter().map(|source| source.path).collect();
        assert_eq!(
            paths,
            vec!["inner/leaf.yml".to_string(), "top.yaml".to_string()]
        );
    }

    #[test]
    fn relative_display_errors_for_paths_outside_the_root() {
        let tempdir = tempdir().expect("tempdir should exist");
        let root = tempdir.path().join("root");
        let outside = tempdir.path().join("outside/file.yaml");
        fs::create_dir_all(&root).expect("root dir");
        fs::create_dir_all(outside.parent().expect("parent")).expect("outside dir");
        fs::write(&outside, "category: Outside\nversion: 1\nfeatures: []\n").expect("outside file");

        let error = relative_display(&root, &outside).expect_err("outside paths should fail");
        assert!(error.to_string().contains("failed to make"));
    }

    #[test]
    fn feature_source_collection_falls_back_when_registry_is_invalid() {
        let tempdir = tempdir().expect("tempdir should exist");
        let feature_root = tempdir.path().join("docs/syu/features");
        fs::create_dir_all(&feature_root).expect("dir");
        fs::write(feature_root.join("features.yaml"), "version: [\n").expect("registry");
        fs::write(
            feature_root.join("browser.yaml"),
            "category: Browser\nversion: 1\nfeatures:\n  - id: FEAT-001\n    title: Browser\n    summary: Summary\n    status: implemented\n",
        )
        .expect("feature");

        let sources = collect_feature_sources(&feature_root).expect("fallback should work");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].path, "browser.yaml");
    }

    #[tokio::test]
    async fn api_route_returns_payload_json() {
        let router = app_router(AppState {
            payload: Arc::new(AppPayload {
                workspace_root: "/repo".to_string(),
                spec_root: "/repo/docs/syu".to_string(),
                source_documents: Vec::new(),
                validation: Default::default(),
            }),
        });

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/api/app-data.json")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let json = String::from_utf8(body.to_vec()).expect("utf8");
        assert!(json.contains("\"workspace_root\":\"/repo\""));
    }

    #[tokio::test]
    async fn api_like_paths_return_not_found() {
        let router = app_router(AppState {
            payload: Arc::new(AppPayload::default()),
        });

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/api/not-a-real-endpoint")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn static_routes_serve_embedded_app() {
        let router = app_router(AppState {
            payload: Arc::new(AppPayload::default()),
        });

        let root = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(root.status(), StatusCode::OK);
        assert_eq!(
            root.headers().get(header::CONTENT_TYPE),
            Some(&HeaderValue::from_static("text/html; charset=utf-8"))
        );

        let nested = router
            .oneshot(
                Request::builder()
                    .uri("/requirements/REQ-CORE-017")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(nested.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn unknown_assets_return_not_found() {
        let router = app_router(AppState {
            payload: Arc::new(AppPayload::default()),
        });

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/assets/missing.js")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
