// REQ-CORE-009
// FEAT-INIT-007
// FEAT-INIT-005
// FEAT-INIT-004
// FEAT-INIT-003
// FEAT-INIT-002

use anyhow::{Result, bail};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use crate::{
    cli::{InitArgs, OutputFormat, StarterTemplate},
    command::shell_quote_path,
    config::{SyuConfig, render_config},
    coverage::normalize_relative_path,
};

use super::prompt::{
    PromptIo, StdioPromptIo, ensure_prompt_terminal, prompt_bool, prompt_optional_with_default,
    prompt_with_default,
};

const DEFAULT_SPEC_ROOT: &str = "docs/syu";
const INIT_INTERACTIVE_NON_TTY_MESSAGE: &str =
    "`syu init --interactive` requires a terminal to prompt for starter settings";
const INIT_INTERACTIVE_JSON_MESSAGE: &str =
    "`syu init --interactive` does not support `--format json`; use text output instead";

#[cfg(test)]
const GENERATED_PATHS: &[&str] = &[
    "syu.yaml",
    "docs/syu/philosophy/foundation.yaml",
    "docs/syu/policies/policies.yaml",
    "docs/syu/requirements/core/core.yaml",
    "docs/syu/features/features.yaml",
    "docs/syu/features/core/core.yaml",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct StarterIdPrefixes {
    philosophy: String,
    policy: String,
    requirement: String,
    feature: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedInitOptions {
    project_name: String,
    spec_root: PathBuf,
    template: StarterTemplate,
    id_prefixes: StarterIdPrefixes,
    strict_validate_defaults: bool,
}

impl StarterIdPrefixes {
    fn philosophy_id(&self) -> String {
        format!("{}-001", self.philosophy)
    }

    fn policy_id(&self) -> String {
        format!("{}-001", self.policy)
    }

    fn requirement_id(&self) -> String {
        format!("{}-001", self.requirement)
    }

    fn feature_id(&self) -> String {
        format!("{}-001", self.feature)
    }
}

// FEAT-INIT-001
pub fn run_init_command(args: &InitArgs) -> Result<i32> {
    let mut prompt_io = StdioPromptIo;
    run_init_command_with_prompt_io(args, &mut prompt_io)
}

fn run_init_command_with_prompt_io(args: &InitArgs, prompt_io: &mut impl PromptIo) -> Result<i32> {
    ensure_workspace_path_can_be_created(&args.workspace)?;
    validate_interactive_init_mode(args, prompt_io)?;
    let resolved = resolve_init_options_with_prompt_io(args, &args.workspace, prompt_io)?;
    let workspace = prepare_workspace_root(&args.workspace)?;

    let files = scaffold_files(
        &resolved.project_name,
        &resolved.spec_root,
        resolved.template,
        &resolved.id_prefixes,
        resolved.strict_validate_defaults,
    );
    ensure_writable_targets(
        &workspace,
        files.iter().map(|(path, _)| PathBuf::from(path)),
        args.force,
    )?;

    for (relative_path, contents) in &files {
        let full_path = workspace.join(relative_path);
        let parent = full_path
            .parent()
            .expect("generated scaffold paths should always have a parent");
        fs::create_dir_all(parent)?;
        fs::write(full_path, contents)?;
    }

    let created_paths: Vec<&str> = files.iter().map(|(path, _)| path.as_str()).collect();

    match args.format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::json!({
                    "workspace": workspace.display().to_string(),
                    "created_files": created_paths,
                })
            );
        }
        OutputFormat::Text => {
            let workspace_arg = shell_quote_path(&workspace);
            let absolute_spec_root = workspace.join(&resolved.spec_root);
            let philosophy_path = resolved.spec_root.join("philosophy/foundation.yaml");
            let policy_path = resolved.spec_root.join("policies/policies.yaml");
            let requirement_path = resolved
                .spec_root
                .join(requirement_document_path(resolved.template));
            let feature_path = resolved
                .spec_root
                .join(feature_document_path(resolved.template));
            println!("initialized syu workspace at {}", workspace.display());
            println!();
            println!("Created files:");
            for path in &created_paths {
                println!("  {path}");
            }
            println!();
            println!("What to do next:");
            println!(
                "  1. Edit the spec files in {}/ to describe your project",
                absolute_spec_root.display()
            );
            println!("     - {}  (core principles)", path_label(&philosophy_path));
            println!("     - {}  (governance rules)", path_label(&policy_path));
            println!(
                "     - {}  (concrete requirements)",
                path_label(&requirement_path)
            );
            println!(
                "     - {}  (feature definitions)",
                path_label(&feature_path)
            );
            println!("  2. Run `syu validate {workspace_arg}` to check your spec for consistency");
            println!(
                "  3. Run `syu browse {workspace_arg}` for terminal exploration, or `syu app {workspace_arg}` for the browser UI"
            );
            println!("  4. Commit the generated files to version control");
            println!(
                "  Need a different starter next time? Run `syu templates` before another `syu init`."
            );
        }
    }

    Ok(0)
}

fn resolve_init_options_with_prompt_io(
    args: &InitArgs,
    workspace: &Path,
    prompt_io: &mut impl PromptIo,
) -> Result<ResolvedInitOptions> {
    if !args.interactive {
        let project_name = args
            .name
            .clone()
            .unwrap_or_else(|| infer_project_name(workspace));
        let spec_root = resolve_init_spec_root(args.spec_root.as_deref())?;
        let template = args.template;
        let id_prefixes = resolve_init_id_prefixes(
            template,
            args.id_prefix.as_deref(),
            args.philosophy_prefix.as_deref(),
            args.policy_prefix.as_deref(),
            args.requirement_prefix.as_deref(),
            args.feature_prefix.as_deref(),
        )?;
        return Ok(ResolvedInitOptions {
            project_name,
            spec_root,
            template,
            id_prefixes,
            strict_validate_defaults: false,
        });
    }

    let default_project_name = args
        .name
        .clone()
        .unwrap_or_else(|| infer_project_name(workspace));
    let project_name = prompt_with_default(prompt_io, "Project name", &default_project_name)?;
    let template = prompt_for_starter_template(prompt_io, args.template)?;
    let spec_root = prompt_for_spec_root(prompt_io, args.spec_root.as_deref())?;
    let id_prefixes = resolve_interactive_id_prefixes(args, template, prompt_io)?;
    let strict_validate_defaults =
        prompt_bool(prompt_io, "Enable stricter validation defaults now?", false)?;

    Ok(ResolvedInitOptions {
        project_name,
        spec_root,
        template,
        id_prefixes,
        strict_validate_defaults,
    })
}

