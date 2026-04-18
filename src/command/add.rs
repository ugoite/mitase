// FEAT-ADD-001
// REQ-CORE-020

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result, bail};

use crate::{
    cli::{AddArgs, LookupKind},
    coverage::normalize_relative_path,
    model::{
        FeatureDocument, FeatureRegistryDocument, PhilosophyDocument, PolicyDocument,
        RequirementDocument,
    },
    workspace::{Workspace, load_workspace},
};

use super::{
    lookup::WorkspaceLookup,
    prompt::{
        PromptIo, StdioPromptIo, ensure_prompt_terminal, prompt_optional, prompt_required,
        prompt_with_default,
    },
    shell_quote_path,
};

const ADD_MISSING_ID_NON_TTY_MESSAGE: &str = "`syu add` needs a definition ID when stdin/stdout are not terminals; pass the ID explicitly or rerun in a terminal to be prompted";
const ADD_INTERACTIVE_KIND_NON_TTY_MESSAGE: &str =
    "`syu add --interactive` requires a terminal to prompt for a feature kind";
const ADD_INTERACTIVE_FILE_NON_TTY_MESSAGE: &str =
    "`syu add --interactive` requires a terminal to prompt for a file path";

pub fn run_add_command(args: &AddArgs) -> Result<i32> {
    if args.layer != LookupKind::Feature && args.kind.is_some() {
        bail!("`--kind` is only supported when scaffolding features");
    }
    let resolved = resolve_add_invocation(args)?;
    let workspace = load_workspace(&resolved.workspace)?;
    if WorkspaceLookup::new(&workspace)
        .find(&resolved.parsed_id.normalized)
        .is_some()
    {
        bail!(
            "{} `{}` already exists in `{}`",
            args.layer.label(),
            resolved.parsed_id.normalized,
            workspace.root.display()
        );
    }
    let target = resolve_target_path(
        &workspace,
        args.layer,
        resolved.file.as_deref(),
        &resolved.parsed_id,
        resolved.feature_kind.as_deref(),
    )?;
    let created_target_file = !target.absolute.exists();
    let feature_registry_update = if args.layer == LookupKind::Feature {
        Some(prepare_feature_registry_update(
            &workspace,
            &target.absolute,
            resolved
                .feature_kind
                .as_deref()
                .expect("features require a registry kind"),
        )?)
    } else {
        None
    };

    let feature_kind = resolved.feature_kind.as_deref();
    write_stub_document(args.layer, &resolved.parsed_id, feature_kind, &target)?;

    let registry_updated = if let Some(update) = feature_registry_update {
        write_feature_registry_update(update)?
    } else {
        false
    };

    print_add_summary(
        &workspace,
        args.layer,
        &resolved.parsed_id.normalized,
        &target.absolute,
        created_target_file,
        registry_updated,
    );
    Ok(0)
}

