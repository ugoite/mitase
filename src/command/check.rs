// FEAT-CHECK-001
// REQ-CORE-001

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::{
    cli::{CheckArgs, OutputFormat},
    config::SyuConfig,
    coverage::validate_symbol_trace_coverage,
    inspect::{apply_symbol_doc_fix, inspect_symbol, supports_rich_inspection},
    language::adapter_for_language,
    model::{
        CheckResult, DefinitionCounts, Feature, Issue, Philosophy, Policy, Requirement, TraceCount,
        TraceReference, TraceSummary,
    },
    rules::{attach_referenced_rules, rule_by_code},
    workspace::{Workspace, load_workspace},
};

#[derive(Debug, Clone, Copy)]
enum TraceRole {
    RequirementTest,
    FeatureImplementation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeliveryStatus {
    Planned,
    Implemented,
}

#[derive(Debug, Clone, Copy)]
struct TraceValidationTarget<'a> {
    owner_id: &'a str,
    role: TraceRole,
    status: Option<DeliveryStatus>,
}

#[derive(Debug, Default, Clone)]
struct AutofixSummary {
    updated_files: BTreeSet<PathBuf>,
    symbol_updates: usize,
}

impl TraceRole {
    fn label(self) -> &'static str {
        match self {
            Self::RequirementTest => "test",
            Self::FeatureImplementation => "implementation",
        }
    }

    fn subject_kind(self) -> &'static str {
        match self {
            Self::RequirementTest => "requirement",
            Self::FeatureImplementation => "feature",
        }
    }

    fn relation_name(self) -> &'static str {
        match self {
            Self::RequirementTest => "tests",
            Self::FeatureImplementation => "implementations",
        }
    }
}

// FEAT-CHECK-001
pub fn run_check_command(args: &CheckArgs) -> Result<i32> {
    let (result, fix_summary) = match load_workspace(&args.workspace) {
        Ok(workspace) => {
            let should_fix = effective_fix(args, &workspace.config);
            let fix_summary = if should_fix {
                Some(apply_autofix(&workspace)?)
            } else {
                None
            };
            let result = if should_fix {
                collect_check_result(&args.workspace)
            } else {
                collect_check_result_from_workspace(&workspace)
            };
            (result, fix_summary)
        }
        Err(error) => (
            CheckResult::from_load_error(args.workspace.to_path_buf(), error.to_string()),
            None,
        ),
    };

    match args.format {
        OutputFormat::Text => {
            if let Some(summary) = fix_summary
                && !summary.updated_files.is_empty()
            {
                println!(
                    "applied {} autofix updates across {} files",
                    summary.symbol_updates,
                    summary.updated_files.len()
                );
            }
            print!("{}", render_text_report(&result));
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&result)
                    .expect("serializing CheckResult to JSON should succeed")
            );
        }
    }

    Ok(if result.is_success() { 0 } else { 1 })
}

// FEAT-CHECK-001
pub fn collect_check_result(workspace_path: &Path) -> CheckResult {
    match load_workspace(workspace_path) {
        Ok(workspace) => collect_check_result_from_workspace(&workspace),
        Err(error) => attach_referenced_rules(CheckResult::from_load_error(
            workspace_path.to_path_buf(),
            error.to_string(),
        )),
    }
}

fn collect_check_result_from_workspace(workspace: &Workspace) -> CheckResult {
    let definition_counts = DefinitionCounts {
        philosophies: workspace.philosophies.len(),
        policies: workspace.policies.len(),
        requirements: workspace.requirements.len(),
        features: workspace.features.len(),
    };

    let mut issues = Vec::new();
    let mut trace_summary = TraceSummary::default();

    validate_unique_ids(
        "philosophy",
        workspace.philosophies.iter().map(|item| item.id.as_str()),
        &mut issues,
    );
    validate_unique_ids(
        "policy",
        workspace.policies.iter().map(|item| item.id.as_str()),
        &mut issues,
    );
    validate_unique_ids(
        "requirement",
        workspace.requirements.iter().map(|item| item.id.as_str()),
        &mut issues,
    );
    validate_unique_ids(
        "feature",
        workspace.features.iter().map(|item| item.id.as_str()),
        &mut issues,
    );

    let philosophies_by_id: HashMap<_, _> = workspace
        .philosophies
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect();
    let policies_by_id: HashMap<_, _> = workspace
        .policies
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect();
    let requirements_by_id: HashMap<_, _> = workspace
        .requirements
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect();
    let features_by_id: HashMap<_, _> = workspace
        .features
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect();

    for philosophy in &workspace.philosophies {
        validate_philosophy(philosophy, &policies_by_id, &mut issues);
    }

    for policy in &workspace.policies {
        validate_policy(
            policy,
            &philosophies_by_id,
            &requirements_by_id,
            &mut issues,
        );
    }

    for requirement in &workspace.requirements {
        validate_requirement(
            requirement,
            &policies_by_id,
            &features_by_id,
            &workspace.config,
            &workspace.root,
            &mut issues,
            &mut trace_summary.requirement_traces,
        );
    }

    for feature in &workspace.features {
        validate_feature(
            feature,
            &requirements_by_id,
            &workspace.config,
            &workspace.root,
            &mut issues,
            &mut trace_summary.feature_traces,
        );
    }

    validate_orphaned_definitions(workspace, &mut issues);
    validate_symbol_trace_coverage(workspace, &mut issues);

    issues.sort_by(|left, right| {
        (
            format!("{:?}", left.severity),
            left.code.as_str(),
            left.subject.as_str(),
            left.location.as_deref().unwrap_or(""),
            left.message.as_str(),
        )
            .cmp(&(
                format!("{:?}", right.severity),
                right.code.as_str(),
                right.subject.as_str(),
                right.location.as_deref().unwrap_or(""),
                right.message.as_str(),
            ))
    });

    attach_referenced_rules(CheckResult {
        workspace_root: workspace.root.clone(),
        definition_counts,
        trace_summary,
        issues,
        referenced_rules: Vec::new(),
    })
}

fn effective_fix(args: &CheckArgs, config: &SyuConfig) -> bool {
    if args.fix {
        true
    } else if args.no_fix {
        false
    } else {
        config.validate.default_fix
    }
}

#[allow(clippy::question_mark)]
fn apply_autofix(workspace: &Workspace) -> Result<AutofixSummary> {
    let mut summary = AutofixSummary::default();

    for requirement in &workspace.requirements {
        if normalize_delivery_status(&requirement.status) != Some(DeliveryStatus::Implemented) {
            continue;
        }
        if let Err(error) = apply_autofix_for_trace_map(
            &workspace.root,
            &workspace.config,
            &requirement.id,
            &requirement.tests,
            &mut summary,
        ) {
            return Err(error);
        }
    }

    for feature in &workspace.features {
        if normalize_delivery_status(&feature.status) != Some(DeliveryStatus::Implemented) {
            continue;
        }
        if let Err(error) = apply_autofix_for_trace_map(
            &workspace.root,
            &workspace.config,
            &feature.id,
            &feature.implementations,
            &mut summary,
        ) {
            return Err(error);
        }
    }

    Ok(summary)
}

#[allow(clippy::question_mark)]
fn apply_autofix_for_trace_map(
    root: &Path,
    config: &SyuConfig,
    owner_id: &str,
    references_by_language: &BTreeMap<String, Vec<TraceReference>>,
    summary: &mut AutofixSummary,
) -> Result<()> {
    for (language, references) in references_by_language {
        for reference in references {
            if let Err(error) =
                apply_autofix_for_reference(root, config, owner_id, language, reference, summary)
            {
                return Err(error);
            }
        }
    }

    Ok(())
}

fn apply_autofix_for_reference(
    root: &Path,
    config: &SyuConfig,
    owner_id: &str,
    language: &str,
    reference: &TraceReference,
    summary: &mut AutofixSummary,
) -> Result<()> {
    let Some(adapter) = adapter_for_language(language) else {
        return Ok(());
    };

    if reference.file.as_os_str().is_empty() || reference.symbols.is_empty() {
        return Ok(());
    }

    let path = root.join(&reference.file);
    if !path.is_file() || !adapter.supports_path(&path) {
        return Ok(());
    }

    let mut contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(_) => return Ok(()),
    };
    let mut changed = false;
    let mut updated_symbols = 0;

    for symbol in reference
        .symbols
        .iter()
        .map(|symbol| symbol.trim())
        .filter(|symbol| !symbol.is_empty() && *symbol != "*")
    {
        let mut required = reference.doc_contains.clone();
        if !contents.contains(owner_id) {
            required.push(owner_id.to_string());
        }
        if required.is_empty() {
            continue;
        }

        let updated =
            match apply_symbol_doc_fix(language, config, &path, &contents, symbol, &required) {
                Ok(Some(updated)) => updated,
                Ok(None) => continue,
                Err(error) => return Err(error),
            };

        contents = updated;
        if let Err(error) = fs::write(&path, &contents) {
            return Err(error.into());
        }
        changed = true;
        updated_symbols += 1;
    }

    if changed {
        summary.updated_files.insert(path);
        summary.symbol_updates += updated_symbols;
    }

    Ok(())
}