fn validate_interactive_init_mode(args: &InitArgs, prompt_io: &impl PromptIo) -> Result<()> {
    if !args.interactive {
        return Ok(());
    }

    ensure_prompt_terminal(prompt_io, INIT_INTERACTIVE_NON_TTY_MESSAGE)?;
    if args.format == OutputFormat::Json {
        bail!(INIT_INTERACTIVE_JSON_MESSAGE);
    }

    Ok(())
}

fn prompt_for_starter_template(
    prompt_io: &mut impl PromptIo,
    default: StarterTemplate,
) -> Result<StarterTemplate> {
    let template_choices = "generic|rust-only|python-only|polyglot";
    let label = format!("Starter template ({template_choices})");
    loop {
        let raw = prompt_with_default(prompt_io, &label, default.label())?;
        match parse_starter_template_prompt(&raw) {
            Some(template) => return Ok(template),
            None => eprintln!(
                "Starter template must be one of: generic, rust-only, python-only, polyglot."
            ),
        }
    }
}

fn parse_starter_template_prompt(raw: &str) -> Option<StarterTemplate> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "generic" | "1" => Some(StarterTemplate::Generic),
        "rust" | "rust-only" | "2" => Some(StarterTemplate::RustOnly),
        "python" | "python-only" | "3" => Some(StarterTemplate::PythonOnly),
        "polyglot" | "mixed" | "4" => Some(StarterTemplate::Polyglot),
        _ => None,
    }
}

fn prompt_for_spec_root(prompt_io: &mut impl PromptIo, current: Option<&Path>) -> Result<PathBuf> {
    let default_spec_root = current
        .unwrap_or_else(|| Path::new(DEFAULT_SPEC_ROOT))
        .display()
        .to_string();
    loop {
        let raw = prompt_with_default(prompt_io, "Spec root", &default_spec_root)?;
        match resolve_init_spec_root(Some(Path::new(raw.as_str()))) {
            Ok(spec_root) => return Ok(spec_root),
            Err(error) => eprintln!("{error:#}"),
        }
    }
}

fn ensure_workspace_path_can_be_created(path: &Path) -> Result<()> {
    if path.exists() && !path.is_dir() {
        bail!("workspace path `{}` is not a directory", path.display());
    }

    Ok(())
}

fn prepare_workspace_root(path: &Path) -> Result<PathBuf> {
    ensure_workspace_path_can_be_created(path)?;
    fs::create_dir_all(path)?;
    path.canonicalize().map_err(Into::into)
}

fn resolve_init_spec_root(spec_root: Option<&Path>) -> Result<PathBuf> {
    let raw = spec_root.unwrap_or_else(|| Path::new(DEFAULT_SPEC_ROOT));
    let normalized = normalize_relative_path(raw);

    if normalized.as_os_str().is_empty()
        || normalized.is_absolute()
        || normalized
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!(
            "`--spec-root` must stay within the workspace root as a relative path: `{}`",
            raw.display()
        );
    }

    Ok(PathBuf::from(path_label(&normalized)))
}

fn infer_project_name(workspace: &Path) -> String {
    workspace
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("project")
        .to_string()
}

fn ensure_writable_targets(
    workspace: &Path,
    targets: impl Iterator<Item = PathBuf>,
    force: bool,
) -> Result<()> {
    if force {
        return Ok(());
    }

    let existing: Vec<_> = targets
        .map(|target| workspace.join(target))
        .filter(|path| path.exists())
        .collect();

    if existing.is_empty() {
        return Ok(());
    }

    let paths = existing
        .iter()
        .map(|path| format!("`{}`", path.display()))
        .collect::<Vec<_>>()
        .join(", ");
    bail!("refusing to overwrite existing files without --force: {paths}");
}

fn resolve_init_id_prefixes(
    template: StarterTemplate,
    id_prefix: Option<&str>,
    philosophy_prefix: Option<&str>,
    policy_prefix: Option<&str>,
    requirement_prefix: Option<&str>,
    feature_prefix: Option<&str>,
) -> Result<StarterIdPrefixes> {
    let mut prefixes = if let Some(stem) = id_prefix {
        let stem = normalize_shared_id_stem(stem)?;
        StarterIdPrefixes {
            philosophy: format!("PHIL-{stem}"),
            policy: format!("POL-{stem}"),
            requirement: format!("REQ-{stem}"),
            feature: format!("FEAT-{stem}"),
        }
    } else {
        default_id_prefixes(template)
    };

    if let Some(prefix) = philosophy_prefix {
        prefixes.philosophy = normalize_typed_prefix(prefix, "PHIL", "--philosophy-prefix")?;
    }
    if let Some(prefix) = policy_prefix {
        prefixes.policy = normalize_typed_prefix(prefix, "POL", "--policy-prefix")?;
    }
    if let Some(prefix) = requirement_prefix {
        prefixes.requirement = normalize_typed_prefix(prefix, "REQ", "--requirement-prefix")?;
    }
    if let Some(prefix) = feature_prefix {
        prefixes.feature = normalize_typed_prefix(prefix, "FEAT", "--feature-prefix")?;
    }

    Ok(prefixes)
}

fn resolve_interactive_id_prefixes(
    args: &InitArgs,
    template: StarterTemplate,
    prompt_io: &mut impl PromptIo,
) -> Result<StarterIdPrefixes> {
    if args.philosophy_prefix.is_some()
        || args.policy_prefix.is_some()
        || args.requirement_prefix.is_some()
        || args.feature_prefix.is_some()
    {
        return resolve_init_id_prefixes(
            template,
            args.id_prefix.as_deref(),
            args.philosophy_prefix.as_deref(),
            args.policy_prefix.as_deref(),
            args.requirement_prefix.as_deref(),
            args.feature_prefix.as_deref(),
        );
    }

    loop {
        let shared_default = args.id_prefix.as_deref();
        let shared_stem =
            prompt_optional_with_default(prompt_io, "Shared ID stem (optional)", shared_default)?;
        match resolve_init_id_prefixes(template, shared_stem.as_deref(), None, None, None, None) {
            Ok(prefixes) => return Ok(prefixes),
            Err(error) => eprintln!("{error:#}"),
        }
    }
}

