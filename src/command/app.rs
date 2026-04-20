// FEAT-APP-001
// REQ-CORE-017

use std::{
    collections::{BTreeSet, hash_map::DefaultHasher},
    env, fs,
    hash::{Hash, Hasher},
    io::{Read, Write},
    net::{IpAddr, SocketAddr},
    path::{Component, Path, PathBuf},
    sync::{Arc, RwLock},
    time::Duration,
};

use anyhow::{Context, Result, anyhow, bail};
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
    config::{SyuConfig, load_config, resolve_spec_root},
    coverage::normalize_relative_path,
    model::{CheckResult, FeatureRegistryDocument, TraceReference},
    workspace::{
        load_feature_documents_with_paths, load_requirement_documents_with_paths,
        resolve_workspace_root,
    },
};

static APP_DIST: Dir<'_> = include_dir!("$OUT_DIR/syu-app-dist");

#[derive(Clone)]
struct AppState {
    workspace_root: PathBuf,
    config: SyuConfig,
    current: Arc<RwLock<CurrentAppState>>,
}

#[derive(Debug, Clone)]
struct CurrentAppData {
    snapshot: String,
    payload: AppPayload,
}

#[derive(Debug, Clone)]
enum CurrentAppState {
    Ready(CurrentAppData),
    Error(String),
}

impl AppState {
    fn new(workspace_root: PathBuf, config: SyuConfig) -> Self {
        Self {
            workspace_root,
            config,
            current: Arc::new(RwLock::new(CurrentAppState::Error(
                "app data refresh failed".to_string(),
            ))),
        }
    }

    fn current_data(&self) -> Result<CurrentAppData> {
        match self
            .current
            .read()
            .map_err(|_| anyhow!("app refresh state lock poisoned"))?
            .clone()
        {
            CurrentAppState::Ready(data) => Ok(data),
            CurrentAppState::Error(message) => Err(anyhow!(message)),
        }
    }

    fn current_snapshot(&self) -> Result<Option<String>> {
        match &*self
            .current
            .read()
            .map_err(|_| anyhow!("app refresh state lock poisoned"))?
        {
            CurrentAppState::Ready(data) => Ok(Some(data.snapshot.clone())),
            CurrentAppState::Error(_) => Ok(None),
        }
    }

    fn replace_current(&self, next: CurrentAppState) -> Result<()> {
        let result = match &next {
            CurrentAppState::Ready(_) => Ok(()),
            CurrentAppState::Error(message) => Err(anyhow!(message.clone())),
        };

        *self
            .current
            .write()
            .map_err(|_| anyhow!("app refresh state lock poisoned"))? = next;

        result
    }

    fn refresh_current(&self) -> Result<()> {
        self.refresh_current_with(
            || load_current_snapshot(&self.workspace_root, &self.config),
            || load_current_payload(&self.workspace_root, &self.config),
        )
    }

    fn refresh_current_with<LoadSnapshot, LoadPayload>(
        &self,
        load_snapshot: LoadSnapshot,
        load_payload: LoadPayload,
    ) -> Result<()>
    where
        LoadSnapshot: FnOnce() -> Result<String>,
        LoadPayload: FnOnce() -> Result<AppPayload>,
    {
        let current_snapshot = self.current_snapshot()?;
        let next = match load_snapshot() {
            Ok(snapshot) => {
                if current_snapshot.as_deref() == Some(snapshot.as_str()) {
                    return Ok(());
                }

                match load_payload() {
                    Ok(payload) => CurrentAppState::Ready(CurrentAppData { snapshot, payload }),
                    Err(error) => CurrentAppState::Error(format!("{error:#}")),
                }
            }
            Err(error) => CurrentAppState::Error(format!("{error:#}")),
        };

        self.replace_current(next)
    }
}

#[derive(serde::Serialize)]
struct HealthStatus {
    status: &'static str,
    version: &'static str,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct AppVersion {
    snapshot: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AppServerSettings {
    bind: String,
    port: u16,
}

fn bind_failure_message(
    workspace_root: &Path,
    bind: IpAddr,
    port: u16,
    error: &std::io::Error,
) -> String {
    let likely_cause = if error.kind() == std::io::ErrorKind::AddrInUse {
        "The selected port is likely already in use."
    } else {
        "The selected address or port may be unavailable on this machine."
    };
    let workspace = workspace_root.display();

    format!(
        "failed to bind `{bind}:{port}`. {likely_cause} Try `syu app {workspace} --port <free-port>` to retry with a different port, or set `app.port` in syu.yaml to change the default. {error}"
    )
}

fn require_remote_bind_opt_in(bind: IpAddr, allow_remote: bool) -> Result<()> {
    if bind.is_loopback() || allow_remote {
        return Ok(());
    }

    Err(anyhow!(
        "refusing to bind `syu app` to non-loopback address `{bind}` without `--allow-remote`. \
This protects workspace data and source documents from accidental network exposure. \
Use `--bind 127.0.0.1` to keep the browser UI local, or pass `--allow-remote` when remote access is intentional."
    ))
}

pub fn run_app_command(args: &AppArgs) -> Result<i32> {
    let workspace_root = canonical_workspace_root(&args.workspace)?;
    let loaded = load_config(&workspace_root)?;
    let settings = resolve_app_server_settings(args, &loaded.config);
    let bind = settings
        .bind
        .parse::<IpAddr>()
        .with_context(|| format!("invalid bind address `{}`", settings.bind))?;
    require_remote_bind_opt_in(bind, args.allow_remote)?;
    build_app_payload_from_config(&workspace_root, &loaded.config)?;
    println!("workspace: {}", workspace_root.display());
    let state = AppState::new(workspace_root.clone(), loaded.config);
    let _ = state.refresh_current();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to create runtime for `syu app`")?;

    runtime.block_on(async move {
        let router = app_router(state.clone());
        let refresher = tokio::spawn(refresh_current_until_shutdown(state.clone()));
        let listener = tokio::net::TcpListener::bind((bind, settings.port))
            .await
            .map_err(|error| {
                anyhow!(bind_failure_message(
                    &workspace_root,
                    bind,
                    settings.port,
                    &error,
                ))
            })?;
        let local_addr = listener
            .local_addr()
            .context("failed to inspect bind address")?;
        let server = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(shutdown_signal())
                .await
                .context("local app server exited unexpectedly")
        });
        emit_startup_lines(non_loopback_warning_lines(local_addr.ip()))?;
        tokio::task::spawn_blocking(move || wait_for_ready(local_addr))
            .await
            .context("local app readiness probe panicked")??;
        emit_startup_lines(startup_lines(local_addr))?;

        let result = server.await.context("local app server task panicked")?;
        refresher.abort();
        result
    })?;

    Ok(0)
}