fn render_text_report(result: &CheckResult) -> String {
    let mut output = String::new();
    let status = if result.is_success() {
        "passed"
    } else {
        "failed"
    };

    writeln!(&mut output, "syu validate {status}").expect("writing to String must succeed");
    writeln!(
        &mut output,
        "workspace: {}",
        result.workspace_root.display()
    )
    .expect("writing to String must succeed");
    writeln!(
        &mut output,
        "definitions: philosophies={} policies={} requirements={} features={}",
        result.definition_counts.philosophies,
        result.definition_counts.policies,
        result.definition_counts.requirements,
        result.definition_counts.features
    )
    .expect("writing to String must succeed");
    writeln!(
        &mut output,
        "traceability: requirements={}/{} features={}/{}",
        result.trace_summary.requirement_traces.validated,
        result.trace_summary.requirement_traces.declared,
        result.trace_summary.feature_traces.validated,
        result.trace_summary.feature_traces.declared
    )
    .expect("writing to String must succeed");

    if !result.issues.is_empty() {
        writeln!(&mut output).expect("writing to String must succeed");
        writeln!(&mut output, "issues:").expect("writing to String must succeed");
        for issue in &result.issues {
            let location = issue
                .location
                .as_deref()
                .map(|value| format!(" ({value})"))
                .unwrap_or_default();
            writeln!(
                &mut output,
                "- [{:?}] {}{} {}{}: {}",
                issue.severity,
                issue.code,
                rule_title_suffix(&issue.code),
                issue.subject,
                location,
                issue.message
            )
            .expect("writing to String must succeed");
            if let Some(rule) = rule_by_code(&issue.code) {
                writeln!(
                    &mut output,
                    "  rule: {} / {} / {}",
                    rule.genre, rule.code, rule.title
                )
                .expect("writing to String must succeed");
            }
        }
    }

    if !result.referenced_rules.is_empty() {
        writeln!(&mut output).expect("writing to String must succeed");
        writeln!(&mut output, "referenced rules:").expect("writing to String must succeed");
        for rule in &result.referenced_rules {
            writeln!(
                &mut output,
                "- [{}] {} ({})",
                rule.severity, rule.code, rule.genre
            )
            .expect("writing to String must succeed");
            writeln!(&mut output, "  title: {}", rule.title)
                .expect("writing to String must succeed");
            writeln!(
                &mut output,
                "  summary: {}",
                collapse_whitespace(&rule.summary)
            )
            .expect("writing to String must succeed");
            writeln!(
                &mut output,
                "  description: {}",
                collapse_whitespace(&rule.description)
            )
            .expect("writing to String must succeed");
        }
    }

    output
}

fn rule_title_suffix(code: &str) -> String {
    rule_by_code(code)
        .map(|rule| format!(" ({})", rule.title))
        .unwrap_or_default()
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_delivery_status(status: &str) -> Option<DeliveryStatus> {
    match status.trim() {
        "planned" | "planed" => Some(DeliveryStatus::Planned),
        "implemented" => Some(DeliveryStatus::Implemented),
        _ => None,
    }
}

fn validate_delivery_status(
    kind: &str,
    id: &str,
    status: &str,
    config: &SyuConfig,
    issues: &mut Vec<Issue>,
) -> Option<DeliveryStatus> {
    if status.trim().is_empty() {
        return None;
    }

    let Some(normalized) = normalize_delivery_status(status) else {
        issues.push(Issue::error(
            "invalid-status",
            format!("{kind} {id}"),
            Some("status".to_string()),
            format!(
                "{kind} `{id}` has unsupported status `{}`. Use `planned` or `implemented`.",
                status.trim()
            ),
            Some("Change the status to `planned` or `implemented`.".to_string()),
        ));
        return None;
    };

    if normalized == DeliveryStatus::Planned && !config.validate.allow_planned {
        issues.push(Issue::error(
            "planned-status-disallowed",
            format!("{kind} {id}"),
            Some("status".to_string()),
            format!("{kind} `{id}` is marked `planned`, but `syu.yaml` forbids planned items."),
            Some(format!(
                "Change `{id}` to `implemented` or set `validate.allow_planned: true`."
            )),
        ));
    }

    Some(normalized)
}

fn validate_unique_ids<'a>(
    kind: &str,
    ids: impl Iterator<Item = &'a str>,
    issues: &mut Vec<Issue>,
) {
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id.to_string()) {
            issues.push(Issue::error(
                "duplicate-id",
                format!("{kind} {id}"),
                None,
                format!("Duplicate {kind} id `{id}` was found in the specification set."),
                Some(format!(
                    "Ensure `{id}` is declared only once across all {kind} YAML files."
                )),
            ));
        }
    }
}

fn validate_philosophy(
    philosophy: &Philosophy,
    policies_by_id: &HashMap<&str, &Policy>,
    issues: &mut Vec<Issue>,
) {
    validate_non_empty_field("philosophy", "id", &philosophy.id, issues);
    validate_non_empty_field("philosophy", "title", &philosophy.title, issues);
    validate_non_empty_field(
        "philosophy",
        "product_design_principle",
        &philosophy.product_design_principle,
        issues,
    );
    validate_non_empty_field(
        "philosophy",
        "coding_guideline",
        &philosophy.coding_guideline,
        issues,
    );

    if philosophy.linked_policies.is_empty() {
        issues.push(Issue::warning(
            "missing-links",
            format!("philosophy {}", philosophy.id),
            None,
            "Philosophy does not link to any policies.".to_string(),
            Some(format!(
                "Add at least one policy link to `{}` so the philosophy influences executable behavior.",
                philosophy.id
            )),
        ));
    }

    for policy_id in &philosophy.linked_policies {
        match policies_by_id.get(policy_id.as_str()) {
            Some(policy) => {
                if !policy
                    .linked_philosophies
                    .iter()
                    .any(|item| item == &philosophy.id)
                {
                    issues.push(Issue::error(
                        "reciprocal-link-missing",
                        format!("philosophy {}", philosophy.id),
                        Some(policy_id.clone()),
                        format!(
                            "Policy `{policy_id}` does not link back to philosophy `{}`.",
                            philosophy.id
                        ),
                        Some(format!(
                            "Add `{}` to `linked_philosophies` in policy `{policy_id}`.",
                            philosophy.id
                        )),
                    ));
                }
            }
            None => issues.push(Issue::error(
                "missing-reference",
                format!("philosophy {}", philosophy.id),
                Some(policy_id.clone()),
                format!("Linked policy `{policy_id}` does not exist."),
                Some(format!(
                    "Declare policy `{policy_id}` or remove it from philosophy `{}`.",
                    philosophy.id
                )),
            )),
        }
    }
}

fn validate_policy(
    policy: &Policy,
    philosophies_by_id: &HashMap<&str, &Philosophy>,
    requirements_by_id: &HashMap<&str, &Requirement>,
    issues: &mut Vec<Issue>,
) {
    validate_non_empty_field("policy", "id", &policy.id, issues);
    validate_non_empty_field("policy", "title", &policy.title, issues);
    validate_non_empty_field("policy", "summary", &policy.summary, issues);
    validate_non_empty_field("policy", "description", &policy.description, issues);

    if policy.linked_philosophies.is_empty() {
        issues.push(Issue::warning(
            "missing-links",
            format!("policy {}", policy.id),
            None,
            "Policy does not link to any philosophies.".to_string(),
            Some(format!(
                "Add at least one philosophy link to `{}`.",
                policy.id
            )),
        ));
    }

    if policy.linked_requirements.is_empty() {
        issues.push(Issue::warning(
            "missing-links",
            format!("policy {}", policy.id),
            None,
            "Policy does not link to any requirements.".to_string(),
            Some(format!(
                "Add at least one requirement link to `{}`.",
                policy.id
            )),
        ));
    }

    for philosophy_id in &policy.linked_philosophies {
        match philosophies_by_id.get(philosophy_id.as_str()) {
            Some(philosophy) => {
                if !philosophy
                    .linked_policies
                    .iter()
                    .any(|item| item == &policy.id)
                {
                    issues.push(Issue::error(
                        "reciprocal-link-missing",
                        format!("policy {}", policy.id),
                        Some(philosophy_id.clone()),
                        format!(
                            "Philosophy `{philosophy_id}` does not link back to policy `{}`.",
                            policy.id
                        ),
                        Some(format!(
                            "Add `{}` to `linked_policies` in philosophy `{philosophy_id}`.",
                            policy.id
                        )),
                    ));
                }
            }
            None => issues.push(Issue::error(
                "missing-reference",
                format!("policy {}", policy.id),
                Some(philosophy_id.clone()),
                format!("Linked philosophy `{philosophy_id}` does not exist."),
                Some(format!(
                    "Declare philosophy `{philosophy_id}` or remove it from policy `{}`.",
                    policy.id
                )),
            )),
        }
    }

    for requirement_id in &policy.linked_requirements {
        match requirements_by_id.get(requirement_id.as_str()) {
            Some(requirement) => {
                if !requirement
                    .linked_policies
                    .iter()
                    .any(|item| item == &policy.id)
                {
                    issues.push(Issue::error(
                        "reciprocal-link-missing",
                        format!("policy {}", policy.id),
                        Some(requirement_id.clone()),
                        format!(
                            "Requirement `{requirement_id}` does not link back to policy `{}`.",
                            policy.id
                        ),
                        Some(format!(
                            "Add `{}` to `linked_policies` in requirement `{requirement_id}`.",
                            policy.id
                        )),
                    ));
                }
            }
            None => issues.push(Issue::error(
                "missing-reference",
                format!("policy {}", policy.id),
                Some(requirement_id.clone()),
                format!("Linked requirement `{requirement_id}` does not exist."),
                Some(format!(
                    "Declare requirement `{requirement_id}` or remove it from policy `{}`.",
                    policy.id
                )),
            )),
        }
    }
}