fn default_id_prefixes(template: StarterTemplate) -> StarterIdPrefixes {
    match template {
        StarterTemplate::Generic => StarterIdPrefixes {
            philosophy: "PHIL".to_string(),
            policy: "POL".to_string(),
            requirement: "REQ".to_string(),
            feature: "FEAT".to_string(),
        },
        StarterTemplate::RustOnly => StarterIdPrefixes {
            philosophy: "PHIL-RUST".to_string(),
            policy: "POL-RUST".to_string(),
            requirement: "REQ-RUST".to_string(),
            feature: "FEAT-RUST".to_string(),
        },
        StarterTemplate::PythonOnly => StarterIdPrefixes {
            philosophy: "PHIL-PY".to_string(),
            policy: "POL-PY".to_string(),
            requirement: "REQ-PY".to_string(),
            feature: "FEAT-PY".to_string(),
        },
        StarterTemplate::Polyglot => StarterIdPrefixes {
            philosophy: "PHIL-MIX".to_string(),
            policy: "POL-MIX".to_string(),
            requirement: "REQ-MIX".to_string(),
            feature: "FEAT-MIX".to_string(),
        },
    }
}

fn normalize_shared_id_stem(raw: &str) -> Result<String> {
    let normalized = normalize_id_token(raw, "--id-prefix")?;
    if ["PHIL", "POL", "REQ", "FEAT"].contains(&normalized.as_str())
        || ["PHIL-", "POL-", "REQ-", "FEAT-"]
            .into_iter()
            .any(|prefix| normalized.starts_with(prefix))
    {
        bail!(
            "`--id-prefix` expects a shared stem like `STORE`, not a full typed prefix like `{}`",
            raw.trim()
        );
    }
    Ok(normalized)
}

fn normalize_typed_prefix(raw: &str, expected: &str, flag: &str) -> Result<String> {
    let normalized = normalize_id_token(raw, flag)?;
    if normalized == expected || normalized.starts_with(&format!("{expected}-")) {
        return Ok(normalized);
    }
    bail!(
        "{flag} must be `{expected}` or start with `{expected}-`: `{}`",
        raw.trim()
    );
}

fn normalize_id_token(raw: &str, flag: &str) -> Result<String> {
    let normalized = raw.trim().to_ascii_uppercase();
    if normalized.is_empty()
        || normalized.split('-').any(|segment| segment.is_empty())
        || !normalized
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '-')
    {
        bail!(
            "{flag} must contain only ASCII letters, numbers, and single hyphens: `{}`",
            raw.trim()
        );
    }
    Ok(normalized)
}

fn scaffold_files(
    project_name: &str,
    spec_root: &Path,
    template: StarterTemplate,
    id_prefixes: &StarterIdPrefixes,
    strict_validate_defaults: bool,
) -> Vec<(String, String)> {
    vec![
        (
            "syu.yaml".to_string(),
            render_default_config(spec_root, strict_validate_defaults)
                .expect("config template should render"),
        ),
        (
            path_label(&spec_root.join("philosophy/foundation.yaml")),
            philosophy_template(project_name, template, id_prefixes),
        ),
        (
            path_label(&spec_root.join("policies/policies.yaml")),
            policy_template(project_name, template, id_prefixes),
        ),
        (
            path_label(&spec_root.join(requirement_document_path(template))),
            requirement_template(project_name, template, id_prefixes),
        ),
        (
            path_label(&spec_root.join("features/features.yaml")),
            feature_registry_template(template),
        ),
        (
            path_label(&spec_root.join(feature_document_path(template))),
            feature_template(project_name, template, id_prefixes),
        ),
    ]
}

fn render_default_config(spec_root: &Path, strict_validate_defaults: bool) -> Result<String> {
    let mut config = SyuConfig::default();
    config.spec.root = spec_root.to_path_buf();
    if strict_validate_defaults {
        config.validate.require_symbol_trace_coverage = true;
    }
    render_config(&config)
}

fn philosophy_template(
    project_name: &str,
    template: StarterTemplate,
    id_prefixes: &StarterIdPrefixes,
) -> String {
    let philosophy_id = id_prefixes.philosophy_id();
    let policy_id = id_prefixes.policy_id();
    match template {
        StarterTemplate::Generic => format!(
            "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: {philosophy_id}\n    title: {project_name} should turn intent into executable agreements\n    product_design_principle: |\n      The project should keep philosophy, policy, requirements, and features\n      explicit enough that contributors can validate changes mechanically.\n    coding_guideline: |\n      Prefer stable IDs, typed data, and explicit traceability over conventions\n      that live only in contributor memory.\n    linked_policies:\n      - {policy_id}\n"
        ),
        StarterTemplate::RustOnly => format!(
            "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: {philosophy_id}\n    title: {project_name} should keep Rust traces explicit\n    product_design_principle: |\n      The project should keep Rust-first traceability small, reviewable, and\n      obvious to contributors reading the code.\n    coding_guideline: |\n      Prefer stable IDs and Rust doc comments on traced symbols from the first\n      requirement onward.\n    linked_policies:\n      - {policy_id}\n"
        ),
        StarterTemplate::PythonOnly => format!(
            "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: {philosophy_id}\n    title: {project_name} should keep Python traces explicit\n    product_design_principle: |\n      The project should keep Python traceability small, reviewable, and easy\n      to understand from docstrings alone.\n    coding_guideline: |\n      Prefer stable IDs and clear docstrings on traced Python symbols from the\n      first requirement onward.\n    linked_policies:\n      - {policy_id}\n"
        ),
        StarterTemplate::Polyglot => format!(
            "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: {philosophy_id}\n    title: {project_name} should keep polyglot traces coherent\n    product_design_principle: |\n      The project should prove that one specification can stay understandable\n      even when implementation and tests span multiple languages.\n    coding_guideline: |\n      Prefer stable IDs and short language-native docs on every traced symbol.\n    linked_policies:\n      - {policy_id}\n"
        ),
    }
}

