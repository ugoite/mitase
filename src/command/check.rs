// FEAT-CHECK-001
// REQ-CORE-001

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde::Serialize;

use crate::{
    cli::{CheckArgs, OutputFormat, ValidationGenreFilter, ValidationSeverityFilter},
    config::SyuConfig,
    coverage::validate_symbol_trace_coverage,
    inspect::{apply_symbol_doc_fix, inspect_symbol, supports_rich_inspection},
    language::adapter_for_language,
    model::{
        CheckResult, DefinitionCounts, Feature, Issue, Philosophy, Policy, Requirement, Severity,
        TraceCount, TraceReference, TraceSummary,
    },
    rules::{attach_referenced_rules, referenced_rules, rule_by_code, rule_genre},
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

#[derive(Debug, Clone, Default)]
struct IssueFilters {
    severities: BTreeSet<ValidationSeverityFilter>,
    genres: BTreeSet<ValidationGenreFilter>,
    rules: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize)]
struct FilteredIssueView {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    severities: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    genres: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    rules: Vec<String>,
    displayed_issue_count: usize,
    total_issue_count: usize,
    hidden_issue_count: usize,
}

#[derive(Debug, Serialize)]
struct JsonCheckOutput<'a> {
    #[serde(flatten)]
    result: &'a CheckResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    filtered_view: Option<&'a FilteredIssueView>,
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

impl IssueFilters {
    fn from_args(args: &CheckArgs) -> Self {
        Self {
            severities: args.severity.iter().copied().collect(),
            genres: args.genre.iter().copied().collect(),
            rules: args
                .rule
                .iter()
                .map(|rule| rule.trim())
                .filter(|rule| !rule.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
        }
    }

    fn is_active(&self) -> bool {
        !self.severities.is_empty() || !self.genres.is_empty() || !self.rules.is_empty()
    }

    fn matches(&self, issue: &Issue) -> bool {
        (self.severities.is_empty()
            || self
                .severities
                .iter()
                .any(|candidate| candidate.as_str() == severity_label(&issue.severity)))
            && (self.rules.is_empty() || self.rules.contains(&issue.code))
            && (self.genres.is_empty()
                || rule_genre(&issue.code).is_some_and(|genre| {
                    self.genres
                        .iter()
                        .any(|candidate| candidate.as_str() == genre)
                }))
    }
}

impl FilteredIssueView {
    fn from_filters(
        filters: &IssueFilters,
        displayed_issue_count: usize,
        total_issue_count: usize,
    ) -> Self {
        Self {
            severities: filters
                .severities
                .iter()
                .map(|severity| severity.as_str().to_string())
                .collect(),
            genres: filters
                .genres
                .iter()
                .map(|genre| genre.as_str().to_string())
                .collect(),
            rules: filters.rules.iter().cloned().collect(),
            displayed_issue_count,
            total_issue_count,
            hidden_issue_count: total_issue_count.saturating_sub(displayed_issue_count),
        }
    }

    fn describe_filters(&self) -> String {
        let mut parts = Vec::new();

        if !self.severities.is_empty() {
            parts.push(format!("severity={}", self.severities.join(",")));
        }
        if !self.genres.is_empty() {
            parts.push(format!("genre={}", self.genres.join(",")));
        }
        if !self.rules.is_empty() {
            parts.push(format!("rule={}", self.rules.join(",")));
        }

        parts.join(" ")
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
    let overall_success = result.is_success();
    let filters = IssueFilters::from_args(args);
    let (result, filtered_view) = filter_check_result(result, &filters);

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
            print!(
                "{}",
                render_text_report(overall_success, &result, filtered_view.as_ref())
            );
        }
        OutputFormat::Json => {
            let output = JsonCheckOutput {
                result: &result,
                filtered_view: filtered_view.as_ref(),
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&output)
                    .expect("serializing validate output to JSON should succeed")
            );
        }
    }

    Ok(if overall_success { 0 } else { 1 })
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