fn startup_lines(local_addr: SocketAddr) -> Vec<String> {
    vec![
        format!("syu app listening on http://{local_addr}"),
        format!("syu app ready: http://{local_addr}"),
        format!("Open http://{local_addr} in your browser."),
        "Press Ctrl-C to stop.".to_string(),
    ]
}

fn non_loopback_warning_lines(bind: IpAddr) -> Vec<String> {
    if bind.is_loopback() {
        return Vec::new();
    }

    vec![
        format!(
            "warning: syu app is bound to {bind}, so workspace data and source documents may be reachable from other machines on your network."
        ),
        "warning: use --bind 127.0.0.1 to keep the browser UI local to this machine.".to_string(),
    ]
}

fn emit_startup_lines(lines: Vec<String>) -> Result<()> {
    for message in lines {
        println!("{message}");
        std::io::stdout()
            .flush()
            .context("failed to flush stdout")?;
    }

    Ok(())
}

fn resolve_app_server_settings(args: &AppArgs, config: &SyuConfig) -> AppServerSettings {
    AppServerSettings {
        bind: args.bind.clone().unwrap_or_else(|| config.app.bind.clone()),
        port: args.port.unwrap_or(config.app.port),
    }
}

fn canonical_workspace_root(workspace_root: &Path) -> Result<PathBuf> {
    resolve_workspace_root(workspace_root)
}

fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/api/app-data.json", get(app_data))
        .route("/api/version", get(app_version))
        .route("/health", get(health))
        .route("/healthz", get(healthz))
        .fallback(get(serve_static))
        .with_state(state)
}

async fn app_data(State(state): State<AppState>) -> std::result::Result<Response, AppError> {
    let current = state.current_data()?;
    let mut response = Json(current.payload).into_response();
    response.headers_mut().insert(
        "x-syu-snapshot",
        HeaderValue::from_str(&current.snapshot).context("invalid snapshot header value")?,
    );
    Ok(response)
}

async fn app_version(
    State(state): State<AppState>,
) -> std::result::Result<Json<AppVersion>, AppError> {
    let current = state.current_data()?;
    Ok(Json(AppVersion {
        snapshot: current.snapshot,
    }))
}

async fn health() -> Json<HealthStatus> {
    Json(HealthStatus {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn healthz() -> &'static str {
    "ok"
}

async fn refresh_current_until_shutdown(state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        interval.tick().await;
        refresh_current_once(&state);
    }
}

fn refresh_current_once(state: &AppState) {
    if let Err(error) = state.refresh_current() {
        eprintln!("syu app refresh failed: {error:#}");
    }
}

fn load_current_payload(workspace_root: &Path, config: &SyuConfig) -> Result<AppPayload> {
    build_app_payload_from_config(workspace_root, config)
}

fn load_current_snapshot(workspace_root: &Path, config: &SyuConfig) -> Result<String> {
    app_payload_snapshot(workspace_root, config)
}

fn app_payload_snapshot(workspace_root: &Path, config: &SyuConfig) -> Result<String> {
    let spec_root = resolve_spec_root(workspace_root, config);
    let mut hasher = DefaultHasher::new();
    spec_snapshot(&spec_root)?.hash(&mut hasher);

    for dependency in app_snapshot_dependencies(workspace_root, &spec_root, config) {
        dependency.hash_state(workspace_root, &mut hasher);
    }

    Ok(format!("{:016x}", hasher.finish()))
}

fn spec_snapshot(spec_root: &Path) -> Result<String> {
    if !spec_root.is_dir() {
        bail!(
            "failed to read spec root `{}` because it is not a directory",
            spec_root.display()
        );
    }

    let mut sources = Vec::new();
    let philosophy = section_sources(spec_root, "philosophy", SectionKind::Philosophy)?;
    let policies = section_sources(spec_root, "policies", SectionKind::Policies)?;
    let requirements = section_sources(spec_root, "requirements", SectionKind::Requirements)?;
    sources.extend(philosophy);
    sources.extend(policies);
    sources.extend(requirements);
    sources.extend(feature_sources(spec_root)?);
    sources.sort_by(|left, right| {
        (left.section.label(), left.path.as_str())
            .cmp(&(right.section.label(), right.path.as_str()))
    });

    let mut hasher = DefaultHasher::new();
    for source in sources {
        source.section.label().hash(&mut hasher);
        source.path.hash(&mut hasher);
        source.content.hash(&mut hasher);
    }
    Ok(format!("{:016x}", hasher.finish()))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum SnapshotDependency {
    File(PathBuf),
    ReadDirError(PathBuf, String),
}

impl SnapshotDependency {
    fn hash_state(&self, workspace_root: &Path, hasher: &mut DefaultHasher) {
        match self {
            Self::File(path) => {
                "file".hash(hasher);
                path.hash(hasher);
                match fs::read(workspace_root.join(path)) {
                    Ok(bytes) => {
                        "readable".hash(hasher);
                        bytes.hash(hasher);
                    }
                    Err(error) => {
                        "unreadable".hash(hasher);
                        format!("{:?}", error.kind()).hash(hasher);
                    }
                }
            }
            Self::ReadDirError(path, kind) => {
                "read_dir_error".hash(hasher);
                path.hash(hasher);
                kind.hash(hasher);
            }
        }
    }
}

fn app_snapshot_dependencies(
    workspace_root: &Path,
    spec_root: &Path,
    config: &SyuConfig,
) -> BTreeSet<SnapshotDependency> {
    let mut dependencies = BTreeSet::new();

    if let Ok(documents) = load_requirement_documents_with_paths(&spec_root.join("requirements")) {
        for document in documents {
            for requirement in document.document.requirements {
                collect_trace_map_snapshot_dependencies(&requirement.tests, &mut dependencies);
            }
        }
    }

    if let Ok(documents) = load_feature_documents_with_paths(&spec_root.join("features")) {
        for document in documents {
            for feature in document.document.features {
                collect_trace_map_snapshot_dependencies(
                    &feature.implementations,
                    &mut dependencies,
                );
            }
        }
    }

    if config.validate.require_symbol_trace_coverage {
        collect_coverage_snapshot_dependencies(workspace_root, &mut dependencies);
    }

    dependencies
}

fn collect_trace_map_snapshot_dependencies(
    references_by_language: &std::collections::BTreeMap<String, Vec<TraceReference>>,
    dependencies: &mut BTreeSet<SnapshotDependency>,
) {
    for references in references_by_language.values() {
        for reference in references {
            if let Some(path) = normalized_trace_snapshot_path(&reference.file) {
                dependencies.insert(SnapshotDependency::File(path));
            }
        }
    }
}

fn normalized_trace_snapshot_path(file: &Path) -> Option<PathBuf> {
    let portable = file.to_string_lossy().replace('\\', "/");
    let normalized = normalize_relative_path(Path::new(&portable));

    if normalized.as_os_str().is_empty()
        || normalized.is_absolute()
        || normalized
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return None;
    }

    Some(normalized)
}

fn collect_coverage_snapshot_dependencies(
    workspace_root: &Path,
    dependencies: &mut BTreeSet<SnapshotDependency>,
) {
    const COVERAGE_EXTENSIONS: &[&str] = &["rs", "py", "ts", "tsx", "js", "jsx"];

    collect_snapshot_files_with_extensions(
        workspace_root,
        &workspace_root.join("src"),
        COVERAGE_EXTENSIONS,
        dependencies,
    );
    collect_snapshot_files_with_extensions(
        workspace_root,
        &workspace_root.join("tests"),
        COVERAGE_EXTENSIONS,
        dependencies,
    );
}

fn collect_snapshot_files_with_extensions(
    workspace_root: &Path,
    directory: &Path,
    extensions: &[&str],
    dependencies: &mut BTreeSet<SnapshotDependency>,
) {
    if !directory.exists() {
        return;
    }

    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            dependencies.insert(SnapshotDependency::ReadDirError(
                relative_workspace_path(workspace_root, directory),
                format!("{:?}", error.kind()),
            ));
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_snapshot_files_with_extensions(workspace_root, &path, extensions, dependencies);
            continue;
        }

        let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        if extensions.contains(&extension) {
            dependencies.insert(SnapshotDependency::File(relative_workspace_path(
                workspace_root,
                &path,
            )));
        }
    }
}