#[derive(Debug, Clone)]
struct ResolvedAddInvocation {
    workspace: PathBuf,
    parsed_id: ParsedId,
    feature_kind: Option<String>,
    file: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct ParsedId {
    normalized: String,
    typed_prefix: String,
    title: String,
    folder_slug: String,
    file_slug: String,
}

#[derive(Debug, Clone, Copy)]
struct ReciprocalLinkField {
    yaml_key: &'static str,
    linked_kind_label: &'static str,
}

const PHILOSOPHY_RECIPROCAL_LINK_FIELDS: &[ReciprocalLinkField] = &[ReciprocalLinkField {
    yaml_key: "linked_policies",
    linked_kind_label: "policy",
}];
const POLICY_RECIPROCAL_LINK_FIELDS: &[ReciprocalLinkField] = &[
    ReciprocalLinkField {
        yaml_key: "linked_philosophies",
        linked_kind_label: "philosophy",
    },
    ReciprocalLinkField {
        yaml_key: "linked_requirements",
        linked_kind_label: "requirement",
    },
];
const REQUIREMENT_RECIPROCAL_LINK_FIELDS: &[ReciprocalLinkField] = &[
    ReciprocalLinkField {
        yaml_key: "linked_policies",
        linked_kind_label: "policy",
    },
    ReciprocalLinkField {
        yaml_key: "linked_features",
        linked_kind_label: "feature",
    },
];
const FEATURE_RECIPROCAL_LINK_FIELDS: &[ReciprocalLinkField] = &[ReciprocalLinkField {
    yaml_key: "linked_requirements",
    linked_kind_label: "requirement",
}];

impl ParsedId {
    fn parse(layer: LookupKind, raw: &str) -> Result<Self> {
        let normalized = normalize_definition_id(raw, layer)?;
        let segments: Vec<_> = normalized.split('-').collect();
        let suffix = &segments[1..segments.len() - 1];
        let title = if suffix.is_empty() {
            default_title(layer)
        } else {
            title_case_tokens(suffix)
        };
        let folder_slug = suffix
            .first()
            .map(|segment| segment.to_ascii_lowercase())
            .unwrap_or_else(|| default_folder_slug(layer));
        let file_slug = if suffix.len() > 1 {
            suffix[1..]
                .iter()
                .map(|segment| segment.to_ascii_lowercase())
                .collect::<Vec<_>>()
                .join("-")
        } else {
            folder_slug.clone()
        };

        Ok(Self {
            typed_prefix: segments[..segments.len() - 1].join("-"),
            normalized,
            title,
            folder_slug,
            file_slug,
        })
    }
}

#[derive(Debug, Clone)]
struct TargetPath {
    absolute: PathBuf,
}

struct FeatureRegistryUpdate {
    path: PathBuf,
    updated_contents: Option<String>,
}

fn resolve_add_invocation(args: &AddArgs) -> Result<ResolvedAddInvocation> {
    let mut prompt_io = StdioPromptIo;
    resolve_add_invocation_with_prompt_io(args, &mut prompt_io)
}

fn resolve_add_invocation_with_prompt_io(
    args: &AddArgs,
    prompt_io: &mut impl PromptIo,
) -> Result<ResolvedAddInvocation> {
    let (raw_id, workspace) = resolve_workspace_and_id(args)?;
    let parsed_id = match raw_id {
        Some(id) => ParsedId::parse(args.layer, &id)?,
        None => prompt_for_parsed_id(args.layer, prompt_io)?,
    };
    let feature_kind = match args.layer {
        LookupKind::Feature => Some(resolve_feature_kind(args, &parsed_id, prompt_io)?),
        _ => None,
    };
    let file =
        resolve_interactive_file_prompt(args, &parsed_id, feature_kind.as_deref(), prompt_io)?;

    Ok(ResolvedAddInvocation {
        workspace,
        parsed_id,
        feature_kind,
        file,
    })
}

fn resolve_workspace_and_id(args: &AddArgs) -> Result<(Option<String>, PathBuf)> {
    let mut id = args.id.clone();
    let mut workspace = args.workspace.clone();

    if args.interactive
        && workspace == Path::new(".")
        && id.as_deref().is_some_and(|candidate| {
            ParsedId::parse(args.layer, candidate).is_err() && Path::new(candidate).exists()
        })
    {
        workspace = PathBuf::from(
            id.take()
                .expect("interactive workspace override should still be present"),
        );
    }

    Ok((id, workspace))
}

fn resolve_feature_kind(
    args: &AddArgs,
    parsed_id: &ParsedId,
    prompt_io: &mut impl PromptIo,
) -> Result<String> {
    if let Some(kind) = args.kind.as_deref() {
        return normalize_feature_kind(kind);
    }
    if args.interactive {
        ensure_prompt_terminal(prompt_io, ADD_INTERACTIVE_KIND_NON_TTY_MESSAGE)?;
        loop {
            let raw =
                prompt_with_default(prompt_io, "Feature kind", parsed_id.folder_slug.as_str())?;
            match normalize_feature_kind(&raw) {
                Ok(kind) => return Ok(kind),
                Err(error) => eprintln!("{error:#}"),
            }
        }
    }

    normalize_feature_kind(parsed_id.folder_slug.as_str())
}

fn resolve_interactive_file_prompt(
    args: &AddArgs,
    parsed_id: &ParsedId,
    feature_kind: Option<&str>,
    prompt_io: &mut impl PromptIo,
) -> Result<Option<PathBuf>> {
    if let Some(file) = &args.file {
        return Ok(Some(file.clone()));
    }
    if !args.interactive {
        return Ok(None);
    }

    ensure_prompt_terminal(prompt_io, ADD_INTERACTIVE_FILE_NON_TTY_MESSAGE)?;
    let default_relative = default_document_path(args.layer, parsed_id, feature_kind);
    let default_relative_display = default_relative.display().to_string();
    let prompted_path = prompt_optional(prompt_io, "YAML file", Some(&default_relative_display))?;
    Ok(prompted_path.map(PathBuf::from))
}

fn prompt_for_parsed_id(layer: LookupKind, prompt_io: &mut impl PromptIo) -> Result<ParsedId> {
    ensure_prompt_terminal(prompt_io, ADD_MISSING_ID_NON_TTY_MESSAGE)?;

    loop {
        let raw = prompt_required(prompt_io, "Definition ID")?;
        match ParsedId::parse(layer, &raw) {
            Ok(parsed) => return Ok(parsed),
            Err(error) => eprintln!("{error:#}"),
        }
    }
}

fn normalize_definition_id(raw: &str, layer: LookupKind) -> Result<String> {
    let normalized = raw.trim().to_ascii_uppercase();
    if normalized.is_empty()
        || normalized.split('-').any(|segment| segment.is_empty())
        || !normalized
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '-')
    {
        bail!(
            "{} IDs must contain only ASCII letters, numbers, and single hyphens: `{}`",
            layer.label(),
            raw.trim()
        );
    }

    let segments: Vec<_> = normalized.split('-').collect();
    let expected_prefix = match layer {
        LookupKind::Philosophy => "PHIL",
        LookupKind::Policy => "POL",
        LookupKind::Requirement => "REQ",
        LookupKind::Feature => "FEAT",
    };
    if segments.first().copied() != Some(expected_prefix) {
        bail!(
            "{} IDs must start with `{expected_prefix}-`: `{}`",
            layer.label(),
            raw.trim()
        );
    }
    if segments.len() < 2
        || !segments
            .last()
            .expect("segments should exist")
            .chars()
            .all(|ch| ch.is_ascii_digit())
    {
        bail!(
            "{} IDs must end with a numeric segment like `{expected_prefix}-001`: `{}`",
            layer.label(),
            raw.trim()
        );
    }

    Ok(normalized)
}

fn normalize_feature_kind(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed != trimmed.to_ascii_lowercase()
        || trimmed.split('-').any(|segment| segment.is_empty())
        || !trimmed
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        bail!(
            "feature `--kind` must contain only lowercase ASCII letters, numbers, and single hyphens: `{}`",
            raw.trim()
        );
    }
    Ok(trimmed.to_string())
}

fn resolve_target_path(
    workspace: &Workspace,
    layer: LookupKind,
    explicit_file: Option<&Path>,
    parsed_id: &ParsedId,
    feature_kind: Option<&str>,
) -> Result<TargetPath> {
    let default_relative = default_document_path(layer, parsed_id, feature_kind);
    let absolute = if let Some(file) = explicit_file {
        resolve_explicit_file(workspace, file)?
    } else {
        workspace.spec_root.join(default_relative)
    };

    ensure_target_within_spec_root(workspace, layer, &absolute)?;
    Ok(TargetPath { absolute })
}

fn default_document_path(
    layer: LookupKind,
    parsed_id: &ParsedId,
    feature_kind: Option<&str>,
) -> PathBuf {
    match layer {
        LookupKind::Philosophy => PathBuf::from("philosophy/foundation.yaml"),
        LookupKind::Policy => PathBuf::from("policies/policies.yaml"),
        LookupKind::Requirement => PathBuf::from(format!(
            "requirements/{}/{}.yaml",
            parsed_id.folder_slug, parsed_id.file_slug
        )),
        LookupKind::Feature => {
            let kind = feature_kind.expect("feature default paths require a kind");
            PathBuf::from(format!("features/{kind}/{}.yaml", parsed_id.file_slug))
        }
    }
}