fn severity_label(severity: &Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

fn filter_check_result(
    result: CheckResult,
    filters: &IssueFilters,
) -> (CheckResult, Option<FilteredIssueView>) {
    if !filters.is_active() {
        return (result, None);
    }

    let CheckResult {
        workspace_root,
        definition_counts,
        trace_summary,
        issues: existing_issues,
        referenced_rules: _,
    } = result;

    let total_issue_count = existing_issues.len();
    let issues: Vec<_> = existing_issues
        .into_iter()
        .filter(|issue| filters.matches(issue))
        .collect();
    let displayed_issue_count = issues.len();
    let filtered_view =
        FilteredIssueView::from_filters(filters, displayed_issue_count, total_issue_count);
    let referenced_rules = referenced_rules(&issues);

    (
        CheckResult {
            workspace_root,
            definition_counts,
            trace_summary,
            issues,
            referenced_rules,
        },
        Some(filtered_view),
    )
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

fn render_text_report(
    overall_success: bool,
    result: &CheckResult,
    filtered_view: Option<&FilteredIssueView>,
) -> String {
    let mut output = String::new();
    let status = if overall_success { "passed" } else { "failed" };
    let filtered_suffix = if filtered_view.is_some() {
        " (filtered view)"
    } else {
        ""
    };

    writeln!(&mut output, "syu validate {status}{filtered_suffix}")
        .expect("writing to String must succeed");
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

    if let Some(filtered_view) = filtered_view {
        writeln!(&mut output, "filters: {}", filtered_view.describe_filters())
            .expect("writing to String must succeed");
        writeln!(
            &mut output,
            "showing {} of {} issues after filtering",
            filtered_view.displayed_issue_count, filtered_view.total_issue_count
        )
        .expect("writing to String must succeed");
    }

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
    } else if let Some(filtered_view) = filtered_view
        && filtered_view.total_issue_count > 0
    {
        writeln!(&mut output).expect("writing to String must succeed");
        writeln!(&mut output, "issues:").expect("writing to String must succeed");
        writeln!(&mut output, "- no issues matched the active filters.")
            .expect("writing to String must succeed");
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
            "SYU-delivery-invalid-001",
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
            "SYU-delivery-planned-001",
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

fn format_issue_id_list(ids: &[String]) -> String {
    ids.iter()
        .map(|id| format!("`{id}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn validate_linked_delivery_states(
    owner_kind: &str,
    owner_id: &str,
    owner_status: Option<DeliveryStatus>,
    linked_kind_plural: &str,
    linked_statuses: Vec<(&str, Option<DeliveryStatus>)>,
    issues: &mut Vec<Issue>,
) {
    let Some(owner_status) = owner_status else {
        return;
    };

    let mut planned_ids = Vec::new();
    let mut implemented_ids = Vec::new();

    for (linked_id, linked_status) in linked_statuses {
        match linked_status {
            Some(DeliveryStatus::Planned) => planned_ids.push(linked_id.to_string()),
            Some(DeliveryStatus::Implemented) => implemented_ids.push(linked_id.to_string()),
            None => {}
        }
    }

    match owner_status {
        DeliveryStatus::Planned if !implemented_ids.is_empty() => issues.push(Issue::warning(
            "SYU-delivery-agreement-001",
            format!("{owner_kind} {owner_id}"),
            Some("status".to_string()),
            format!(
                "{owner_kind} `{owner_id}` is marked `planned` but links to implemented {linked_kind_plural}: {}.",
                format_issue_id_list(&implemented_ids),
            ),
            Some(format!(
                "Mark `{owner_id}` implemented if the linked work is already delivered, or revisit the linked item statuses and traces."
            )),
        )),
        DeliveryStatus::Implemented if !planned_ids.is_empty() && implemented_ids.is_empty() => {
            issues.push(Issue::warning(
                "SYU-delivery-agreement-001",
                format!("{owner_kind} {owner_id}"),
                Some("status".to_string()),
                format!(
                    "{owner_kind} `{owner_id}` is marked `implemented` but links to planned {linked_kind_plural} and none are implemented: {}.",
                    format_issue_id_list(&planned_ids),
                ),
                Some(format!(
                    "Mark at least one linked item implemented, or change `{owner_id}` back to `planned` if delivery is still in progress."
                )),
            ));
        }
        _ => {}
    }
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
                "SYU-workspace-duplicate-001",
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

fn validate_duplicate_links(
    owner_kind: &str,
    owner_id: &str,
    relation_name: &str,
    target_kind: &str,
    values: &[String],
    issues: &mut Vec<Issue>,
) {
    let mut seen = HashSet::new();

    for value in values {
        if seen.insert(value.as_str()) {
            continue;
        }

        issues.push(Issue::error(
            "SYU-graph-duplicate-001",
            format!("{owner_kind} {owner_id}"),
            Some(relation_name.to_string()),
            format!(
                "{owner_kind} `{owner_id}` repeats linked {target_kind} `{value}` in `{relation_name}`."
            ),
            Some(format!(
                "Remove the duplicate `{value}` entry from `{relation_name}` in {owner_kind} `{owner_id}`."
            )),
        ));
    }
}

fn validate_duplicate_trace_references(
    owner_id: &str,
    role: TraceRole,
    language: &str,
    references: &[TraceReference],
    issues: &mut Vec<Issue>,
) {
    let mut seen = HashSet::new();
    let subject = format!("{} {}", role.subject_kind(), owner_id);

    for reference in references {
        let key = (
            reference.file.clone(),
            reference.symbols.clone(),
            reference.doc_contains.clone(),
        );
        if seen.insert(key) {
            continue;
        }

        issues.push(Issue::error(
            "SYU-trace-duplicate-001",
            subject.clone(),
            Some(format_reference_location(language, reference)),
            format!(
                "{} `{owner_id}` repeats the same `{language}` {} entry: {}.",
                role.subject_kind(),
                role.relation_name(),
                describe_trace_reference(reference),
            ),
            Some(format!(
                "Remove the duplicate `{language}` {} entry from `{owner_id}`.",
                role.relation_name()
            )),
        ));
    }
}

fn describe_trace_reference(reference: &TraceReference) -> String {
    let symbols = reference
        .symbols
        .iter()
        .map(|symbol| format!("`{symbol}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let doc_contains = reference
        .doc_contains
        .iter()
        .map(|snippet| format!("`{snippet}`"))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "file=`{}` symbols=[{symbols}] doc_contains=[{doc_contains}]",
        reference.file.display()
    )
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
    validate_duplicate_links(
        "philosophy",
        &philosophy.id,
        "linked_policies",
        "policy",
        &philosophy.linked_policies,
        issues,
    );

    if philosophy.linked_policies.is_empty() {
        issues.push(Issue::warning(
            "SYU-graph-links-001",
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
                        "SYU-graph-reciprocal-001",
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
                "SYU-graph-reference-001",
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
    validate_duplicate_links(
        "policy",
        &policy.id,
        "linked_philosophies",
        "philosophy",
        &policy.linked_philosophies,
        issues,
    );
    validate_duplicate_links(
        "policy",
        &policy.id,
        "linked_requirements",
        "requirement",
        &policy.linked_requirements,
        issues,
    );

    if policy.linked_philosophies.is_empty() {
        issues.push(Issue::warning(
            "SYU-graph-links-001",
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
            "SYU-graph-links-001",
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
                        "SYU-graph-reciprocal-001",
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
                "SYU-graph-reference-001",
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
                        "SYU-graph-reciprocal-001",
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
                "SYU-graph-reference-001",
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
    validate_duplicate_links(
        "requirement",
        &requirement.id,
        "linked_policies",
        "policy",
        &requirement.linked_policies,
        issues,
    );
    validate_duplicate_links(
        "requirement",
        &requirement.id,
        "linked_features",
        "feature",
        &requirement.linked_features,
        issues,
    );

    if requirement.linked_policies.is_empty() {
        issues.push(Issue::warning(
            "SYU-graph-links-001",
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
            "SYU-graph-links-001",
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
                        "SYU-graph-reciprocal-001",
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
                "SYU-graph-reference-001",
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
                        "SYU-graph-reciprocal-001",
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
                "SYU-graph-reference-001",
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

    validate_linked_delivery_states(
        "requirement",
        &requirement.id,
        status,
        "features",
        requirement
            .linked_features
            .iter()
            .filter_map(|feature_id| {
                features_by_id.get(feature_id.as_str()).map(|feature| {
                    (
                        feature_id.as_str(),
                        normalize_delivery_status(&feature.status),
                    )
                })
            })
            .collect(),
        issues,
    );

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
    validate_duplicate_links(
        "feature",
        &feature.id,
        "linked_requirements",
        "requirement",
        &feature.linked_requirements,
        issues,
    );

    if feature.linked_requirements.is_empty() {
        issues.push(Issue::warning(
            "SYU-graph-links-001",
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
                        "SYU-graph-reciprocal-001",
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
                "SYU-graph-reference-001",
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

    validate_linked_delivery_states(
        "feature",
        &feature.id,
        status,
        "requirements",
        feature
            .linked_requirements
            .iter()
            .filter_map(|requirement_id| {
                requirements_by_id
                    .get(requirement_id.as_str())
                    .map(|requirement| {
                        (
                            requirement_id.as_str(),
                            normalize_delivery_status(&requirement.status),
                        )
                    })
            })
            .collect(),
        issues,
    );

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
    for (language, references) in references_by_language {
        validate_duplicate_trace_references(
            target.owner_id,
            target.role,
            language,
            references,
            issues,
        );
    }

    match target.status {
        Some(DeliveryStatus::Planned) => {
            if !references_by_language.is_empty() {
                issues.push(Issue::error(
                    "SYU-delivery-planned-002",
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
                    "SYU-delivery-implemented-001",
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
                "SYU-delivery-missing-001",
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
        "SYU-graph-orphaned-001",
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
            "SYU-trace-language-001",
            subject,
            Some(format_reference_location(language, reference)),
            format!(
                "Language `{language}` is not supported. Built-in adapters currently cover Rust, Python, TypeScript, Shell, YAML, JSON, Markdown, and Gitignore."
            ),
            Some(format!(
                "Use a supported language alias such as `rust`, `python`, `typescript`, `shell`, `yaml`, `json`, `markdown`, or `gitignore` for `{owner_id}`."
            )),
        ));
        return false;
    };

    if reference.file.as_os_str().is_empty() {
        issues.push(Issue::error(
            "SYU-trace-file-001",
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
            "SYU-trace-file-002",
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
            "SYU-trace-extension-001",
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
                "SYU-trace-unreadable-001",
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
            "SYU-trace-id-001",
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
            "SYU-trace-symbol-001",
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
                "SYU-trace-docscope-001",
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
                "SYU-trace-symbol-002",
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
                        "SYU-trace-inspection-001",
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
                "SYU-trace-symbol-003",
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
                                "SYU-trace-doc-001",
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
                        "SYU-trace-docsupport-001",
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
            "SYU-workspace-blank-001",
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
        collections::{BTreeMap, BTreeSet, HashMap},
        fs,
        path::{Path, PathBuf},
    };

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use tempfile::tempdir;

    use crate::{
        cli::{ValidationGenreFilter, ValidationSeverityFilter},
        config::SyuConfig,
        model::{Feature, Issue, Philosophy, Policy, Requirement, TraceReference},
        rules::referenced_rules,
        workspace::Workspace,
    };

    use super::{
        FilteredIssueView, IssueFilters, TraceRole, apply_autofix, apply_autofix_for_reference,
        collect_check_result, describe_trace_reference, filter_check_result,
        format_reference_location, render_text_report, run_check_command, validate_duplicate_links,
        validate_duplicate_trace_references, validate_feature, validate_non_empty_field,
        validate_philosophy, validate_policy, validate_requirement, validate_unique_ids,
        verify_trace_reference,
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
        assert_eq!(result.issues[0].code, "SYU-workspace-load-001");
    }

    #[test]
    fn run_check_command_handles_workspace_load_errors() {
        let code = run_check_command(&crate::cli::CheckArgs {
            workspace: PathBuf::from("/definitely/missing-syu-workspace"),
            format: crate::cli::OutputFormat::Json,
            severity: Vec::new(),
            genre: Vec::new(),
            rule: Vec::new(),
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
        fs::create_dir_all(workspace.join("docs/syu/philosophy")).expect("philosophy dir");
        fs::create_dir_all(workspace.join("docs/syu/policies")).expect("policies dir");
        fs::create_dir_all(workspace.join("docs/syu/requirements")).expect("requirements dir");
        fs::create_dir_all(workspace.join("docs/syu/features")).expect("features dir");

        fs::write(
            workspace.join("syu.yaml"),
            "version: 1\nruntimes:\n  python:\n    command: false\n",
        )
        .expect("config should exist");
        fs::write(
            workspace.join("docs/syu/philosophy/foundation.yaml"),
            "category: Foundations\nversion: 1\n\nphilosophies:\n  - id: PHIL-1\n    title: Foundation\n    product_design_principle: Keep it clear.\n    coding_guideline: Keep it explicit.\n    linked_policies:\n      - POL-1\n",
        )
        .expect("philosophy should exist");
        fs::write(
            workspace.join("docs/syu/policies/policies.yaml"),
            "category: Policies\nversion: 1\n\npolicies:\n  - id: POL-1\n    title: Policy\n    summary: Rule summary.\n    description: Rule description.\n    linked_philosophies:\n      - PHIL-1\n    linked_requirements:\n      - REQ-1\n",
        )
        .expect("policy should exist");
        fs::write(
            workspace.join("docs/syu/requirements/core.yaml"),
            "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-1\n    title: Requirement\n    description: Requirement description.\n    priority: high\n    status: implemented\n    linked_policies:\n      - POL-1\n    linked_features:\n      - FEAT-1\n    tests:\n      python:\n        - file: tests/test_sample.py\n          symbols:\n            - requirement_test\n          doc_contains:\n            - Requirement docs\n",
        )
        .expect("requirement should exist");
        fs::write(
            workspace.join("docs/syu/features/features.yaml"),
            "version: 1\nfiles:\n  - kind: core\n    file: core.yaml\n",
        )
        .expect("feature registry should exist");
        fs::write(
            workspace.join("docs/syu/features/core.yaml"),
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
            severity: Vec::new(),
            genre: Vec::new(),
            rule: Vec::new(),
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

        let report = render_text_report(true, &result, None);
        assert!(report.contains("syu validate passed"));
        assert!(report.contains("issues:"));
        assert!(report.contains("[Warning] warn subject: message"));
    }

    #[test]
    fn filter_check_result_scopes_visible_issues() {
        let issues = vec![
            Issue::warning("SYU-graph-links-001", "subject", None, "message", None),
            Issue::error("SYU-trace-file-001", "subject", None, "message", None),
        ];
        let result = crate::model::CheckResult {
            workspace_root: PathBuf::from("."),
            definition_counts: Default::default(),
            trace_summary: Default::default(),
            referenced_rules: referenced_rules(&issues),
            issues,
        };
        let filters = IssueFilters {
            severities: [ValidationSeverityFilter::Warning].into_iter().collect(),
            genres: [ValidationGenreFilter::Graph].into_iter().collect(),
            rules: BTreeSet::new(),
        };

        let (filtered, filtered_view) = filter_check_result(result, &filters);

        assert_eq!(filtered.issues.len(), 1);
        assert_eq!(filtered.issues[0].code, "SYU-graph-links-001");
        assert_eq!(filtered.referenced_rules.len(), 1);
        assert_eq!(filtered.referenced_rules[0].code, "SYU-graph-links-001");
        let filtered_view = filtered_view.expect("filters should produce filtered metadata");
        assert_eq!(filtered_view.displayed_issue_count, 1);
        assert_eq!(filtered_view.total_issue_count, 2);
        assert_eq!(filtered_view.hidden_issue_count, 1);
    }

    #[test]
    fn render_text_report_explains_filtered_views_without_matches() {
        let result = crate::model::CheckResult {
            workspace_root: PathBuf::from("."),
            definition_counts: Default::default(),
            trace_summary: Default::default(),
            referenced_rules: Vec::new(),
            issues: Vec::new(),
        };
        let filtered_view = FilteredIssueView {
            severities: vec!["warning".to_string()],
            genres: Vec::new(),
            rules: Vec::new(),
            displayed_issue_count: 0,
            total_issue_count: 2,
            hidden_issue_count: 2,
        };

        let report = render_text_report(false, &result, Some(&filtered_view));

        assert!(report.contains("syu validate failed (filtered view)"));
        assert!(report.contains("filters: severity=warning"));
        assert!(report.contains("showing 0 of 2 issues after filtering"));
        assert!(report.contains("no issues matched the active filters."));
    }

    #[test]
    fn filtered_issue_view_describes_genre_filters() {
        let filtered_view = FilteredIssueView {
            severities: Vec::new(),
            genres: vec!["trace".to_string()],
            rules: Vec::new(),
            displayed_issue_count: 1,
            total_issue_count: 1,
            hidden_issue_count: 0,
        };

        assert_eq!(filtered_view.describe_filters(), "genre=trace");
    }

    #[test]
    fn validate_unique_ids_reports_duplicates() {
        let mut issues = Vec::new();
        validate_unique_ids("feature", ["FEAT-1", "FEAT-1"].into_iter(), &mut issues);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "SYU-workspace-duplicate-001");
    }

    #[test]
    fn validate_duplicate_links_reports_repeated_relationships() {
        let mut issues = Vec::new();
        validate_duplicate_links(
            "requirement",
            "REQ-1",
            "linked_features",
            "feature",
            &["FEAT-1".to_string(), "FEAT-1".to_string()],
            &mut issues,
        );

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "SYU-graph-duplicate-001");
        assert_eq!(issues[0].location.as_deref(), Some("linked_features"));
        assert!(issues[0].message.contains("linked feature `FEAT-1`"));
    }

    #[test]
    fn validate_duplicate_trace_references_reports_exact_entries() {
        let mut issues = Vec::new();
        let duplicate = TraceReference {
            file: PathBuf::from("src/lib.rs"),
            symbols: vec!["trace_symbol".to_string()],
            doc_contains: vec!["REQ-1".to_string()],
        };

        validate_duplicate_trace_references(
            "REQ-1",
            TraceRole::RequirementTest,
            "rust",
            &[duplicate.clone(), duplicate.clone()],
            &mut issues,
        );

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "SYU-trace-duplicate-001");
        assert_eq!(issues[0].location.as_deref(), Some("rust:src/lib.rs"));
        assert!(issues[0].message.contains("file=`src/lib.rs`"));
        assert!(issues[0].message.contains("symbols=[`trace_symbol`]"));
        assert_eq!(
            describe_trace_reference(&duplicate),
            "file=`src/lib.rs` symbols=[`trace_symbol`] doc_contains=[`REQ-1`]"
        );
    }

    #[test]
    fn validate_non_empty_field_reports_blank_values() {
        let mut issues = Vec::new();
        validate_non_empty_field("feature", "title", "   ", &mut issues);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "SYU-workspace-blank-001");
        assert_eq!(issues[0].location.as_deref(), Some("title"));
    }

    #[test]
    fn validate_philosophy_reports_blank_and_reference_errors() {
        let mut entry = philosophy("PHIL-1");
        entry.title.clear();
        entry.linked_policies.push("POL-1".to_string());

        let mut issues = Vec::new();
        validate_philosophy(&entry, &HashMap::new(), &mut issues);

        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-workspace-blank-001")
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-graph-reference-001")
        );
    }

    #[test]
    fn validate_philosophy_warns_when_unlinked() {
        let entry = philosophy("PHIL-1");
        let mut issues = Vec::new();
        validate_philosophy(&entry, &HashMap::new(), &mut issues);
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-graph-links-001")
        );
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
                .any(|issue| issue.code == "SYU-graph-reciprocal-001")
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

        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-workspace-blank-001")
        );
        assert!(
            issues
                .iter()
                .filter(|issue| issue.code == "SYU-graph-reciprocal-001")
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
                .filter(|issue| issue.code == "SYU-graph-links-001")
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
            issue.code == "SYU-graph-reference-001"
                && issue.location.as_deref() == Some("PHIL-MISSING")
        }));
        assert!(issues.iter().any(|issue| {
            issue.code == "SYU-graph-reference-001"
                && issue.location.as_deref() == Some("REQ-MISSING")
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

        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-workspace-blank-001")
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-graph-links-001")
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-delivery-missing-001")
        );
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
            issue.code == "SYU-graph-reciprocal-001" && issue.location.as_deref() == Some("POL-1")
        }));
        assert!(issues.iter().any(|issue| {
            issue.code == "SYU-graph-reference-001"
                && issue.location.as_deref() == Some("POL-MISSING")
        }));
        assert!(issues.iter().any(|issue| {
            issue.code == "SYU-graph-reciprocal-001" && issue.location.as_deref() == Some("FEAT-1")
        }));
        assert!(issues.iter().any(|issue| {
            issue.code == "SYU-graph-reference-001"
                && issue.location.as_deref() == Some("FEAT-MISSING")
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

        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-workspace-blank-001")
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-graph-links-001")
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-delivery-implemented-001")
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
                .any(|issue| issue.code == "SYU-delivery-planned-002")
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

        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-delivery-invalid-001")
        );
    }

    #[test]
    fn validate_requirement_warns_when_planned_links_to_implemented_features() {
        let mut entry = requirement("REQ-1");
        entry.status = "planned".to_string();
        entry.linked_features = vec!["FEAT-1".to_string(), "FEAT-2".to_string()];

        let linked_feature_one = feature("FEAT-1");
        let mut linked_feature_two = feature("FEAT-2");
        linked_feature_two.status = "planned".to_string();

        let mut features = HashMap::new();
        features.insert("FEAT-1", &linked_feature_one);
        features.insert("FEAT-2", &linked_feature_two);

        let mut issues = Vec::new();
        let mut trace_count = Default::default();
        validate_requirement(
            &entry,
            &HashMap::new(),
            &features,
            &SyuConfig::default(),
            Path::new("."),
            &mut issues,
            &mut trace_count,
        );

        let agreement = issues
            .iter()
            .find(|issue| issue.code == "SYU-delivery-agreement-001")
            .expect("agreement warning");
        assert!(agreement.message.contains("implemented features"));
        assert!(agreement.message.contains("`FEAT-1`"));
        assert!(!agreement.message.contains("`FEAT-2`"));
    }

    #[test]
    fn validate_requirement_ignores_linked_features_with_unknown_delivery_state() {
        let mut entry = requirement("REQ-1");
        entry.status = "planned".to_string();
        entry.linked_features = vec!["FEAT-1".to_string()];

        let mut linked_feature = feature("FEAT-1");
        linked_feature.status = "proposed".to_string();

        let mut features = HashMap::new();
        features.insert("FEAT-1", &linked_feature);

        let mut issues = Vec::new();
        let mut trace_count = Default::default();
        validate_requirement(
            &entry,
            &HashMap::new(),
            &features,
            &SyuConfig::default(),
            Path::new("."),
            &mut issues,
            &mut trace_count,
        );

        assert!(
            !issues
                .iter()
                .any(|issue| issue.code == "SYU-delivery-agreement-001")
        );
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

        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-delivery-invalid-001")
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-trace-language-001")
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
                .any(|issue| issue.code == "SYU-delivery-planned-001")
        );
    }

    #[test]
    fn validate_feature_warns_when_implemented_links_only_to_planned_requirements() {
        let mut entry = feature("FEAT-1");
        entry.linked_requirements = vec!["REQ-1".to_string(), "REQ-2".to_string()];

        let mut linked_requirement_one = requirement("REQ-1");
        linked_requirement_one.status = "planned".to_string();
        let mut linked_requirement_two = requirement("REQ-2");
        linked_requirement_two.status = "planned".to_string();

        let mut requirements = HashMap::new();
        requirements.insert("REQ-1", &linked_requirement_one);
        requirements.insert("REQ-2", &linked_requirement_two);

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

        let agreement = issues
            .iter()
            .find(|issue| issue.code == "SYU-delivery-agreement-001")
            .expect("agreement warning");
        assert!(agreement.message.contains("none are implemented"));
        assert!(agreement.message.contains("`REQ-1`"));
        assert!(agreement.message.contains("`REQ-2`"));
    }

    #[test]
    fn validate_feature_skips_delivery_agreement_warning_when_any_requirement_is_implemented() {
        let mut entry = feature("FEAT-1");
        entry.linked_requirements = vec!["REQ-1".to_string(), "REQ-2".to_string()];

        let mut linked_requirement_one = requirement("REQ-1");
        linked_requirement_one.status = "planned".to_string();
        let linked_requirement_two = requirement("REQ-2");

        let mut requirements = HashMap::new();
        requirements.insert("REQ-1", &linked_requirement_one);
        requirements.insert("REQ-2", &linked_requirement_two);

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

        assert!(
            !issues
                .iter()
                .any(|issue| issue.code == "SYU-delivery-agreement-001")
        );
    }

    #[test]
    fn validate_feature_warning_mentions_absence_of_implemented_requirements() {
        let mut entry = feature("FEAT-1");
        entry.linked_requirements = vec!["REQ-1".to_string(), "REQ-2".to_string()];

        let mut linked_requirement_one = requirement("REQ-1");
        linked_requirement_one.status = "planned".to_string();
        let mut linked_requirement_two = requirement("REQ-2");
        linked_requirement_two.status = "proposed".to_string();

        let mut requirements = HashMap::new();
        requirements.insert("REQ-1", &linked_requirement_one);
        requirements.insert("REQ-2", &linked_requirement_two);

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

        let agreement = issues
            .iter()
            .find(|issue| issue.code == "SYU-delivery-agreement-001")
            .expect("agreement warning");
        assert!(agreement.message.contains("none are implemented"));
        assert!(agreement.message.contains("`REQ-1`"));
        assert!(!agreement.message.contains("`REQ-2`"));
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
            issue.code == "SYU-graph-reciprocal-001" && issue.location.as_deref() == Some("REQ-1")
        }));
        assert!(issues.iter().any(|issue| {
            issue.code == "SYU-graph-reference-001"
                && issue.location.as_deref() == Some("REQ-MISSING")
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
        assert_eq!(issues[0].code, "SYU-trace-language-001");
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
        assert_eq!(issues[0].code, "SYU-trace-file-001");
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
        assert_eq!(issues[0].code, "SYU-trace-file-002");
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
                .any(|issue| issue.code == "SYU-trace-extension-001")
        );
        assert!(issues.iter().any(|issue| issue.code == "SYU-trace-id-001"));
        assert!(
            issues
                .iter()
                .filter(|issue| issue.code == "SYU-trace-symbol-002")
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
                .any(|issue| issue.code == "SYU-trace-symbol-003")
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
                .any(|issue| issue.code == "SYU-trace-symbol-001")
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
    fn verify_trace_reference_accepts_valid_gitignore_files() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join(".gitignore");
        fs::write(&path, "# FEAT-CONTRIB-002\n/.worktrees/\n").expect("file should exist");

        let reference = TraceReference {
            file: PathBuf::from(".gitignore"),
            symbols: vec!["FEAT-CONTRIB-002".to_string(), "/.worktrees/".to_string()],
            doc_contains: Vec::new(),
        };
        let mut issues = Vec::new();
        assert!(verify_trace_reference(
            tempdir.path(),
            &SyuConfig::default(),
            "FEAT-CONTRIB-002",
            TraceRole::FeatureImplementation,
            "gitignore",
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
                .any(|issue| issue.code == "SYU-trace-inspection-001")
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
                .filter(|issue| issue.code == "SYU-trace-doc-001")
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
                .any(|issue| issue.code == "SYU-trace-docsupport-001")
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
                .any(|issue| issue.code == "SYU-trace-docscope-001")
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
            spec_root: root.join("docs/syu"),
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
            spec_root: root.join("docs/syu"),
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
            spec_root: tempdir.path().join("docs/syu"),
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
            spec_root: tempdir.path().join("docs/syu"),
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
                .any(|issue| issue.code == "SYU-trace-unreadable-001")
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