fn policy_template(
    project_name: &str,
    template: StarterTemplate,
    id_prefixes: &StarterIdPrefixes,
) -> String {
    let philosophy_id = id_prefixes.philosophy_id();
    let policy_id = id_prefixes.policy_id();
    let requirement_id = id_prefixes.requirement_id();
    match template {
        StarterTemplate::Generic => format!(
            "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: {policy_id}\n    title: Every change in {project_name} should remain traceable\n    summary: Define rules that turn philosophy into a verifiable workflow.\n    description: |\n      A specification entry is only useful when contributors can trace it to\n      concrete requirements, features, code, and tests inside the repository.\n    linked_philosophies:\n      - {philosophy_id}\n    linked_requirements:\n      - {requirement_id}\n"
        ),
        StarterTemplate::RustOnly => format!(
            "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: {policy_id}\n    title: Rust requirement and feature traces must stay documented in {project_name}\n    summary: Start with a Rust-first workflow that stays explicit in code review.\n    description: |\n      Rust requirement and feature traces should point to symbols whose doc\n      comments carry both the stable ID and a short explanation.\n    linked_philosophies:\n      - {philosophy_id}\n    linked_requirements:\n      - {requirement_id}\n"
        ),
        StarterTemplate::PythonOnly => format!(
            "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: {policy_id}\n    title: Python requirement and feature traces must stay documented in {project_name}\n    summary: Start with a Python-first workflow that stays explicit in docstrings.\n    description: |\n      Python requirement and feature traces should point to symbols whose\n      docstrings carry both the stable ID and a short explanation.\n    linked_philosophies:\n      - {philosophy_id}\n    linked_requirements:\n      - {requirement_id}\n"
        ),
        StarterTemplate::Polyglot => format!(
            "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: {policy_id}\n    title: Polyglot requirement and feature traces must stay verifiable in {project_name}\n    summary: Start with one specification flow that can grow across languages.\n    description: |\n      The starter workspace should make it obvious how one requirement and one\n      feature can stay linked even when implementation later spans Rust,\n      Python, and TypeScript.\n    linked_philosophies:\n      - {philosophy_id}\n    linked_requirements:\n      - {requirement_id}\n"
        ),
    }
}

fn requirement_template(
    project_name: &str,
    template: StarterTemplate,
    id_prefixes: &StarterIdPrefixes,
) -> String {
    let requirement_id = id_prefixes.requirement_id();
    let policy_id = id_prefixes.policy_id();
    let feature_id = id_prefixes.feature_id();
    match template {
        StarterTemplate::Generic => format!(
            "category: Core Requirements\nprefix: {}\n\nrequirements:\n  - id: {requirement_id}\n    title: Bootstrap {project_name} with a four-layer specification\n    description: |\n      The project should keep philosophy, policy, requirements, and features in\n      YAML so contributors can evolve behavior deliberately.\n    priority: high\n    status: planned\n    linked_policies:\n      - {policy_id}\n    linked_features:\n      - {feature_id}\n    tests: {{}}\n",
            id_prefixes.requirement
        ),
        StarterTemplate::RustOnly => format!(
            "category: Rust Requirements\nprefix: {}\n\nrequirements:\n  - id: {requirement_id}\n    title: Bootstrap {project_name} with Rust-first trace conventions\n    description: |\n      The project should start with a Rust-oriented requirement that can later\n      claim documented Rust test symbols without restructuring the workspace.\n    priority: high\n    status: planned\n    linked_policies:\n      - {policy_id}\n    linked_features:\n      - {feature_id}\n    tests: {{}}\n",
            id_prefixes.requirement
        ),
        StarterTemplate::PythonOnly => format!(
            "category: Python Requirements\nprefix: {}\n\nrequirements:\n  - id: {requirement_id}\n    title: Bootstrap {project_name} with Python-first trace conventions\n    description: |\n      The project should start with a Python-oriented requirement that can later\n      claim documented Python test symbols without restructuring the workspace.\n    priority: high\n    status: planned\n    linked_policies:\n      - {policy_id}\n    linked_features:\n      - {feature_id}\n    tests: {{}}\n",
            id_prefixes.requirement
        ),
        StarterTemplate::Polyglot => format!(
            "category: Polyglot Requirements\nprefix: {}\n\nrequirements:\n  - id: {requirement_id}\n    title: Bootstrap {project_name} with polyglot trace conventions\n    description: |\n      The project should start with one requirement that can later trace into\n      Rust, Python, and TypeScript without changing the layered layout.\n    priority: high\n    status: planned\n    linked_policies:\n      - {policy_id}\n    linked_features:\n      - {feature_id}\n    tests: {{}}\n",
            id_prefixes.requirement
        ),
    }
}

fn feature_registry_template(template: StarterTemplate) -> String {
    let feature_document = Path::new(feature_document_path(template))
        .strip_prefix("features/")
        .expect("feature path should stay under features/");
    format!(
        "version: \"{}\"\nupdated: \"generated by syu init\"\n\nfiles:\n  - kind: {}\n    file: {}\n",
        SyuConfig::default().version,
        feature_kind(template),
        path_label(feature_document)
    )
}

fn feature_template(
    project_name: &str,
    template: StarterTemplate,
    id_prefixes: &StarterIdPrefixes,
) -> String {
    let feature_id = id_prefixes.feature_id();
    let requirement_id = id_prefixes.requirement_id();
    match template {
        StarterTemplate::Generic => format!(
            "category: Core Features\nversion: 1\n\nfeatures:\n  - id: {feature_id}\n    title: Bootstrap the {project_name} spec workspace\n    summary: Provide a starter structure that contributors can extend.\n    status: planned\n    linked_requirements:\n      - {requirement_id}\n    implementations: {{}}\n"
        ),
        StarterTemplate::RustOnly => format!(
            "category: Rust Features\nversion: 1\n\nfeatures:\n  - id: {feature_id}\n    title: Bootstrap the {project_name} Rust spec workspace\n    summary: Provide a Rust-oriented starter structure that contributors can extend.\n    status: planned\n    linked_requirements:\n      - {requirement_id}\n    implementations: {{}}\n"
        ),
        StarterTemplate::PythonOnly => format!(
            "category: Python Features\nversion: 1\n\nfeatures:\n  - id: {feature_id}\n    title: Bootstrap the {project_name} Python spec workspace\n    summary: Provide a Python-oriented starter structure that contributors can extend.\n    status: planned\n    linked_requirements:\n      - {requirement_id}\n    implementations: {{}}\n"
        ),
        StarterTemplate::Polyglot => format!(
            "category: Polyglot Features\nversion: 1\n\nfeatures:\n  - id: {feature_id}\n    title: Bootstrap the {project_name} polyglot spec workspace\n    summary: Provide a starter structure that can grow across Rust, Python, and TypeScript.\n    status: planned\n    linked_requirements:\n      - {requirement_id}\n    implementations: {{}}\n"
        ),
    }
}

