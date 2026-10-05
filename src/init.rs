//! Bounded workspace bootstrap (`mitase init`).
//!
//! This module owns the single narrow exception to the workspace-mutation
//! ban: it may create missing Mitase-owned bootstrap metadata (`mitase.yaml`
//! and the spec-root marker). It must never author normative specification
//! meaning, implementation artifacts, or verification artifacts.
//!
//! Planning is split from writing on purpose. [`plan_init`] resolves the
//! workspace, validates the requested spec root, and checks every conflict
//! without touching the filesystem. [`apply_init`] only executes a plan.

use anyhow::{Context, Result, bail};
use mitase_project_model::{CONFIG_SCHEMA, DEFAULT_SPEC_ROOT, EffectiveProjectConfig};
use mitase_spec_model::RepoPath;
use serde::Serialize;
use std::{
    fmt::Display,
    fs,
    path::{Path, PathBuf},
};

pub const CONFIG_FILE_NAME: &str = "mitase.yaml";
pub const GITKEEP_FILE_NAME: &str = ".gitkeep";

const MINIMAL_CONFIG_SOURCE: &str = "schema: mitase/config/v1\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InitStatus {
    Initialized,
    AlreadyInitialized,
    Planned,
}

impl Display for InitStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = match self {
            InitStatus::Initialized => "initialized",
            InitStatus::AlreadyInitialized => "already_initialized",
            InitStatus::Planned => "planned",
        };
        formatter.write_str(status)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct InitReport {
    pub status: InitStatus,
    pub workspace: String,
    pub spec_root: String,
    pub created: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct InitPlan {
    workspace_root: PathBuf,
    requested_root: RepoPath,
    write_config: bool,
    config_source: String,
    marker_roots: Vec<RepoPath>,
    status: InitStatus,
    dry_run: bool,
}

impl InitPlan {
    pub fn report(&self) -> InitReport {
        let mut created = Vec::new();
        if self.write_config {
            created.push(CONFIG_FILE_NAME.to_owned());
        }
        created.extend(marker_display_paths(&self.marker_roots));
        InitReport {
            status: self.status,
            workspace: self.workspace_root.to_string_lossy().into_owned(),
            spec_root: self.requested_root.to_string_lossy().into_owned(),
            created,
        }
    }
}

fn marker_display_paths(roots: &[RepoPath]) -> Vec<String> {
    roots
        .iter()
        .map(|root| {
            format!(
                "{}/{GITKEEP_FILE_NAME}",
                root.to_string_lossy().replace('\\', "/")
            )
        })
        .collect()
}

/// Resolve the bootstrap plan without changing the filesystem.
pub fn plan_init(workspace: &Path, spec_root: Option<&Path>, dry_run: bool) -> Result<InitPlan> {
    let workspace_root = workspace
        .canonicalize()
        .with_context(|| format!("resolve workspace {}", workspace.display()))?;
    if !workspace_root.is_dir() {
        bail!("workspace {} is not a directory", workspace.display());
    }
    let requested = match spec_root {
        Some(root) => RepoPath::new(root)
            .map_err(|error| anyhow::anyhow!("invalid --spec-root {}: {error}", root.display()))?,
        None => RepoPath::new(DEFAULT_SPEC_ROOT).map_err(|error| {
            anyhow::anyhow!("invalid default spec root {DEFAULT_SPEC_ROOT}: {error}")
        })?,
    };
    reject_workspace_escape(&workspace_root, &requested)?;

    let config_path = workspace_root.join(CONFIG_FILE_NAME);
    let existing = if config_path.is_file() {
        Some(read_existing_config(&workspace_root, &config_path)?)
    } else if config_path.exists() {
        bail!("{CONFIG_FILE_NAME} exists but is not a file");
    } else {
        None
    };

    let (write_config, config_source, ensure_roots, requested_root) = match existing {
        None => {
            let source = config_source_for(&requested);
            (true, source, vec![requested.clone()], requested.clone())
        }
        Some(config) => {
            let configured = config.workspace.spec_roots.clone();
            if spec_root.is_some() {
                if !configured.contains(&requested) {
                    bail!(
                        "workspace is already initialized with a different spec root; edit {CONFIG_FILE_NAME} explicitly"
                    );
                }
                (
                    false,
                    String::new(),
                    vec![requested.clone()],
                    requested.clone(),
                )
            } else if configured.is_empty() {
                (
                    false,
                    String::new(),
                    vec![requested.clone()],
                    requested.clone(),
                )
            } else {
                let primary = configured
                    .first()
                    .cloned()
                    .unwrap_or_else(|| requested.clone());
                (false, String::new(), configured, primary)
            }
        }
    };

    let marker_roots = ensure_roots
        .iter()
        .filter(|root| marker_is_missing(&workspace_root, root))
        .cloned()
        .collect::<Vec<_>>();
    let status = if dry_run {
        InitStatus::Planned
    } else if write_config || !marker_roots.is_empty() {
        InitStatus::Initialized
    } else {
        InitStatus::AlreadyInitialized
    };
    Ok(InitPlan {
        workspace_root,
        requested_root,
        write_config,
        config_source,
        marker_roots,
        status,
        dry_run,
    })
}

/// Execute a plan. A dry-run plan is accepted and writes nothing.
pub fn apply_init(plan: &InitPlan) -> Result<()> {
    if plan.dry_run {
        return Ok(());
    }
    for root in &plan.marker_roots {
        let directory = plan.workspace_root.join(root.as_path());
        if !directory.exists() {
            fs::create_dir_all(&directory)
                .with_context(|| format!("create {}", directory.display()))?;
        }
        if dir_is_empty(&directory)? {
            fs::write(directory.join(GITKEEP_FILE_NAME), "")
                .with_context(|| format!("write {}", directory.display()))?;
        }
    }
    if plan.write_config {
        write_config_atomically(&plan.workspace_root, &plan.config_source)?;
    }
    Ok(())
}

fn config_source_for(requested: &RepoPath) -> String {
    if requested.as_path() == Path::new(DEFAULT_SPEC_ROOT) {
        MINIMAL_CONFIG_SOURCE.to_owned()
    } else {
        format!(
            "schema: {CONFIG_SCHEMA}\nworkspace:\n  spec_roots: [{}]\n",
            requested.to_string_lossy().replace('\\', "/")
        )
    }
}

fn read_existing_config(root: &Path, config_path: &Path) -> Result<EffectiveProjectConfig> {
    let source = fs::read_to_string(config_path)
        .with_context(|| format!("read {}", config_path.display()))?;
    EffectiveProjectConfig::from_source(root, &source)
        .map_err(|error| anyhow::anyhow!("invalid {CONFIG_FILE_NAME}: {error}"))
}

fn reject_workspace_escape(workspace_root: &Path, requested: &RepoPath) -> Result<()> {
    let joined = workspace_root.join(requested.as_path());
    let mut probe = joined.as_path();
    loop {
        if fs::symlink_metadata(probe).is_ok() {
            let canonical = probe
                .canonicalize()
                .with_context(|| format!("resolve {}", probe.display()))?;
            if !canonical.starts_with(workspace_root) {
                bail!(
                    "--spec-root {} escapes the workspace",
                    requested.to_string_lossy()
                );
            }
            return Ok(());
        }
        match probe.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => probe = parent,
            _ => bail!(
                "--spec-root {} escapes the workspace",
                requested.to_string_lossy()
            ),
        }
    }
}