fn relative_workspace_path(workspace_root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .to_path_buf()
}

fn section_sources(
    spec_root: &Path,
    directory: &str,
    section: SectionKind,
) -> Result<Vec<SourceDocument>> {
    collect_yaml_sources_recursive(&spec_root.join(directory), section)
}

fn feature_sources(spec_root: &Path) -> Result<Vec<SourceDocument>> {
    collect_feature_sources(&spec_root.join("features"))
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

fn wait_for_ready(local_addr: SocketAddr) -> Result<()> {
    wait_for_ready_with_retry(
        local_addr,
        50,
        Duration::from_millis(100),
        Duration::from_millis(50),
    )
}

fn wait_for_ready_with_retry(
    local_addr: SocketAddr,
    attempts: usize,
    connect_timeout: Duration,
    retry_delay: Duration,
) -> Result<()> {
    for _ in 0..attempts {
        if let Ok(mut stream) = std::net::TcpStream::connect_timeout(&local_addr, connect_timeout) {
            stream
                .set_read_timeout(Some(connect_timeout))
                .context("failed to configure readiness probe read timeout")?;
            stream
                .set_write_timeout(Some(connect_timeout))
                .context("failed to configure readiness probe write timeout")?;
            if readiness_probe_succeeds(&mut stream, local_addr) {
                return Ok(());
            }
        }

        std::thread::sleep(retry_delay);
    }

    bail!("local app server did not become ready at http://{local_addr}")
}

fn readiness_probe_succeeds(stream: &mut (impl Read + Write), local_addr: SocketAddr) -> bool {
    if !readiness_probe_request_sent(stream, local_addr) {
        return false;
    }

    let mut response = String::new();
    stream.read_to_string(&mut response).is_ok() && response.contains("200 OK")
}

fn readiness_probe_request_sent(stream: &mut impl Write, local_addr: SocketAddr) -> bool {
    if write!(
        stream,
        "GET /health HTTP/1.1\r\nHost: {local_addr}\r\nConnection: close\r\n\r\n"
    )
    .is_err()
        || stream.flush().is_err()
    {
        return false;
    }

    true
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
fn build_app_payload(workspace_root: &Path) -> Result<AppPayload> {
    let workspace_root = canonical_workspace_root(workspace_root)?;
    let loaded = load_config(&workspace_root)?;
    build_app_payload_from_config(&workspace_root, &loaded.config)
}

fn build_app_payload_from_config(workspace_root: &Path, config: &SyuConfig) -> Result<AppPayload> {
    let spec_root = resolve_spec_root(workspace_root, config);
    let (workspace_label, spec_label) = browser_root_labels(workspace_root, &spec_root);

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
        workspace_root: workspace_label,
        spec_root: spec_label,
        source_documents,
        validation: validation_snapshot(collect_check_result(workspace_root)),
    })
}

fn browser_root_labels(workspace_root: &Path, spec_root: &Path) -> (String, String) {
    let workspace_label = redacted_root_label(workspace_root);
    let spec_label = match relative_display(workspace_root, spec_root) {
        Ok(relative) if relative.is_empty() => ".".to_string(),
        Ok(relative) => relative,
        Err(_) => "external spec root".to_string(),
    };
    (workspace_label, spec_label)
}

fn redacted_root_label(path: &Path) -> String {
    if let Some(label) = redacted_relative_label(env::current_dir().ok(), path, ".") {
        return label;
    }

    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    if let Some(label) = redacted_relative_label(home, path, "~") {
        return label;
    }

    let fallback = trailing_path_components_label(path, 2);
    if !fallback.is_empty() {
        return fallback;
    }

    "workspace".to_string()
}

fn redacted_relative_label(root: Option<PathBuf>, path: &Path, prefix: &str) -> Option<String> {
    let root = root?;
    let root = root.canonicalize().unwrap_or(root);
    let relative = path.strip_prefix(&root).ok()?;
    let relative = path_label(relative);
    if relative.is_empty() {
        Some(prefix.to_string())
    } else {
        Some(format!("{prefix}/{relative}"))
    }
}