fn requirement_document_path(template: StarterTemplate) -> &'static str {
    match template {
        StarterTemplate::Generic => "requirements/core/core.yaml",
        StarterTemplate::RustOnly => "requirements/core/rust.yaml",
        StarterTemplate::PythonOnly => "requirements/core/python.yaml",
        StarterTemplate::Polyglot => "requirements/core/polyglot.yaml",
    }
}

fn feature_document_path(template: StarterTemplate) -> &'static str {
    match template {
        StarterTemplate::Generic => "features/core/core.yaml",
        StarterTemplate::RustOnly => "features/languages/rust.yaml",
        StarterTemplate::PythonOnly => "features/languages/python.yaml",
        StarterTemplate::Polyglot => "features/languages/polyglot.yaml",
    }
}

fn feature_kind(template: StarterTemplate) -> &'static str {
    match template {
        StarterTemplate::Generic => "core",
        StarterTemplate::RustOnly => "rust",
        StarterTemplate::PythonOnly => "python",
        StarterTemplate::Polyglot => "polyglot",
    }
}

fn path_label(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, fs};

    use tempfile::tempdir;

    use crate::cli::{InitArgs, StarterTemplate};
    use crate::command::prompt::PromptIo;

    use super::{
        DEFAULT_SPEC_ROOT, GENERATED_PATHS, INIT_INTERACTIVE_JSON_MESSAGE, default_id_prefixes,
        ensure_writable_targets, feature_document_path, feature_kind, infer_project_name,
        parse_starter_template_prompt, path_label, prompt_for_spec_root,
        prompt_for_starter_template, requirement_document_path, resolve_init_id_prefixes,
        resolve_init_spec_root, resolve_interactive_id_prefixes, run_init_command,
        run_init_command_with_prompt_io, scaffold_files,
    };

    #[derive(Default)]
    struct FakePromptIo {
        terminal: bool,
        lines: VecDeque<String>,
        prompts: Vec<(String, Option<String>)>,
    }

    impl PromptIo for FakePromptIo {
        fn is_terminal(&self) -> bool {
            self.terminal
        }

        fn prompt_line(&mut self, label: &str, default: Option<&str>) -> anyhow::Result<String> {
            self.prompts.push((
                label.to_string(),
                default.map(std::string::ToString::to_string),
            ));
            Ok(self.lines.pop_front().unwrap_or_default())
        }
    }

    #[test]
    fn infer_project_name_uses_workspace_directory() {
        assert_eq!(
            infer_project_name(std::path::Path::new("/tmp/example-project")),
            "example-project"
        );
    }

    #[test]
    fn scaffold_files_include_all_expected_templates() {
        let files = scaffold_files(
            "demo",
            std::path::Path::new(DEFAULT_SPEC_ROOT),
            StarterTemplate::Generic,
            &default_id_prefixes(StarterTemplate::Generic),
            false,
        );
        let paths: Vec<_> = files.into_iter().map(|(path, _)| path).collect();
        assert_eq!(paths.len(), GENERATED_PATHS.len());
        for expected in GENERATED_PATHS {
            assert!(paths.iter().any(|path| path == expected));
        }
    }

    #[test]
    fn resolve_init_id_prefixes_accepts_shared_stems_and_overrides() {
        let args = InitArgs {
            workspace: std::path::PathBuf::from("."),
            interactive: false,
            name: None,
            spec_root: None,
            template: StarterTemplate::RustOnly,
            id_prefix: Some("store".to_string()),
            philosophy_prefix: Some("phil-guiding".to_string()),
            policy_prefix: Some("pol-governance".to_string()),
            requirement_prefix: Some("req-auth".to_string()),
            feature_prefix: Some("feat-auth".to_string()),
            force: false,
            format: crate::cli::OutputFormat::Text,
        };

        let prefixes = resolve_init_id_prefixes(
            args.template,
            args.id_prefix.as_deref(),
            args.philosophy_prefix.as_deref(),
            args.policy_prefix.as_deref(),
            args.requirement_prefix.as_deref(),
            args.feature_prefix.as_deref(),
        )
        .expect("prefixes should resolve");
        assert_eq!(prefixes.philosophy, "PHIL-GUIDING");
        assert_eq!(prefixes.policy, "POL-GOVERNANCE");
        assert_eq!(prefixes.requirement, "REQ-AUTH");
        assert_eq!(prefixes.feature, "FEAT-AUTH");
    }

    #[test]
    fn resolve_init_id_prefixes_rejects_full_prefixes_as_shared_stems() {
        let args = InitArgs {
            workspace: std::path::PathBuf::from("."),
            interactive: false,
            name: None,
            spec_root: None,
            template: StarterTemplate::Generic,
            id_prefix: Some("REQ-STORE".to_string()),
            philosophy_prefix: None,
            policy_prefix: None,
            requirement_prefix: None,
            feature_prefix: None,
            force: false,
            format: crate::cli::OutputFormat::Text,
        };

        let error = resolve_init_id_prefixes(
            args.template,
            args.id_prefix.as_deref(),
            args.philosophy_prefix.as_deref(),
            args.policy_prefix.as_deref(),
            args.requirement_prefix.as_deref(),
            args.feature_prefix.as_deref(),
        )
        .expect_err("typed stems should be rejected");
        assert!(error.to_string().contains("--id-prefix"));
    }

    #[test]
    fn resolve_init_id_prefixes_rejects_typed_overrides_without_expected_prefix() {
        let args = InitArgs {
            workspace: std::path::PathBuf::from("."),
            interactive: false,
            name: None,
            spec_root: None,
            template: StarterTemplate::Generic,
            id_prefix: None,
            philosophy_prefix: Some("store".to_string()),
            policy_prefix: None,
            requirement_prefix: None,
            feature_prefix: None,
            force: false,
            format: crate::cli::OutputFormat::Text,
        };

        let error = resolve_init_id_prefixes(
            args.template,
            args.id_prefix.as_deref(),
            args.philosophy_prefix.as_deref(),
            args.policy_prefix.as_deref(),
            args.requirement_prefix.as_deref(),
            args.feature_prefix.as_deref(),
        )
        .expect_err("typed overrides should require a prefix");
        let message = error.to_string();
        assert!(message.contains("--philosophy-prefix"));
        assert!(message.contains("PHIL"));
    }

    #[test]
    fn resolve_init_id_prefixes_rejects_invalid_override_tokens() {
        let args = InitArgs {
            workspace: std::path::PathBuf::from("."),
            interactive: false,
            name: None,
            spec_root: None,
            template: StarterTemplate::Generic,
            id_prefix: None,
            philosophy_prefix: None,
            policy_prefix: None,
            requirement_prefix: None,
            feature_prefix: Some("feat_store".to_string()),
            force: false,
            format: crate::cli::OutputFormat::Text,
        };

        let error = resolve_init_id_prefixes(
            args.template,
            args.id_prefix.as_deref(),
            args.philosophy_prefix.as_deref(),
            args.policy_prefix.as_deref(),
            args.requirement_prefix.as_deref(),
            args.feature_prefix.as_deref(),
        )
        .expect_err("invalid override tokens should be rejected");
        let message = error.to_string();
        assert!(message.contains("--feature-prefix"));
        assert!(message.contains("single hyphens"));
    }

    #[test]
    fn ensure_writable_targets_reports_existing_files_without_force() {
        let tempdir = tempdir().expect("tempdir should exist");
        fs::write(tempdir.path().join("syu.yaml"), "existing").expect("file should exist");

        let error = ensure_writable_targets(
            tempdir.path(),
            [std::path::PathBuf::from("syu.yaml")].into_iter(),
            false,
        )
        .expect_err("overwrite should be rejected");

        assert!(error.to_string().contains("refusing to overwrite"));
    }

    #[test]
    fn init_command_creates_scaffolded_workspace() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path().join("demo");
        let args = InitArgs {
            workspace: workspace.clone(),
            interactive: false,
            name: Some("demo".to_string()),
            spec_root: None,
            template: StarterTemplate::Generic,
            id_prefix: None,
            philosophy_prefix: None,
            policy_prefix: None,
            requirement_prefix: None,
            feature_prefix: None,
            force: false,
            format: crate::cli::OutputFormat::Text,
        };

        let code = run_init_command(&args).expect("init should succeed");
        assert_eq!(code, 0);

        for path in GENERATED_PATHS {
            assert!(
                workspace.join(path).exists(),
                "missing generated file: {path}"
            );
        }
    }

    #[test]
    fn init_command_honors_force_overwrite() {
        let tempdir = tempdir().expect("tempdir should exist");
        fs::create_dir_all(tempdir.path()).expect("workspace dir");
        fs::write(tempdir.path().join("syu.yaml"), "old").expect("old config");

        let args = InitArgs {
            workspace: tempdir.path().to_path_buf(),
            interactive: false,
            name: Some("forced".to_string()),
            spec_root: None,
            template: StarterTemplate::Generic,
            id_prefix: None,
            philosophy_prefix: None,
            policy_prefix: None,
            requirement_prefix: None,
            feature_prefix: None,
            force: true,
            format: crate::cli::OutputFormat::Text,
        };

        run_init_command(&args).expect("force init should succeed");
        let config =
            fs::read_to_string(tempdir.path().join("syu.yaml")).expect("config should exist");
        assert!(config.contains("default_fix: false"));
        assert!(config.contains("allow_planned: true"));
        assert!(config.contains(crate::config::current_cli_version()));
    }

    #[test]
    fn interactive_init_requires_a_terminal() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path().join("demo");
        let mut prompt_io = FakePromptIo::default();
        let error = run_init_command_with_prompt_io(
            &InitArgs {
                workspace: workspace.clone(),
                interactive: true,
                name: None,
                spec_root: None,
                template: StarterTemplate::Generic,
                id_prefix: None,
                philosophy_prefix: None,
                policy_prefix: None,
                requirement_prefix: None,
                feature_prefix: None,
                force: false,
                format: crate::cli::OutputFormat::Text,
            },
            &mut prompt_io,
        )
        .expect_err("interactive init should require a terminal");

        assert!(
            error
                .to_string()
                .contains("`syu init --interactive` requires a terminal")
        );
        assert!(!workspace.exists());
    }

    #[test]
    fn interactive_init_rejects_json_output_before_creating_workspace() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path().join("demo");
        let mut prompt_io = FakePromptIo {
            terminal: true,
            ..Default::default()
        };

        let error = run_init_command_with_prompt_io(
            &InitArgs {
                workspace: workspace.clone(),
                interactive: true,
                name: None,
                spec_root: None,
                template: StarterTemplate::Generic,
                id_prefix: None,
                philosophy_prefix: None,
                policy_prefix: None,
                requirement_prefix: None,
                feature_prefix: None,
                force: false,
                format: crate::cli::OutputFormat::Json,
            },
            &mut prompt_io,
        )
        .expect_err("interactive json output should be rejected");

        assert_eq!(error.to_string(), INIT_INTERACTIVE_JSON_MESSAGE);
        assert!(
            !workspace.exists(),
            "interactive validation should fail before creating the workspace"
        );
    }

    #[test]
    fn prompt_helpers_support_template_and_spec_root_guidance() {
        let mut prompt_io = FakePromptIo {
            terminal: true,
            lines: VecDeque::from([
                "rust".to_string(),
                "../spec".to_string(),
                "spec/contracts".to_string(),
            ]),
            ..Default::default()
        };

        let template = prompt_for_starter_template(&mut prompt_io, StarterTemplate::Generic)
            .expect("template");
        let spec_root = prompt_for_spec_root(&mut prompt_io, None).expect("spec root");

        assert_eq!(template, StarterTemplate::RustOnly);
        assert_eq!(spec_root, std::path::PathBuf::from("spec/contracts"));
        assert_eq!(prompt_io.prompts.len(), 3);
    }

    #[test]
    fn prompt_for_starter_template_retries_invalid_values() {
        let mut prompt_io = FakePromptIo {
            terminal: true,
            lines: VecDeque::from(["unknown".to_string(), "4".to_string()]),
            ..Default::default()
        };

        let template = prompt_for_starter_template(&mut prompt_io, StarterTemplate::Generic)
            .expect("template prompt should retry");

        assert_eq!(template, StarterTemplate::Polyglot);
        assert_eq!(prompt_io.prompts.len(), 2);
    }

    #[test]
    fn parse_starter_template_prompt_accepts_remaining_aliases() {
        assert_eq!(
            parse_starter_template_prompt("python"),
            Some(StarterTemplate::PythonOnly)
        );
        assert_eq!(
            parse_starter_template_prompt("mixed"),
            Some(StarterTemplate::Polyglot)
        );
        assert_eq!(parse_starter_template_prompt("unknown"), None);
    }

    #[test]
    fn resolve_interactive_id_prefixes_uses_explicit_overrides_without_prompting() {
        let args = InitArgs {
            workspace: std::path::PathBuf::from("."),
            interactive: true,
            name: None,
            spec_root: None,
            template: StarterTemplate::Generic,
            id_prefix: Some("store".to_string()),
            philosophy_prefix: Some("PHIL-guiding".to_string()),
            policy_prefix: None,
            requirement_prefix: None,
            feature_prefix: None,
            force: false,
            format: crate::cli::OutputFormat::Text,
        };
        let mut prompt_io = FakePromptIo {
            terminal: true,
            ..Default::default()
        };

        let prefixes =
            resolve_interactive_id_prefixes(&args, StarterTemplate::Generic, &mut prompt_io)
                .expect("explicit overrides should bypass prompting");

        assert_eq!(prefixes.philosophy, "PHIL-GUIDING");
        assert_eq!(prefixes.policy, "POL-STORE");
        assert!(prompt_io.prompts.is_empty());
    }

    #[test]
    fn resolve_interactive_id_prefixes_accepts_blank_defaults_and_retries_invalid_values() {
        let default_args = InitArgs {
            workspace: std::path::PathBuf::from("."),
            interactive: true,
            name: None,
            spec_root: None,
            template: StarterTemplate::Generic,
            id_prefix: Some("store".to_string()),
            philosophy_prefix: None,
            policy_prefix: None,
            requirement_prefix: None,
            feature_prefix: None,
            force: false,
            format: crate::cli::OutputFormat::Text,
        };
        let mut blank_prompt_io = FakePromptIo {
            terminal: true,
            lines: VecDeque::from([String::new()]),
            ..Default::default()
        };

        let blank_prefixes = resolve_interactive_id_prefixes(
            &default_args,
            StarterTemplate::Generic,
            &mut blank_prompt_io,
        )
        .expect("blank prompt should keep the default shared stem");
        assert_eq!(blank_prefixes.requirement, "REQ-STORE");

        let retry_args = InitArgs {
            id_prefix: None,
            ..default_args
        };
        let mut retry_prompt_io = FakePromptIo {
            terminal: true,
            lines: VecDeque::from(["REQ-STORE".to_string(), "store".to_string()]),
            ..Default::default()
        };

        let retry_prefixes = resolve_interactive_id_prefixes(
            &retry_args,
            StarterTemplate::Generic,
            &mut retry_prompt_io,
        )
        .expect("interactive prompt should retry invalid shared stems");
        assert_eq!(retry_prefixes.feature, "FEAT-STORE");
        assert_eq!(retry_prompt_io.prompts.len(), 2);
    }

    #[test]
    fn interactive_init_writes_strict_config_and_scaffold_choices() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path().join("demo");
        let mut prompt_io = FakePromptIo {
            terminal: true,
            lines: VecDeque::from([
                String::new(),
                "rust-only".to_string(),
                "spec/contracts".to_string(),
                "store".to_string(),
                "yes".to_string(),
            ]),
            ..Default::default()
        };

        let code = run_init_command_with_prompt_io(
            &InitArgs {
                workspace: workspace.clone(),
                interactive: true,
                name: None,
                spec_root: None,
                template: StarterTemplate::Generic,
                id_prefix: None,
                philosophy_prefix: None,
                policy_prefix: None,
                requirement_prefix: None,
                feature_prefix: None,
                force: false,
                format: crate::cli::OutputFormat::Text,
            },
            &mut prompt_io,
        )
        .expect("interactive init should succeed");

        assert_eq!(code, 0);
        let config = fs::read_to_string(workspace.join("syu.yaml")).expect("config should exist");
        assert!(config.contains("root: spec/contracts"));
        assert!(config.contains("allow_planned: true"));
        assert!(config.contains("require_symbol_trace_coverage: true"));
        let requirement =
            fs::read_to_string(workspace.join("spec/contracts/requirements/core/rust.yaml"))
                .expect("requirement should exist");
        let feature =
            fs::read_to_string(workspace.join("spec/contracts/features/languages/rust.yaml"))
                .expect("feature should exist");
        assert!(requirement.contains("REQ-STORE-001"));
        assert!(feature.contains("FEAT-STORE-001"));
        let validate_code = crate::command::check::run_check_command(&crate::cli::CheckArgs {
            workspace: workspace.clone(),
            format: crate::cli::OutputFormat::Json,
            severity: Vec::new(),
            genre: Vec::new(),
            rule: Vec::new(),
            id: Vec::new(),
            fix: false,
            no_fix: false,
            allow_planned: None,
            require_non_orphaned_items: None,
            require_reciprocal_links: None,
            require_symbol_trace_coverage: None,
            warning_exit_code: None,
            quiet: false,
        })
        .expect("interactive init workspace should validate");
        assert_eq!(validate_code, 0);
        assert_eq!(prompt_io.prompts.len(), 5);
    }

    #[test]
    fn scaffold_files_default_to_planned_status() {
        let files = scaffold_files(
            "demo",
            std::path::Path::new(DEFAULT_SPEC_ROOT),
            StarterTemplate::Generic,
            &default_id_prefixes(StarterTemplate::Generic),
            false,
        );
        let requirement = files
            .iter()
            .find(|(path, _)| path == "docs/syu/requirements/core/core.yaml")
            .expect("requirement template");
        let feature = files
            .iter()
            .find(|(path, _)| path == "docs/syu/features/core/core.yaml")
            .expect("feature template");

        assert!(requirement.1.contains("status: planned"));
        assert!(feature.1.contains("status: planned"));
    }

    #[test]
    fn init_command_rejects_file_workspaces() {
        let tempdir = tempdir().expect("tempdir should exist");
        let file_path = tempdir.path().join("workspace-file");
        fs::write(&file_path, "not a directory").expect("file should exist");

        let error = run_init_command(&InitArgs {
            workspace: file_path.clone(),
            interactive: false,
            name: None,
            spec_root: None,
            template: StarterTemplate::Generic,
            id_prefix: None,
            philosophy_prefix: None,
            policy_prefix: None,
            requirement_prefix: None,
            feature_prefix: None,
            force: false,
            format: crate::cli::OutputFormat::Text,
        })
        .expect_err("file workspace should be rejected");

        assert!(error.to_string().contains(&format!(
            "workspace path `{}` is not a directory",
            file_path.display()
        )));
    }

    #[test]
    fn init_command_reports_directory_creation_failures() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path().join("demo");
        fs::create_dir_all(&workspace).expect("workspace should exist");
        fs::write(workspace.join("docs"), "blocking file").expect("blocking file should exist");

        let error = run_init_command(&InitArgs {
            workspace,
            interactive: false,
            name: Some("demo".to_string()),
            spec_root: None,
            template: StarterTemplate::Generic,
            id_prefix: None,
            philosophy_prefix: None,
            policy_prefix: None,
            requirement_prefix: None,
            feature_prefix: None,
            force: false,
            format: crate::cli::OutputFormat::Text,
        })
        .expect_err("directory creation should fail");

        assert!(!error.to_string().is_empty());
    }

    #[test]
    fn init_command_reports_file_write_failures() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path().join("demo");
        fs::create_dir_all(workspace.join("docs/syu/features/core/core.yaml"))
            .expect("blocking directory should exist");

        let error = run_init_command(&InitArgs {
            workspace,
            interactive: false,
            name: Some("demo".to_string()),
            spec_root: None,
            template: StarterTemplate::Generic,
            id_prefix: None,
            philosophy_prefix: None,
            policy_prefix: None,
            requirement_prefix: None,
            feature_prefix: None,
            force: true,
            format: crate::cli::OutputFormat::Text,
        })
        .expect_err("file write should fail");

        assert!(!error.to_string().is_empty());
    }

    #[test]
    fn init_command_json_format_includes_created_files() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path().join("demo");
        let args = InitArgs {
            workspace: workspace.clone(),
            interactive: false,
            name: Some("demo".to_string()),
            spec_root: None,
            template: StarterTemplate::Generic,
            id_prefix: None,
            philosophy_prefix: None,
            policy_prefix: None,
            requirement_prefix: None,
            feature_prefix: None,
            force: false,
            format: crate::cli::OutputFormat::Json,
        };

        let code = run_init_command(&args).expect("init should succeed");
        assert_eq!(code, 0);

        for path in GENERATED_PATHS {
            assert!(
                workspace.join(path).exists(),
                "missing generated file: {path}"
            );
        }
    }

    #[test]
    fn scaffold_files_support_language_oriented_templates() {
        let spec_root = std::path::Path::new(DEFAULT_SPEC_ROOT);
        for template in [
            StarterTemplate::RustOnly,
            StarterTemplate::PythonOnly,
            StarterTemplate::Polyglot,
        ] {
            let files = scaffold_files(
                "demo",
                spec_root,
                template,
                &default_id_prefixes(template),
                false,
            );
            let paths: Vec<_> = files.iter().map(|(path, _)| path.as_str()).collect();
            assert!(paths.contains(&"syu.yaml"));
            assert!(paths.contains(&"docs/syu/philosophy/foundation.yaml"));
            assert!(paths.contains(&"docs/syu/policies/policies.yaml"));
            let requirement_path = path_label(&spec_root.join(requirement_document_path(template)));
            let feature_path = path_label(&spec_root.join(feature_document_path(template)));
            assert!(paths.contains(&requirement_path.as_str()));
            assert!(paths.contains(&feature_path.as_str()));

            let registry = files
                .iter()
                .find(|(path, _)| path == "docs/syu/features/features.yaml")
                .expect("feature registry should exist");
            assert!(registry.1.contains(feature_kind(template)));
        }
    }

    #[test]
    fn scaffold_files_support_language_templates_in_custom_spec_roots() {
        let spec_root = std::path::Path::new("spec/contracts");
        let files = scaffold_files(
            "demo",
            spec_root,
            StarterTemplate::RustOnly,
            &default_id_prefixes(StarterTemplate::RustOnly),
            false,
        );
        let paths: Vec<_> = files.iter().map(|(path, _)| path.as_str()).collect();

        assert!(paths.contains(&"spec/contracts/philosophy/foundation.yaml"));
        assert!(paths.contains(&"spec/contracts/policies/policies.yaml"));
        assert!(paths.contains(&"spec/contracts/requirements/core/rust.yaml"));
        assert!(paths.contains(&"spec/contracts/features/languages/rust.yaml"));

        let registry = files
            .iter()
            .find(|(path, _)| path == "spec/contracts/features/features.yaml")
            .expect("feature registry should exist");
        assert!(registry.1.contains("kind: rust"));
        assert!(registry.1.contains("file: languages/rust.yaml"));
    }

    #[test]
    fn resolve_init_spec_root_normalizes_relative_paths() {
        assert_eq!(
            resolve_init_spec_root(Some(std::path::Path::new("./spec/./contracts")))
                .expect("spec root should normalize"),
            std::path::PathBuf::from("spec/contracts")
        );
        assert_eq!(
            resolve_init_spec_root(None).expect("default spec root should resolve"),
            std::path::PathBuf::from(DEFAULT_SPEC_ROOT)
        );
    }

    #[test]
    fn resolve_init_spec_root_rejects_paths_outside_workspace() {
        let parent = resolve_init_spec_root(Some(std::path::Path::new("../spec")))
            .expect_err("parent paths should fail");
        assert!(parent.to_string().contains("--spec-root"));

        let absolute = resolve_init_spec_root(Some(std::path::Path::new("/tmp/spec")))
            .expect_err("absolute paths should fail");
        assert!(absolute.to_string().contains("--spec-root"));
    }
}