fn marker_is_missing(workspace_root: &Path, root: &RepoPath) -> bool {
    let directory = workspace_root.join(root.as_path());
    if !directory.exists() {
        return true;
    }
    dir_is_empty(&directory).unwrap_or(false)
}

fn dir_is_empty(directory: &Path) -> Result<bool> {
    Ok(fs::read_dir(directory)
        .with_context(|| format!("read {}", directory.display()))?
        .next()
        .is_none())
}

fn write_config_atomically(root: &Path, source: &str) -> Result<()> {
    use std::io::Write as _;
    let config_path = root.join(CONFIG_FILE_NAME);
    let mut temp = tempfile::NamedTempFile::new_in(root).context("create temporary mitase.yaml")?;
    temp.write_all(source.as_bytes())
        .context("write temporary mitase.yaml")?;
    temp.persist_noclobber(&config_path)
        .with_context(|| format!("create {}", config_path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mitase_project_model::EffectiveProjectConfig;
    use tempfile::tempdir;

    fn workspace_with(marker: &str) -> tempfile::TempDir {
        let temp = tempdir().expect("temp workspace");
        fs::create_dir_all(temp.path().join(marker)).expect("spec root");
        temp
    }

    #[test]
    fn default_plan_creates_minimal_config_and_marker() {
        let temp = tempdir().expect("temp workspace");
        let plan = plan_init(temp.path(), None, false).expect("init plan");
        let report = plan.report();
        assert_eq!(report.status, InitStatus::Initialized);
        assert_eq!(report.spec_root, DEFAULT_SPEC_ROOT);
        assert_eq!(
            report.created,
            vec![
                CONFIG_FILE_NAME.to_owned(),
                format!("{DEFAULT_SPEC_ROOT}/{GITKEEP_FILE_NAME}"),
            ]
        );
        apply_init(&plan).expect("apply init");
        assert_eq!(
            fs::read_to_string(temp.path().join(CONFIG_FILE_NAME)).expect("config"),
            MINIMAL_CONFIG_SOURCE,
        );
        assert!(
            temp.path()
                .join(DEFAULT_SPEC_ROOT)
                .join(GITKEEP_FILE_NAME)
                .is_file()
        );
        let generated = fs::read_to_string(temp.path().join(CONFIG_FILE_NAME)).expect("config");
        EffectiveProjectConfig::from_source(temp.path(), &generated)
            .expect("generated config parses");
    }

    #[test]
    fn custom_spec_root_plan_writes_explicit_config() {
        let temp = tempdir().expect("temp workspace");
        let plan =
            plan_init(temp.path(), Some(Path::new("spec/contracts")), false).expect("init plan");
        let report = plan.report();
        assert_eq!(report.spec_root, "spec/contracts");
        assert_eq!(
            report.created,
            vec![
                CONFIG_FILE_NAME.to_owned(),
                format!("spec/contracts/{GITKEEP_FILE_NAME}"),
            ]
        );
        apply_init(&plan).expect("apply init");
        let generated = fs::read_to_string(temp.path().join(CONFIG_FILE_NAME)).expect("config");
        assert!(generated.contains("spec_roots: [spec/contracts]"));
        let effective = EffectiveProjectConfig::from_source(temp.path(), &generated)
            .expect("generated config parses");
        assert_eq!(
            effective.workspace.spec_roots,
            vec![RepoPath::new("spec/contracts").expect("spec root")]
        );
        assert!(
            temp.path()
                .join("spec/contracts")
                .join(GITKEEP_FILE_NAME)
                .is_file()
        );
    }

    #[test]
    fn absolute_spec_root_is_rejected() {
        let temp = tempdir().expect("temp workspace");
        let error = plan_init(temp.path(), Some(Path::new("/absolute/path")), false)
            .expect_err("absolute spec root");
        assert!(error.to_string().contains("invalid --spec-root"));
        assert!(!temp.path().join(CONFIG_FILE_NAME).exists());
    }

    #[test]
    fn parent_escape_spec_root_is_rejected() {
        let temp = tempdir().expect("temp workspace");
        for escape in ["../other-repo", "docs/../../other-repo"] {
            let error = plan_init(temp.path(), Some(Path::new(escape)), false)
                .expect_err("escaped spec root");
            assert!(
                error.to_string().contains("invalid --spec-root"),
                "unexpected error: {error}"
            );
        }
        assert!(!temp.path().join(CONFIG_FILE_NAME).exists());
    }

    #[test]
    #[cfg(unix)]
    fn symlink_escape_spec_root_is_rejected() {
        let temp = tempdir().expect("temp workspace");
        let outside = tempdir().expect("outside directory");
        std::os::unix::fs::symlink(outside.path(), temp.path().join("linked")).expect("symlink");
        let error = plan_init(temp.path(), Some(Path::new("linked/spec")), false)
            .expect_err("symlinked spec root");
        assert!(error.to_string().contains("escapes the workspace"));
        assert!(!temp.path().join(CONFIG_FILE_NAME).exists());
    }

    #[test]
    fn existing_valid_config_is_already_initialized() {
        let temp = workspace_with(DEFAULT_SPEC_ROOT);
        fs::write(
            temp.path().join(DEFAULT_SPEC_ROOT).join(GITKEEP_FILE_NAME),
            "",
        )
        .expect("marker");
        fs::write(temp.path().join(CONFIG_FILE_NAME), MINIMAL_CONFIG_SOURCE).expect("config");
        let plan = plan_init(temp.path(), None, false).expect("init plan");
        let report = plan.report();
        assert_eq!(report.status, InitStatus::AlreadyInitialized);
        assert!(report.created.is_empty());
        apply_init(&plan).expect("apply init");
        assert_eq!(
            fs::read_to_string(temp.path().join(CONFIG_FILE_NAME)).expect("config"),
            MINIMAL_CONFIG_SOURCE,
        );
    }

    #[test]
    fn existing_invalid_config_is_never_replaced() {
        let temp = workspace_with(DEFAULT_SPEC_ROOT);
        fs::write(
            temp.path().join(CONFIG_FILE_NAME),
            "schema: mitase/config/v1\nversion: 1\n",
        )
        .expect("config");
        let before = fs::read(temp.path().join(CONFIG_FILE_NAME)).expect("config bytes");
        let error = plan_init(temp.path(), None, false).expect_err("invalid config");
        assert!(error.to_string().contains(CONFIG_FILE_NAME));
        assert_eq!(
            fs::read(temp.path().join(CONFIG_FILE_NAME)).expect("config bytes"),
            before
        );
    }

    #[test]
    fn explicit_spec_root_must_match_existing_config() {
        let temp = workspace_with("spec/contracts");
        fs::write(
            temp.path().join(CONFIG_FILE_NAME),
            "schema: mitase/config/v1\nworkspace:\n  spec_roots: [spec/contracts]\n",
        )
        .expect("config");
        fs::write(
            temp.path().join("spec/contracts").join(GITKEEP_FILE_NAME),
            "",
        )
        .expect("marker");
        let matching = plan_init(temp.path(), Some(Path::new("spec/contracts")), false)
            .expect("matching spec root");
        assert_eq!(matching.report().status, InitStatus::AlreadyInitialized);
        assert!(matching.report().created.is_empty());
        let error = plan_init(temp.path(), Some(Path::new("other/root")), false)
            .expect_err("different spec root");
        assert!(error.to_string().contains("different spec root"));
    }

    #[test]
    fn missing_marker_is_repaired_without_touching_config() {
        let temp = workspace_with("spec/contracts");
        let source = "schema: mitase/config/v1\nworkspace:\n  spec_roots: [spec/contracts]\n";
        fs::write(temp.path().join(CONFIG_FILE_NAME), source).expect("config");
        let plan = plan_init(temp.path(), None, false).expect("init plan");
        let report = plan.report();
        assert_eq!(report.status, InitStatus::Initialized);
        assert_eq!(report.created, vec!["spec/contracts/.gitkeep".to_owned()]);
        apply_init(&plan).expect("apply init");
        assert_eq!(
            fs::read_to_string(temp.path().join(CONFIG_FILE_NAME)).expect("config"),
            source
        );
        assert!(
            temp.path()
                .join("spec/contracts")
                .join(GITKEEP_FILE_NAME)
                .is_file()
        );
    }

    #[test]
    fn nonempty_spec_root_is_left_alone() {
        let temp = workspace_with(DEFAULT_SPEC_ROOT);
        fs::write(
            temp.path().join(DEFAULT_SPEC_ROOT).join("requirement.yaml"),
            "meaning\n",
        )
        .expect("spec document");
        fs::write(temp.path().join(CONFIG_FILE_NAME), MINIMAL_CONFIG_SOURCE).expect("config");
        let plan = plan_init(temp.path(), None, false).expect("init plan");
        assert_eq!(plan.report().status, InitStatus::AlreadyInitialized);
        assert!(plan.report().created.is_empty());
        apply_init(&plan).expect("apply init");
        assert!(
            !temp
                .path()
                .join(DEFAULT_SPEC_ROOT)
                .join(GITKEEP_FILE_NAME)
                .exists()
        );
    }

    #[test]
    fn identical_inputs_produce_identical_reports() {
        let temp = tempdir().expect("temp workspace");
        let first = plan_init(temp.path(), Some(Path::new("spec/contracts")), true)
            .expect("first plan")
            .report();
        let second = plan_init(temp.path(), Some(Path::new("spec/contracts")), true)
            .expect("second plan")
            .report();
        assert_eq!(
            serde_json::to_string(&first).expect("first report"),
            serde_json::to_string(&second).expect("second report"),
        );
    }

    #[test]
    fn dry_run_leaves_the_workspace_untouched() {
        let temp = tempdir().expect("temp workspace");
        let plan = plan_init(temp.path(), None, true).expect("init plan");
        assert_eq!(plan.report().status, InitStatus::Planned);
        apply_init(&plan).expect("dry run applies nothing");
        assert!(!temp.path().join(CONFIG_FILE_NAME).exists());
        assert!(!temp.path().join(DEFAULT_SPEC_ROOT).exists());
    }

    #[test]
    fn apply_is_idempotent() {
        let temp = tempdir().expect("temp workspace");
        let plan = plan_init(temp.path(), None, false).expect("init plan");
        apply_init(&plan).expect("apply init");
        let again = plan_init(temp.path(), None, false).expect("second plan");
        assert_eq!(again.report().status, InitStatus::AlreadyInitialized);
        assert!(again.report().created.is_empty());
    }
}