fn resolve_explicit_file(workspace: &Workspace, raw: &Path) -> Result<PathBuf> {
    if raw.is_absolute() {
        bail!(
            "`--file` must stay inside `{}` as a relative path",
            workspace.spec_root.display()
        );
    }

    let normalized = normalize_relative_path(raw);
    if normalized.as_os_str().is_empty()
        || normalized.is_absolute()
        || normalized
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!(
            "`--file` must stay within the workspace or configured spec root: `{}`",
            raw.display()
        );
    }

    let workspace_relative = workspace.root.join(&normalized);
    if workspace_relative.starts_with(&workspace.spec_root) {
        return Ok(workspace_relative);
    }

    Ok(workspace.spec_root.join(normalized))
}

fn ensure_target_within_spec_root(
    workspace: &Workspace,
    layer: LookupKind,
    path: &Path,
) -> Result<()> {
    if !path.starts_with(&workspace.spec_root) {
        bail!(
            "target path `{}` must stay within `{}`",
            path.display(),
            workspace.spec_root.display()
        );
    }
    if !matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("yaml" | "yml")
    ) {
        bail!(
            "target path must point to a YAML document: `{}`",
            path.display()
        );
    }

    let layer_root = match layer {
        LookupKind::Philosophy => workspace.spec_root.join("philosophy"),
        LookupKind::Policy => workspace.spec_root.join("policies"),
        LookupKind::Requirement => workspace.spec_root.join("requirements"),
        LookupKind::Feature => workspace.spec_root.join("features"),
    };
    if !path.starts_with(&layer_root) {
        bail!(
            "{} stubs must stay under `{}`",
            layer.label(),
            layer_root.display()
        );
    }
    if layer == LookupKind::Feature && path == workspace.spec_root.join("features/features.yaml") {
        bail!("feature stubs must use a feature document path, not `features/features.yaml`");
    }

    Ok(())
}

fn write_stub_document(
    layer: LookupKind,
    parsed_id: &ParsedId,
    feature_kind: Option<&str>,
    target: &TargetPath,
) -> Result<()> {
    target
        .absolute
        .parent()
        .map(fs::create_dir_all)
        .transpose()?;

    if target.absolute.exists() {
        validate_existing_document(layer, &target.absolute, parsed_id)?;
        append_yaml_list_item(&target.absolute, render_item_block(layer, parsed_id))?;
    } else {
        let document = render_new_document(layer, parsed_id, feature_kind);
        fs::write(&target.absolute, document)?;
    }

    Ok(())
}

fn validate_existing_document(layer: LookupKind, path: &Path, parsed_id: &ParsedId) -> Result<()> {
    let raw = fs::read_to_string(path).with_context(|| {
        format!(
            "failed to read {} document `{}`",
            layer.label(),
            path.display()
        )
    })?;

    match layer {
        LookupKind::Philosophy => {
            let _: PhilosophyDocument = serde_yaml::from_str(&raw).with_context(|| {
                format!(
                    "failed to parse philosophy document from `{}`",
                    path.display()
                )
            })?;
        }
        LookupKind::Policy => {
            let _: PolicyDocument = serde_yaml::from_str(&raw).with_context(|| {
                format!("failed to parse policy document from `{}`", path.display())
            })?;
        }
        LookupKind::Requirement => {
            let document: RequirementDocument = serde_yaml::from_str(&raw).with_context(|| {
                format!(
                    "failed to parse requirement document from `{}`",
                    path.display()
                )
            })?;
            if document.prefix.trim() != parsed_id.typed_prefix {
                bail!(
                    "requirement document `{}` uses prefix `{}`, which does not match `{}`",
                    path.display(),
                    document.prefix,
                    parsed_id.typed_prefix
                );
            }
        }
        LookupKind::Feature => {
            let _: FeatureDocument = serde_yaml::from_str(&raw).with_context(|| {
                format!("failed to parse feature document from `{}`", path.display())
            })?;
        }
    }

    Ok(())
}

fn append_yaml_list_item(path: &Path, item_block: String) -> Result<()> {
    let mut existing = fs::read_to_string(path)
        .with_context(|| format!("failed to read YAML document `{}`", path.display()))?;
    while existing.ends_with('\n') {
        existing.pop();
    }
    existing.push_str("\n\n");
    existing.push_str(&item_block);
    existing.push('\n');
    fs::write(path, existing)
        .with_context(|| format!("failed to update YAML document `{}`", path.display()))
}

fn render_new_document(
    layer: LookupKind,
    parsed_id: &ParsedId,
    feature_kind: Option<&str>,
) -> String {
    match layer {
        LookupKind::Philosophy => format!(
            "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n{}\n",
            render_item_block(layer, parsed_id)
        ),
        LookupKind::Policy => format!(
            "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n{}\n",
            render_item_block(layer, parsed_id)
        ),
        LookupKind::Requirement => format!(
            "category: {} Requirements\nprefix: {}\n\nrequirements:\n{}\n",
            title_case_slug(&parsed_id.folder_slug),
            parsed_id.typed_prefix,
            render_item_block(layer, parsed_id)
        ),
        LookupKind::Feature => format!(
            "category: {} Features\nversion: 1\n\nfeatures:\n{}\n",
            title_case_slug(feature_kind.unwrap_or(parsed_id.folder_slug.as_str())),
            render_item_block(layer, parsed_id)
        ),
    }
}

fn reciprocal_link_fields(layer: LookupKind) -> &'static [ReciprocalLinkField] {
    match layer {
        LookupKind::Philosophy => PHILOSOPHY_RECIPROCAL_LINK_FIELDS,
        LookupKind::Policy => POLICY_RECIPROCAL_LINK_FIELDS,
        LookupKind::Requirement => REQUIREMENT_RECIPROCAL_LINK_FIELDS,
        LookupKind::Feature => FEATURE_RECIPROCAL_LINK_FIELDS,
    }
}