fn validate_requirement(
    requirement: &Requirement,
    policies_by_id: &HashMap<&str, &Policy>,
    features_by_id: &HashMap<&str, &Feature>,
    config: &SyuConfig,
    root: &Path,
    issues: &mut Vec<Issue>,
    trace_count: &mut TraceCount,
) {
    validate_non_empty_field("requirement", "id", &requirement.id, issues);
    validate_non_empty_field("requirement", "title", &requirement.title, issues);
    validate_non_empty_field(
        "requirement",
        "description",
        &requirement.description,
        issues,
    );
    validate_non_empty_field("requirement", "priority", &requirement.priority, issues);
    validate_non_empty_field("requirement", "status", &requirement.status, issues);
    let status = validate_delivery_status(
        "requirement",
        &requirement.id,
        &requirement.status,
        config,
        issues,
    );

    if requirement.linked_policies.is_empty() {
        issues.push(Issue::warning(
            "missing-links",
            format!("requirement {}", requirement.id),
            None,
            "Requirement does not link to any policies.".to_string(),
            Some(format!(
                "Add at least one policy link to `{}`.",
                requirement.id
            )),
        ));
    }

    if requirement.linked_features.is_empty() {
        issues.push(Issue::warning(
            "missing-links",
            format!("requirement {}", requirement.id),
            None,
            "Requirement does not link to any features.".to_string(),
            Some(format!(
                "Add at least one feature link to `{}`.",
                requirement.id
            )),
        ));
    }

    for policy_id in &requirement.linked_policies {
        match policies_by_id.get(policy_id.as_str()) {
            Some(policy) => {
                if !policy
                    .linked_requirements
                    .iter()
                    .any(|item| item == &requirement.id)
                {
                    issues.push(Issue::error(
                        "reciprocal-link-missing",
                        format!("requirement {}", requirement.id),
                        Some(policy_id.clone()),
                        format!(
                            "Policy `{policy_id}` does not link back to requirement `{}`.",
                            requirement.id
                        ),
                        Some(format!(
                            "Add `{}` to `linked_requirements` in policy `{policy_id}`.",
                            requirement.id
                        )),
                    ));
                }
            }
            None => issues.push(Issue::error(
                "missing-reference",
                format!("requirement {}", requirement.id),
                Some(policy_id.clone()),
                format!("Linked policy `{policy_id}` does not exist."),
                Some(format!(
                    "Declare policy `{policy_id}` or remove it from requirement `{}`.",
                    requirement.id
                )),
            )),
        }
    }

    for feature_id in &requirement.linked_features {
        match features_by_id.get(feature_id.as_str()) {
            Some(feature) => {
                if !feature
                    .linked_requirements
                    .iter()
                    .any(|item| item == &requirement.id)
                {
                    issues.push(Issue::error(
                        "reciprocal-link-missing",
                        format!("requirement {}", requirement.id),
                        Some(feature_id.clone()),
                        format!(
                            "Feature `{feature_id}` does not link back to requirement `{}`.",
                            requirement.id
                        ),
                        Some(format!(
                            "Add `{}` to `linked_requirements` in feature `{feature_id}`.",
                            requirement.id
                        )),
                    ));
                }
            }
            None => issues.push(Issue::error(
                "missing-reference",
                format!("requirement {}", requirement.id),
                Some(feature_id.clone()),
                format!("Linked feature `{feature_id}` does not exist."),
                Some(format!(
                    "Declare feature `{feature_id}` or remove it from requirement `{}`.",
                    requirement.id
                )),
            )),
        }
    }

    validate_trace_map(
        root,
        config,
        TraceValidationTarget {
            owner_id: &requirement.id,
            role: TraceRole::RequirementTest,
            status,
        },
        &requirement.tests,
        issues,
        trace_count,
    );
}

fn validate_feature(
    feature: &Feature,
    requirements_by_id: &HashMap<&str, &Requirement>,
    config: &SyuConfig,
    root: &Path,
    issues: &mut Vec<Issue>,
    trace_count: &mut TraceCount,
) {
    validate_non_empty_field("feature", "id", &feature.id, issues);
    validate_non_empty_field("feature", "title", &feature.title, issues);
    validate_non_empty_field("feature", "summary", &feature.summary, issues);
    validate_non_empty_field("feature", "status", &feature.status, issues);
    let status = validate_delivery_status("feature", &feature.id, &feature.status, config, issues);

    if feature.linked_requirements.is_empty() {
        issues.push(Issue::warning(
            "missing-links",
            format!("feature {}", feature.id),
            None,
            "Feature does not link to any requirements.".to_string(),
            Some(format!(
                "Add at least one requirement link to `{}`.",
                feature.id
            )),
        ));
    }

    for requirement_id in &feature.linked_requirements {
        match requirements_by_id.get(requirement_id.as_str()) {
            Some(requirement) => {
                if !requirement
                    .linked_features
                    .iter()
                    .any(|item| item == &feature.id)
                {
                    issues.push(Issue::error(
                        "reciprocal-link-missing",
                        format!("feature {}", feature.id),
                        Some(requirement_id.clone()),
                        format!(
                            "Requirement `{requirement_id}` does not link back to feature `{}`.",
                            feature.id
                        ),
                        Some(format!(
                            "Add `{}` to `linked_features` in requirement `{requirement_id}`.",
                            feature.id
                        )),
                    ));
                }
            }
            None => issues.push(Issue::error(
                "missing-reference",
                format!("feature {}", feature.id),
                Some(requirement_id.clone()),
                format!("Linked requirement `{requirement_id}` does not exist."),
                Some(format!(
                    "Declare requirement `{requirement_id}` or remove it from feature `{}`.",
                    feature.id
                )),
            )),
        }
    }

    validate_trace_map(
        root,
        config,
        TraceValidationTarget {
            owner_id: &feature.id,
            role: TraceRole::FeatureImplementation,
            status,
        },
        &feature.implementations,
        issues,
        trace_count,
    );
}

fn validate_trace_map(
    root: &Path,
    config: &SyuConfig,
    target: TraceValidationTarget<'_>,
    references_by_language: &BTreeMap<String, Vec<TraceReference>>,
    issues: &mut Vec<Issue>,
    trace_count: &mut TraceCount,
) {
    let subject = format!("{} {}", target.role.subject_kind(), target.owner_id);
    match target.status {
        Some(DeliveryStatus::Planned) => {
            if !references_by_language.is_empty() {
                issues.push(Issue::error(
                    "planned-trace-present",
                    subject,
                    Some("status".to_string()),
                    format!(
                        "{} `{owner_id}` is marked `planned` and must not declare any {}.",
                        target.role.subject_kind(),
                        target.role.relation_name(),
                        owner_id = target.owner_id
                    ),
                    Some(format!(
                        "Remove the declared {} from `{owner_id}` or change its status to `implemented`.",
                        target.role.relation_name(),
                        owner_id = target.owner_id
                    )),
                ));
            }
            return;
        }
        Some(DeliveryStatus::Implemented) => {
            if references_by_language.is_empty() {
                issues.push(Issue::error(
                    "implemented-trace-missing",
                    subject,
                    Some("status".to_string()),
                    format!(
                        "{} `{owner_id}` is marked `implemented` but does not declare any {}.",
                        target.role.subject_kind(),
                        target.role.relation_name(),
                        owner_id = target.owner_id
                    ),
                    Some(format!(
                        "Add at least one {} mapping to `{owner_id}` or mark it `planned`.",
                        target.role.relation_name(),
                        owner_id = target.owner_id
                    )),
                ));
                return;
            }
        }
        None if references_by_language.is_empty() => {
            issues.push(Issue::warning(
                "missing-trace",
                subject,
                None,
                format!(
                    "{} `{owner_id}` does not declare any {}.",
                    target.role.subject_kind(),
                    target.role.relation_name(),
                    owner_id = target.owner_id
                ),
                Some(format!(
                    "Add at least one {} mapping to `{owner_id}`.",
                    target.role.relation_name(),
                    owner_id = target.owner_id
                )),
            ));
            return;
        }
        None => {}
    }

    for (language, references) in references_by_language {
        for reference in references {
            trace_count.declared += 1;
            if verify_trace_reference(
                root,
                config,
                target.owner_id,
                target.role,
                language,
                reference,
                issues,
            ) {
                trace_count.validated += 1;
            }
        }
    }
}