fn trailing_path_components_label(path: &Path, count: usize) -> String {
    let segments: Vec<_> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(segment) => Some(segment.to_os_string()),
            _ => None,
        })
        .collect();
    if segments.is_empty() {
        return String::new();
    }

    let mut label = PathBuf::new();
    for segment in segments.iter().skip(segments.len().saturating_sub(count)) {
        label.push(segment);
    }
    path_label(&label)
}

fn path_label(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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

struct AppError(anyhow::Error);

impl From<anyhow::Error> for AppError {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        eprintln!("syu app request failed: {:#}", self.0);
        (StatusCode::INTERNAL_SERVER_ERROR, "app data refresh failed").into_response()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{BTreeSet, hash_map::DefaultHasher},
        fs,
        hash::Hasher,
        io::{Error, ErrorKind, Read, Write as _},
        net::TcpListener,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
        thread,
        time::Duration,
    };

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use axum::{
        body::{Body, to_bytes},
        http::{HeaderValue, Request, StatusCode, header},
    };
    use tempfile::tempdir;
    use tower::util::ServiceExt;

    use crate::{
        cli::AppArgs,
        config::{SyuConfig, load_config},
    };

    use super::{
        AppPayload, AppServerSettings, AppState, AppVersion, SectionKind, Severity,
        SnapshotDependency, app_router, bind_failure_message, browser_root_labels,
        build_app_payload, canonical_workspace_root, collect_feature_sources,
        collect_snapshot_files_with_extensions, collect_yaml_sources_recursive,
        content_type_for_path, is_asset_like, load_current_snapshot, non_loopback_warning_lines,
        normalized_asset_path, normalized_trace_snapshot_path, readiness_probe_request_sent,
        readiness_probe_succeeds, redacted_relative_label, redacted_root_label,
        refresh_current_once, relative_display, require_remote_bind_opt_in,
        resolve_app_server_settings, spec_snapshot, startup_lines, trailing_path_components_label,
        validation_snapshot, wait_for_ready_with_retry,
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

    fn write_config(root: &Path, bind: &str, port: u16) {
        write_config_with(root, bind, port, false);
    }

    fn write_config_with(root: &Path, bind: &str, port: u16, require_symbol_trace_coverage: bool) {
        fs::write(
            root.join("syu.yaml"),
            format!(
                "version: {}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_symbol_trace_coverage: {require_symbol_trace_coverage}\napp:\n  bind: {bind}\n  port: {port}\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
                env!("CARGO_PKG_VERSION"),
            ),
        )
        .expect("config");
    }

    fn app_state(root: &Path) -> AppState {
        let config = load_config(root).expect("config should load").config;
        let state = AppState::new(root.to_path_buf(), config);
        let _ = state.refresh_current();
        state
    }

    fn failing_payload() -> anyhow::Result<AppPayload> {
        Err(anyhow::anyhow!("payload load failed"))
    }

    fn write_snapshot_workspace(root: &Path, require_symbol_trace_coverage: bool) {
        let spec_root = create_workspace_skeleton(root);
        write_config_with(root, "127.0.0.1", 3000, require_symbol_trace_coverage);
        fs::create_dir_all(root.join("src")).expect("src dir");
        fs::create_dir_all(root.join("tests")).expect("tests dir");

        fs::write(
            spec_root.join("philosophy/foundation.yaml"),
            "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Keep traces explicit\n    product_design_principle: Keep source, tests, and specs connected.\n    coding_guideline: Prefer explicit file links.\n    linked_policies:\n      - POL-001\n",
        )
        .expect("philosophy");
        fs::write(
            spec_root.join("policies/policies.yaml"),
            "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Requirements need evidence\n    summary: Keep links reciprocal.\n    description: Requirement and feature records should point to code and tests.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-001\n",
        )
        .expect("policy");
        fs::write(
            spec_root.join("requirements/core.yaml"),
            "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Requirement title\n    description: Requirement description.\n    priority: high\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n    tests:\n      rust:\n        - file: tests/requirement_trace.rs\n          symbols:\n            - requirement_trace\n",
        )
        .expect("requirement");
        fs::write(
            spec_root.join("features/features.yaml"),
            "version: \"0.0.1-alpha.8\"\nupdated: \"2026-04\"\n\nfiles:\n  - kind: core\n    file: core.yaml\n",
        )
        .expect("feature registry");
        fs::write(
            spec_root.join("features/core.yaml"),
            "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Feature title\n    summary: Feature summary.\n    status: implemented\n    linked_requirements:\n      - REQ-001\n    implementations:\n      rust:\n        - file: src/feature_impl.rs\n          symbols:\n            - feature_impl\n",
        )
        .expect("feature");
        fs::write(
            root.join("src/feature_impl.rs"),
            "pub fn feature_impl() {}\n",
        )
        .expect("feature source");
        fs::write(
            root.join("src/untracked.rs"),
            "pub fn coverage_target() {}\n",
        )
        .expect("coverage source");
        fs::write(
            root.join("tests/requirement_trace.rs"),
            "#[test]\nfn requirement_trace() {}\n",
        )
        .expect("requirement source");
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
    fn resolve_app_server_settings_uses_defaults_when_config_is_missing() {
        let config = SyuConfig::default();
        let settings = resolve_app_server_settings(
            &AppArgs {
                workspace: PathBuf::from("."),
                bind: None,
                port: None,
                allow_remote: false,
            },
            &config,
        );

        assert_eq!(
            settings,
            AppServerSettings {
                bind: "127.0.0.1".to_string(),
                port: 3000,
            }
        );
    }

    #[test]
    fn resolve_app_server_settings_uses_config_when_flags_are_absent() {
        let tempdir = tempdir().expect("tempdir should exist");
        create_workspace_skeleton(tempdir.path());
        write_config(tempdir.path(), "0.0.0.0", 4123);
        let config = load_config(tempdir.path())
            .expect("config should load")
            .config;

        let settings = resolve_app_server_settings(
            &AppArgs {
                workspace: tempdir.path().to_path_buf(),
                bind: None,
                port: None,
                allow_remote: false,
            },
            &config,
        );

        assert_eq!(
            settings,
            AppServerSettings {
                bind: "0.0.0.0".to_string(),
                port: 4123,
            }
        );
    }

    #[test]
    fn resolve_app_server_settings_prefers_cli_flags_over_config() {
        let tempdir = tempdir().expect("tempdir should exist");
        create_workspace_skeleton(tempdir.path());
        write_config(tempdir.path(), "0.0.0.0", 4123);
        let config = load_config(tempdir.path())
            .expect("config should load")
            .config;

        let settings = resolve_app_server_settings(
            &AppArgs {
                workspace: tempdir.path().to_path_buf(),
                bind: Some("127.0.0.1".to_string()),
                port: Some(5123),
                allow_remote: false,
            },
            &config,
        );

        assert_eq!(
            settings,
            AppServerSettings {
                bind: "127.0.0.1".to_string(),
                port: 5123,
            }
        );
    }

    #[test]
    fn canonical_workspace_root_discovers_parent_workspace_config() {
        let tempdir = tempdir().expect("tempdir should exist");
        create_workspace_skeleton(tempdir.path());
        write_config(tempdir.path(), "127.0.0.1", 3000);
        let nested = tempdir.path().join("frontend/nested");
        fs::create_dir_all(&nested).expect("nested dir");

        let workspace_root =
            canonical_workspace_root(&nested).expect("workspace root should resolve");
        assert_eq!(
            workspace_root,
            tempdir
                .path()
                .canonicalize()
                .expect("workspace should canonicalize")
        );
    }

    #[test]
    fn non_loopback_warning_lines_skip_loopback_addresses() {
        assert!(non_loopback_warning_lines("127.0.0.1".parse().expect("valid ip")).is_empty());
        assert!(non_loopback_warning_lines("::1".parse().expect("valid ip")).is_empty());
    }

    #[test]
    fn startup_lines_keep_ready_messages_on_stdout() {
        let lines = startup_lines("0.0.0.0:4123".parse().expect("valid socket"));
        assert_eq!(
            lines,
            vec![
                "syu app listening on http://0.0.0.0:4123".to_string(),
                "syu app ready: http://0.0.0.0:4123".to_string(),
                "Open http://0.0.0.0:4123 in your browser.".to_string(),
                "Press Ctrl-C to stop.".to_string(),
            ]
        );
    }

    #[test]
    fn non_loopback_warning_lines_emit_stdout_warnings_before_ready_messages() {
        let lines = non_loopback_warning_lines("0.0.0.0".parse().expect("valid ip"));
        assert_eq!(
            lines,
            vec![
                "warning: syu app is bound to 0.0.0.0, so workspace data and source documents may be reachable from other machines on your network."
                    .to_string(),
                "warning: use --bind 127.0.0.1 to keep the browser UI local to this machine."
                    .to_string(),
            ]
        );
    }

    #[test]
    fn require_remote_bind_opt_in_allows_loopback_without_flag() {
        require_remote_bind_opt_in("127.0.0.1".parse().expect("valid ip"), false)
            .expect("loopback should stay allowed");
    }

    #[test]
    fn require_remote_bind_opt_in_rejects_non_loopback_without_flag() {
        let error = require_remote_bind_opt_in("0.0.0.0".parse().expect("valid ip"), false)
            .expect_err("non-loopback should require explicit opt-in");
        let message = error.to_string();
        assert!(message.contains("--allow-remote"));
        assert!(message.contains("127.0.0.1"));
        assert!(message.contains("accidental network exposure"));
    }

    #[test]
    fn require_remote_bind_opt_in_allows_non_loopback_with_flag() {
        require_remote_bind_opt_in("0.0.0.0".parse().expect("valid ip"), true)
            .expect("explicit opt-in should allow non-loopback binds");
    }

    #[test]
    fn bind_failure_message_mentions_port_retries_for_addr_in_use() {
        let message = bind_failure_message(
            Path::new("tests/fixtures/workspaces/passing"),
            "127.0.0.1".parse().expect("valid ip"),
            3000,
            &Error::from(ErrorKind::AddrInUse),
        );

        assert!(message.contains("failed to bind `127.0.0.1:3000`"));
        assert!(message.contains("selected port is likely already in use"));
        assert!(message.contains("syu app tests/fixtures/workspaces/passing --port <free-port>"));
        assert!(message.contains("app.port"));
    }

    #[test]
    fn bind_failure_message_covers_non_addr_in_use_errors() {
        let message = bind_failure_message(
            Path::new("tests/fixtures/workspaces/passing"),
            "127.0.0.1".parse().expect("valid ip"),
            3000,
            &Error::from(ErrorKind::AddrNotAvailable),
        );

        assert!(message.contains("address or port may be unavailable on this machine"));
    }

    #[test]
    fn build_app_payload_collects_current_workspace_sources() {
        let payload = build_app_payload(&fixture_root("passing")).expect("payload should build");
        assert_eq!(
            payload.workspace_root,
            "./tests/fixtures/workspaces/passing"
        );
        assert_eq!(payload.spec_root, "docs/syu");
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
        assert_eq!(payload.validation.definition_counts.features, 6);
    }

    #[test]
    fn browser_root_labels_redact_external_spec_roots() {
        let workspace_root = Path::new("/tmp/workspace");
        let spec_root = Path::new("/tmp/shared-spec/docs/syu");
        let (workspace_label, spec_label) = browser_root_labels(workspace_root, spec_root);
        assert_eq!(workspace_label, "tmp/workspace");
        assert_eq!(spec_label, "external spec root");
    }

    #[test]
    fn redacted_root_label_uses_home_relative_paths() {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .expect("a home directory environment variable should exist for tests");
        let workspace_root = home.join("src/example/spec-workspace");
        assert_eq!(
            redacted_root_label(&workspace_root),
            "~/src/example/spec-workspace"
        );
    }

    #[cfg(unix)]
    #[test]
    fn redacted_root_label_falls_back_to_trailing_components() {
        assert_eq!(
            redacted_root_label(Path::new("/var/tmp/spec-workspace")),
            "tmp/spec-workspace"
        );
    }

    #[cfg(unix)]
    #[test]
    fn redacted_root_label_uses_workspace_for_root_paths() {
        assert_eq!(redacted_root_label(Path::new("/")), "workspace");
        assert!(trailing_path_components_label(Path::new("/"), 2).is_empty());
    }

    #[test]
    fn redacted_relative_label_returns_prefix_for_matching_roots() {
        let tempdir = tempdir().expect("tempdir should exist");
        assert_eq!(
            redacted_relative_label(Some(tempdir.path().to_path_buf()), tempdir.path(), ".")
                .as_deref(),
            Some(".")
        );
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

    #[test]
    fn wait_for_ready_reports_non_ready_servers() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
        let addr = listener.local_addr().expect("addr");

        let server = thread::spawn(move || {
            for _ in 0..3 {
                let (mut stream, _) = listener.accept().expect("request should connect");
                let mut buffer = [0_u8; 1024];
                let _ = stream.read(&mut buffer);
                let _ = stream.write_all(
                    b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 2\r\nConnection: close\r\n\r\nno",
                );
                let _ = stream.flush();
            }
        });

        let error =
            wait_for_ready_with_retry(addr, 3, Duration::from_millis(20), Duration::from_millis(1))
                .expect_err("non-ready servers should fail");
        assert!(
            !error.to_string().is_empty(),
            "errors should remain observable"
        );
        for _ in 0..3 {
            let _ = std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(20));
        }
        server.join().expect("server thread");
    }

    #[test]
    fn wait_for_ready_accepts_ok_health_responses() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
        let addr = listener.local_addr().expect("addr");

        listener
            .set_nonblocking(true)
            .expect("listener should become nonblocking");

        let server = thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(1);

            while std::time::Instant::now() < deadline {
                if let Ok((mut stream, _)) = listener.accept() {
                    let mut buffer = [0_u8; 1024];
                    let _ = stream.read(&mut buffer);
                    let _ = stream.write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"status\":\"ok\"}",
                    );
                    let _ = stream.flush();
                } else {
                    thread::sleep(Duration::from_millis(5));
                }
            }
        });

        wait_for_ready_with_retry(
            addr,
            5,
            Duration::from_millis(200),
            Duration::from_millis(5),
        )
        .expect("ready servers should succeed");
        server.join().expect("server thread");
    }

    #[test]
    fn wait_for_ready_reports_unreachable_servers() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
        let addr = listener.local_addr().expect("addr");
        drop(listener);

        let error =
            wait_for_ready_with_retry(addr, 2, Duration::from_millis(20), Duration::from_millis(1))
                .expect_err("missing servers should fail");
        assert!(error.to_string().contains("did not become ready"));
    }

    #[test]
    fn wait_for_ready_retries_when_peer_closes_immediately() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
        let addr = listener.local_addr().expect("addr");

        let server = thread::spawn(move || {
            for _ in 0..2 {
                if let Ok((stream, _)) = listener.accept() {
                    drop(stream);
                }
            }
        });

        let error =
            wait_for_ready_with_retry(addr, 2, Duration::from_millis(20), Duration::from_millis(1))
                .expect_err("immediate disconnects should fail");
        assert!(error.to_string().contains("did not become ready"));
        for _ in 0..2 {
            let _ = std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(20));
        }
        server.join().expect("server thread");
    }

    #[test]
    fn readiness_probe_returns_false_when_write_fails() {
        let mut stream = SyntheticProbeStream::new(ProbeFailure::Write);
        assert!(!readiness_probe_succeeds(
            &mut stream,
            "127.0.0.1:3000".parse().expect("socket address")
        ));
    }

    #[test]
    fn synthetic_probe_stream_allows_flush_when_write_fails_first() {
        let mut stream = SyntheticProbeStream::new(ProbeFailure::Write);
        stream
            .flush()
            .expect("write-failure mode should not fail on flush");
    }

    #[test]
    fn readiness_probe_returns_false_when_flush_fails() {
        let mut writer = SyntheticProbeStream::new(ProbeFailure::Flush);
        assert!(!readiness_probe_request_sent(
            &mut writer,
            "127.0.0.1:3000".parse().expect("socket address")
        ));
        assert!(
            writer.wrote,
            "probe should write the request before flush fails"
        );
    }

    #[test]
    fn readiness_probe_returns_true_for_ok_responses() {
        let mut stream = SyntheticProbeStream::ok();
        assert!(readiness_probe_succeeds(
            &mut stream,
            "127.0.0.1:3000".parse().expect("socket address")
        ));
        assert!(stream.wrote, "probe should send the readiness request");
    }

    #[tokio::test]
    async fn api_route_returns_payload_json() {
        let router = app_router(app_state(&fixture_root("passing")));

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
        assert!(
            response.headers().contains_key("x-syu-snapshot"),
            "payload responses should expose the snapshot header"
        );
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let json = String::from_utf8(body.to_vec()).expect("utf8");
        assert!(json.contains("\"workspace_root\""));
        assert!(json.contains("\"source_documents\""));
    }

    #[tokio::test]
    async fn api_version_changes_when_workspace_files_change() {
        let tempdir = tempdir().expect("tempdir should exist");
        let root = tempdir.path();
        create_workspace_skeleton(root);
        write_config(root, "127.0.0.1", 3000);
        fs::write(
            root.join("docs/syu/philosophy/foundation.yaml"),
            "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Original title\n    product_design_principle: Keep it small.\n    coding_guideline: Keep it explicit.\n    linked_policies: []\n",
        )
        .expect("philosophy");

        let state = app_state(root);
        let router = app_router(state.clone());

        let first = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/version")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(first.status(), StatusCode::OK);
        let first_body = to_bytes(first.into_body(), usize::MAX).await.expect("body");
        let first_version: AppVersion = serde_json::from_slice(&first_body).expect("json");

        fs::write(
            root.join("docs/syu/philosophy/foundation.yaml"),
            "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Updated title\n    product_design_principle: Keep it small.\n    coding_guideline: Keep it explicit.\n    linked_policies: []\n",
        )
        .expect("updated philosophy");
        state.refresh_current().expect("refresh should succeed");

        let second = router
            .oneshot(
                Request::builder()
                    .uri("/api/version")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let second_body = to_bytes(second.into_body(), usize::MAX)
            .await
            .expect("body");
        let second_version: AppVersion = serde_json::from_slice(&second_body).expect("json");

        assert_ne!(first_version.snapshot, second_version.snapshot);
    }

    #[test]
    fn spec_snapshot_changes_when_yaml_content_changes() {
        let tempdir = tempdir().expect("tempdir should exist");
        let spec_root = create_workspace_skeleton(tempdir.path());
        let philosophy = spec_root.join("philosophy/foundation.yaml");
        fs::write(
            &philosophy,
            "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Alpha title\n    product_design_principle: Keep it small.\n    coding_guideline: Keep it explicit.\n    linked_policies: []\n",
        )
        .expect("philosophy");

        let first = spec_snapshot(&spec_root).expect("snapshot");

        fs::write(
            &philosophy,
            "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Beta title\n    product_design_principle: Keep it small.\n    coding_guideline: Keep it explicit.\n    linked_policies: []\n",
        )
        .expect("updated philosophy");

        let second = spec_snapshot(&spec_root).expect("snapshot");
        assert_ne!(first, second);
    }

    #[test]
    fn spec_snapshot_tracks_policy_and_requirement_documents() {
        let tempdir = tempdir().expect("tempdir should exist");
        let spec_root = create_workspace_skeleton(tempdir.path());
        fs::write(
            spec_root.join("policies/policies.yaml"),
            "category: Policies\nversion: 1\npolicies:\n  - id: POL-001\n    title: Keep links reciprocal.\n    rationale: Enforce consistency.\n    requirement: Every requirement links back.\n    linked_philosophies: []\n    linked_requirements: []\n",
        )
        .expect("policy");
        fs::write(
            spec_root.join("requirements/core.yaml"),
            "category: Core\nversion: 1\nrequirements:\n  - id: REQ-001\n    title: Requirement title\n    statement: Requirement statement.\n    status: planned\n    linked_policies: []\n    linked_features: []\n",
        )
        .expect("requirement");

        let first = spec_snapshot(&spec_root).expect("snapshot");

        fs::write(
            spec_root.join("requirements/core.yaml"),
            "category: Core\nversion: 1\nrequirements:\n  - id: REQ-001\n    title: Updated requirement title\n    statement: Requirement statement.\n    status: planned\n    linked_policies: []\n    linked_features: []\n",
        )
        .expect("updated requirement");

        let second = spec_snapshot(&spec_root).expect("snapshot");
        assert_ne!(first, second);
    }

    #[test]
    fn load_current_snapshot_changes_when_traced_file_content_changes() {
        let tempdir = tempdir().expect("tempdir should exist");
        write_snapshot_workspace(tempdir.path(), false);
        let config = load_config(tempdir.path())
            .expect("config should load")
            .config;

        let first = load_current_snapshot(tempdir.path(), &config).expect("snapshot");

        fs::write(
            tempdir.path().join("tests/requirement_trace.rs"),
            "#[test]\nfn requirement_trace() { assert!(true); }\n",
        )
        .expect("updated trace");

        let second = load_current_snapshot(tempdir.path(), &config).expect("snapshot");
        assert_ne!(first, second);
    }

    #[test]
    fn load_current_snapshot_changes_when_symbol_coverage_inputs_change() {
        let tempdir = tempdir().expect("tempdir should exist");
        write_snapshot_workspace(tempdir.path(), true);
        let config = load_config(tempdir.path())
            .expect("config should load")
            .config;

        let first = load_current_snapshot(tempdir.path(), &config).expect("snapshot");

        fs::write(
            tempdir.path().join("src/untracked.rs"),
            "pub fn coverage_target() { println!(\"changed\"); }\n",
        )
        .expect("updated source");

        let second = load_current_snapshot(tempdir.path(), &config).expect("snapshot");
        assert_ne!(first, second);
    }

    #[test]
    fn normalized_trace_snapshot_path_rejects_non_relative_inputs() {
        assert_eq!(normalized_trace_snapshot_path(Path::new("")), None);
        assert_eq!(
            normalized_trace_snapshot_path(Path::new("../src/lib.rs")),
            None
        );
        assert_eq!(
            normalized_trace_snapshot_path(Path::new("/tmp/src/lib.rs")),
            None
        );
        assert_eq!(
            normalized_trace_snapshot_path(Path::new("src\\lib.rs")),
            Some(PathBuf::from("src/lib.rs"))
        );
    }

    #[test]
    fn collect_snapshot_files_with_extensions_skips_missing_dirs_and_unsupported_files() {
        let tempdir = tempdir().expect("tempdir should exist");
        let src_dir = tempdir.path().join("src/nested");
        fs::create_dir_all(&src_dir).expect("src dir");
        fs::write(src_dir.join("feature.rs"), "pub fn feature() {}\n").expect("rust source");
        fs::write(tempdir.path().join("src/README"), "not a tracked source").expect("readme");

        let mut dependencies = BTreeSet::new();
        collect_snapshot_files_with_extensions(
            tempdir.path(),
            &tempdir.path().join("missing"),
            &["rs"],
            &mut dependencies,
        );
        collect_snapshot_files_with_extensions(
            tempdir.path(),
            &tempdir.path().join("src"),
            &["rs"],
            &mut dependencies,
        );

        assert!(
            dependencies.contains(&SnapshotDependency::File(PathBuf::from(
                "src/nested/feature.rs",
            )))
        );
        assert!(!dependencies.contains(&SnapshotDependency::File(PathBuf::from("src/README",))));
    }

    #[cfg(unix)]
    #[test]
    fn collect_snapshot_files_with_extensions_records_unreadable_directories() {
        let tempdir = tempdir().expect("tempdir should exist");
        let locked_dir = tempdir.path().join("src/locked");
        fs::create_dir_all(&locked_dir).expect("locked dir");
        set_mode(&locked_dir, 0o000);

        let mut dependencies = BTreeSet::new();
        collect_snapshot_files_with_extensions(
            tempdir.path(),
            &locked_dir,
            &["rs"],
            &mut dependencies,
        );

        assert!(dependencies.contains(&SnapshotDependency::ReadDirError(
            PathBuf::from("src/locked"),
            "PermissionDenied".to_string(),
        )));

        set_mode(&locked_dir, 0o755);
    }

    #[cfg(unix)]
    #[test]
    fn snapshot_dependency_hash_state_distinguishes_unreadable_inputs() {
        let tempdir = tempdir().expect("tempdir should exist");
        let file = tempdir.path().join("locked.rs");
        fs::write(&file, "pub fn locked() {}\n").expect("locked file");
        set_mode(&file, 0o000);

        let mut file_hash = DefaultHasher::new();
        SnapshotDependency::File(PathBuf::from("locked.rs"))
            .hash_state(tempdir.path(), &mut file_hash);

        let mut dir_error_hash = DefaultHasher::new();
        SnapshotDependency::ReadDirError(PathBuf::from("src"), "PermissionDenied".to_string())
            .hash_state(tempdir.path(), &mut dir_error_hash);

        assert_ne!(
            file_hash.finish(),
            dir_error_hash.finish(),
            "different dependency kinds should contribute different snapshot hashes"
        );

        set_mode(&file, 0o644);
    }

    #[test]
    fn refresh_current_once_keeps_refresh_failures_non_fatal() {
        let tempdir = tempdir().expect("tempdir should exist");
        write_config(tempdir.path(), "127.0.0.1", 3000);
        let state = app_state(tempdir.path());

        refresh_current_once(&state);
    }

    #[test]
    fn refresh_current_skips_payload_reload_when_snapshot_is_unchanged() {
        let tempdir = tempdir().expect("tempdir should exist");
        write_config(tempdir.path(), "127.0.0.1", 3000);
        let state = AppState::new(
            tempdir.path().to_path_buf(),
            load_config(tempdir.path())
                .expect("config should load")
                .config,
        );
        let payload_loads = AtomicUsize::new(0);

        state
            .refresh_current_with(
                || Ok("same-snapshot".to_string()),
                || {
                    payload_loads.fetch_add(1, Ordering::SeqCst);
                    Ok(AppPayload::default())
                },
            )
            .expect("initial refresh should succeed");
        state
            .refresh_current_with(|| Ok("same-snapshot".to_string()), failing_payload)
            .expect("unchanged snapshots should not rebuild the payload");

        assert_eq!(
            payload_loads.load(Ordering::SeqCst),
            1,
            "payloads should only load once when the snapshot does not change"
        );
        assert_eq!(
            state.current_data().expect("ready state").snapshot,
            "same-snapshot",
            "unchanged refreshes should preserve the last ready snapshot"
        );
    }

    #[test]
    fn refresh_current_enters_error_state_when_payload_reload_fails() {
        let tempdir = tempdir().expect("tempdir should exist");
        write_config(tempdir.path(), "127.0.0.1", 3000);
        let state = AppState::new(
            tempdir.path().to_path_buf(),
            load_config(tempdir.path())
                .expect("config should load")
                .config,
        );

        let error = state
            .refresh_current_with(|| Ok("updated-snapshot".to_string()), failing_payload)
            .expect_err("payload failures should propagate");

        assert!(
            error.to_string().contains("payload load failed"),
            "error should preserve the payload loader failure: {error:#}"
        );
        assert!(
            state
                .current_data()
                .expect_err("failed payload reloads should leave the app in an error state")
                .to_string()
                .contains("payload load failed")
        );
    }

    #[tokio::test]
    async fn api_errors_hide_internal_details_from_clients() {
        let tempdir = tempdir().expect("tempdir should exist");
        write_config(tempdir.path(), "127.0.0.1", 3000);
        let router = app_router(app_state(tempdir.path()));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/api/version")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        assert_eq!(
            String::from_utf8(body.to_vec()).expect("utf8"),
            "app data refresh failed"
        );
    }

    #[tokio::test]
    async fn api_payload_refreshes_when_workspace_files_change() {
        let tempdir = tempdir().expect("tempdir should exist");
        let root = tempdir.path();
        create_workspace_skeleton(root);
        write_config(root, "127.0.0.1", 3000);
        fs::write(
            root.join("docs/syu/philosophy/foundation.yaml"),
            "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Original title\n    product_design_principle: Keep it small.\n    coding_guideline: Keep it explicit.\n    linked_policies: []\n",
        )
        .expect("philosophy");

        let state = app_state(root);
        let router = app_router(state.clone());

        let first = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/app-data.json")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let first_body = to_bytes(first.into_body(), usize::MAX).await.expect("body");
        let first_json = String::from_utf8(first_body.to_vec()).expect("utf8");

        fs::write(
            root.join("docs/syu/philosophy/foundation.yaml"),
            "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Updated title\n    product_design_principle: Keep it small.\n    coding_guideline: Keep it explicit.\n    linked_policies: []\n",
        )
        .expect("updated philosophy");
        state.refresh_current().expect("refresh should succeed");

        let second = router
            .oneshot(
                Request::builder()
                    .uri("/api/app-data.json")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let second_body = to_bytes(second.into_body(), usize::MAX)
            .await
            .expect("body");
        let second_json = String::from_utf8(second_body.to_vec()).expect("utf8");

        assert_ne!(first_json, second_json);
        assert!(second_json.contains("Updated title"));
    }

    #[tokio::test]
    async fn api_like_paths_return_not_found() {
        let router = app_router(app_state(&fixture_root("passing")));

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
        let router = app_router(app_state(&fixture_root("passing")));

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
        let router = app_router(app_state(&fixture_root("passing")));

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

    enum ProbeFailure {
        Write,
        Flush,
    }

    struct SyntheticProbeStream {
        fail_on: Option<ProbeFailure>,
        response: &'static [u8],
        wrote: bool,
    }

    impl SyntheticProbeStream {
        fn new(fail_on: ProbeFailure) -> Self {
            Self {
                fail_on: Some(fail_on),
                response: b"",
                wrote: false,
            }
        }

        fn ok() -> Self {
            Self {
                fail_on: None,
                response: b"HTTP/1.1 200 OK\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"status\":\"ok\"}",
                wrote: false,
            }
        }
    }

    impl Read for SyntheticProbeStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.response.is_empty() {
                return Ok(0);
            }

            let len = self.response.len().min(buf.len());
            buf[..len].copy_from_slice(&self.response[..len]);
            self.response = &self.response[len..];
            Ok(len)
        }
    }

    impl std::io::Write for SyntheticProbeStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            match self.fail_on {
                Some(ProbeFailure::Write) => {
                    Err(Error::new(ErrorKind::BrokenPipe, "synthetic write failure"))
                }
                Some(ProbeFailure::Flush) | None => {
                    self.wrote = true;
                    Ok(buf.len())
                }
            }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            match self.fail_on {
                Some(ProbeFailure::Flush) => {
                    Err(Error::new(ErrorKind::BrokenPipe, "synthetic flush failure"))
                }
                Some(ProbeFailure::Write) | None => Ok(()),
            }
        }
    }
}