fn render_reciprocal_link_stub(layer: LookupKind) -> String {
    reciprocal_link_fields(layer)
        .iter()
        .map(|field| format!("    {}: []", field.yaml_key))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_instruction_list(items: &[String]) -> String {
    let (last, head) = items
        .split_last()
        .expect("instruction lists should contain at least one item");
    if head.is_empty() {
        last.clone()
    } else {
        format!("{} and {last}", head.join(", "))
    }
}

fn suggested_linked_kinds(layer: LookupKind) -> &'static [LookupKind] {
    match layer {
        LookupKind::Philosophy => &[LookupKind::Policy],
        // Policies usually attach to an existing project philosophy; synthesizing a
        // "matching" philosophy ID from a policy ID is too misleading to suggest.
        LookupKind::Policy => &[LookupKind::Requirement],
        LookupKind::Requirement => &[LookupKind::Policy, LookupKind::Feature],
        LookupKind::Feature => &[LookupKind::Requirement],
    }
}

fn suggested_linked_id(id: &str, kind: LookupKind) -> String {
    let mut segments = id.split('-').collect::<Vec<_>>();
    segments[0] = match kind {
        LookupKind::Philosophy => "PHIL",
        LookupKind::Policy => "POL",
        LookupKind::Requirement => "REQ",
        LookupKind::Feature => "FEAT",
    };
    segments.join("-")
}

fn scaffold_missing_link_instruction(
    workspace: &Workspace,
    layer: LookupKind,
    id: &str,
) -> Option<String> {
    let lookup = WorkspaceLookup::new(workspace);
    let workspace_arg = shell_quote_path(&workspace.root);
    let linked_kinds = suggested_linked_kinds(layer)
        .iter()
        .filter_map(|kind| {
            let suggested_id = suggested_linked_id(id, *kind);
            (lookup.find(&suggested_id).is_none()).then_some((*kind, suggested_id))
        })
        .collect::<Vec<_>>();

    if linked_kinds.is_empty() {
        return None;
    }

    let commands = linked_kinds
        .iter()
        .map(|(kind, suggested_id)| {
            format!(
                "`syu add {} {} {workspace_arg}`",
                kind.label(),
                suggested_id
            )
        })
        .collect::<Vec<_>>();

    if linked_kinds.len() == 1 {
        let (kind, _) = linked_kinds[0];
        Some(format!(
            "If the linked {} stub does not exist yet, scaffold it with {}.",
            kind.label(),
            commands[0]
        ))
    } else {
        let linked_labels = linked_kinds
            .iter()
            .map(|(kind, _)| kind.label().to_string())
            .collect::<Vec<_>>();
        Some(format!(
            "If linked {} stubs are still missing, scaffold them with {}.",
            format_instruction_list(&linked_labels),
            format_instruction_list(&commands)
        ))
    }
}

fn reciprocal_link_entry_instruction(layer: LookupKind, id: &str) -> String {
    let fields = reciprocal_link_fields(layer)
        .iter()
        .map(|field| format!("`{}:`", field.yaml_key))
        .collect::<Vec<_>>();
    let (first, rest) = fields
        .split_first()
        .expect("each layer should define at least one reciprocal link field");

    if rest.is_empty() {
        format!("Add at least one {first} entry in `{id}`.")
    } else {
        let remaining = rest
            .iter()
            .map(|field| format!("one {field} entry"))
            .collect::<Vec<_>>();
        format!(
            "Add at least one {first} entry and {} in `{id}`.",
            format_instruction_list(&remaining)
        )
    }
}

fn reciprocal_link_back_instruction(layer: LookupKind, id: &str) -> String {
    let linked_kinds = reciprocal_link_fields(layer)
        .iter()
        .map(|field| field.linked_kind_label.to_string())
        .collect::<Vec<_>>();
    let verb = if linked_kinds.len() == 1 {
        "it links"
    } else {
        "they link"
    };

    format!(
        "Update each linked {} so {verb} back to `{id}`.",
        format_instruction_list(&linked_kinds)
    )
}

fn render_item_block(layer: LookupKind, parsed_id: &ParsedId) -> String {
    let reciprocal_links = render_reciprocal_link_stub(layer);
    match layer {
        LookupKind::Philosophy => format!(
            "  - id: {}\n    title: {}\n    product_design_principle: |\n      Describe the durable product design principle for {}.\n    coding_guideline: |\n      Describe the coding guideline that keeps {} actionable in day-to-day work.\n{}",
            parsed_id.normalized,
            parsed_id.title,
            parsed_id.normalized,
            parsed_id.normalized,
            reciprocal_links
        ),
        LookupKind::Policy => format!(
            "  - id: {}\n    title: {}\n    summary: Document the rule enforced by {}.\n    description: |\n      Describe how this policy turns philosophy into a concrete contributor rule.\n{}",
            parsed_id.normalized, parsed_id.title, parsed_id.normalized, reciprocal_links
        ),
        LookupKind::Requirement => format!(
            "  - id: {}\n    title: {}\n    description: |\n      Describe the concrete requirement that {} adds to the repository.\n    priority: high\n    status: planned\n{}\n    tests: {{}}",
            parsed_id.normalized, parsed_id.title, parsed_id.normalized, reciprocal_links
        ),
        LookupKind::Feature => format!(
            "  - id: {}\n    title: {}\n    summary: Describe the shipped capability that {} represents.\n    status: planned\n{}\n    implementations: {{}}",
            parsed_id.normalized, parsed_id.title, parsed_id.normalized, reciprocal_links
        ),
    }
}

fn prepare_feature_registry_update(
    workspace: &Workspace,
    feature_document: &Path,
    kind: &str,
) -> Result<FeatureRegistryUpdate> {
    let registry_path = workspace.spec_root.join("features/features.yaml");
    let raw = fs::read_to_string(&registry_path).with_context(|| {
        format!(
            "failed to read feature registry `{}`",
            registry_path.display()
        )
    })?;
    let registry: FeatureRegistryDocument = serde_yaml::from_str(&raw).with_context(|| {
        format!(
            "failed to parse feature registry from `{}`",
            registry_path.display()
        )
    })?;
    let relative = feature_document
        .strip_prefix(workspace.spec_root.join("features"))
        .with_context(|| {
            format!(
                "feature document `{}` must stay under `{}`",
                feature_document.display(),
                workspace.spec_root.join("features").display()
            )
        })?;
    let portable = path_label(relative);

    if let Some(existing) = registry
        .files
        .iter()
        .find(|entry| path_label(&entry.file) == portable)
    {
        if existing.kind != kind {
            bail!(
                "feature registry already tracks `{portable}` under kind `{}`",
                existing.kind
            );
        }
        return Ok(FeatureRegistryUpdate {
            path: registry_path,
            updated_contents: None,
        });
    }

    let mut updated = raw;
    while updated.ends_with('\n') {
        updated.pop();
    }
    updated.push_str(&format!("\n  - kind: {kind}\n    file: {portable}\n"));
    Ok(FeatureRegistryUpdate {
        path: registry_path,
        updated_contents: Some(updated),
    })
}