fn validate_orphaned_definitions(workspace: &Workspace, issues: &mut Vec<Issue>) {
    if !workspace.config.validate.require_non_orphaned_items {
        return;
    }

    for philosophy in &workspace.philosophies {
        report_orphaned_definition(
            "philosophy",
            &philosophy.id,
            philosophy.linked_policies.len(),
            issues,
        );
    }

    for policy in &workspace.policies {
        report_orphaned_definition(
            "policy",
            &policy.id,
            policy.linked_philosophies.len() + policy.linked_requirements.len(),
            issues,
        );
    }

    for requirement in &workspace.requirements {
        report_orphaned_definition(
            "requirement",
            &requirement.id,
            requirement.linked_policies.len() + requirement.linked_features.len(),
            issues,
        );
    }

    for feature in &workspace.features {
        report_orphaned_definition(
            "feature",
            &feature.id,
            feature.linked_requirements.len(),
            issues,
        );
    }
}

fn report_orphaned_definition(
    kind: &str,
    id: &str,
    adjacent_link_count: usize,
    issues: &mut Vec<Issue>,
) {
    if adjacent_link_count > 0 {
        return;
    }

    issues.push(Issue::error(
        "orphaned-definition",
        format!("{kind} {id}"),
        None,
        format!(
            "{kind} `{id}` is isolated from the layered graph and does not link to any adjacent definitions."
        ),
        Some(format!(
            "Link `{id}` to at least one adjacent-layer definition so it participates in the specification graph."
        )),
    ));
}

fn verify_trace_reference(
    root: &Path,
    config: &SyuConfig,
    owner_id: &str,
    role: TraceRole,
    language: &str,
    reference: &TraceReference,
    issues: &mut Vec<Issue>,
) -> bool {
    let subject = format!("{} {}", role.subject_kind(), owner_id);
    let Some(adapter) = adapter_for_language(language) else {
        issues.push(Issue::error(
            "unsupported-language",
            subject,
            Some(format_reference_location(language, reference)),
            format!(
                "Language `{language}` is not supported. Built-in adapters currently cover Rust, Python, TypeScript, Shell, YAML, JSON, and Markdown."
            ),
            Some(format!(
                "Use a supported language alias such as `rust`, `python`, `typescript`, `shell`, `yaml`, `json`, or `markdown` for `{owner_id}`."
            )),
        ));
        return false;
    };

    if reference.file.as_os_str().is_empty() {
        issues.push(Issue::error(
            "trace-file-missing",
            subject.clone(),
            Some(format_reference_location(language, reference)),
            format!(
                "Declared {} mapping for `{owner_id}` does not specify a file path.",
                role.label()
            ),
            Some(format!(
                "Add a file path to the `{language}` {} mapping for `{owner_id}`.",
                role.relation_name()
            )),
        ));
        return false;
    }

    let path = root.join(&reference.file);
    if !path.is_file() {
        issues.push(Issue::error(
            "trace-file-missing",
            subject.clone(),
            Some(format_reference_location(language, reference)),
            format!(
                "Declared {} file `{}` does not exist.",
                role.label(),
                reference.file.display()
            ),
            Some(format!(
                "Create `{}` or update `{owner_id}` to point to the correct {} file.",
                reference.file.display(),
                role.label()
            )),
        ));
        return false;
    }

    let mut success = true;
    if !adapter.supports_path(&path) {
        issues.push(Issue::error(
            "extension-mismatch",
            subject.clone(),
            Some(format_reference_location(language, reference)),
            format!(
                "File `{}` does not match the `{}` adapter extensions.",
                reference.file.display(),
                adapter.canonical_name()
            ),
            Some(format!(
                "Use a `{}` file extension or change the declared language for `{}`.",
                adapter.canonical_name(),
                reference.file.display()
            )),
        ));
        success = false;
    }

    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => {
            issues.push(Issue::error(
                "trace-file-unreadable",
                subject,
                Some(format_reference_location(language, reference)),
                format!(
                    "Declared {} file `{}` could not be read: {error}",
                    role.label(),
                    reference.file.display()
                ),
                Some(format!(
                    "Ensure `{}` is readable before running `syu validate`.",
                    reference.file.display()
                )),
            ));
            return false;
        }
    };

    if !contents.contains(owner_id) {
        issues.push(Issue::error(
            "trace-id-missing",
            subject.clone(),
            Some(format_reference_location(language, reference)),
            format!(
                "Declared {} file `{}` does not mention `{owner_id}`.",
                role.label(),
                reference.file.display()
            ),
            Some(format!(
                "Add `{owner_id}` to `{}` so the {} remains explicitly traceable.",
                reference.file.display(),
                role.label()
            )),
        ));
        success = false;
    }

    if reference.symbols.is_empty() {
        issues.push(Issue::error(
            "trace-symbol-missing",
            subject.clone(),
            Some(format_reference_location(language, reference)),
            format!(
                "Declared {} file `{}` does not list any symbols to verify.",
                role.label(),
                reference.file.display()
            ),
            Some(format!(
                "Add one or more symbols to the `{language}` mapping for `{owner_id}`."
            )),
        ));
        success = false;
    }

    let has_wildcard = reference.symbols.iter().any(|symbol| symbol.trim() == "*");
    if has_wildcard {
        if !reference.doc_contains.is_empty() {
            issues.push(Issue::error(
                "trace-doc-unsupported",
                subject.clone(),
                Some(format_reference_location(language, reference)),
                format!(
                    "Wildcard trace mappings in `{}` cannot use `doc_contains` because they do not point to a single symbol.",
                    reference.file.display()
                ),
                Some(
                    "Remove `doc_contains` or replace `*` with explicit symbol names for documentation checks."
                        .to_string(),
                ),
            ));
            success = false;
        }
        return success;
    }

    for symbol in &reference.symbols {
        if symbol.trim().is_empty() {
            issues.push(Issue::error(
                "trace-symbol-missing",
                subject.clone(),
                Some(format_reference_location(language, reference)),
                format!(
                    "Declared {} file `{}` contains an empty symbol entry.",
                    role.label(),
                    reference.file.display()
                ),
                Some(format!(
                    "Remove blank symbol entries from the `{language}` mapping for `{owner_id}`."
                )),
            ));
            success = false;
            continue;
        }

        let inspection = if supports_rich_inspection(language) {
            match inspect_symbol(language, config, &path, &contents, symbol) {
                Ok(result) => result,
                Err(error) => {
                    issues.push(Issue::error(
                        "trace-inspection-failed",
                        subject.clone(),
                        Some(format_reference_location(language, reference)),
                        format!(
                            "Failed to inspect symbol `{symbol}` in `{}` with the `{language}` inspector: {error}",
                            reference.file.display()
                        ),
                        Some(format!(
                            "Fix the parser/runtime configuration for `{language}` or update `{}` so `syu validate` can inspect it.",
                            reference.file.display()
                        )),
                    ));
                    success = false;
                    continue;
                }
            }
        } else {
            None
        };

        let symbol_exists = inspection.is_some() || adapter.symbol_exists(&contents, symbol);
        if !symbol_exists {
            issues.push(Issue::error(
                "trace-symbol-missing",
                subject.clone(),
                Some(format_reference_location(language, reference)),
                format!(
                    "Declared symbol `{symbol}` was not found in `{}`.",
                    reference.file.display()
                ),
                Some(format!(
                    "Add symbol `{symbol}` to `{}` or update the YAML mapping for `{owner_id}`.",
                    reference.file.display()
                )),
            ));
            success = false;
            continue;
        }

        if !reference.doc_contains.is_empty() {
            match inspection {
                Some(inspection) => {
                    for snippet in &reference.doc_contains {
                        if snippet.trim().is_empty() {
                            continue;
                        }

                        if !inspection.docs.contains(snippet) {
                            issues.push(Issue::error(
                                "trace-doc-missing",
                                subject.clone(),
                                Some(format_reference_location(language, reference)),
                                format!(
                                    "Documentation for symbol `{symbol}` in `{}` does not include `{snippet}`.",
                                    reference.file.display()
                                ),
                                Some(format!(
                                    "Add `{snippet}` to the documentation for `{symbol}` or run `syu validate --fix`."
                                )),
                            ));
                            success = false;
                        }
                    }
                }
                None => {
                    issues.push(Issue::error(
                        "trace-doc-unsupported",
                        subject.clone(),
                        Some(format_reference_location(language, reference)),
                        format!(
                            "Language `{language}` does not provide rich documentation inspection for symbol `{symbol}`."
                        ),
                        Some(format!(
                            "Remove `doc_contains` from the `{language}` mapping for `{owner_id}` or switch to a language with rich inspection support."
                        )),
                    ));
                    success = false;
                }
            }
        }
    }

    success
}