fn write_feature_registry_update(update: FeatureRegistryUpdate) -> Result<bool> {
    let Some(updated) = update.updated_contents else {
        return Ok(false);
    };
    fs::write(&update.path, updated).with_context(|| {
        format!(
            "failed to update feature registry `{}`",
            update.path.display()
        )
    })?;
    Ok(true)
}

fn print_add_summary(
    workspace: &Workspace,
    layer: LookupKind,
    id: &str,
    target: &Path,
    created_target_file: bool,
    registry_updated: bool,
) {
    let action = if created_target_file {
        "created"
    } else {
        "updated"
    };
    println!(
        "{action} {} stub `{id}` in {}",
        layer.label(),
        display_workspace_path(workspace, target)
    );
    if registry_updated {
        println!(
            "updated {}",
            display_workspace_path(
                workspace,
                &workspace.spec_root.join("features/features.yaml")
            )
        );
    }
    println!();
    println!("Next steps:");
    for (index, step) in add_follow_up_steps(workspace, layer, id, target)
        .into_iter()
        .enumerate()
    {
        println!("  {}. {step}", index + 1);
    }
}

fn add_follow_up_steps(
    workspace: &Workspace,
    layer: LookupKind,
    id: &str,
    target: &Path,
) -> Vec<String> {
    let target_label = display_workspace_path(workspace, target);
    let mut steps = vec![format!(
        "Edit {target_label} and fill the stub fields for `{id}`."
    )];
    steps.push(reciprocal_link_entry_instruction(layer, id));
    if let Some(scaffold_instruction) = scaffold_missing_link_instruction(workspace, layer, id) {
        steps.push(scaffold_instruction);
    }
    steps.push(reciprocal_link_back_instruction(layer, id));

    steps.push(format!(
        "Run `syu validate {}` once the reciprocal links are in place.",
        shell_quote_path(&workspace.root)
    ));
    steps
}

fn display_workspace_path(workspace: &Workspace, path: &Path) -> String {
    path.strip_prefix(&workspace.root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn path_label(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn default_title(layer: LookupKind) -> String {
    match layer {
        LookupKind::Philosophy => "New philosophy".to_string(),
        LookupKind::Policy => "New policy".to_string(),
        LookupKind::Requirement => "New requirement".to_string(),
        LookupKind::Feature => "New feature".to_string(),
    }
}

fn default_folder_slug(layer: LookupKind) -> String {
    match layer {
        LookupKind::Philosophy => "philosophy".to_string(),
        LookupKind::Policy => "policies".to_string(),
        LookupKind::Requirement | LookupKind::Feature => "core".to_string(),
    }
}

fn title_case_tokens(tokens: &[&str]) -> String {
    tokens
        .iter()
        .map(|segment| title_case_slug(&segment.to_ascii_lowercase()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn title_case_slug(slug: &str) -> String {
    slug.split('-')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            let first = chars.next().expect("empty segments are filtered out");
            format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::{
        collections::BTreeMap,
        collections::VecDeque,
        fs,
        path::{Path, PathBuf},
    };

    use tempfile::{TempDir, tempdir};

    use crate::{
        cli::{AddArgs, InitArgs, LookupKind, OutputFormat, StarterTemplate},
        command::prompt::PromptIo,
        config::SyuConfig,
        model::Requirement,
        workspace::Workspace,
    };

    use super::{
        FeatureRegistryUpdate, ParsedId, TargetPath, add_follow_up_steps, default_document_path,
        default_folder_slug, default_title, ensure_target_within_spec_root,
        normalize_definition_id, normalize_feature_kind, prepare_feature_registry_update,
        prompt_for_parsed_id, render_item_block, render_new_document,
        resolve_add_invocation_with_prompt_io, resolve_explicit_file, resolve_feature_kind,
        resolve_interactive_file_prompt, run_add_command, suggested_linked_id, title_case_slug,
        validate_existing_document, write_feature_registry_update, write_stub_document,
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

        fn prompt_line(&mut self, label: &str, default: Option<&str>) -> Result<String> {
            self.prompts.push((
                label.to_string(),
                default.map(std::string::ToString::to_string),
            ));
            Ok(self.lines.pop_front().unwrap_or_default())
        }
    }

    fn test_workspace() -> (TempDir, Workspace) {
        let tempdir = tempdir().expect("tempdir should exist");
        let root = tempdir.path().join("workspace");
        let spec_root = root.join("docs/spec");
        fs::create_dir_all(&spec_root).expect("spec root should be created");
        (
            tempdir,
            Workspace {
                root,
                spec_root,
                config: SyuConfig::default(),
                philosophies: Vec::new(),
                policies: Vec::new(),
                requirements: Vec::new(),
                features: Vec::new(),
            },
        )
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directory should exist");
        }
        fs::write(path, contents).expect("test fixture file should be written");
    }

    #[test]
    fn normalize_definition_id_enforces_expected_prefixes() {
        assert_eq!(
            normalize_definition_id("req-auth-001", LookupKind::Requirement)
                .expect("requirement ids should normalize"),
            "REQ-AUTH-001"
        );
        assert!(normalize_definition_id("FEAT-AUTH-001", LookupKind::Requirement).is_err());
        assert!(normalize_definition_id("REQ-AUTH", LookupKind::Requirement).is_err());
    }

    #[test]
    fn normalize_definition_id_covers_policy_and_invalid_character_cases() {
        assert_eq!(
            normalize_definition_id("pol-001", LookupKind::Policy)
                .expect("policy ids should normalize"),
            "POL-001"
        );
        assert!(normalize_definition_id("POL 001", LookupKind::Policy).is_err());
    }

    #[test]
    fn parsed_ids_infer_title_folder_and_file_slugs() {
        let parsed = ParsedId::parse(LookupKind::Feature, "FEAT-AUTH-LOGIN-001")
            .expect("feature ids should parse");
        assert_eq!(parsed.typed_prefix, "FEAT-AUTH-LOGIN");
        assert_eq!(parsed.title, "Auth Login");
        assert_eq!(parsed.folder_slug, "auth");
        assert_eq!(parsed.file_slug, "login");
    }

    #[test]
    fn default_document_path_uses_kind_folder_for_features() {
        let parsed =
            ParsedId::parse(LookupKind::Feature, "FEAT-AUTH-LOGIN-001").expect("feature id");
        assert_eq!(
            default_document_path(LookupKind::Feature, &parsed, Some("auth")),
            PathBuf::from("features/auth/login.yaml")
        );
    }

    #[test]
    fn helper_defaults_cover_policy_and_feature_layers() {
        let parsed = ParsedId::parse(LookupKind::Policy, "POL-001").expect("policy id");

        assert_eq!(
            default_document_path(LookupKind::Policy, &parsed, None),
            PathBuf::from("policies/policies.yaml")
        );
        assert_eq!(default_title(LookupKind::Policy), "New policy");
        assert_eq!(default_title(LookupKind::Feature), "New feature");
        assert_eq!(default_folder_slug(LookupKind::Policy), "policies");
    }

    #[test]
    fn add_follow_up_steps_explain_reciprocal_links_for_each_layer() {
        let (_tempdir, workspace) = test_workspace();
        let philosophy_target = workspace.spec_root.join("philosophy/foundation.yaml");
        let policy_target = workspace.spec_root.join("policies/policies.yaml");
        let requirement_target = workspace.spec_root.join("requirements/auth/auth.yaml");
        let feature_target = workspace.spec_root.join("features/auth/login.yaml");

        let philosophy_steps = add_follow_up_steps(
            &workspace,
            LookupKind::Philosophy,
            "PHIL-001",
            &philosophy_target,
        );
        let policy_steps =
            add_follow_up_steps(&workspace, LookupKind::Policy, "POL-001", &policy_target);
        let requirement_steps = add_follow_up_steps(
            &workspace,
            LookupKind::Requirement,
            "REQ-AUTH-001",
            &requirement_target,
        );
        let feature_steps = add_follow_up_steps(
            &workspace,
            LookupKind::Feature,
            "FEAT-AUTH-LOGIN-001",
            &feature_target,
        );

        assert!(philosophy_steps[1].contains("linked_policies"));
        assert!(philosophy_steps[2].contains("syu add policy POL-001"));
        assert!(philosophy_steps[3].contains("linked policy"));
        assert!(policy_steps[1].contains("linked_philosophies"));
        assert!(policy_steps[1].contains("linked_requirements"));
        assert!(policy_steps[2].contains("syu add requirement REQ-001"));
        assert!(!policy_steps[2].contains("syu add philosophy"));
        assert!(requirement_steps[1].contains("linked_policies"));
        assert!(requirement_steps[1].contains("linked_features"));
        assert!(requirement_steps[2].contains("syu add policy POL-AUTH-001"));
        assert!(requirement_steps[2].contains("syu add feature FEAT-AUTH-001"));
        assert!(feature_steps[1].contains("linked_requirements"));
        assert!(feature_steps[2].contains("syu add requirement REQ-AUTH-LOGIN-001"));
        assert!(feature_steps[3].contains("linked requirement"));
        assert!(
            feature_steps
                .last()
                .expect("feature guidance should include validation")
                .contains("syu validate")
        );
    }

    #[test]
    fn suggested_linked_id_rewrites_prefixes_for_each_layer() {
        assert_eq!(
            suggested_linked_id("POL-AUTH-001", LookupKind::Philosophy),
            "PHIL-AUTH-001"
        );
        assert_eq!(
            suggested_linked_id("REQ-AUTH-001", LookupKind::Policy),
            "POL-AUTH-001"
        );
        assert_eq!(
            suggested_linked_id("FEAT-AUTH-001", LookupKind::Requirement),
            "REQ-AUTH-001"
        );
        assert_eq!(
            suggested_linked_id("REQ-AUTH-001", LookupKind::Feature),
            "FEAT-AUTH-001"
        );
    }

    #[test]
    fn add_follow_up_steps_skip_existing_scaffold_commands() {
        let (_tempdir, mut workspace) = test_workspace();
        workspace.requirements.push(Requirement {
            id: "REQ-AUTH-001".to_string(),
            title: "Existing requirement".to_string(),
            description: "Already scaffolded".to_string(),
            priority: "high".to_string(),
            status: "planned".to_string(),
            linked_policies: Vec::new(),
            linked_features: Vec::new(),
            tests: BTreeMap::new(),
        });
        let policy_target = workspace.spec_root.join("policies/policies.yaml");

        let policy_steps = add_follow_up_steps(
            &workspace,
            LookupKind::Policy,
            "POL-AUTH-001",
            &policy_target,
        );

        assert_eq!(policy_steps.len(), 4);
        assert!(
            !policy_steps
                .iter()
                .any(|step| step.contains("syu add requirement REQ-AUTH-001"))
        );
        assert!(
            !policy_steps
                .iter()
                .any(|step| step.contains("syu add philosophy"))
        );
    }

    #[test]
    fn resolve_explicit_file_accepts_workspace_and_spec_relative_paths() {
        let (_tempdir, workspace) = test_workspace();

        assert_eq!(
            resolve_explicit_file(
                &workspace,
                std::path::Path::new("docs/spec/features/auth/login.yaml")
            )
            .expect("workspace-relative paths should work"),
            workspace.spec_root.join("features/auth/login.yaml")
        );
        assert_eq!(
            resolve_explicit_file(&workspace, std::path::Path::new("features/auth/login.yaml"))
                .expect("spec-relative paths should work"),
            workspace.spec_root.join("features/auth/login.yaml")
        );
    }

    #[test]
    fn resolve_explicit_file_rejects_absolute_and_invalid_paths() {
        let (_tempdir, workspace) = test_workspace();

        assert!(resolve_explicit_file(&workspace, Path::new("/tmp/escape.yaml")).is_err());
        assert!(resolve_explicit_file(&workspace, Path::new("../escape.yaml")).is_err());
    }

    #[test]
    fn normalize_feature_kind_rejects_invalid_values() {
        assert_eq!(
            normalize_feature_kind("auth-login").expect("valid kind"),
            "auth-login"
        );
        assert!(normalize_feature_kind("Auth").is_err());
        assert!(normalize_feature_kind("auth login").is_err());
    }

    #[test]
    fn title_case_slug_expands_hyphenated_names() {
        assert_eq!(title_case_slug("auth-login"), "Auth Login");
    }

    #[test]
    fn ensure_target_within_spec_root_rejects_invalid_targets() {
        let (_tempdir, workspace) = test_workspace();

        assert!(
            ensure_target_within_spec_root(
                &workspace,
                LookupKind::Requirement,
                &workspace.root.join("outside.yaml")
            )
            .is_err()
        );
        assert!(
            ensure_target_within_spec_root(
                &workspace,
                LookupKind::Requirement,
                &workspace.spec_root.join("requirements/core.txt")
            )
            .is_err()
        );
        assert!(
            ensure_target_within_spec_root(
                &workspace,
                LookupKind::Policy,
                &workspace.spec_root.join("requirements/core.yaml")
            )
            .is_err()
        );
        assert!(
            ensure_target_within_spec_root(
                &workspace,
                LookupKind::Policy,
                &workspace.spec_root.join("policies/policies.yaml")
            )
            .is_ok()
        );
    }

    #[test]
    fn write_stub_document_creates_new_parent_directories() {
        let (_tempdir, workspace) = test_workspace();
        let parsed = ParsedId::parse(LookupKind::Requirement, "REQ-AUTH-001").expect("id");
        let target = TargetPath {
            absolute: workspace.spec_root.join("requirements/auth/auth.yaml"),
        };

        write_stub_document(LookupKind::Requirement, &parsed, None, &target)
            .expect("stub write should succeed");

        assert!(target.absolute.exists());
    }

    #[test]
    fn validate_existing_document_reports_read_errors() {
        let (_tempdir, workspace) = test_workspace();
        let parsed = ParsedId::parse(LookupKind::Philosophy, "PHIL-001").expect("id");

        assert!(
            validate_existing_document(
                LookupKind::Philosophy,
                &workspace.spec_root.join("philosophy/missing.yaml"),
                &parsed
            )
            .is_err()
        );
    }

    #[test]
    fn validate_existing_document_rejects_invalid_yaml_for_all_layers() {
        let (_tempdir, workspace) = test_workspace();
        let invalid = "not: [valid";

        let philosophy_path = workspace.spec_root.join("philosophy/foundation.yaml");
        write_file(&philosophy_path, invalid);
        assert!(
            validate_existing_document(
                LookupKind::Philosophy,
                &philosophy_path,
                &ParsedId::parse(LookupKind::Philosophy, "PHIL-001").expect("philosophy id")
            )
            .is_err()
        );

        let policy_path = workspace.spec_root.join("policies/policies.yaml");
        write_file(&policy_path, invalid);
        assert!(
            validate_existing_document(
                LookupKind::Policy,
                &policy_path,
                &ParsedId::parse(LookupKind::Policy, "POL-001").expect("policy id")
            )
            .is_err()
        );

        let requirement_path = workspace.spec_root.join("requirements/core/core.yaml");
        write_file(&requirement_path, invalid);
        assert!(
            validate_existing_document(
                LookupKind::Requirement,
                &requirement_path,
                &ParsedId::parse(LookupKind::Requirement, "REQ-CORE-001").expect("requirement id")
            )
            .is_err()
        );

        let feature_path = workspace.spec_root.join("features/core/core.yaml");
        write_file(&feature_path, invalid);
        assert!(
            validate_existing_document(
                LookupKind::Feature,
                &feature_path,
                &ParsedId::parse(LookupKind::Feature, "FEAT-CORE-001").expect("feature id")
            )
            .is_err()
        );
    }

    #[test]
    fn validate_existing_document_rejects_requirement_prefix_mismatches() {
        let (_tempdir, workspace) = test_workspace();
        let path = workspace.spec_root.join("requirements/core/core.yaml");
        write_file(
            &path,
            "category: Core Requirements\nprefix: REQ-OTHER\n\nrequirements:\n  - id: REQ-OTHER-001\n    title: Other\n    description: |\n      Example.\n    priority: high\n    status: planned\n    linked_policies: []\n    linked_features: []\n    tests: {}\n",
        );

        assert!(
            validate_existing_document(
                LookupKind::Requirement,
                &path,
                &ParsedId::parse(LookupKind::Requirement, "REQ-CORE-001").expect("requirement id")
            )
            .is_err()
        );
    }

    #[test]
    fn validate_existing_document_accepts_matching_requirement_prefixes() {
        let (_tempdir, workspace) = test_workspace();
        let path = workspace.spec_root.join("requirements/core/core.yaml");
        write_file(
            &path,
            "category: Core Requirements\nprefix: REQ-CORE\n\nrequirements:\n  - id: REQ-CORE-001\n    title: Core\n    description: |\n      Example.\n    priority: high\n    status: planned\n    linked_policies: []\n    linked_features: []\n    tests: {}\n",
        );

        validate_existing_document(
            LookupKind::Requirement,
            &path,
            &ParsedId::parse(LookupKind::Requirement, "REQ-CORE-001").expect("requirement id"),
        )
        .expect("matching prefixes should validate");
    }

    #[test]
    fn render_helpers_cover_non_requirement_layers() {
        let philosophy = ParsedId::parse(LookupKind::Philosophy, "PHIL-001").expect("id");
        let policy = ParsedId::parse(LookupKind::Policy, "POL-001").expect("id");
        let feature = ParsedId::parse(LookupKind::Feature, "FEAT-AUTH-LOGIN-001").expect("id");

        assert!(
            render_new_document(LookupKind::Philosophy, &philosophy, None)
                .contains("category: Philosophy")
        );
        assert!(
            render_new_document(LookupKind::Policy, &policy, None).contains("category: Policies")
        );
        assert!(render_item_block(LookupKind::Policy, &policy).contains("linked_requirements"));
        assert!(
            render_new_document(LookupKind::Feature, &feature, Some("auth"))
                .contains("category: Auth Features")
        );
    }

    #[test]
    fn prepare_feature_registry_update_reports_read_parse_and_root_errors() {
        let (_tempdir, workspace) = test_workspace();
        let feature_path = workspace.spec_root.join("features/auth/login.yaml");

        assert!(prepare_feature_registry_update(&workspace, &feature_path, "auth").is_err());

        let registry_path = workspace.spec_root.join("features/features.yaml");
        write_file(&registry_path, "version: 1\nfiles: [");
        assert!(prepare_feature_registry_update(&workspace, &feature_path, "auth").is_err());

        write_file(
            &registry_path,
            "version: \"1\"\nfiles:\n  - kind: core\n    file: core/core.yaml\n",
        );
        assert!(
            prepare_feature_registry_update(
                &workspace,
                &workspace.spec_root.join("requirements/core/core.yaml"),
                "auth"
            )
            .is_err()
        );
    }

    #[test]
    fn write_feature_registry_update_reports_write_errors() {
        let tempdir = tempdir().expect("tempdir should exist");
        let update = FeatureRegistryUpdate {
            path: tempdir.path().to_path_buf(),
            updated_contents: Some("version: \"1\"\nfiles: []\n".to_string()),
        };

        assert!(write_feature_registry_update(update).is_err());
    }

    #[test]
    fn run_add_command_scaffolds_requirement_stubs_in_initialized_workspaces() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path().join("workspace");
        crate::command::init::run_init_command(&InitArgs {
            workspace: workspace.clone(),
            interactive: false,
            name: Some("workspace".to_string()),
            spec_root: None,
            template: StarterTemplate::Generic,
            id_prefix: None,
            philosophy_prefix: None,
            policy_prefix: None,
            requirement_prefix: None,
            feature_prefix: None,
            force: false,
            format: OutputFormat::Text,
        })
        .expect("workspace init should succeed");

        let code = run_add_command(&AddArgs {
            layer: LookupKind::Requirement,
            id: Some("REQ-AUTH-001".to_string()),
            workspace: workspace.clone(),
            interactive: false,
            file: None,
            kind: None,
        })
        .expect("add command should succeed");
        assert_eq!(code, 0);
        assert!(
            workspace
                .join("docs/syu/requirements/auth/auth.yaml")
                .exists()
        );
    }

    #[test]
    fn resolve_add_invocation_reports_missing_id_on_non_terminal_streams() {
        let mut prompt_io = FakePromptIo {
            terminal: false,
            ..Default::default()
        };
        let error = resolve_add_invocation_with_prompt_io(
            &AddArgs {
                layer: LookupKind::Requirement,
                id: None,
                workspace: PathBuf::from("."),
                interactive: false,
                file: None,
                kind: None,
            },
            &mut prompt_io,
        )
        .expect_err("non-terminal add runs should require an explicit ID");

        assert!(
            error
                .to_string()
                .contains("needs a definition ID when stdin/stdout are not terminals")
        );
        assert!(prompt_io.prompts.is_empty());
    }

    #[test]
    fn prompt_for_parsed_id_retries_blank_and_invalid_values() {
        let mut prompt_io = FakePromptIo {
            terminal: true,
            lines: VecDeque::from([
                String::new(),
                "not-an-id".to_string(),
                "REQ-AUTH-001".to_string(),
            ]),
            ..Default::default()
        };

        let parsed =
            prompt_for_parsed_id(LookupKind::Requirement, &mut prompt_io).expect("prompt succeeds");

        assert_eq!(parsed.normalized, "REQ-AUTH-001");
        assert_eq!(prompt_io.prompts.len(), 3);
        assert!(
            prompt_io
                .prompts
                .iter()
                .all(|(label, default)| label == "Definition ID" && default.is_none())
        );
    }

    #[test]
    fn resolve_feature_kind_interactive_retries_invalid_values_and_accepts_defaults() {
        let parsed = ParsedId::parse(LookupKind::Feature, "FEAT-AUTH-LOGIN-001").expect("id");
        let mut prompt_io = FakePromptIo {
            terminal: true,
            lines: VecDeque::from(["Auth".to_string(), String::new()]),
            ..Default::default()
        };

        let kind = resolve_feature_kind(
            &AddArgs {
                layer: LookupKind::Feature,
                id: Some(parsed.normalized.clone()),
                workspace: PathBuf::from("."),
                interactive: true,
                file: None,
                kind: None,
            },
            &parsed,
            &mut prompt_io,
        )
        .expect("feature kind prompts should recover");

        assert_eq!(kind, "auth");
        assert_eq!(
            prompt_io.prompts,
            vec![
                ("Feature kind".to_string(), Some("auth".to_string())),
                ("Feature kind".to_string(), Some("auth".to_string())),
            ]
        );
    }

    #[test]
    fn resolve_interactive_file_prompt_returns_none_for_blank_overrides() {
        let parsed = ParsedId::parse(LookupKind::Feature, "FEAT-AUTH-LOGIN-001").expect("id");
        let mut prompt_io = FakePromptIo {
            terminal: true,
            lines: VecDeque::from([String::new()]),
            ..Default::default()
        };

        let file = resolve_interactive_file_prompt(
            &AddArgs {
                layer: LookupKind::Feature,
                id: Some(parsed.normalized.clone()),
                workspace: PathBuf::from("."),
                interactive: true,
                file: None,
                kind: None,
            },
            &parsed,
            Some("auth"),
            &mut prompt_io,
        )
        .expect("file prompts should succeed");

        assert_eq!(file, None);
        assert_eq!(
            prompt_io.prompts,
            vec![(
                "YAML file".to_string(),
                Some("features/auth/login.yaml".to_string())
            )]
        );
    }
}