fn format_reference_location(language: &str, reference: &TraceReference) -> String {
    format!("{language}:{}", reference.file.display())
}

fn validate_non_empty_field(kind: &str, field_name: &str, value: &str, issues: &mut Vec<Issue>) {
    if value.trim().is_empty() {
        issues.push(Issue::error(
            "blank-field",
            kind.to_string(),
            Some(field_name.to_string()),
            format!("Field `{field_name}` must not be blank."),
            Some(format!(
                "Populate `{field_name}` in the {kind} definition before running `syu validate`."
            )),
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{BTreeMap, HashMap},
        fs,
        path::{Path, PathBuf},
    };

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use tempfile::tempdir;

    use crate::{
        config::SyuConfig,
        model::{Feature, Issue, Philosophy, Policy, Requirement, TraceReference},
        workspace::Workspace,
    };

    use super::{
        TraceRole, apply_autofix, apply_autofix_for_reference, collect_check_result,
        format_reference_location, render_text_report, run_check_command, validate_feature,
        validate_non_empty_field, validate_philosophy, validate_policy, validate_requirement,
        validate_unique_ids, verify_trace_reference,
    };

    fn philosophy(id: &str) -> Philosophy {
        Philosophy {
            id: id.to_string(),
            title: "Title".to_string(),
            product_design_principle: "Principle".to_string(),
            coding_guideline: "Guideline".to_string(),
            linked_policies: Vec::new(),
        }
    }

    fn policy(id: &str) -> Policy {
        Policy {
            id: id.to_string(),
            title: "Title".to_string(),
            summary: "Summary".to_string(),
            description: "Description".to_string(),
            linked_philosophies: Vec::new(),
            linked_requirements: Vec::new(),
        }
    }

    fn requirement(id: &str) -> Requirement {
        Requirement {
            id: id.to_string(),
            title: "Title".to_string(),
            description: "Description".to_string(),
            priority: "high".to_string(),
            status: "implemented".to_string(),
            linked_policies: Vec::new(),
            linked_features: Vec::new(),
            tests: BTreeMap::new(),
        }
    }

    fn feature(id: &str) -> Feature {
        Feature {
            id: id.to_string(),
            title: "Title".to_string(),
            summary: "Summary".to_string(),
            status: "implemented".to_string(),
            linked_requirements: Vec::new(),
            implementations: BTreeMap::new(),
        }
    }

    #[test]
    fn collect_check_result_reports_load_errors() {
        let result = collect_check_result(Path::new("/definitely/missing-syu-workspace"));
        assert!(!result.is_success());
        assert_eq!(result.issues[0].code, "load-failed");
    }

    #[test]
    fn run_check_command_handles_workspace_load_errors() {
        let code = run_check_command(&crate::cli::CheckArgs {
            workspace: PathBuf::from("/definitely/missing-syu-workspace"),
            format: crate::cli::OutputFormat::Json,
            fix: false,
            no_fix: false,
        })
        .expect("command should render load errors");

        assert_eq!(code, 1);
    }

    #[test]
    fn run_check_command_propagates_autofix_errors() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path().join("workspace");
        fs::create_dir_all(workspace.join("docs/spec/philosophy")).expect("philosophy dir");
        fs::create_dir_all(workspace.join("docs/spec/policies")).expect("policies dir");
        fs::create_dir_all(workspace.join("docs/spec/requirements")).expect("requirements dir");
        fs::create_dir_all(workspace.join("docs/spec/features")).expect("features dir");

        fs::write(
            workspace.join("syu.yaml"),
            "version: 1\nruntimes:\n  python:\n    command: false\n",
        )
        .expect("config should exist");
        fs::write(
            workspace.join("docs/spec/philosophy/foundation.yaml"),
            "category: Foundations\nversion: 1\n\nphilosophies:\n  - id: PHIL-1\n    title: Foundation\n    product_design_principle: Keep it clear.\n    coding_guideline: Keep it explicit.\n    linked_policies:\n      - POL-1\n",
        )
        .expect("philosophy should exist");
        fs::write(
            workspace.join("docs/spec/policies/policies.yaml"),
            "category: Policies\nversion: 1\n\npolicies:\n  - id: POL-1\n    title: Policy\n    summary: Rule summary.\n    description: Rule description.\n    linked_philosophies:\n      - PHIL-1\n    linked_requirements:\n      - REQ-1\n",
        )
        .expect("policy should exist");
        fs::write(
            workspace.join("docs/spec/requirements/core.yaml"),
            "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-1\n    title: Requirement\n    description: Requirement description.\n    priority: high\n    status: implemented\n    linked_policies:\n      - POL-1\n    linked_features:\n      - FEAT-1\n    tests:\n      python:\n        - file: tests/test_sample.py\n          symbols:\n            - requirement_test\n          doc_contains:\n            - Requirement docs\n",
        )
        .expect("requirement should exist");
        fs::write(
            workspace.join("docs/spec/features/features.yaml"),
            "version: 1\nfiles:\n  - kind: core\n    file: core.yaml\n",
        )
        .expect("feature registry should exist");
        fs::write(
            workspace.join("docs/spec/features/core.yaml"),
            "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-1\n    title: Feature\n    summary: Feature summary.\n    status: implemented\n    linked_requirements:\n      - REQ-1\n    implementations: {}\n",
        )
        .expect("feature should exist");
        fs::create_dir_all(workspace.join("tests")).expect("tests dir");
        fs::write(
            workspace.join("tests/test_sample.py"),
            "def requirement_test():\n    return 1\n",
        )
        .expect("python test should exist");

        let error = run_check_command(&crate::cli::CheckArgs {
            workspace,
            format: crate::cli::OutputFormat::Text,
            fix: true,
            no_fix: false,
        })
        .expect_err("autofix failures should bubble up");

        assert!(error.to_string().contains("Python inspector failed"));
    }

    #[test]
    fn render_text_report_lists_issues() {
        let result = crate::model::CheckResult {
            workspace_root: PathBuf::from("."),
            definition_counts: Default::default(),
            trace_summary: Default::default(),
            issues: vec![Issue::warning("warn", "subject", None, "message", None)],
            referenced_rules: Vec::new(),
        };

        let report = render_text_report(&result);
        assert!(report.contains("syu validate passed"));
        assert!(report.contains("issues:"));
        assert!(report.contains("[Warning] warn subject: message"));
    }

    #[test]
    fn validate_unique_ids_reports_duplicates() {
        let mut issues = Vec::new();
        validate_unique_ids("feature", ["FEAT-1", "FEAT-1"].into_iter(), &mut issues);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "duplicate-id");
    }

    #[test]
    fn validate_non_empty_field_reports_blank_values() {
        let mut issues = Vec::new();
        validate_non_empty_field("feature", "title", "   ", &mut issues);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "blank-field");
        assert_eq!(issues[0].location.as_deref(), Some("title"));
    }

    #[test]
    fn validate_philosophy_reports_blank_and_reference_errors() {
        let mut entry = philosophy("PHIL-1");
        entry.title.clear();
        entry.linked_policies.push("POL-1".to_string());

        let mut issues = Vec::new();
        validate_philosophy(&entry, &HashMap::new(), &mut issues);

        assert!(issues.iter().any(|issue| issue.code == "blank-field"));
        assert!(issues.iter().any(|issue| issue.code == "missing-reference"));
    }

    #[test]
    fn validate_philosophy_warns_when_unlinked() {
        let entry = philosophy("PHIL-1");
        let mut issues = Vec::new();
        validate_philosophy(&entry, &HashMap::new(), &mut issues);
        assert!(issues.iter().any(|issue| issue.code == "missing-links"));
    }

    #[test]
    fn validate_philosophy_reports_missing_backlink() {
        let mut entry = philosophy("PHIL-1");
        entry.linked_policies.push("POL-1".to_string());

        let mut linked_policy = policy("POL-1");
        linked_policy.linked_philosophies.clear();

        let mut policy_map = HashMap::new();
        policy_map.insert("POL-1", &linked_policy);

        let mut issues = Vec::new();
        validate_philosophy(&entry, &policy_map, &mut issues);
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "reciprocal-link-missing")
        );
    }

    #[test]
    fn validate_policy_reports_reference_and_backlink_errors() {
        let mut entry = policy("POL-1");
        entry.summary.clear();
        entry.linked_philosophies.push("PHIL-1".to_string());
        entry.linked_requirements.push("REQ-1".to_string());

        let referenced_philosophy = philosophy("PHIL-1");
        let referenced_requirement = requirement("REQ-1");

        let mut philosophies = HashMap::new();
        philosophies.insert("PHIL-1", &referenced_philosophy);
        let mut requirements = HashMap::new();
        requirements.insert("REQ-1", &referenced_requirement);

        let mut issues = Vec::new();
        validate_policy(&entry, &philosophies, &requirements, &mut issues);

        assert!(issues.iter().any(|issue| issue.code == "blank-field"));
        assert!(
            issues
                .iter()
                .filter(|issue| issue.code == "reciprocal-link-missing")
                .count()
                >= 2
        );
    }

    #[test]
    fn validate_policy_warns_when_unlinked() {
        let entry = policy("POL-1");
        let mut issues = Vec::new();
        validate_policy(&entry, &HashMap::new(), &HashMap::new(), &mut issues);
        assert!(
            issues
                .iter()
                .filter(|issue| issue.code == "missing-links")
                .count()
                >= 2
        );
    }

    #[test]
    fn validate_policy_reports_missing_reference_errors() {
        let mut entry = policy("POL-1");
        entry.linked_philosophies.push("PHIL-MISSING".to_string());
        entry.linked_requirements.push("REQ-MISSING".to_string());

        let mut issues = Vec::new();
        validate_policy(&entry, &HashMap::new(), &HashMap::new(), &mut issues);

        assert!(issues.iter().any(|issue| {
            issue.code == "missing-reference" && issue.location.as_deref() == Some("PHIL-MISSING")
        }));
        assert!(issues.iter().any(|issue| {
            issue.code == "missing-reference" && issue.location.as_deref() == Some("REQ-MISSING")
        }));
    }

    #[test]
    fn validate_requirement_reports_missing_trace_and_links() {
        let mut entry = requirement("REQ-1");
        entry.status.clear();

        let mut issues = Vec::new();
        let mut trace_count = Default::default();
        validate_requirement(
            &entry,
            &HashMap::new(),
            &HashMap::new(),
            &SyuConfig::default(),
            Path::new("."),
            &mut issues,
            &mut trace_count,
        );

        assert!(issues.iter().any(|issue| issue.code == "blank-field"));
        assert!(issues.iter().any(|issue| issue.code == "missing-links"));
        assert!(issues.iter().any(|issue| issue.code == "missing-trace"));
        assert_eq!(trace_count.declared, 0);
    }

    #[test]
    fn validate_requirement_reports_missing_and_backlink_errors() {
        let mut entry = requirement("REQ-1");
        entry.linked_policies = vec!["POL-1".to_string(), "POL-MISSING".to_string()];
        entry.linked_features = vec!["FEAT-1".to_string(), "FEAT-MISSING".to_string()];

        let linked_policy = policy("POL-1");
        let linked_feature = feature("FEAT-1");

        let mut policies = HashMap::new();
        policies.insert("POL-1", &linked_policy);
        let mut features = HashMap::new();
        features.insert("FEAT-1", &linked_feature);

        let mut issues = Vec::new();
        let mut trace_count = Default::default();
        validate_requirement(
            &entry,
            &policies,
            &features,
            &SyuConfig::default(),
            Path::new("."),
            &mut issues,
            &mut trace_count,
        );

        assert!(issues.iter().any(|issue| {
            issue.code == "reciprocal-link-missing" && issue.location.as_deref() == Some("POL-1")
        }));
        assert!(issues.iter().any(|issue| {
            issue.code == "missing-reference" && issue.location.as_deref() == Some("POL-MISSING")
        }));
        assert!(issues.iter().any(|issue| {
            issue.code == "reciprocal-link-missing" && issue.location.as_deref() == Some("FEAT-1")
        }));
        assert!(issues.iter().any(|issue| {
            issue.code == "missing-reference" && issue.location.as_deref() == Some("FEAT-MISSING")
        }));
    }

    #[test]
    fn validate_feature_reports_missing_trace_and_links() {
        let mut entry = feature("FEAT-1");
        entry.summary.clear();

        let mut issues = Vec::new();
        let mut trace_count = Default::default();
        validate_feature(
            &entry,
            &HashMap::new(),
            &SyuConfig::default(),
            Path::new("."),
            &mut issues,
            &mut trace_count,
        );

        assert!(issues.iter().any(|issue| issue.code == "blank-field"));
        assert!(issues.iter().any(|issue| issue.code == "missing-links"));
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "implemented-trace-missing")
        );
        assert_eq!(trace_count.validated, 0);
    }

    #[test]
    fn validate_requirement_rejects_traces_when_planned() {
        let mut entry = requirement("REQ-1");
        entry.status = "planned".to_string();
        entry.tests.insert(
            "rust".to_string(),
            vec![TraceReference {
                file: PathBuf::from("src/lib.rs"),
                symbols: vec!["trace".to_string()],
                doc_contains: Vec::new(),
            }],
        );

        let mut issues = Vec::new();
        let mut trace_count = Default::default();
        validate_requirement(
            &entry,
            &HashMap::new(),
            &HashMap::new(),
            &SyuConfig::default(),
            Path::new("."),
            &mut issues,
            &mut trace_count,
        );

        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "planned-trace-present")
        );
        assert_eq!(trace_count.declared, 0);
    }

    #[test]
    fn validate_requirement_rejects_invalid_status_values() {
        let mut entry = requirement("REQ-1");
        entry.status = "proposed".to_string();

        let mut issues = Vec::new();
        let mut trace_count = Default::default();
        validate_requirement(
            &entry,
            &HashMap::new(),
            &HashMap::new(),
            &SyuConfig::default(),
            Path::new("."),
            &mut issues,
            &mut trace_count,
        );

        assert!(issues.iter().any(|issue| issue.code == "invalid-status"));
    }

    #[test]
    fn validate_requirement_with_invalid_status_and_traces_still_checks_references() {
        let mut entry = requirement("REQ-1");
        entry.status = "proposed".to_string();
        entry.tests.insert(
            "go".to_string(),
            vec![TraceReference {
                file: PathBuf::from("trace.go"),
                symbols: vec!["trace".to_string()],
                doc_contains: Vec::new(),
            }],
        );

        let mut issues = Vec::new();
        let mut trace_count = Default::default();
        validate_requirement(
            &entry,
            &HashMap::new(),
            &HashMap::new(),
            &SyuConfig::default(),
            Path::new("."),
            &mut issues,
            &mut trace_count,
        );

        assert!(issues.iter().any(|issue| issue.code == "invalid-status"));
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "unsupported-language")
        );
        assert_eq!(trace_count.declared, 1);
    }

    #[test]
    fn validate_feature_rejects_planned_status_when_disallowed() {
        let mut entry = feature("FEAT-1");
        entry.status = "planed".to_string();

        let mut config = SyuConfig::default();
        config.validate.allow_planned = false;

        let mut issues = Vec::new();
        let mut trace_count = Default::default();
        validate_feature(
            &entry,
            &HashMap::new(),
            &config,
            Path::new("."),
            &mut issues,
            &mut trace_count,
        );

        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "planned-status-disallowed")
        );
    }

    #[test]
    fn validate_feature_reports_missing_and_backlink_errors() {
        let mut entry = feature("FEAT-1");
        entry.linked_requirements = vec!["REQ-1".to_string(), "REQ-MISSING".to_string()];

        let linked_requirement = requirement("REQ-1");
        let mut requirements = HashMap::new();
        requirements.insert("REQ-1", &linked_requirement);

        let mut issues = Vec::new();
        let mut trace_count = Default::default();
        validate_feature(
            &entry,
            &requirements,
            &SyuConfig::default(),
            Path::new("."),
            &mut issues,
            &mut trace_count,
        );

        assert!(issues.iter().any(|issue| {
            issue.code == "reciprocal-link-missing" && issue.location.as_deref() == Some("REQ-1")
        }));
        assert!(issues.iter().any(|issue| {
            issue.code == "missing-reference" && issue.location.as_deref() == Some("REQ-MISSING")
        }));
    }

    #[test]
    fn verify_trace_reference_reports_unsupported_languages() {
        let reference = TraceReference {
            file: PathBuf::from("test.go"),
            symbols: vec!["main".to_string()],
            doc_contains: Vec::new(),
        };
        let mut issues = Vec::new();
        assert!(!verify_trace_reference(
            Path::new("."),
            &SyuConfig::default(),
            "REQ-1",
            TraceRole::RequirementTest,
            "go",
            &reference,
            &mut issues,
        ));
        assert_eq!(issues[0].code, "unsupported-language");
    }

    #[test]
    fn verify_trace_reference_reports_missing_file_path() {
        let reference = TraceReference {
            file: PathBuf::new(),
            symbols: vec!["main".to_string()],
            doc_contains: Vec::new(),
        };
        let mut issues = Vec::new();
        assert!(!verify_trace_reference(
            Path::new("."),
            &SyuConfig::default(),
            "REQ-1",
            TraceRole::RequirementTest,
            "rust",
            &reference,
            &mut issues,
        ));
        assert_eq!(issues[0].code, "trace-file-missing");
    }

    #[test]
    fn verify_trace_reference_reports_missing_files() {
        let reference = TraceReference {
            file: PathBuf::from("missing.rs"),
            symbols: vec!["main".to_string()],
            doc_contains: Vec::new(),
        };
        let mut issues = Vec::new();
        assert!(!verify_trace_reference(
            Path::new("."),
            &SyuConfig::default(),
            "REQ-1",
            TraceRole::RequirementTest,
            "rust",
            &reference,
            &mut issues,
        ));
        assert_eq!(issues[0].code, "trace-file-missing");
    }

    #[test]
    fn verify_trace_reference_reports_extension_id_and_blank_symbol_errors() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("trace.txt");
        fs::write(&path, "fn unrelated() {}\n").expect("file should exist");

        let reference = TraceReference {
            file: PathBuf::from("trace.txt"),
            symbols: vec![String::new()],
            doc_contains: Vec::new(),
        };
        let mut issues = Vec::new();
        assert!(!verify_trace_reference(
            tempdir.path(),
            &SyuConfig::default(),
            "REQ-1",
            TraceRole::RequirementTest,
            "rust",
            &reference,
            &mut issues,
        ));

        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "extension-mismatch")
        );
        assert!(issues.iter().any(|issue| issue.code == "trace-id-missing"));
        assert!(
            issues
                .iter()
                .filter(|issue| issue.code == "trace-symbol-missing")
                .count()
                >= 1
        );
    }

    #[test]
    fn verify_trace_reference_reports_missing_symbol() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("trace.rs");
        fs::write(&path, "// REQ-1\nfn different_symbol() {}\n").expect("file should exist");

        let reference = TraceReference {
            file: PathBuf::from("trace.rs"),
            symbols: vec!["expected_symbol".to_string()],
            doc_contains: Vec::new(),
        };
        let mut issues = Vec::new();
        assert!(!verify_trace_reference(
            tempdir.path(),
            &SyuConfig::default(),
            "REQ-1",
            TraceRole::RequirementTest,
            "rust",
            &reference,
            &mut issues,
        ));
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "trace-symbol-missing")
        );
    }

    #[test]
    fn verify_trace_reference_reports_empty_symbol_lists() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("trace.rs");
        fs::write(&path, "// REQ-1\nfn expected() {}\n").expect("file should exist");

        let reference = TraceReference {
            file: PathBuf::from("trace.rs"),
            symbols: Vec::new(),
            doc_contains: Vec::new(),
        };
        let mut issues = Vec::new();
        assert!(!verify_trace_reference(
            tempdir.path(),
            &SyuConfig::default(),
            "REQ-1",
            TraceRole::RequirementTest,
            "rust",
            &reference,
            &mut issues,
        ));
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "trace-symbol-missing")
        );
    }

    #[test]
    fn verify_trace_reference_accepts_valid_shell_files() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("install.sh");
        fs::write(&path, "# FEAT-1\ninstall_syu() {\n  echo ok\n}\n").expect("file should exist");

        let reference = TraceReference {
            file: PathBuf::from("install.sh"),
            symbols: vec!["install_syu".to_string()],
            doc_contains: Vec::new(),
        };
        let mut issues = Vec::new();
        assert!(verify_trace_reference(
            tempdir.path(),
            &SyuConfig::default(),
            "FEAT-1",
            TraceRole::FeatureImplementation,
            "shell",
            &reference,
            &mut issues,
        ));
        assert!(issues.is_empty());
    }

    #[test]
    fn verify_trace_reference_reports_inspection_and_doc_errors() {
        let tempdir = tempdir().expect("tempdir should exist");

        let python_path = tempdir.path().join("trace.py");
        fs::write(&python_path, "def expected():\n    return 1\n")
            .expect("python file should exist");
        let mut python_issues = Vec::new();
        let python_config = SyuConfig {
            runtimes: crate::config::RuntimeConfigSet {
                python: crate::config::RuntimeConfig {
                    command: "false".to_string(),
                },
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(!verify_trace_reference(
            tempdir.path(),
            &python_config,
            "REQ-1",
            TraceRole::RequirementTest,
            "python",
            &TraceReference {
                file: PathBuf::from("trace.py"),
                symbols: vec!["expected".to_string()],
                doc_contains: Vec::new(),
            },
            &mut python_issues,
        ));
        assert!(
            python_issues
                .iter()
                .any(|issue| issue.code == "trace-inspection-failed")
        );

        let rust_path = tempdir.path().join("trace.rs");
        fs::write(&rust_path, "/// REQ-1\npub fn expected() {}\n").expect("rust file should exist");
        let mut rust_issues = Vec::new();
        assert!(!verify_trace_reference(
            tempdir.path(),
            &SyuConfig::default(),
            "REQ-1",
            TraceRole::RequirementTest,
            "rust",
            &TraceReference {
                file: PathBuf::from("trace.rs"),
                symbols: vec!["expected".to_string()],
                doc_contains: vec!["   ".to_string(), "Explain expected".to_string()],
            },
            &mut rust_issues,
        ));
        assert_eq!(
            rust_issues
                .iter()
                .filter(|issue| issue.code == "trace-doc-missing")
                .count(),
            1
        );

        let shell_path = tempdir.path().join("trace.sh");
        fs::write(&shell_path, "# REQ-1\nexpected() {\n  echo ok\n}\n")
            .expect("shell file should exist");
        let mut shell_issues = Vec::new();
        assert!(!verify_trace_reference(
            tempdir.path(),
            &SyuConfig::default(),
            "REQ-1",
            TraceRole::RequirementTest,
            "shell",
            &TraceReference {
                file: PathBuf::from("trace.sh"),
                symbols: vec!["expected".to_string()],
                doc_contains: vec!["Explain expected".to_string()],
            },
            &mut shell_issues,
        ));
        assert!(
            shell_issues
                .iter()
                .any(|issue| issue.code == "trace-doc-unsupported")
        );

        let wildcard_path = tempdir.path().join("wildcard.rs");
        fs::write(&wildcard_path, "/// REQ-1\npub fn expected() {}\n")
            .expect("wildcard file should exist");
        let mut wildcard_issues = Vec::new();
        assert!(!verify_trace_reference(
            tempdir.path(),
            &SyuConfig::default(),
            "REQ-1",
            TraceRole::RequirementTest,
            "rust",
            &TraceReference {
                file: PathBuf::from("wildcard.rs"),
                symbols: vec!["*".to_string()],
                doc_contains: vec!["Explain expected".to_string()],
            },
            &mut wildcard_issues,
        ));
        assert!(
            wildcard_issues
                .iter()
                .any(|issue| issue.code == "trace-doc-unsupported")
        );
    }

    #[test]
    fn apply_autofix_for_reference_skips_unfixable_inputs() {
        let tempdir = tempdir().expect("tempdir should exist");
        let root = tempdir.path();
        let mut summary = super::AutofixSummary::default();

        apply_autofix_for_reference(
            root,
            &SyuConfig::default(),
            "REQ-1",
            "go",
            &TraceReference {
                file: PathBuf::from("trace.go"),
                symbols: vec!["expected".to_string()],
                doc_contains: vec!["Explain expected".to_string()],
            },
            &mut summary,
        )
        .expect("unsupported languages should be ignored");

        apply_autofix_for_reference(
            root,
            &SyuConfig::default(),
            "REQ-1",
            "rust",
            &TraceReference {
                file: PathBuf::from("trace.rs"),
                symbols: Vec::new(),
                doc_contains: vec!["Explain expected".to_string()],
            },
            &mut summary,
        )
        .expect("empty symbol lists should be ignored");

        apply_autofix_for_reference(
            root,
            &SyuConfig::default(),
            "REQ-1",
            "rust",
            &TraceReference {
                file: PathBuf::from("missing.rs"),
                symbols: vec!["expected".to_string()],
                doc_contains: vec!["Explain expected".to_string()],
            },
            &mut summary,
        )
        .expect("missing files should be ignored");

        let directory = root.join("nested");
        fs::create_dir_all(&directory).expect("directory should exist");
        apply_autofix_for_reference(
            root,
            &SyuConfig::default(),
            "REQ-1",
            "rust",
            &TraceReference {
                file: PathBuf::from("nested"),
                symbols: vec!["expected".to_string()],
                doc_contains: vec!["Explain expected".to_string()],
            },
            &mut summary,
        )
        .expect("directories should be ignored");

        let no_update_path = root.join("trace.rs");
        fs::write(&no_update_path, "/// REQ-1\npub fn expected() {}\n")
            .expect("trace file should exist");
        apply_autofix_for_reference(
            root,
            &SyuConfig::default(),
            "REQ-1",
            "rust",
            &TraceReference {
                file: PathBuf::from("trace.rs"),
                symbols: vec!["expected".to_string()],
                doc_contains: Vec::new(),
            },
            &mut summary,
        )
        .expect("already traced symbols should be left alone");

        assert!(summary.updated_files.is_empty());
        assert_eq!(summary.symbol_updates, 0);
    }

    #[test]
    fn apply_autofix_updates_requirement_and_feature_traces() {
        let tempdir = tempdir().expect("tempdir should exist");
        let root = tempdir.path().to_path_buf();

        fs::write(
            root.join("requirement.rs"),
            "pub fn requirement_test() {}\n",
        )
        .expect("requirement file should exist");
        fs::write(root.join("feature.rs"), "pub fn feature_impl() {}\n")
            .expect("feature file should exist");

        let mut req = requirement("REQ-1");
        req.tests.insert(
            "rust".to_string(),
            vec![TraceReference {
                file: PathBuf::from("requirement.rs"),
                symbols: vec!["requirement_test".to_string()],
                doc_contains: vec!["Requirement docs".to_string()],
            }],
        );

        let mut feat = feature("FEAT-1");
        feat.implementations.insert(
            "rust".to_string(),
            vec![TraceReference {
                file: PathBuf::from("feature.rs"),
                symbols: vec!["feature_impl".to_string()],
                doc_contains: vec!["Feature docs".to_string()],
            }],
        );

        let summary = apply_autofix(&Workspace {
            root: root.clone(),
            spec_root: root.join("docs/spec"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: vec![req],
            features: vec![feat],
        })
        .expect("autofix should succeed");

        assert_eq!(summary.symbol_updates, 2);
        assert_eq!(summary.updated_files.len(), 2);
        let requirement_contents =
            fs::read_to_string(root.join("requirement.rs")).expect("requirement contents");
        assert!(requirement_contents.contains("REQ-1"));
        assert!(requirement_contents.contains("Requirement docs"));
        let feature_contents =
            fs::read_to_string(root.join("feature.rs")).expect("feature contents");
        assert!(feature_contents.contains("FEAT-1"));
        assert!(feature_contents.contains("Feature docs"));
    }

    #[test]
    fn apply_autofix_skips_planned_requirement_and_feature_entries() {
        let tempdir = tempdir().expect("tempdir should exist");
        let root = tempdir.path().to_path_buf();

        fs::write(
            root.join("requirement.rs"),
            "pub fn requirement_test() {}\n",
        )
        .expect("requirement file should exist");
        fs::write(root.join("feature.rs"), "pub fn feature_impl() {}\n")
            .expect("feature file should exist");

        let mut req = requirement("REQ-1");
        req.status = "planned".to_string();
        req.tests.insert(
            "rust".to_string(),
            vec![TraceReference {
                file: PathBuf::from("requirement.rs"),
                symbols: vec!["requirement_test".to_string()],
                doc_contains: vec!["Requirement docs".to_string()],
            }],
        );

        let mut feat = feature("FEAT-1");
        feat.status = "planned".to_string();
        feat.implementations.insert(
            "rust".to_string(),
            vec![TraceReference {
                file: PathBuf::from("feature.rs"),
                symbols: vec!["feature_impl".to_string()],
                doc_contains: vec!["Feature docs".to_string()],
            }],
        );

        let summary = apply_autofix(&Workspace {
            root: root.clone(),
            spec_root: root.join("docs/spec"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: vec![req],
            features: vec![feat],
        })
        .expect("planned entries should be skipped");

        assert_eq!(summary.symbol_updates, 0);
        assert!(summary.updated_files.is_empty());
        assert_eq!(
            fs::read_to_string(root.join("requirement.rs")).expect("requirement contents"),
            "pub fn requirement_test() {}\n"
        );
        assert_eq!(
            fs::read_to_string(root.join("feature.rs")).expect("feature contents"),
            "pub fn feature_impl() {}\n"
        );
    }

    #[test]
    fn apply_autofix_propagates_requirement_inspection_errors() {
        let tempdir = tempdir().expect("tempdir should exist");
        let root = tempdir.path().to_path_buf();
        fs::write(
            root.join("requirement.py"),
            "def requirement_test():\n    return 1\n",
        )
        .expect("requirement file should exist");

        let mut req = requirement("REQ-1");
        req.tests.insert(
            "python".to_string(),
            vec![TraceReference {
                file: PathBuf::from("requirement.py"),
                symbols: vec!["requirement_test".to_string()],
                doc_contains: vec!["Requirement docs".to_string()],
            }],
        );

        let workspace = Workspace {
            root,
            spec_root: tempdir.path().join("docs/spec"),
            config: SyuConfig {
                runtimes: crate::config::RuntimeConfigSet {
                    python: crate::config::RuntimeConfig {
                        command: "false".to_string(),
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: vec![req],
            features: Vec::new(),
        };

        let error =
            apply_autofix(&workspace).expect_err("python inspection failure should bubble up");
        assert!(error.to_string().contains("Python inspector failed"));
    }

    #[cfg(unix)]
    #[test]
    fn apply_autofix_propagates_feature_write_errors() {
        let tempdir = tempdir().expect("tempdir should exist");
        let root = tempdir.path().to_path_buf();
        let feature_path = root.join("feature.rs");
        fs::write(&feature_path, "pub fn feature_impl() {}\n").expect("feature file should exist");

        let mut permissions = fs::metadata(&feature_path).expect("metadata").permissions();
        permissions.set_mode(0o400);
        fs::set_permissions(&feature_path, permissions).expect("permissions should update");

        let mut feat = feature("FEAT-1");
        feat.implementations.insert(
            "rust".to_string(),
            vec![TraceReference {
                file: PathBuf::from("feature.rs"),
                symbols: vec!["feature_impl".to_string()],
                doc_contains: vec!["Feature docs".to_string()],
            }],
        );

        let workspace = Workspace {
            root,
            spec_root: tempdir.path().join("docs/spec"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: vec![feat],
        };

        let error = apply_autofix(&workspace).expect_err("write failure should bubble up");

        let mut restore = fs::metadata(&feature_path).expect("metadata").permissions();
        restore.set_mode(0o644);
        fs::set_permissions(&feature_path, restore).expect("permissions should restore");

        assert!(error.to_string().contains("Permission denied"));
    }

    #[test]
    fn apply_autofix_for_reference_leaves_already_documented_symbols_unchanged() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("trace.rs");
        fs::write(
            &path,
            "/// REQ-1\n/// Explain expected\npub fn expected() {}\n",
        )
        .expect("trace file should exist");

        let mut summary = super::AutofixSummary::default();
        apply_autofix_for_reference(
            tempdir.path(),
            &SyuConfig::default(),
            "REQ-1",
            "rust",
            &TraceReference {
                file: PathBuf::from("trace.rs"),
                symbols: vec!["expected".to_string()],
                doc_contains: vec!["Explain expected".to_string()],
            },
            &mut summary,
        )
        .expect("autofix should succeed");

        assert!(summary.updated_files.is_empty());
        assert_eq!(summary.symbol_updates, 0);
    }

    #[cfg(unix)]
    #[test]
    fn apply_autofix_for_reference_ignores_unreadable_files() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("trace.rs");
        fs::write(&path, "pub fn expected() {}\n").expect("trace file should exist");

        let mut permissions = fs::metadata(&path).expect("metadata").permissions();
        permissions.set_mode(0o000);
        fs::set_permissions(&path, permissions).expect("permissions should update");

        let mut summary = super::AutofixSummary::default();
        let result = apply_autofix_for_reference(
            tempdir.path(),
            &SyuConfig::default(),
            "REQ-1",
            "rust",
            &TraceReference {
                file: PathBuf::from("trace.rs"),
                symbols: vec!["expected".to_string()],
                doc_contains: vec!["Explain expected".to_string()],
            },
            &mut summary,
        );

        let mut restore = fs::metadata(&path).expect("metadata").permissions();
        restore.set_mode(0o644);
        fs::set_permissions(&path, restore).expect("permissions should restore");

        assert!(result.is_ok());
        assert!(summary.updated_files.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn verify_trace_reference_reports_unreadable_files() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("trace.rs");
        fs::write(&path, "// REQ-1\nfn expected() {}\n").expect("file should exist");

        let mut permissions = fs::metadata(&path).expect("metadata").permissions();
        permissions.set_mode(0o000);
        fs::set_permissions(&path, permissions).expect("permissions should update");

        let reference = TraceReference {
            file: PathBuf::from("trace.rs"),
            symbols: vec!["expected".to_string()],
            doc_contains: Vec::new(),
        };
        let mut issues = Vec::new();
        let result = verify_trace_reference(
            tempdir.path(),
            &SyuConfig::default(),
            "REQ-1",
            TraceRole::RequirementTest,
            "rust",
            &reference,
            &mut issues,
        );

        let mut restore = fs::metadata(&path).expect("metadata").permissions();
        restore.set_mode(0o644);
        fs::set_permissions(&path, restore).expect("permissions should restore");

        assert!(!result);
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "trace-file-unreadable")
        );
    }

    #[test]
    fn reference_locations_include_language_and_path() {
        let reference = TraceReference {
            file: PathBuf::from("src/lib.rs"),
            symbols: vec!["run".to_string()],
            doc_contains: Vec::new(),
        };
        assert_eq!(
            format_reference_location("rust", &reference),
            "rust:src/lib.rs"
        );
    }
}
