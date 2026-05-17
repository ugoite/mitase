// FEAT-CHECK-001
// REQ-CORE-001

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fmt::Write,
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::Result;
use serde::Serialize;

use crate::{
    cli::{CheckArgs, OutputFormat, ValidationGenreFilter, ValidationSeverityFilter},
    command::shell_quote_path,
    config::{SyuConfig, TraceOwnershipMode},
    coverage::{
        normalize_relative_path, normalized_symbol_trace_coverage_ignored_paths,
        path_matches_ignored_generated_directory, validate_symbol_trace_coverage,
    },
    history::{HistoricalIdIndex, build_historical_id_index},
    inspect::{apply_symbol_doc_fix, inspect_symbol, supports_rich_inspection},
    language::adapter_for_language,
    model::{
        CheckResult, DefinitionCounts, Feature, FeatureDocument, FeatureRegistryDocument,
        FeatureRegistryEntry, Issue, OwnershipEntry, OwnershipManifest, Philosophy,
        PhilosophyDocument, Policy, PolicyDocument, Requirement, RequirementDocument, Severity,
        TraceCount, TraceReference, TraceSummary,
    },
    rules::{all_rules, attach_referenced_rules, referenced_rules, rule_genre},
    workspace::{
        Workspace, load_philosophy_documents_with_paths, load_policy_documents_with_paths,
        load_requirement_documents_with_paths, load_workspace,
        load_workspace_allowing_missing_feature_registry,
    },
};

use super::issue_text::{TextIssueFormat, format_text_issue};

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

#[derive(Debug, Clone, Copy)]
struct ValidationResources<'a> {
    config: &'a SyuConfig,
    root: &'a Path,
    spec_only: bool,
}

#[derive(Debug, Clone, Copy)]
struct RequirementValidationIndex<'a> {
    policies_by_id: &'a HashMap<&'a str, &'a Policy>,
    features_by_id: &'a HashMap<&'a str, &'a Feature>,
}

#[derive(Debug, Default, Clone)]
struct AutofixSummary {
    updated_files: BTreeSet<PathBuf>,
    symbol_updates: usize,
}

#[derive(Debug)]
struct MutableLoadedDocument<T> {
    path: PathBuf,
    document: T,
    changed: bool,
}

#[derive(Debug, Default, Clone, Serialize)]
struct AutofixPlan {
    planned_updates: usize,
    updated_files: BTreeSet<PathBuf>,
    changes: Vec<AutofixPlanChange>,
}

#[derive(Debug, Clone, Serialize)]
struct AutofixPlanChange {
    path: PathBuf,
    rules: Vec<&'static str>,
    summary: String,
}

#[derive(Debug, Default)]
struct AutofixRun {
    summary: AutofixSummary,
    plan: Option<AutofixPlan>,
}

#[derive(Debug, Default)]
struct AutofixTransaction {
    backups: BTreeMap<PathBuf, Option<String>>,
    writes_committed: usize,
}

impl AutofixTransaction {
    fn write(&mut self, path: &Path, contents: &str) -> Result<()> {
        if !self.backups.contains_key(path) {
            self.backups.insert(
                path.to_path_buf(),
                match fs::read_to_string(path) {
                    Ok(original) => Some(original),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                    Err(error) => return Err(error.into()),
                },
            );
        }

        fs::write(path, contents).map_err(Into::into).inspect(|_| {
            self.writes_committed += 1;
        })
    }

    fn committed_writes(&self) -> usize {
        self.writes_committed
    }

    fn rollback(&self) -> Result<()> {
        for (path, original) in self.backups.iter().rev() {
            match original {
                Some(contents) => fs::write(path, contents)?,
                None => {
                    fs::remove_file(path)?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AutofixMode {
    Apply,
    Plan,
}

#[derive(Debug, Clone)]
enum OwnershipDeclaration {
    Satisfied,
    Missing {
        manifest_path: Option<PathBuf>,
    },
    Invalid {
        manifest_path: PathBuf,
        reason: String,
    },
}

#[derive(Debug, Clone, Default)]
struct IssueFilters {
    severities: BTreeSet<ValidationSeverityFilter>,
    genres: BTreeSet<ValidationGenreFilter>,
    rules: BTreeSet<String>,
    ids: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize)]
struct FilteredIssueView {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    severities: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    genres: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    rules: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    ids: Vec<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    autofix_plan: Option<&'a AutofixPlan>,
}

#[derive(Debug, Clone)]
struct TextReportSummary {
    checked_rule_count: usize,
    workspace_item_count: usize,
    checked_genres: Vec<&'static str>,
    disabled_checks: Vec<DisabledRuleNotice>,
}

#[derive(Debug, Clone)]
struct DisabledRuleNotice {
    config_key: &'static str,
    rule_count: usize,
}

#[derive(Debug, Clone, Copy)]
struct DisabledRuleGroup {
    config_key: &'static str,
    codes: &'static [&'static str],
}

const RULE_GENRE_ORDER: &[&str] = &["workspace", "graph", "delivery", "trace", "coverage"];
const ORPHAN_RULE_CODES: &[&str] = &["SYU-graph-orphaned-001"];
const RECIPROCAL_RULE_CODES: &[&str] = &["SYU-graph-reciprocal-001"];
const COVERAGE_RULE_CODES: &[&str] = &[
    "SYU-coverage-walk-001",
    "SYU-coverage-read-001",
    "SYU-coverage-parse-001",
    "SYU-coverage-public-001",
    "SYU-coverage-test-001",
];
const HISTORICAL_ID_RULE_CODES: &[&str] = &["SYU-workspace-historical-001"];

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
            ids: args
                .id
                .iter()
                .map(|id| id.trim())
                .filter(|id| !id.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
        }
    }

    fn is_active(&self) -> bool {
        !self.severities.is_empty()
            || !self.genres.is_empty()
            || !self.rules.is_empty()
            || !self.ids.is_empty()
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
            && (self.ids.is_empty()
                || self.ids.iter().any(|id| {
                    issue.subject.ends_with(id)
                        || issue.message.contains(id)
                        || issue
                            .suggestion
                            .as_deref()
                            .is_some_and(|suggestion| suggestion.contains(id))
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
            ids: filters.ids.iter().cloned().collect(),
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
        if !self.ids.is_empty() {
            parts.push(format!("id={}", self.ids.join(",")));
        }

        parts.join(" ")
    }
}

impl TextReportSummary {
    fn from_config(config: &SyuConfig, definition_counts: &DefinitionCounts) -> Self {
        let disabled_groups = disabled_rule_groups(config);
        let disabled_codes: HashSet<_> = disabled_groups
            .iter()
            .flat_map(|group| group.codes.iter().copied())
            .collect();
        let disabled_checks: Vec<_> = disabled_groups
            .into_iter()
            .map(DisabledRuleNotice::from_group)
            .collect();
        let enabled_rules: Vec<_> = all_rules()
            .into_iter()
            .filter(|rule| !disabled_codes.contains(rule.code.as_str()))
            .collect();
        let checked_genres = RULE_GENRE_ORDER
            .iter()
            .copied()
            .filter(|genre| enabled_rules.iter().any(|rule| rule.genre == *genre))
            .collect();

        Self {
            checked_rule_count: enabled_rules.len(),
            workspace_item_count: workspace_item_count(definition_counts),
            checked_genres,
            disabled_checks,
        }
    }
}

impl DisabledRuleNotice {
    fn from_group(group: DisabledRuleGroup) -> Self {
        Self {
            config_key: group.config_key,
            rule_count: group.codes.len(),
        }
    }

    fn describe(&self) -> String {
        let noun = if self.rule_count == 1 {
            "rule"
        } else {
            "rules"
        };
        format!("{}=false ({} {})", self.config_key, self.rule_count, noun)
    }
}

fn disabled_rule_groups(config: &SyuConfig) -> Vec<DisabledRuleGroup> {
    let mut groups = Vec::new();
    if !config.validate.require_non_orphaned_items {
        groups.push(DisabledRuleGroup {
            config_key: "validate.require_non_orphaned_items",
            codes: ORPHAN_RULE_CODES,
        });
    }
    if !config.validate.require_reciprocal_links {
        groups.push(DisabledRuleGroup {
            config_key: "validate.require_reciprocal_links",
            codes: RECIPROCAL_RULE_CODES,
        });
    }
    if !config.validate.require_symbol_trace_coverage {
        groups.push(DisabledRuleGroup {
            config_key: "validate.require_symbol_trace_coverage",
            codes: COVERAGE_RULE_CODES,
        });
    }
    if !config.validate.historical_ids.enabled {
        groups.push(DisabledRuleGroup {
            config_key: "validate.historical_ids.enabled",
            codes: HISTORICAL_ID_RULE_CODES,
        });
    }
    groups
}

fn workspace_item_count(definition_counts: &DefinitionCounts) -> usize {
    definition_counts.philosophies
        + definition_counts.policies
        + definition_counts.requirements
        + definition_counts.features
}

// FEAT-CHECK-001
pub fn run_check_command(args: &CheckArgs) -> Result<i32> {
    let workspace_result = load_workspace(&args.workspace).or_else(|error| {
        if !error.to_string().contains("feature registry") {
            return Err(error);
        }

        let workspace = load_workspace_allowing_missing_feature_registry(&args.workspace)?;
        if effective_fix(args, &workspace.config) {
            Ok(workspace)
        } else {
            Err(error)
        }
    });

    let (result, autofix_run, text_summary) = match workspace_result {
        Ok(workspace) => {
            let should_fix = effective_fix(args, &workspace.config);
            if args.dry_run && !should_fix {
                return Err(anyhow::anyhow!(
                    "`--dry-run` requires an effective autofix path; pass `--fix` or remove `--no-fix`."
                ));
            }
            let autofix_run = if should_fix {
                Some(if args.dry_run {
                    plan_autofix(&workspace)?
                } else {
                    apply_autofix_run(&workspace)?
                })
            } else {
                None
            };
            let workspace = if should_fix && !args.dry_run {
                with_validate_overrides(load_workspace(&args.workspace)?, args)
            } else {
                with_validate_overrides(workspace, args)
            };
            let mut result =
                collect_check_result_from_workspace_with_mode(&workspace, args.spec_only);
            apply_cli_override_issue_guidance(&mut result, args);
            let text_summary =
                TextReportSummary::from_config(&workspace.config, &result.definition_counts);
            (result, autofix_run, Some(text_summary))
        }
        Err(error) => (
            CheckResult::from_load_error(args.workspace.to_path_buf(), error.to_string()),
            None,
            None,
        ),
    };
    let overall_success = result.is_success();
    let warning_only_success = overall_success
        && result
            .issues
            .iter()
            .any(|issue| issue.severity == Severity::Warning);
    let filters = IssueFilters::from_args(args);
    let (result, filtered_view) = filter_check_result(result, &filters);

    match args.format {
        OutputFormat::Text => {
            if let Some(run) = autofix_run.as_ref() {
                if args.dry_run {
                    if let Some(plan) = run.plan.as_ref()
                        && !plan.updated_files.is_empty()
                    {
                        println!(
                            "planned {} autofix updates across {} files (dry run; no files changed)",
                            plan.planned_updates,
                            plan.updated_files.len()
                        );
                        print!("{}", render_autofix_plan(plan));
                    }
                } else if !run.summary.updated_files.is_empty() {
                    println!(
                        "applied {} autofix updates across {} files",
                        run.summary.symbol_updates,
                        run.summary.updated_files.len()
                    );
                }
            }
            print!(
                "{}",
                render_text_report(
                    overall_success,
                    &result,
                    args.workspace.as_path(),
                    filtered_view.as_ref(),
                    text_summary.as_ref(),
                    args.spec_only,
                    args.quiet,
                )
            );
        }
        OutputFormat::Json => {
            let output = JsonCheckOutput {
                result: &result,
                filtered_view: filtered_view.as_ref(),
                autofix_plan: autofix_run.as_ref().and_then(|run| run.plan.as_ref()),
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&output)
                    .expect("serializing validate output to JSON should succeed")
            );
        }
    }

    Ok(
        match (
            overall_success,
            warning_only_success,
            args.warning_exit_code,
        ) {
            (false, _, _) => 1,
            (true, true, Some(code)) => i32::from(code.get()),
            (true, _, _) => 0,
        },
    )
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

pub(crate) fn collect_check_result_from_workspace(workspace: &Workspace) -> CheckResult {
    collect_check_result_from_workspace_with_mode(workspace, false)
}

fn collect_check_result_from_workspace_with_mode(
    workspace: &Workspace,
    spec_only: bool,
) -> CheckResult {
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
    validate_feature_registry_entries(workspace, &mut issues);

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
    let validation_resources = ValidationResources {
        config: &workspace.config,
        root: &workspace.root,
        spec_only,
    };
    let mut historical_id_index_error = None;
    let historical_ids = if workspace.config.validate.historical_ids.enabled {
        match build_historical_id_index(&workspace.root, &workspace.config) {
            Ok(index) => Some(index),
            Err(error) => {
                historical_id_index_error = Some(error.to_string());
                None
            }
        }
    } else {
        None
    };
    let requirement_validation_index = RequirementValidationIndex {
        policies_by_id: &policies_by_id,
        features_by_id: &features_by_id,
    };

    for philosophy in &workspace.philosophies {
        validate_philosophy(philosophy, &policies_by_id, &workspace.config, &mut issues);
    }

    for policy in &workspace.policies {
        validate_policy(
            policy,
            &philosophies_by_id,
            &requirements_by_id,
            &workspace.config,
            &mut issues,
        );
    }

    for requirement in &workspace.requirements {
        validate_requirement(
            requirement,
            requirement_validation_index,
            validation_resources,
            &mut issues,
            &mut trace_summary.requirement_traces,
        );
    }

    for feature in &workspace.features {
        validate_feature(
            feature,
            &requirements_by_id,
            validation_resources,
            &mut issues,
            &mut trace_summary.feature_traces,
        );
    }

    if let Some(historical_ids) = historical_ids.as_ref() {
        validate_historical_id_reuse(workspace, historical_ids, &mut issues);
    }
    if let Some(error) = historical_id_index_error {
        issues.push(Issue::error(
            "SYU-workspace-load-001",
            "workspace",
            Some(workspace.root.display().to_string()),
            format!("Failed to build the historical ID index: {error}."),
            Some(
                "Fix the Git history or set `validate.historical_ids.enabled: false` if historical ID validation is intentionally unavailable."
                    .to_string(),
            ),
        ));
    }

    validate_orphaned_definitions(workspace, &mut issues);
    if !spec_only {
        validate_symbol_trace_coverage(workspace, &mut issues);
    }

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

fn with_validate_overrides(mut workspace: Workspace, args: &CheckArgs) -> Workspace {
    if let Some(value) = args.allow_planned {
        workspace.config.validate.allow_planned = value;
    }
    if let Some(value) = args.require_non_orphaned_items {
        workspace.config.validate.require_non_orphaned_items = value;
    }
    if let Some(value) = args.require_reciprocal_links {
        workspace.config.validate.require_reciprocal_links = value;
    }
    if let Some(value) = args.require_symbol_trace_coverage {
        workspace.config.validate.require_symbol_trace_coverage = value;
    }
    workspace
}

fn apply_cli_override_issue_guidance(result: &mut CheckResult, args: &CheckArgs) {
    if args.allow_planned != Some(false) {
        return;
    }

    for issue in result
        .issues
        .iter_mut()
        .filter(|issue| issue.code == "SYU-delivery-planned-001")
    {
        let (kind, id) = issue
            .subject
            .split_once(' ')
            .expect("planned-item diagnostics should include kind and id");

        issue.message = format!(
            "{kind} `{id}` is marked `planned`, but `--allow-planned=false` forbids planned items for this run."
        );
        issue.suggestion = Some(format!(
            "Change `{id}` to `implemented` or rerun without `--allow-planned=false`."
        ));
    }
}

#[cfg(test)]
#[allow(clippy::question_mark)]
fn apply_autofix(workspace: &Workspace) -> Result<AutofixSummary> {
    Ok(apply_autofix_run(workspace)?.summary)
}

fn apply_autofix_run(workspace: &Workspace) -> Result<AutofixRun> {
    run_autofix(workspace, AutofixMode::Apply)
}

fn plan_autofix(workspace: &Workspace) -> Result<AutofixRun> {
    run_autofix(workspace, AutofixMode::Plan)
}

fn run_autofix(workspace: &Workspace, mode: AutofixMode) -> Result<AutofixRun> {
    let mut run = AutofixRun::default();
    if mode == AutofixMode::Plan {
        run.plan = Some(AutofixPlan::default());
    }
    let mut transaction = AutofixTransaction::default();
    let result = (|| -> Result<()> {
        for requirement in &workspace.requirements {
            if normalize_delivery_status(&requirement.status) != Some(DeliveryStatus::Implemented) {
                continue;
            }
            apply_autofix_for_trace_map_with_transaction(
                &workspace.root,
                &workspace.config,
                &requirement.id,
                &requirement.tests,
                &mut run,
                mode,
                &mut transaction,
            )?;
        }

        for feature in &workspace.features {
            if normalize_delivery_status(&feature.status) != Some(DeliveryStatus::Implemented) {
                continue;
            }
            apply_autofix_for_trace_map_with_transaction(
                &workspace.root,
                &workspace.config,
                &feature.id,
                &feature.implementations,
                &mut run,
                mode,
                &mut transaction,
            )?;
        }

        apply_graph_autofix(workspace, &mut transaction, &mut run, mode)?;

        Ok(())
    })();

    match result {
        Ok(()) => Ok(run),
        Err(error) if mode == AutofixMode::Apply => {
            Err(finalize_autofix_error(&transaction, error))
        }
        Err(error) => Err(error),
    }
}

#[derive(Debug, Clone, Copy)]
struct ItemLocation {
    doc_index: usize,
    item_index: usize,
}

fn apply_graph_autofix(
    workspace: &Workspace,
    transaction: &mut AutofixTransaction,
    run: &mut AutofixRun,
    mode: AutofixMode,
) -> Result<()> {
    let philosophy_root = workspace.spec_root.join("philosophy");
    let policy_root = workspace.spec_root.join("policies");
    let requirement_root = workspace.spec_root.join("requirements");
    let feature_root = workspace.spec_root.join("features");
    let root = &workspace.root;
    let description = "synchronize graph and trace hygiene";

    let mut philosophy_docs = if philosophy_root.is_dir() {
        load_philosophy_documents_with_paths(&philosophy_root)?
            .into_iter()
            .map(|loaded| MutableLoadedDocument {
                path: loaded.path,
                document: loaded.document,
                changed: false,
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut policy_docs = if policy_root.is_dir() {
        load_policy_documents_with_paths(&policy_root)?
            .into_iter()
            .map(|loaded| MutableLoadedDocument {
                path: loaded.path,
                document: loaded.document,
                changed: false,
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut requirement_docs = if requirement_root.is_dir() {
        load_requirement_documents_with_paths(&requirement_root)?
            .into_iter()
            .map(|loaded| MutableLoadedDocument {
                path: loaded.path,
                document: loaded.document,
                changed: false,
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut feature_docs = if feature_root.is_dir() {
        load_feature_documents_for_autofix(&feature_root)?
    } else {
        Vec::new()
    };

    dedupe_philosophy_links(&mut philosophy_docs);
    dedupe_policy_links(&mut policy_docs);
    dedupe_requirement_links(&mut requirement_docs);
    dedupe_feature_links(&mut feature_docs);
    normalize_trace_hygiene_for_requirement_documents(&mut requirement_docs);
    normalize_trace_hygiene_for_feature_documents(&mut feature_docs);

    let philosophy_index = index_philosophies(&philosophy_docs);
    let policy_index = index_policies(&policy_docs);
    let requirement_index = index_requirements(&requirement_docs);
    let feature_index = index_features(&feature_docs);

    apply_philosophy_reciprocals(&philosophy_docs, &mut policy_docs, &policy_index);
    apply_policy_reciprocals(
        &policy_docs,
        &mut philosophy_docs,
        &philosophy_index,
        &mut requirement_docs,
        &requirement_index,
    );
    apply_requirement_reciprocals(&requirement_docs, &mut feature_docs, &feature_index);
    apply_feature_reciprocals(&feature_docs, &mut requirement_docs, &requirement_index);

    if feature_root.is_dir()
        && let Some(updated_registry) = sync_feature_registry(&feature_root, &feature_docs)?
    {
        let registry_path = feature_root.join("features.yaml");
        let change = AutofixChangeRequest {
            root: &workspace.root,
            path: &registry_path,
            transaction,
            run,
            mode,
            rules: Vec::new(),
            summary_text: format!(
                "sync feature registry entries in `{}`",
                registry_path.display()
            ),
            contents: Some(serde_yaml::to_string(&updated_registry)?),
        };
        write_or_plan_autofix_change(change)?;
    }

    write_modified_documents(&philosophy_docs, root, transaction, run, mode, description)?;
    write_modified_documents(&policy_docs, root, transaction, run, mode, description)?;
    write_modified_documents(&requirement_docs, root, transaction, run, mode, description)?;
    write_modified_documents(&feature_docs, root, transaction, run, mode, description)?;

    Ok(())
}

fn finalize_autofix_error(transaction: &AutofixTransaction, error: anyhow::Error) -> anyhow::Error {
    if transaction.committed_writes() > 0 {
        match transaction.rollback() {
            Ok(()) => anyhow::anyhow!("autofix rolled back partial changes: {error}"),
            Err(rollback_error) => anyhow::anyhow!(
                "failed to roll back autofix changes after {error}: {rollback_error}"
            ),
        }
    } else {
        anyhow::anyhow!("autofix failed before applying changes: {error}")
    }
}

fn apply_autofix_for_trace_map_with_transaction(
    root: &Path,
    config: &SyuConfig,
    owner_id: &str,
    references_by_language: &BTreeMap<String, Vec<TraceReference>>,
    run: &mut AutofixRun,
    mode: AutofixMode,
    transaction: &mut AutofixTransaction,
) -> Result<()> {
    for (language, references) in references_by_language {
        let mut seen_references = BTreeSet::new();
        for reference in references {
            let normalized_reference = normalize_trace_reference(reference);
            if !seen_references.insert(trace_reference_dedup_key(&normalized_reference)) {
                continue;
            }
            apply_autofix_for_reference_with_transaction(
                root,
                config,
                owner_id,
                language,
                &normalized_reference,
                run,
                mode,
                transaction,
            )?;
        }
    }

    Ok(())
}

fn normalize_trace_reference(reference: &TraceReference) -> TraceReference {
    let mut normalized = reference.clone();
    if let Some(preferred_path) = preferred_trace_file_path(&normalized.file) {
        normalized.file = preferred_path;
    }
    normalized
}

fn trace_reference_dedup_key(reference: &TraceReference) -> String {
    let mut key = String::new();
    key.push_str(&reference.file.to_string_lossy());
    key.push('|');
    key.push_str(&reference.symbols.join(","));
    key.push('|');
    key.push_str(&reference.doc_contains.join(","));
    key.push('|');
    if let Some(method) = reference.method.as_deref() {
        key.push_str(method);
    }
    key.push('|');
    if let Some(path) = reference.path.as_deref() {
        key.push_str(path);
    }
    key
}
fn load_feature_documents_for_autofix(
    feature_root: &Path,
) -> Result<Vec<MutableLoadedDocument<FeatureDocument>>> {
    let registry_path = feature_root.join("features.yaml");
    if registry_path.exists() {
        let raw = fs::read_to_string(&registry_path)?;
        let _registry: FeatureRegistryDocument = serde_yaml::from_str(&raw)?;
    }
    let mut discovered_paths = Vec::new();
    collect_feature_yaml_paths(feature_root, &mut discovered_paths)?;
    discovered_paths.sort();

    let mut documents = Vec::new();
    for path in discovered_paths {
        if path == registry_path {
            continue;
        }
        if !looks_like_feature_document(&path)? {
            continue;
        }
        let raw = fs::read_to_string(&path)?;
        let document: FeatureDocument = serde_yaml::from_str(&raw)?;
        documents.push(MutableLoadedDocument {
            path,
            document,
            changed: false,
        });
    }

    Ok(documents)
}

fn dedupe_philosophy_links(documents: &mut [MutableLoadedDocument<PhilosophyDocument>]) {
    for document in documents {
        for philosophy in &mut document.document.philosophies {
            if dedupe_unique_values(&mut philosophy.linked_policies) {
                document.changed = true;
            }
        }
    }
}

fn dedupe_policy_links(documents: &mut [MutableLoadedDocument<PolicyDocument>]) {
    for document in documents {
        for policy in &mut document.document.policies {
            let changed_philosophies = dedupe_unique_values(&mut policy.linked_philosophies);
            let changed_requirements = dedupe_unique_values(&mut policy.linked_requirements);
            if changed_philosophies || changed_requirements {
                document.changed = true;
            }
        }
    }
}

fn dedupe_requirement_links(documents: &mut [MutableLoadedDocument<RequirementDocument>]) {
    for document in documents {
        for requirement in &mut document.document.requirements {
            let changed_policies = dedupe_unique_values(&mut requirement.linked_policies);
            let changed_features = dedupe_unique_values(&mut requirement.linked_features);
            if changed_policies || changed_features {
                document.changed = true;
            }
        }
    }
}

fn dedupe_feature_links(documents: &mut [MutableLoadedDocument<FeatureDocument>]) {
    for document in documents {
        for feature in &mut document.document.features {
            if dedupe_unique_values(&mut feature.linked_requirements) {
                document.changed = true;
            }
        }
    }
}

fn dedupe_unique_values(values: &mut Vec<String>) -> bool {
    let before = values.len();
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(value.clone()));
    values.len() != before
}

fn index_philosophies(
    documents: &[MutableLoadedDocument<PhilosophyDocument>],
) -> HashMap<String, ItemLocation> {
    let mut index = HashMap::new();
    for (doc_index, document) in documents.iter().enumerate() {
        for (item_index, philosophy) in document.document.philosophies.iter().enumerate() {
            index.insert(
                philosophy.id.clone(),
                ItemLocation {
                    doc_index,
                    item_index,
                },
            );
        }
    }
    index
}

fn index_policies(
    documents: &[MutableLoadedDocument<PolicyDocument>],
) -> HashMap<String, ItemLocation> {
    let mut index = HashMap::new();
    for (doc_index, document) in documents.iter().enumerate() {
        for (item_index, policy) in document.document.policies.iter().enumerate() {
            index.insert(
                policy.id.clone(),
                ItemLocation {
                    doc_index,
                    item_index,
                },
            );
        }
    }
    index
}

fn index_requirements(
    documents: &[MutableLoadedDocument<RequirementDocument>],
) -> HashMap<String, ItemLocation> {
    let mut index = HashMap::new();
    for (doc_index, document) in documents.iter().enumerate() {
        for (item_index, requirement) in document.document.requirements.iter().enumerate() {
            index.insert(
                requirement.id.clone(),
                ItemLocation {
                    doc_index,
                    item_index,
                },
            );
        }
    }
    index
}

fn index_features(
    documents: &[MutableLoadedDocument<FeatureDocument>],
) -> HashMap<String, ItemLocation> {
    let mut index = HashMap::new();
    for (doc_index, document) in documents.iter().enumerate() {
        for (item_index, feature) in document.document.features.iter().enumerate() {
            index.insert(
                feature.id.clone(),
                ItemLocation {
                    doc_index,
                    item_index,
                },
            );
        }
    }
    index
}

fn apply_philosophy_reciprocals(
    philosophies: &[MutableLoadedDocument<PhilosophyDocument>],
    policies: &mut [MutableLoadedDocument<PolicyDocument>],
    policy_index: &HashMap<String, ItemLocation>,
) {
    for document in philosophies {
        for philosophy in &document.document.philosophies {
            for policy_id in &philosophy.linked_policies {
                let Some(location) = policy_index.get(policy_id) else {
                    continue;
                };
                if insert_unique_value(
                    &mut policies[location.doc_index].document.policies[location.item_index]
                        .linked_philosophies,
                    &philosophy.id,
                ) {
                    policies[location.doc_index].changed = true;
                }
            }
        }
    }
}

fn apply_policy_reciprocals(
    policies: &[MutableLoadedDocument<PolicyDocument>],
    philosophies: &mut [MutableLoadedDocument<PhilosophyDocument>],
    philosophy_index: &HashMap<String, ItemLocation>,
    requirements: &mut [MutableLoadedDocument<RequirementDocument>],
    requirement_index: &HashMap<String, ItemLocation>,
) {
    for document in policies {
        for policy in &document.document.policies {
            for philosophy_id in &policy.linked_philosophies {
                let Some(location) = philosophy_index.get(philosophy_id) else {
                    continue;
                };
                if insert_unique_value(
                    &mut philosophies[location.doc_index].document.philosophies
                        [location.item_index]
                        .linked_policies,
                    &policy.id,
                ) {
                    philosophies[location.doc_index].changed = true;
                }
            }

            for requirement_id in &policy.linked_requirements {
                let Some(location) = requirement_index.get(requirement_id) else {
                    continue;
                };
                if insert_unique_value(
                    &mut requirements[location.doc_index].document.requirements
                        [location.item_index]
                        .linked_policies,
                    &policy.id,
                ) {
                    requirements[location.doc_index].changed = true;
                }
            }
        }
    }
}

fn apply_requirement_reciprocals(
    requirements: &[MutableLoadedDocument<RequirementDocument>],
    features: &mut [MutableLoadedDocument<FeatureDocument>],
    feature_index: &HashMap<String, ItemLocation>,
) {
    for document in requirements {
        for requirement in &document.document.requirements {
            for feature_id in &requirement.linked_features {
                let Some(location) = feature_index.get(feature_id) else {
                    continue;
                };
                if insert_unique_value(
                    &mut features[location.doc_index].document.features[location.item_index]
                        .linked_requirements,
                    &requirement.id,
                ) {
                    features[location.doc_index].changed = true;
                }
            }
        }
    }
}

fn apply_feature_reciprocals(
    features: &[MutableLoadedDocument<FeatureDocument>],
    requirements: &mut [MutableLoadedDocument<RequirementDocument>],
    requirement_index: &HashMap<String, ItemLocation>,
) {
    for document in features {
        for feature in &document.document.features {
            for requirement_id in &feature.linked_requirements {
                let Some(location) = requirement_index.get(requirement_id) else {
                    continue;
                };
                if insert_unique_value(
                    &mut requirements[location.doc_index].document.requirements
                        [location.item_index]
                        .linked_features,
                    &feature.id,
                ) {
                    requirements[location.doc_index].changed = true;
                }
            }
        }
    }
}

fn insert_unique_value(values: &mut Vec<String>, value: &str) -> bool {
    if values.iter().any(|existing| existing == value) {
        return false;
    }
    values.push(value.to_string());
    true
}

fn feature_registry_kind(feature_root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(feature_root).unwrap_or(path);
    if let Some(parent) = relative.parent()
        && let Some(component) = parent.components().find_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
    {
        return component;
    }
    relative
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "feature".to_string())
}

fn feature_registry_entries_for_documents(
    feature_root: &Path,
    documents: &[MutableLoadedDocument<FeatureDocument>],
) -> Vec<FeatureRegistryEntry> {
    let mut entries = documents
        .iter()
        .map(|document| {
            let relative = normalize_relative_path(
                document
                    .path
                    .strip_prefix(feature_root)
                    .unwrap_or(&document.path),
            );
            FeatureRegistryEntry {
                kind: feature_registry_kind(feature_root, &document.path),
                file: relative,
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.kind.cmp(&right.kind).then(left.file.cmp(&right.file)));
    entries
}

fn feature_registry_entries_match(
    left: &[FeatureRegistryEntry],
    right: &[FeatureRegistryEntry],
) -> bool {
    let mut left = left
        .iter()
        .map(|entry| (entry.kind.clone(), entry.file.clone()))
        .collect::<Vec<_>>();
    let mut right = right
        .iter()
        .map(|entry| (entry.kind.clone(), entry.file.clone()))
        .collect::<Vec<_>>();
    left.sort();
    right.sort();
    left == right
}

fn sync_feature_registry(
    feature_root: &Path,
    documents: &[MutableLoadedDocument<FeatureDocument>],
) -> Result<Option<FeatureRegistryDocument>> {
    let registry_path = feature_root.join("features.yaml");
    let files = feature_registry_entries_for_documents(feature_root, documents);
    match fs::read_to_string(&registry_path) {
        Ok(raw) => {
            let registry: FeatureRegistryDocument = serde_yaml::from_str(&raw)?;
            if feature_registry_entries_match(&registry.files, &files) {
                return Ok(None);
            }

            Ok(Some(FeatureRegistryDocument {
                version: registry.version,
                updated: registry.updated,
                files,
            }))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(Some(FeatureRegistryDocument {
                version: env!("CARGO_PKG_VERSION").to_string(),
                updated: None,
                files,
            }))
        }
        Err(error) => Err(error.into()),
    }
}

fn write_modified_documents<T: serde::Serialize>(
    documents: &[MutableLoadedDocument<T>],
    root: &Path,
    transaction: &mut AutofixTransaction,
    run: &mut AutofixRun,
    mode: AutofixMode,
    description: &str,
) -> Result<()> {
    for document in documents {
        if !document.changed {
            continue;
        }
        let change = AutofixChangeRequest {
            root,
            path: &document.path,
            transaction,
            run,
            mode,
            rules: Vec::new(),
            summary_text: format!("{description} in `{}`", document.path.display()),
            contents: Some(serde_yaml::to_string(&document.document)?),
        };
        write_or_plan_autofix_change(change)?;
    }
    Ok(())
}

fn normalize_trace_hygiene_for_requirement_documents(
    documents: &mut [MutableLoadedDocument<RequirementDocument>],
) {
    for document in documents {
        for requirement in &mut document.document.requirements {
            if let Some(canonical_status) = normalize_delivery_status_label(&requirement.status)
                && requirement.status.trim() != canonical_status
            {
                requirement.status = canonical_status.to_string();
                document.changed = true;
            }

            if normalize_trace_references(&mut requirement.tests) {
                document.changed = true;
            }
        }
    }
}

fn normalize_trace_hygiene_for_feature_documents(
    documents: &mut [MutableLoadedDocument<FeatureDocument>],
) {
    for document in documents {
        for feature in &mut document.document.features {
            if let Some(canonical_status) = normalize_delivery_status_label(&feature.status)
                && feature.status.trim() != canonical_status
            {
                feature.status = canonical_status.to_string();
                document.changed = true;
            }

            if normalize_trace_references(&mut feature.implementations) {
                document.changed = true;
            }
        }
    }
}

fn normalize_trace_references(
    references_by_language: &mut BTreeMap<String, Vec<TraceReference>>,
) -> bool {
    let mut changed = false;
    for references in references_by_language.values_mut() {
        for reference in references.iter_mut() {
            if let Some(preferred_path) = preferred_trace_file_path(&reference.file)
                && preferred_path != reference.file
            {
                reference.file = preferred_path;
                changed = true;
            }
        }

        let mut seen = HashSet::new();
        references.retain(|reference| {
            let key = (
                reference.file.clone(),
                reference.symbols.clone(),
                reference.doc_contains.clone(),
                reference.method.clone(),
                reference.path.clone(),
            );

            if seen.insert(key) {
                true
            } else {
                changed = true;
                false
            }
        });
    }
    changed
}

struct AutofixChangeRequest<'a> {
    root: &'a Path,
    path: &'a Path,
    transaction: &'a mut AutofixTransaction,
    run: &'a mut AutofixRun,
    mode: AutofixMode,
    rules: Vec<&'static str>,
    summary_text: String,
    contents: Option<String>,
}

fn write_or_plan_autofix_change(request: AutofixChangeRequest<'_>) -> Result<()> {
    let AutofixChangeRequest {
        root,
        path,
        transaction,
        run,
        mode,
        rules,
        summary_text,
        contents,
    } = request;
    let summary = &mut run.summary;
    if mode == AutofixMode::Apply {
        let contents = contents.expect("apply mode requires contents");
        transaction.write(path, &contents)?;
    } else {
        let plan = run
            .plan
            .as_mut()
            .expect("plan mode should initialize a plan");
        record_planned_change(plan, root, path, rules, summary_text);
    }
    record_updated_file(summary, root, path);
    summary.symbol_updates += 1;
    Ok(())
}

#[allow(dead_code)]
fn apply_autofix_for_reference(
    root: &Path,
    config: &SyuConfig,
    owner_id: &str,
    language: &str,
    reference: &TraceReference,
    run: &mut AutofixRun,
    mode: AutofixMode,
) -> Result<()> {
    let mut transaction = AutofixTransaction::default();
    apply_autofix_for_reference_with_transaction(
        root,
        config,
        owner_id,
        language,
        reference,
        run,
        mode,
        &mut transaction,
    )
}

#[allow(clippy::too_many_arguments)]
fn apply_autofix_for_reference_with_transaction(
    root: &Path,
    config: &SyuConfig,
    owner_id: &str,
    language: &str,
    reference: &TraceReference,
    run: &mut AutofixRun,
    mode: AutofixMode,
    transaction: &mut AutofixTransaction,
) -> Result<()> {
    let Some(adapter) = adapter_for_language(language) else {
        return Ok(());
    };

    if reference.file.as_os_str().is_empty() || reference.symbols.is_empty() {
        return Ok(());
    }

    let file_path =
        preferred_trace_file_path(&reference.file).unwrap_or_else(|| reference.file.clone());
    let path = root.join(&file_path);
    if !path.is_file() || !adapter.supports_path(&path) {
        return Ok(());
    }

    let mut contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(_) => return Ok(()),
    };
    let mut updated_symbols = 0;
    let mut changed = false;

    if config.validate.trace_ownership_mode == TraceOwnershipMode::Sidecar {
        ensure_sidecar_ownership_manifest(
            root,
            owner_id,
            &path,
            reference,
            run,
            mode,
            transaction,
        )?;
    }

    for symbol in reference
        .symbols
        .iter()
        .map(|symbol| symbol.trim())
        .filter(|symbol| !symbol.is_empty() && *symbol != "*")
    {
        let mut required = reference.doc_contains.clone();
        let needs_inline_owner = config.validate.trace_ownership_mode == TraceOwnershipMode::Inline
            && !contents.contains(owner_id);
        if config.validate.trace_ownership_mode == TraceOwnershipMode::Inline && needs_inline_owner
        {
            required.push(owner_id.to_string());
        }
        if required.is_empty() {
            continue;
        }

        let updated =
            match apply_symbol_doc_fix(language, config, &path, &contents, symbol, &required)? {
                Some(updated) => updated,
                None => continue,
            };

        contents = updated;
        if mode == AutofixMode::Apply {
            transaction.write(&path, &contents)?;
        } else {
            let plan = run
                .plan
                .as_mut()
                .expect("plan mode should initialize a plan");
            let mut rules = Vec::new();
            if !reference.doc_contains.is_empty() {
                rules.push("SYU-trace-doc-001");
            }
            if needs_inline_owner {
                rules.push("SYU-trace-id-001");
            }
            record_planned_change(
                plan,
                root,
                &path,
                rules,
                format!("update `{symbol}` in `{}` for `{owner_id}`", path.display()),
            );
        }
        changed = true;
        updated_symbols += 1;
    }

    if changed {
        record_updated_file(&mut run.summary, root, &path);
        run.summary.symbol_updates += updated_symbols;
    }

    Ok(())
}

fn record_updated_file(summary: &mut AutofixSummary, root: &Path, path: &Path) {
    summary.updated_files.insert(
        path.strip_prefix(root)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| path.to_path_buf()),
    );
}

fn record_planned_change(
    plan: &mut AutofixPlan,
    root: &Path,
    path: &Path,
    rules: Vec<&'static str>,
    summary: String,
) {
    plan.planned_updates += 1;
    plan.updated_files.insert(
        path.strip_prefix(root)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| path.to_path_buf()),
    );
    plan.changes.push(AutofixPlanChange {
        path: path
            .strip_prefix(root)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| path.to_path_buf()),
        rules,
        summary,
    });
}

fn render_autofix_plan(plan: &AutofixPlan) -> String {
    let mut output = String::new();
    if plan.changes.is_empty() {
        return output;
    }

    writeln!(&mut output, "planned fixes:").expect("writing to String must succeed");
    for change in &plan.changes {
        let rules = if change.rules.is_empty() {
            String::from("no direct rule match")
        } else {
            change.rules.join(", ")
        };
        writeln!(
            &mut output,
            "- {} [{}]: {}",
            change.path.display(),
            rules,
            change.summary
        )
        .expect("writing to String must succeed");
    }

    output
}

fn render_text_report(
    overall_success: bool,
    result: &CheckResult,
    workspace_arg: &Path,
    filtered_view: Option<&FilteredIssueView>,
    text_summary: Option<&TextReportSummary>,
    spec_only: bool,
    quiet: bool,
) -> String {
    let mut output = String::new();
    let status = if overall_success { "passed" } else { "failed" };
    let quiet_success = quiet && overall_success && result.issues.is_empty();
    let filtered_suffix = if filtered_view.is_some() {
        " (filtered view)"
    } else {
        ""
    };

    writeln!(&mut output, "syu validate {status}{filtered_suffix}")
        .expect("writing to String must succeed");
    if !quiet_success {
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
        if let Some(summary) = text_summary {
            writeln!(
                &mut output,
                "checks: {} built-in rules across {} workspace items ({})",
                summary.checked_rule_count,
                summary.workspace_item_count,
                summary.checked_genres.join(", ")
            )
            .expect("writing to String must succeed");
            if !summary.disabled_checks.is_empty() {
                let disabled_rule_count: usize = summary
                    .disabled_checks
                    .iter()
                    .map(|notice| notice.rule_count)
                    .sum();
                let noun = if disabled_rule_count == 1 {
                    "rule is"
                } else {
                    "rules are"
                };
                let details = summary
                    .disabled_checks
                    .iter()
                    .map(DisabledRuleNotice::describe)
                    .collect::<Vec<_>>()
                    .join(", ");
                writeln!(
                    &mut output,
                    "note: {disabled_rule_count} built-in {noun} disabled by current config ({details})"
                )
                .expect("writing to String must succeed");
            }
        }
        if spec_only {
            writeln!(
                &mut output,
                "traceability: skipped (--spec-only disables requirement and feature trace enforcement)"
            )
            .expect("writing to String must succeed");
        } else {
            writeln!(
                &mut output,
                "traceability: requirements={}/{} traces validated; features={}/{} traces validated",
                result.trace_summary.requirement_traces.validated,
                result.trace_summary.requirement_traces.declared,
                result.trace_summary.feature_traces.validated,
                result.trace_summary.feature_traces.declared
            )
            .expect("writing to String must succeed");
        }
    }

    if !quiet_success && let Some(filtered_view) = filtered_view {
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
            for line in format_text_issue(issue, TextIssueFormat::Validate) {
                writeln!(&mut output, "{line}").expect("writing to String must succeed");
            }
        }
    } else if !quiet_success
        && let Some(filtered_view) = filtered_view
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

    let workspace_arg = shell_quote_path(workspace_arg);

    if overall_success && result.issues.is_empty() && !quiet {
        writeln!(&mut output).expect("writing to String must succeed");
        writeln!(&mut output, "What to do next:").expect("writing to String must succeed");
        writeln!(
            &mut output,
            "  syu app {workspace_arg}        open the browser UI to explore your workspace"
        )
        .expect("writing to String must succeed");
        writeln!(
            &mut output,
            "  syu browse {workspace_arg}     browse interactively in the terminal"
        )
        .expect("writing to String must succeed");
        writeln!(
            &mut output,
            "  syu report {workspace_arg}     generate a markdown validation report"
        )
        .expect("writing to String must succeed");
        writeln!(
            &mut output,
            "  syu show <ID> {workspace_arg}    inspect a single spec item in detail"
        )
        .expect("writing to String must succeed");
    } else if !overall_success && !quiet {
        writeln!(&mut output).expect("writing to String must succeed");
        writeln!(&mut output, "What to inspect next:").expect("writing to String must succeed");
        writeln!(
            &mut output,
            "  syu show <ID> {workspace_arg}           inspect a specific requirement, feature, policy, or philosophy by ID"
        )
        .expect("writing to String must succeed");
        writeln!(
            &mut output,
            "  syu validate {workspace_arg} --severity error   rerun with only error-level issues if you need the shortest list first"
        )
        .expect("writing to String must succeed");
        writeln!(
            &mut output,
            "  syu validate {workspace_arg} --genre graph      focus on missing links, reciprocal links, or missing definitions"
        )
        .expect("writing to String must succeed");
        writeln!(
            &mut output,
            "  syu app {workspace_arg}                 open the browser UI to inspect the same workspace graph visually"
        )
        .expect("writing to String must succeed");
    }

    output
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_delivery_status(status: &str) -> Option<DeliveryStatus> {
    normalize_delivery_status_label(status).map(|label| match label {
        "planned" => DeliveryStatus::Planned,
        "implemented" => DeliveryStatus::Implemented,
        _ => unreachable!("canonical delivery status labels are exhaustive"),
    })
}

fn normalize_delivery_status_label(status: &str) -> Option<&'static str> {
    match status.trim() {
        "planned" | "planed" => Some("planned"),
        "implemented" => Some("implemented"),
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

fn validate_historical_id_reuse(
    workspace: &Workspace,
    historical_ids: &HistoricalIdIndex,
    issues: &mut Vec<Issue>,
) {
    if !historical_ids.enabled() || !historical_ids.available() {
        return;
    }

    validate_historical_id_reuse_for_ids(
        "philosophy",
        workspace.philosophies.iter().map(|item| item.id.as_str()),
        historical_ids,
        issues,
    );
    validate_historical_id_reuse_for_ids(
        "policy",
        workspace.policies.iter().map(|item| item.id.as_str()),
        historical_ids,
        issues,
    );
    validate_historical_id_reuse_for_ids(
        "requirement",
        workspace.requirements.iter().map(|item| item.id.as_str()),
        historical_ids,
        issues,
    );
    validate_historical_id_reuse_for_ids(
        "feature",
        workspace.features.iter().map(|item| item.id.as_str()),
        historical_ids,
        issues,
    );
}

fn validate_historical_id_reuse_for_ids<'a>(
    kind: &str,
    ids: impl Iterator<Item = &'a str>,
    historical_ids: &HistoricalIdIndex,
    issues: &mut Vec<Issue>,
) {
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            continue;
        }

        let Some(evidence) = historical_ids.deleted_entry(id) else {
            continue;
        };

        let location = Some(evidence.path.display().to_string());
        issues.push(Issue::error(
            "SYU-workspace-historical-001",
            format!("{kind} {id}"),
            location,
            format!(
                "{kind} `{id}` reuses an ID that was last known in `{}` at commit `{}` before it was deleted.",
                evidence.path.display(),
                evidence.commit,
            ),
            Some(
                "Choose a new ID for the replacement item, or temporarily set `validate.historical_ids.enabled: false` while intentionally migrating old data.".to_string(),
            ),
        ));
    }
}

fn validate_feature_registry_entries(workspace: &Workspace, issues: &mut Vec<Issue>) {
    let feature_root = workspace.spec_root.join("features");
    let registry_path = feature_root.join("features.yaml");
    let registry_display = workspace_display_path(workspace, &registry_path);

    match validate_feature_registry_entries_inner(workspace, &feature_root, &registry_path, issues) {
        Ok(()) => {}
        Err(error) => issues.push(Issue::error(
            "SYU-workspace-registry-001",
            "workspace",
            Some(registry_display.clone()),
            format!(
                "Failed to compare feature files against `{registry_display}`: {error}."
            ),
            Some(format!(
                "Fix `{registry_display}` and the feature document tree so `syu` can verify the explicit feature registry."
            )),
        )),
    }
}

fn validate_feature_registry_entries_inner(
    workspace: &Workspace,
    feature_root: &Path,
    registry_path: &Path,
    issues: &mut Vec<Issue>,
) -> Result<()> {
    let raw = fs::read_to_string(registry_path)
        .map_err(anyhow::Error::from)
        .map_err(|error| {
            anyhow::anyhow!(
                "failed to read feature registry `{}`: {error}",
                registry_path.display()
            )
        })?;
    let registry: FeatureRegistryDocument = serde_yaml::from_str(&raw).map_err(|error| {
        anyhow::anyhow!(
            "failed to parse feature registry `{}`: {error}",
            registry_path.display()
        )
    })?;
    let registered_paths: HashSet<_> = registry
        .files
        .into_iter()
        .map(|entry| feature_root.join(entry.file))
        .collect();

    let mut discovered_paths = Vec::new();
    collect_feature_yaml_paths(feature_root, &mut discovered_paths)?;
    discovered_paths.sort();

    for path in discovered_paths {
        if path == registry_path || registered_paths.contains(&path) {
            continue;
        }

        let looks_like_feature = match looks_like_feature_document(&path) {
            Ok(result) => result,
            Err(error) => {
                let candidate_display = workspace_display_path(workspace, &path);
                let registry_display = workspace_display_path(workspace, registry_path);
                issues.push(Issue::error(
                    "SYU-workspace-registry-001",
                    "workspace",
                    Some(candidate_display.clone()),
                    format!(
                        "Failed to inspect feature document candidate `{candidate_display}` while comparing it against `{registry_display}`: {error}."
                    ),
                    Some(format!(
                        "Fix `{candidate_display}` or remove it from the feature tree so `syu` can verify `{registry_display}`."
                    )),
                ));
                continue;
            }
        };
        if !looks_like_feature {
            continue;
        }

        let missing_display = workspace_display_path(workspace, &path);
        let entry_path = path
            .strip_prefix(feature_root)
            .unwrap_or(path.as_path())
            .display()
            .to_string();
        let registry_display = workspace_display_path(workspace, registry_path);
        issues.push(Issue::error(
            "SYU-workspace-registry-001",
            "workspace",
            Some(registry_display.clone()),
            format!(
                "Feature document `{missing_display}` exists on disk but is not listed in `{registry_display}`."
            ),
            Some(format!(
                "Add a `files` entry for `{entry_path}` to `{registry_display}` so `syu list`, `syu browse`, and validation include that feature document."
            )),
        ));
    }

    Ok(())
}

fn collect_feature_yaml_paths(directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(directory)
        .map_err(anyhow::Error::from)
        .map_err(|error| {
            anyhow::anyhow!(
                "failed to read directory `{}`: {error}",
                directory.display()
            )
        })?;

    for entry in entries {
        let path = entry
            .map_err(anyhow::Error::from)
            .map_err(|error| anyhow::anyhow!("failed to read directory entry: {error}"))?
            .path();
        if path.is_dir() {
            collect_feature_yaml_paths(&path, files)?;
        } else if matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("yaml" | "yml")
        ) {
            files.push(path);
        }
    }

    Ok(())
}

fn looks_like_feature_document(path: &Path) -> Result<bool> {
    let raw = fs::read_to_string(path)
        .map_err(anyhow::Error::from)
        .map_err(|error| {
            anyhow::anyhow!(
                "failed to read feature candidate `{}`: {error}",
                path.display()
            )
        })?;
    let value: serde_yaml::Value = serde_yaml::from_str(&raw).map_err(|error| {
        anyhow::anyhow!(
            "failed to parse feature candidate `{}` while checking the registry: {error}",
            path.display()
        )
    })?;

    Ok(value.as_mapping().is_some_and(|mapping| {
        mapping.contains_key(serde_yaml::Value::String("features".to_string()))
    }))
}

fn workspace_display_path(workspace: &Workspace, path: &Path) -> String {
    path.strip_prefix(&workspace.root)
        .unwrap_or(path)
        .display()
        .to_string()
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
            reference.method.clone(),
            reference.path.clone(),
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

fn preferred_trace_file_path(file: &Path) -> Option<PathBuf> {
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
    let mut parts = vec![
        format!("file=`{}`", reference.file.display()),
        format!("symbols=[{symbols}]"),
        format!("doc_contains=[{doc_contains}]"),
    ];

    if let Some(method) = reference
        .method
        .as_deref()
        .filter(|method| !method.trim().is_empty())
    {
        parts.push(format!("method=`{method}`"));
    }
    if let Some(path) = reference
        .path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
    {
        parts.push(format!("path=`{path}`"));
    }

    parts.join(" ")
}

fn evaluate_trace_ownership(
    root: &Path,
    config: &SyuConfig,
    owner_id: &str,
    traced_file: &Path,
    reference: &TraceReference,
    contents: &str,
) -> OwnershipDeclaration {
    match config.validate.trace_ownership_mode {
        TraceOwnershipMode::Mapping => OwnershipDeclaration::Satisfied,
        TraceOwnershipMode::Inline => {
            if contents.contains(owner_id) {
                OwnershipDeclaration::Satisfied
            } else {
                OwnershipDeclaration::Missing {
                    manifest_path: None,
                }
            }
        }
        TraceOwnershipMode::Sidecar => {
            let manifest_path = ownership_manifest_path(traced_file);
            if !manifest_path.is_file() {
                return OwnershipDeclaration::Missing {
                    manifest_path: Some(manifest_path),
                };
            }

            match load_ownership_manifest(&manifest_path) {
                Ok(manifest) => {
                    if manifest_declares_owner(&manifest, owner_id, reference) {
                        OwnershipDeclaration::Satisfied
                    } else {
                        OwnershipDeclaration::Missing {
                            manifest_path: Some(manifest_path),
                        }
                    }
                }
                Err(reason) => OwnershipDeclaration::Invalid {
                    manifest_path: manifest_path
                        .strip_prefix(root)
                        .map(Path::to_path_buf)
                        .unwrap_or(manifest_path),
                    reason,
                },
            }
        }
    }
}

fn ownership_manifest_path(traced_file: &Path) -> PathBuf {
    let file_name = traced_file
        .file_name()
        .map(|file_name| file_name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "trace".to_string());
    traced_file.with_file_name(format!("{file_name}.syu-ownership.yaml"))
}

fn load_ownership_manifest(path: &Path) -> std::result::Result<OwnershipManifest, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read ownership manifest: {error}"))?;
    let manifest: OwnershipManifest = serde_yaml::from_str(&raw)
        .map_err(|error| format!("failed to parse ownership manifest YAML: {error}"))?;
    if manifest.version != 1 {
        return Err(format!(
            "unsupported ownership manifest version `{}` (expected `1`)",
            manifest.version
        ));
    }
    validate_ownership_manifest(&manifest)?;
    Ok(manifest)
}

fn validate_ownership_manifest(manifest: &OwnershipManifest) -> std::result::Result<(), String> {
    let mut owner_ids = BTreeSet::new();
    for entry in &manifest.owners {
        if !owner_ids.insert(entry.id.clone()) {
            return Err(format!(
                "duplicate ownership entry `{}` in `owners`",
                entry.id
            ));
        }
    }
    Ok(())
}

fn manifest_declares_owner(
    manifest: &OwnershipManifest,
    owner_id: &str,
    reference: &TraceReference,
) -> bool {
    let required_symbols = required_ownership_symbols(reference);
    manifest
        .owners
        .iter()
        .filter(|entry| entry.id == owner_id)
        .any(|entry| entry_covers_symbols(entry, &required_symbols))
}

fn entry_covers_symbols(entry: &OwnershipEntry, required_symbols: &[String]) -> bool {
    let declared_symbols = entry
        .symbols
        .iter()
        .map(|symbol| symbol.trim())
        .filter(|symbol| !symbol.is_empty())
        .collect::<BTreeSet<_>>();
    if declared_symbols.contains("*") {
        return true;
    }
    required_symbols
        .iter()
        .all(|symbol| declared_symbols.contains(symbol.as_str()))
}

fn required_ownership_symbols(reference: &TraceReference) -> Vec<String> {
    let symbols = reference
        .symbols
        .iter()
        .map(|symbol| symbol.trim())
        .filter(|symbol| !symbol.is_empty())
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>();
    if symbols.contains("*") {
        return vec!["*".to_string()];
    }
    symbols.into_iter().collect()
}

fn ownership_symbols_hint(reference: &TraceReference) -> String {
    let symbols = required_ownership_symbols(reference);
    if symbols.is_empty() {
        return "the traced symbols".to_string();
    }
    format!(
        "symbols [{}]",
        symbols
            .iter()
            .map(|symbol| format!("`{symbol}`"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn ensure_sidecar_ownership_manifest(
    root: &Path,
    owner_id: &str,
    traced_file: &Path,
    reference: &TraceReference,
    run: &mut AutofixRun,
    mode: AutofixMode,
    transaction: &mut AutofixTransaction,
) -> Result<()> {
    let manifest_path = ownership_manifest_path(traced_file);
    let mut manifest = if manifest_path.is_file() {
        load_ownership_manifest(&manifest_path).map_err(|error| {
            anyhow::anyhow!("failed to load `{}`: {error}", manifest_path.display())
        })?
    } else {
        OwnershipManifest {
            version: 1,
            owners: Vec::new(),
        }
    };

    if !merge_ownership_entry(&mut manifest, owner_id, reference) {
        return Ok(());
    }

    let raw = serde_yaml::to_string(&manifest)?;
    if mode == AutofixMode::Apply {
        transaction.write(&manifest_path, &raw)?;
    } else if let Some(plan) = run.plan.as_mut() {
        record_planned_change(
            plan,
            root,
            &manifest_path,
            vec!["SYU-trace-id-001"],
            format!(
                "update sidecar ownership for `{owner_id}` in `{}`",
                manifest_path.display()
            ),
        );
    }
    record_updated_file(&mut run.summary, root, &manifest_path);
    run.summary.symbol_updates += 1;
    Ok(())
}

fn merge_ownership_entry(
    manifest: &mut OwnershipManifest,
    owner_id: &str,
    reference: &TraceReference,
) -> bool {
    let required_symbols = required_ownership_symbols(reference);
    if required_symbols.is_empty() {
        return false;
    }

    let mut changed = false;

    if let Some(entry) = manifest
        .owners
        .iter_mut()
        .find(|entry| entry.id == owner_id)
    {
        let normalized_symbols = normalize_ownership_symbols(&entry.symbols, &required_symbols);
        if entry.symbols != normalized_symbols {
            entry.symbols = normalized_symbols;
            changed = true;
        }
    } else {
        manifest.owners.push(OwnershipEntry {
            id: owner_id.to_string(),
            symbols: required_symbols,
        });
        changed = true;
    }

    if changed {
        manifest
            .owners
            .sort_by(|left, right| left.id.cmp(&right.id));
    }
    changed
}

fn normalize_ownership_symbols(existing: &[String], required: &[String]) -> Vec<String> {
    let wildcard_required = required.len() == 1 && required[0] == "*";
    if wildcard_required || existing.iter().any(|symbol| symbol.trim() == "*") {
        return vec!["*".to_string()];
    }

    existing
        .iter()
        .chain(required.iter())
        .map(|symbol| symbol.trim())
        .filter(|symbol| !symbol.is_empty() && *symbol != "*")
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn validate_philosophy(
    philosophy: &Philosophy,
    policies_by_id: &HashMap<&str, &Policy>,
    config: &SyuConfig,
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
                if config.validate.require_reciprocal_links
                    && !policy
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
    config: &SyuConfig,
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
                if config.validate.require_reciprocal_links
                    && !philosophy
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
                if config.validate.require_reciprocal_links
                    && !requirement
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
    index: RequirementValidationIndex<'_>,
    resources: ValidationResources<'_>,
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
        resources.config,
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
        match index.policies_by_id.get(policy_id.as_str()) {
            Some(policy) => {
                if resources.config.validate.require_reciprocal_links
                    && !policy
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
        match index.features_by_id.get(feature_id.as_str()) {
            Some(feature) => {
                if resources.config.validate.require_reciprocal_links
                    && !feature
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
                index
                    .features_by_id
                    .get(feature_id.as_str())
                    .map(|feature| {
                        (
                            feature_id.as_str(),
                            normalize_delivery_status(&feature.status),
                        )
                    })
            })
            .collect(),
        issues,
    );

    if !resources.spec_only {
        validate_trace_map(
            resources.root,
            resources.config,
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
}

fn validate_feature(
    feature: &Feature,
    requirements_by_id: &HashMap<&str, &Requirement>,
    resources: ValidationResources<'_>,
    issues: &mut Vec<Issue>,
    trace_count: &mut TraceCount,
) {
    validate_non_empty_field("feature", "id", &feature.id, issues);
    validate_non_empty_field("feature", "title", &feature.title, issues);
    validate_non_empty_field("feature", "summary", &feature.summary, issues);
    validate_non_empty_field("feature", "status", &feature.status, issues);
    let status = validate_delivery_status(
        "feature",
        &feature.id,
        &feature.status,
        resources.config,
        issues,
    );
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
                if resources.config.validate.require_reciprocal_links
                    && !requirement
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

    if !resources.spec_only {
        validate_trace_map(
            resources.root,
            resources.config,
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
        Some(DeliveryStatus::Implemented) if references_by_language.is_empty() => {
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
        Some(DeliveryStatus::Implemented) => {}
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
    let is_openapi = language == "openapi";
    let adapter = if is_openapi {
        None
    } else {
        let Some(adapter) = adapter_for_language(language) else {
            issues.push(Issue::error(
                    "SYU-trace-language-001",
                    subject,
                    Some(format_reference_location(language, reference)),
                    format!(
                        "Language `{language}` is not supported. Built-in adapters currently cover Rust, Python, Java, TypeScript, Shell, YAML, JSON, Markdown, and Gitignore."
                    ),
                    Some(format!(
                        "Use a supported language alias such as `rust`, `python`, `java`, `typescript`, `shell`, `yaml`, `json`, `markdown`, or `gitignore` for `{owner_id}`."
                    )),
                ));
            return false;
        };
        Some(adapter)
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

    let preferred_path = preferred_trace_file_path(&reference.file);
    if let Some(preferred_path) = preferred_path
        .as_ref()
        .filter(|preferred_path| *preferred_path != &reference.file)
    {
        issues.push(Issue::warning(
            "SYU-trace-file-003",
            subject.clone(),
            Some(format_reference_location(language, reference)),
            format!(
                "Declared {} file path `{}` is not written in canonical repository-relative form; prefer `{}`.",
                role.label(),
                reference.file.display(),
                preferred_path.display()
            ),
            Some(format!(
                "Change the `{language}` {} path for `{owner_id}` to `{}`.",
                role.relation_name(),
                preferred_path.display()
            )),
        ));
    }

    let display_path = preferred_path
        .as_deref()
        .unwrap_or(reference.file.as_path());
    let path = root.join(display_path);
    if !path.is_file() {
        issues.push(Issue::error(
            "SYU-trace-file-002",
            subject.clone(),
            Some(format_reference_location(language, reference)),
            format!(
                "Declared {} file `{}` does not exist.",
                role.label(),
                display_path.display()
            ),
            Some(format!(
                "Create `{}` or update `{owner_id}` to point to the correct {} file.",
                display_path.display(),
                role.label()
            )),
        ));
        return false;
    }

    let mut success = true;
    if !is_openapi {
        let adapter = adapter.expect("non-openapi traces always have an adapter");
        if !adapter.supports_path(&path) {
            issues.push(Issue::error(
                "SYU-trace-extension-001",
                subject.clone(),
                Some(format_reference_location(language, reference)),
                format!(
                    "File `{}` does not match the `{}` adapter extensions.",
                    display_path.display(),
                    adapter.canonical_name()
                ),
                Some(format!(
                    "Use a `{}` file extension or change the declared language for `{}`.",
                    adapter.canonical_name(),
                    display_path.display()
                )),
            ));
            success = false;
        }
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
                    display_path.display()
                ),
                Some(format!(
                    "Ensure `{}` is readable before running `syu validate`.",
                    display_path.display()
                )),
            ));
            return false;
        }
    };

    let ignored_paths = normalized_symbol_trace_coverage_ignored_paths(config);
    if !path_matches_ignored_generated_directory(display_path, &ignored_paths) {
        match evaluate_trace_ownership(root, config, owner_id, &path, reference, &contents) {
            OwnershipDeclaration::Satisfied => {}
            OwnershipDeclaration::Missing { manifest_path } => {
                let (message, suggestion) = match manifest_path {
                    Some(manifest_path) => {
                        let manifest_display = manifest_path
                            .strip_prefix(root)
                            .unwrap_or(manifest_path.as_path());
                        (
                            format!(
                                "Declared {} file `{}` does not declare ownership for `{owner_id}` in sidecar manifest `{}`.",
                                role.label(),
                                display_path.display(),
                                manifest_display.display()
                            ),
                            format!(
                                "Add `{owner_id}` with {} to `{}` so the {} remains explicitly traceable.",
                                ownership_symbols_hint(reference),
                                manifest_display.display(),
                                role.label()
                            ),
                        )
                    }
                    None => (
                        format!(
                            "Declared {} file `{}` does not mention `{owner_id}`.",
                            role.label(),
                            display_path.display()
                        ),
                        format!(
                            "Add `{owner_id}` to `{}` so the {} remains explicitly traceable.",
                            display_path.display(),
                            role.label()
                        ),
                    ),
                };
                issues.push(Issue::error(
                    "SYU-trace-id-001",
                    subject.clone(),
                    Some(format_reference_location(language, reference)),
                    message,
                    Some(suggestion),
                ));
                success = false;
            }
            OwnershipDeclaration::Invalid {
                manifest_path,
                reason,
            } => {
                let manifest_display = manifest_path
                    .strip_prefix(root)
                    .unwrap_or(manifest_path.as_path());
                issues.push(Issue::error(
                    "SYU-trace-id-001",
                    subject.clone(),
                    Some(format_reference_location(language, reference)),
                    format!(
                        "Sidecar ownership manifest `{}` for declared {} file `{}` is invalid: {reason}",
                        manifest_display.display(),
                        role.label(),
                        display_path.display()
                    ),
                    Some(format!(
                        "Fix `{}` or switch `validate.trace_ownership_mode` to `mapping` or `inline`.",
                        manifest_display.display()
                    )),
                ));
                success = false;
            }
        }
    }

    if is_openapi {
        return validate_openapi_trace_reference(
            owner_id,
            role,
            display_path,
            reference,
            &contents,
            issues,
        ) && success;
    }

    let adapter = adapter.expect("non-openapi traces always have an adapter");

    if reference.symbols.is_empty() {
        issues.push(Issue::error(
            "SYU-trace-symbol-001",
            subject.clone(),
            Some(format_reference_location(language, reference)),
            format!(
                "Declared {} file `{}` does not list any symbols to verify.",
                role.label(),
                display_path.display()
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
                    display_path.display()
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
                    display_path.display()
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
                            display_path.display()
                        ),
                        Some(format!(
                            "Fix the parser/runtime configuration for `{language}` or update `{}` so `syu validate` can inspect it.",
                            display_path.display()
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
                    display_path.display()
                ),
                Some(format!(
                    "Add symbol `{symbol}` to `{}` or update the YAML mapping for `{owner_id}`.",
                    display_path.display()
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
                                    display_path.display()
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

fn validate_openapi_trace_reference(
    owner_id: &str,
    role: TraceRole,
    display_path: &Path,
    reference: &TraceReference,
    contents: &str,
    issues: &mut Vec<Issue>,
) -> bool {
    let subject = format!("{} {}", role.subject_kind(), owner_id);
    let location = Some(format_reference_location("openapi", reference));
    let mut success = true;

    if !reference.symbols.is_empty() || !reference.doc_contains.is_empty() {
        issues.push(Issue::error(
            "SYU-trace-openapi-001",
            subject.clone(),
            location.clone(),
            format!(
                "OpenAPI selectors in `{}` should only declare `file`, `method`, and `path` in this version.",
                display_path.display()
            ),
            Some(
                "Remove `symbols` and `doc_contains`, then keep the OpenAPI trace focused on one operation."
                    .to_string(),
            ),
        ));
        success = false;
    }

    let Some(method) = normalized_openapi_method(reference.method.as_deref()) else {
        issues.push(Issue::error(
            "SYU-trace-openapi-001",
            subject.clone(),
            location.clone(),
            format!(
                "OpenAPI selector in `{}` is missing an HTTP method.",
                display_path.display()
            ),
            Some(format!(
                "Add an HTTP method such as `get` or `post` to the OpenAPI trace for `{owner_id}`."
            )),
        ));
        return false;
    };

    let Some(path_template) = normalized_openapi_path(reference.path.as_deref()) else {
        issues.push(Issue::error(
            "SYU-trace-openapi-001",
            subject.clone(),
            location.clone(),
            format!(
                "OpenAPI selector in `{}` is missing a path template.",
                display_path.display()
            ),
            Some(format!(
                "Add a path template such as `/pets/{{petId}}` to the OpenAPI trace for `{owner_id}`."
            )),
        ));
        return false;
    };

    let document: serde_yaml::Value = match serde_yaml::from_str(contents) {
        Ok(document) => document,
        Err(error) => {
            issues.push(Issue::error(
                "SYU-trace-openapi-002",
                subject,
                location,
                format!(
                    "OpenAPI document `{}` could not be parsed as JSON or YAML: {error}",
                    display_path.display()
                ),
                Some(format!(
                    "Fix `{}` so the OpenAPI selector for `{owner_id}` can be validated.",
                    display_path.display()
                )),
            ));
            return false;
        }
    };

    let Some(document_map) = document.as_mapping() else {
        issues.push(Issue::error(
            "SYU-trace-openapi-002",
            subject.clone(),
            location.clone(),
            format!(
                "OpenAPI document `{}` is not a mapping.",
                display_path.display()
            ),
            Some(format!(
                "Fix `{}` so the OpenAPI selector for `{owner_id}` can be validated.",
                display_path.display()
            )),
        ));
        return false;
    };

    let is_openapi = document_map.contains_key(serde_yaml::Value::String("openapi".to_string()))
        || document_map.contains_key(serde_yaml::Value::String("swagger".to_string()));
    if !is_openapi {
        issues.push(Issue::error(
            "SYU-trace-openapi-002",
            subject.clone(),
            location.clone(),
            format!(
                "OpenAPI document `{}` does not declare an OpenAPI version field.",
                display_path.display()
            ),
            Some(format!(
                "Add `openapi` or `swagger` to `{}` or point `{owner_id}` at the correct OpenAPI document.",
                display_path.display()
            )),
        ));
        return false;
    }

    let Some(paths) = document_map
        .get(serde_yaml::Value::String("paths".to_string()))
        .and_then(serde_yaml::Value::as_mapping)
    else {
        issues.push(Issue::error(
            "SYU-trace-openapi-002",
            subject.clone(),
            location.clone(),
            format!(
                "OpenAPI document `{}` does not declare a `paths` object.",
                display_path.display()
            ),
            Some(format!(
                "Add a `paths` section to `{}` or point `{owner_id}` at the correct OpenAPI document.",
                display_path.display()
            )),
        ));
        return false;
    };

    let Some(path_item) = paths.get(serde_yaml::Value::String(path_template.clone())) else {
        issues.push(Issue::error(
            "SYU-trace-openapi-002",
            subject.clone(),
            location.clone(),
            format!(
                "OpenAPI path `{path_template}` was not found in `{}`.",
                display_path.display()
            ),
            Some(format!(
                "Add path `{path_template}` to `{}` or update the OpenAPI trace for `{owner_id}`.",
                display_path.display()
            )),
        ));
        return false;
    };

    let Some(path_item_map) =
        resolve_openapi_path_item(&document, path_item).and_then(serde_yaml::Value::as_mapping)
    else {
        issues.push(Issue::error(
            "SYU-trace-openapi-002",
            subject.clone(),
            location.clone(),
            format!(
                "OpenAPI path `{path_template}` in `{}` is not a valid path item object.",
                display_path.display()
            ),
            Some(format!(
                "Fix path `{path_template}` in `{}` so the OpenAPI selector for `{owner_id}` can resolve an operation.",
                display_path.display()
            )),
        ));
        return false;
    };

    let Some(operation) = path_item_map.get(serde_yaml::Value::String(method.clone())) else {
        issues.push(Issue::error(
            "SYU-trace-openapi-002",
            subject,
            location,
            format!(
                "OpenAPI method `{method}` was not found for path `{path_template}` in `{}`.",
                display_path.display()
            ),
            Some(format!(
                "Add method `{method}` to path `{path_template}` in `{}` or update the OpenAPI trace for `{owner_id}`.",
                display_path.display()
            )),
        ));
        return false;
    };

    if !operation.is_mapping() {
        issues.push(Issue::error(
            "SYU-trace-openapi-002",
            format!("{} {}", role.subject_kind(), owner_id),
            Some(format_reference_location("openapi", reference)),
            format!(
                "OpenAPI operation `{method} {path_template}` in `{}` is not a valid operation object.",
                display_path.display()
            ),
            Some(format!(
                "Fix operation `{method} {path_template}` in `{}` so the selector can be validated.",
                display_path.display()
            )),
        ));
        return false;
    }

    success
}

fn resolve_openapi_path_item<'a>(
    document: &'a serde_yaml::Value,
    path_item: &'a serde_yaml::Value,
) -> Option<&'a serde_yaml::Value> {
    let mut current = path_item;

    for _ in 0..8 {
        let reference = current.as_mapping().and_then(|mapping| {
            mapping
                .get(serde_yaml::Value::String("$ref".to_string()))
                .and_then(serde_yaml::Value::as_str)
        });

        let Some(reference) = reference else {
            return Some(current);
        };

        let pointer = reference.strip_prefix("#/")?;

        let mut next = document;
        for segment in pointer.split('/') {
            let segment = segment.replace("~1", "/").replace("~0", "~");
            next = match next {
                serde_yaml::Value::Mapping(mapping) => {
                    mapping.get(serde_yaml::Value::String(segment))?
                }
                serde_yaml::Value::Sequence(sequence) => {
                    let index = segment.parse::<usize>().ok()?;
                    sequence.get(index)?
                }
                _ => return None,
            };
        }

        current = next;
    }

    None
}

fn normalized_openapi_method(method: Option<&str>) -> Option<String> {
    let method = method?.trim();
    if method.is_empty() {
        return None;
    }

    let normalized = method.to_ascii_lowercase();
    match normalized.as_str() {
        "get" | "put" | "post" | "delete" | "options" | "head" | "patch" | "trace" => {
            Some(normalized)
        }
        _ => None,
    }
}

fn normalized_openapi_path(path: Option<&str>) -> Option<String> {
    let path = path?.trim();
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

fn format_reference_location(language: &str, reference: &TraceReference) -> String {
    if language == "openapi" {
        let mut location = format!("{language}:{}", reference.file.display());
        if let Some(method) = reference
            .method
            .as_deref()
            .map(str::trim)
            .filter(|method| !method.is_empty())
        {
            location.push('#');
            location.push_str(&method.to_ascii_lowercase());
            if let Some(path) = reference
                .path
                .as_deref()
                .map(str::trim)
                .filter(|path| !path.is_empty())
            {
                location.push(' ');
                location.push_str(path);
            }
        }
        return location;
    }

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
        process::Command,
    };

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use tempfile::tempdir;

    use crate::{
        cli::{ValidationGenreFilter, ValidationSeverityFilter},
        config::{SyuConfig, TraceOwnershipMode},
        history::build_historical_id_index,
        model::{
            DefinitionCounts, Feature, FeatureDocument, Issue, OwnershipEntry, OwnershipManifest,
            Philosophy, PhilosophyDocument, Policy, PolicyDocument, Requirement,
            RequirementDocument, TraceReference,
        },
        rules::{all_rules, referenced_rules},
        workspace::Workspace,
    };

    use super::{
        AutofixChangeRequest, AutofixMode, AutofixRun, DeliveryStatus,
        load_feature_documents_for_autofix, normalize_delivery_status, normalize_trace_reference,
        trace_reference_dedup_key, write_or_plan_autofix_change,
    };

    use super::{
        AutofixPlan, AutofixPlanChange, AutofixTransaction, FilteredIssueView,
        HISTORICAL_ID_RULE_CODES, IssueFilters, ItemLocation, MutableLoadedDocument,
        ORPHAN_RULE_CODES, RECIPROCAL_RULE_CODES, RequirementValidationIndex, TextReportSummary,
        TraceRole, ValidationResources, apply_autofix, apply_feature_reciprocals,
        apply_philosophy_reciprocals, apply_policy_reciprocals, apply_requirement_reciprocals,
        collect_check_result, collect_check_result_from_workspace, collect_feature_yaml_paths,
        describe_trace_reference, entry_covers_symbols, feature_registry_kind, filter_check_result,
        finalize_autofix_error, format_reference_location, looks_like_feature_document,
        merge_ownership_entry, ownership_symbols_hint, preferred_trace_file_path,
        render_autofix_plan, render_text_report, required_ownership_symbols,
        resolve_openapi_path_item, run_check_command, sync_feature_registry,
        validate_duplicate_links, validate_duplicate_trace_references, validate_feature,
        validate_feature_registry_entries, validate_historical_id_reuse, validate_non_empty_field,
        validate_openapi_trace_reference, validate_philosophy, validate_policy,
        validate_requirement, validate_unique_ids, verify_trace_reference,
    };

    fn git(workspace: &Path, args: &[&str]) {
        let mut command = Command::new("git");
        command.arg("-C").arg(workspace).args(args);
        let output = command.output().expect("git should run");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
            args,
            stdout,
            stderr
        );
    }

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

    fn validation_resources<'a>(config: &'a SyuConfig) -> ValidationResources<'a> {
        ValidationResources {
            config,
            root: Path::new("."),
            spec_only: false,
        }
    }

    fn requirement_validation_index<'a>(
        policies_by_id: &'a HashMap<&'a str, &'a Policy>,
        features_by_id: &'a HashMap<&'a str, &'a Feature>,
    ) -> RequirementValidationIndex<'a> {
        RequirementValidationIndex {
            policies_by_id,
            features_by_id,
        }
    }

    fn apply_autofix_for_reference(
        root: &Path,
        config: &SyuConfig,
        owner_id: &str,
        language: &str,
        reference: &TraceReference,
        summary: &mut super::AutofixSummary,
    ) -> anyhow::Result<()> {
        let mut run = super::AutofixRun {
            summary: std::mem::take(summary),
            plan: None,
        };
        super::apply_autofix_for_reference(
            root,
            config,
            owner_id,
            language,
            reference,
            &mut run,
            super::AutofixMode::Apply,
        )?;
        *summary = run.summary;
        Ok(())
    }

    fn write_valid_planned_workspace(root: &Path) {
        fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
        fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
        fs::create_dir_all(root.join("docs/syu/requirements")).expect("requirements dir");
        fs::create_dir_all(root.join("docs/syu/features")).expect("features dir");

        fs::write(
            root.join("syu.yaml"),
            format!(
                "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_reciprocal_links: true\n  require_symbol_trace_coverage: false\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
                version = env!("CARGO_PKG_VERSION")
            ),
        )
        .expect("config should exist");
        fs::write(
            root.join("docs/syu/philosophy/foundation.yaml"),
            "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Keep planning explicit\n    product_design_principle: Planned work should stay visible until delivery starts.\n    coding_guideline: Prefer explicit status values over implied intent.\n    linked_policies:\n      - POL-001\n",
        )
        .expect("philosophy should exist");
        fs::write(
            root.join("docs/syu/policies/policies.yaml"),
            "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Planned work should remain reviewable\n    summary: Delivery states should support gradual adoption.\n    description: This fixture covers the validate --fix reload path.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-001\n",
        )
        .expect("policy should exist");
        fs::write(
            root.join("docs/syu/requirements/core.yaml"),
            "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Planned requirements can exist before delivery starts\n    description: Planned items should stay trace-free until implementation begins.\n    priority: high\n    status: planned\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n    tests: {}\n",
        )
        .expect("requirement should exist");
        fs::write(
            root.join("docs/syu/features/features.yaml"),
            format!(
                "version: \"{}\"\nfiles:\n  - kind: core\n    file: core.yaml\n",
                env!("CARGO_PKG_VERSION")
            ),
        )
        .expect("feature registry should exist");
        fs::write(
            root.join("docs/syu/features/core.yaml"),
            "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Planned features can stay undocumented by traces\n    summary: Delivery claims should not appear before the work ships.\n    status: planned\n    linked_requirements:\n      - REQ-001\n    implementations: {}\n",
        )
        .expect("feature should exist");
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
            id: Vec::new(),
            spec_only: false,
            fix: false,
            dry_run: false,
            no_fix: false,
            allow_planned: None,
            require_non_orphaned_items: None,
            require_reciprocal_links: None,
            require_symbol_trace_coverage: None,
            warning_exit_code: None,
            quiet: false,
        })
        .expect("command should render load errors");

        assert_eq!(code, 1);
    }

    #[test]
    fn run_check_command_bootstraps_missing_registry_when_fixing() {
        let tempdir = tempdir().expect("tempdir should exist");
        write_valid_planned_workspace(tempdir.path());
        fs::remove_file(tempdir.path().join("docs/syu/features/features.yaml"))
            .expect("feature registry should be removable");

        let code = run_check_command(&crate::cli::CheckArgs {
            workspace: tempdir.path().to_path_buf(),
            format: crate::cli::OutputFormat::Json,
            severity: Vec::new(),
            genre: Vec::new(),
            rule: Vec::new(),
            id: Vec::new(),
            spec_only: false,
            fix: true,
            dry_run: false,
            no_fix: false,
            allow_planned: None,
            require_non_orphaned_items: None,
            require_reciprocal_links: None,
            require_symbol_trace_coverage: None,
            warning_exit_code: None,
            quiet: false,
        })
        .expect("command should complete");

        assert_eq!(code, 0);
        assert!(
            tempdir
                .path()
                .join("docs/syu/features/features.yaml")
                .is_file()
        );
    }

    #[test]
    fn run_check_command_reports_missing_registry_without_fixing() {
        let tempdir = tempdir().expect("tempdir should exist");
        write_valid_planned_workspace(tempdir.path());
        fs::remove_file(tempdir.path().join("docs/syu/features/features.yaml"))
            .expect("feature registry should be removable");

        let code = run_check_command(&crate::cli::CheckArgs {
            workspace: tempdir.path().to_path_buf(),
            format: crate::cli::OutputFormat::Json,
            severity: Vec::new(),
            genre: Vec::new(),
            rule: Vec::new(),
            id: Vec::new(),
            spec_only: false,
            fix: false,
            dry_run: false,
            no_fix: false,
            allow_planned: None,
            require_non_orphaned_items: None,
            require_reciprocal_links: None,
            require_symbol_trace_coverage: None,
            warning_exit_code: None,
            quiet: false,
        })
        .expect("command should render load errors");

        assert_eq!(code, 1);
        assert!(
            !tempdir
                .path()
                .join("docs/syu/features/features.yaml")
                .is_file()
        );
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
            id: Vec::new(),
            spec_only: false,
            fix: true,
            dry_run: false,
            no_fix: false,
            allow_planned: None,
            require_non_orphaned_items: None,
            require_reciprocal_links: None,
            require_symbol_trace_coverage: None,
            warning_exit_code: None,
            quiet: false,
        })
        .expect_err("autofix failures should bubble up");

        assert!(
            error
                .to_string()
                .contains("autofix failed before applying changes")
        );
    }

    #[test]
    fn run_check_command_with_fix_reloads_workspace_before_cli_overrides() {
        let tempdir = tempdir().expect("tempdir should exist");
        write_valid_planned_workspace(tempdir.path());

        let code = run_check_command(&crate::cli::CheckArgs {
            workspace: tempdir.path().to_path_buf(),
            format: crate::cli::OutputFormat::Json,
            severity: Vec::new(),
            genre: Vec::new(),
            rule: Vec::new(),
            id: Vec::new(),
            spec_only: false,
            fix: true,
            dry_run: false,
            no_fix: false,
            allow_planned: Some(false),
            require_non_orphaned_items: None,
            require_reciprocal_links: None,
            require_symbol_trace_coverage: None,
            warning_exit_code: None,
            quiet: false,
        })
        .expect("command should complete");

        assert_eq!(code, 1);
    }

    #[test]
    fn apply_autofix_reports_registry_read_errors() {
        let tempdir = tempdir().expect("tempdir should exist");
        write_valid_planned_workspace(tempdir.path());
        let registry_path = tempdir.path().join("docs/syu/features/features.yaml");
        fs::remove_file(&registry_path).expect("feature registry should be removable");
        fs::create_dir(&registry_path).expect("directory registry path should be creatable");

        let workspace =
            crate::workspace::load_workspace_allowing_missing_feature_registry(tempdir.path())
                .expect("workspace should still load");
        let error = apply_autofix(&workspace).expect_err("registry read failure should bubble up");

        assert!(error.to_string().contains("Is a directory"));
    }

    #[test]
    fn sync_feature_registry_reports_read_errors() {
        let tempdir = tempdir().expect("tempdir should exist");
        let feature_root = tempdir.path().join("docs/syu/features");
        fs::create_dir_all(&feature_root).expect("feature root should exist");
        fs::create_dir(feature_root.join("features.yaml"))
            .expect("registry path should be creatable as a directory");

        let error =
            sync_feature_registry(&feature_root, &[]).expect_err("read failure should bubble up");
        assert!(error.to_string().contains("Is a directory"));
    }

    #[test]
    fn run_check_command_rejects_dry_run_without_an_effective_fix_path() {
        let tempdir = tempdir().expect("tempdir should exist");
        write_valid_planned_workspace(tempdir.path());

        let error = run_check_command(&crate::cli::CheckArgs {
            workspace: tempdir.path().to_path_buf(),
            format: crate::cli::OutputFormat::Text,
            severity: Vec::new(),
            genre: Vec::new(),
            rule: Vec::new(),
            id: Vec::new(),
            spec_only: false,
            fix: false,
            dry_run: true,
            no_fix: false,
            allow_planned: None,
            require_non_orphaned_items: None,
            require_reciprocal_links: None,
            require_symbol_trace_coverage: None,
            warning_exit_code: None,
            quiet: false,
        })
        .expect_err("dry-run without fix should fail");

        assert!(
            error
                .to_string()
                .contains("requires an effective autofix path")
        );
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

        let report = render_text_report(true, &result, Path::new("."), None, None, false, false);
        assert!(report.contains("syu validate passed"));
        assert!(report.contains("issues:"));
        assert!(report.contains("[Warning] warn subject: message"));
    }

    #[test]
    fn text_report_summary_counts_disabled_rule_groups_from_group_definitions() {
        let config = SyuConfig {
            validate: crate::config::ValidateConfig {
                require_non_orphaned_items: false,
                require_reciprocal_links: false,
                require_symbol_trace_coverage: true,
                historical_ids: crate::config::HistoricalIdsConfig {
                    enabled: false,
                    start_ref: None,
                },
                ..Default::default()
            },
            ..Default::default()
        };

        let summary = TextReportSummary::from_config(
            &config,
            &DefinitionCounts {
                philosophies: 1,
                policies: 2,
                requirements: 3,
                features: 4,
            },
        );

        assert_eq!(
            summary.checked_rule_count,
            all_rules().len()
                - ORPHAN_RULE_CODES.len()
                - RECIPROCAL_RULE_CODES.len()
                - HISTORICAL_ID_RULE_CODES.len()
        );
        assert_eq!(summary.workspace_item_count, 10);
        assert_eq!(
            summary
                .disabled_checks
                .iter()
                .map(|notice| notice.describe())
                .collect::<Vec<_>>(),
            vec![
                "validate.require_non_orphaned_items=false (1 rule)".to_string(),
                "validate.require_reciprocal_links=false (1 rule)".to_string(),
                "validate.historical_ids.enabled=false (1 rule)".to_string(),
            ]
        );
    }

    #[test]
    fn collect_check_result_from_workspace_skips_historical_id_index_when_disabled() {
        let tempdir = tempdir().expect("tempdir should exist");
        write_valid_planned_workspace(tempdir.path());
        let mut workspace =
            crate::workspace::load_workspace(tempdir.path()).expect("workspace should load");
        workspace.config.validate.historical_ids.enabled = false;

        let result = collect_check_result_from_workspace(&workspace);

        assert!(result.issues.is_empty());
    }

    #[test]
    fn render_text_report_uses_singular_disabled_rule_summary() {
        let config = SyuConfig {
            validate: crate::config::ValidateConfig {
                require_non_orphaned_items: false,
                require_reciprocal_links: true,
                require_symbol_trace_coverage: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let summary = TextReportSummary::from_config(
            &config,
            &DefinitionCounts {
                philosophies: 1,
                policies: 1,
                requirements: 1,
                features: 1,
            },
        );
        let result = crate::model::CheckResult {
            workspace_root: PathBuf::from("."),
            definition_counts: Default::default(),
            trace_summary: Default::default(),
            issues: Vec::new(),
            referenced_rules: Vec::new(),
        };

        let report = render_text_report(
            true,
            &result,
            Path::new("."),
            None,
            Some(&summary),
            false,
            false,
        );

        assert!(report.contains("note: 1 built-in rule is disabled by current config"));
    }

    #[test]
    fn validate_historical_id_reuse_skips_unavailable_indexes_and_missing_matches() {
        let workspace = test_workspace(Path::new("."));
        let mut issues = Vec::new();

        let unavailable_root = tempdir().expect("tempdir should exist");
        let unavailable_index =
            build_historical_id_index(unavailable_root.path(), &SyuConfig::default())
                .expect("index should build");
        assert!(!unavailable_index.available());
        validate_historical_id_reuse(&workspace, &unavailable_index, &mut issues);
        assert!(issues.is_empty());

        let tempdir = tempdir().expect("tempdir should exist");
        let repo = tempdir.path();
        git(repo, &["init"]);
        git(repo, &["config", "user.name", "Test User"]);
        git(repo, &["config", "user.email", "test@example.com"]);
        fs::write(repo.join("placeholder.txt"), "placeholder\n").expect("placeholder file");
        git(repo, &["add", "."]);
        git(repo, &["commit", "-m", "initial commit"]);

        let available_index =
            build_historical_id_index(repo, &SyuConfig::default()).expect("index should build");
        assert!(available_index.available());

        let mut workspace = test_workspace(Path::new("."));
        workspace.requirements.push(Requirement {
            id: "REQ-000".to_string(),
            title: "Example".to_string(),
            description: "Example".to_string(),
            priority: "high".to_string(),
            status: "implemented".to_string(),
            linked_policies: Vec::new(),
            linked_features: Vec::new(),
            tests: BTreeMap::new(),
        });

        validate_historical_id_reuse(&workspace, &available_index, &mut issues);
        assert!(issues.is_empty());
    }

    #[test]
    fn validate_historical_id_reuse_skips_duplicate_ids_in_the_same_kind() {
        let tempdir = tempdir().expect("tempdir should exist");
        let repo = tempdir.path();
        git(repo, &["init"]);
        git(repo, &["config", "user.name", "Test User"]);
        git(repo, &["config", "user.email", "test@example.com"]);
        fs::write(repo.join("placeholder.txt"), "placeholder\n").expect("placeholder file");
        git(repo, &["add", "."]);
        git(repo, &["commit", "-m", "initial commit"]);

        let available_index =
            build_historical_id_index(repo, &SyuConfig::default()).expect("index should build");
        assert!(available_index.available());

        let mut workspace = test_workspace(Path::new("."));
        workspace.requirements.push(Requirement {
            id: "REQ-000".to_string(),
            title: "Example".to_string(),
            description: "Example".to_string(),
            priority: "high".to_string(),
            status: "implemented".to_string(),
            linked_policies: Vec::new(),
            linked_features: Vec::new(),
            tests: BTreeMap::new(),
        });
        workspace.requirements.push(Requirement {
            id: "REQ-000".to_string(),
            title: "Duplicate".to_string(),
            description: "Duplicate".to_string(),
            priority: "high".to_string(),
            status: "implemented".to_string(),
            linked_policies: Vec::new(),
            linked_features: Vec::new(),
            tests: BTreeMap::new(),
        });

        let mut issues = Vec::new();
        validate_historical_id_reuse(&workspace, &available_index, &mut issues);

        assert!(issues.is_empty());
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
            ids: BTreeSet::new(),
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
            ids: Vec::new(),
            displayed_issue_count: 0,
            total_issue_count: 2,
            hidden_issue_count: 2,
        };

        let report = render_text_report(
            false,
            &result,
            Path::new("."),
            Some(&filtered_view),
            None,
            false,
            false,
        );

        assert!(report.contains("syu validate failed (filtered view)"));
        assert!(report.contains("filters: severity=warning"));
        assert!(report.contains("showing 0 of 2 issues after filtering"));
        assert!(report.contains("no issues matched the active filters."));
    }

    #[test]
    fn render_text_report_quiet_success_suppresses_filtered_footer() {
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
            ids: Vec::new(),
            displayed_issue_count: 0,
            total_issue_count: 2,
            hidden_issue_count: 2,
        };

        let report = render_text_report(
            true,
            &result,
            Path::new("."),
            Some(&filtered_view),
            None,
            false,
            true,
        );

        assert_eq!(report.trim(), "syu validate passed (filtered view)");
    }

    #[test]
    fn filtered_issue_view_describes_genre_filters() {
        let filtered_view = FilteredIssueView {
            severities: Vec::new(),
            genres: vec!["trace".to_string()],
            rules: Vec::new(),
            ids: Vec::new(),
            displayed_issue_count: 1,
            total_issue_count: 1,
            hidden_issue_count: 0,
        };

        assert_eq!(filtered_view.describe_filters(), "genre=trace");
    }

    #[test]
    fn filtered_issue_view_describes_id_filters() {
        let filtered_view = FilteredIssueView {
            severities: Vec::new(),
            genres: Vec::new(),
            rules: Vec::new(),
            ids: vec!["REQ-001".to_string()],
            displayed_issue_count: 1,
            total_issue_count: 3,
            hidden_issue_count: 2,
        };

        assert_eq!(filtered_view.describe_filters(), "id=REQ-001");
    }

    #[test]
    fn validate_unique_ids_reports_duplicates() {
        let mut issues = Vec::new();
        validate_unique_ids("feature", ["FEAT-1", "FEAT-1"].into_iter(), &mut issues);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "SYU-workspace-duplicate-001");
    }

    fn test_workspace(root: &Path) -> Workspace {
        Workspace {
            root: root.to_path_buf(),
            spec_root: root.join("docs/syu"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        }
    }

    #[test]
    fn validate_feature_registry_entries_reports_registry_read_failures() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = test_workspace(tempdir.path());
        fs::create_dir_all(workspace.spec_root.join("features"))
            .expect("features dir should exist");

        let mut issues = Vec::new();
        validate_feature_registry_entries(&workspace, &mut issues);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "SYU-workspace-registry-001");
        assert_eq!(
            issues[0].location.as_deref(),
            Some("docs/syu/features/features.yaml")
        );
        assert!(
            issues[0]
                .message
                .contains("Failed to compare feature files")
        );
        assert!(
            issues[0]
                .message
                .contains("failed to read feature registry")
        );
        assert!(
            issues[0]
                .suggestion
                .as_deref()
                .expect("suggestion should exist")
                .contains("explicit feature registry")
        );
    }

    #[test]
    fn validate_feature_registry_entries_reports_registry_parse_failures() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = test_workspace(tempdir.path());
        let feature_root = workspace.spec_root.join("features");
        fs::create_dir_all(&feature_root).expect("features dir should exist");
        fs::write(feature_root.join("features.yaml"), "version: [").expect("registry should exist");

        let mut issues = Vec::new();
        validate_feature_registry_entries(&workspace, &mut issues);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "SYU-workspace-registry-001");
        assert!(
            issues[0]
                .message
                .contains("failed to parse feature registry")
        );
    }

    #[test]
    fn collect_feature_yaml_paths_reports_directory_read_failures() {
        let tempdir = tempdir().expect("tempdir should exist");
        let mut files = Vec::new();
        let error = collect_feature_yaml_paths(&tempdir.path().join("missing"), &mut files)
            .expect_err("missing directories should fail");

        assert!(error.to_string().contains("failed to read directory"));
    }

    #[test]
    fn looks_like_feature_document_reports_read_and_parse_failures() {
        let tempdir = tempdir().expect("tempdir should exist");
        let missing_error = looks_like_feature_document(&tempdir.path().join("missing.yaml"))
            .expect_err("missing feature candidates should fail");
        assert!(
            missing_error
                .to_string()
                .contains("failed to read feature candidate")
        );

        let invalid = tempdir.path().join("invalid.yaml");
        fs::write(&invalid, "features: [").expect("invalid yaml should exist");
        let parse_error =
            looks_like_feature_document(&invalid).expect_err("invalid yaml should fail");
        assert!(
            parse_error
                .to_string()
                .contains("failed to parse feature candidate")
        );
    }

    #[test]
    fn validate_feature_registry_entries_reports_candidate_parse_failures_on_candidate_path() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = test_workspace(tempdir.path());
        let feature_root = workspace.spec_root.join("features");
        fs::create_dir_all(feature_root.join("extra")).expect("features dir should exist");
        fs::write(
            feature_root.join("features.yaml"),
            format!("version: \"{}\"\nfiles: []\n", env!("CARGO_PKG_VERSION")),
        )
        .expect("feature registry should exist");
        fs::write(feature_root.join("extra/stray.yaml"), "features: [")
            .expect("invalid feature candidate should exist");

        let mut issues = Vec::new();
        validate_feature_registry_entries(&workspace, &mut issues);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "SYU-workspace-registry-001");
        assert_eq!(
            issues[0].location.as_deref(),
            Some("docs/syu/features/extra/stray.yaml")
        );
        assert!(
            issues[0]
                .message
                .contains("failed to parse feature candidate")
        );
        assert!(
            issues[0]
                .suggestion
                .as_deref()
                .expect("suggestion should exist")
                .contains("Fix `docs/syu/features/extra/stray.yaml`")
        );
    }

    #[test]
    fn validate_feature_registry_entries_ignores_non_feature_yaml_candidates() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = test_workspace(tempdir.path());
        let feature_root = workspace.spec_root.join("features");
        fs::create_dir_all(feature_root.join("extra")).expect("features dir should exist");
        fs::write(
            feature_root.join("features.yaml"),
            format!("version: \"{}\"\nfiles: []\n", env!("CARGO_PKG_VERSION")),
        )
        .expect("feature registry should exist");
        fs::write(feature_root.join("extra/catalog.yaml"), "rules: []\n")
            .expect("non-feature yaml should exist");

        let mut issues = Vec::new();
        validate_feature_registry_entries(&workspace, &mut issues);

        assert!(issues.is_empty(), "non-feature yaml should be ignored");
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
            method: None,
            path: None,
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
    fn normalize_trace_reference_canonicalizes_and_deduplicates_files() {
        let canonical = TraceReference {
            file: PathBuf::from("src/lib.rs"),
            symbols: vec!["trace_symbol".to_string()],
            doc_contains: vec!["REQ-1".to_string()],
            method: None,
            path: None,
        };
        let relative = TraceReference {
            file: PathBuf::from("./src/lib.rs"),
            ..canonical.clone()
        };

        let normalized = normalize_trace_reference(&relative);
        let mut seen = BTreeSet::new();
        assert!(seen.insert(trace_reference_dedup_key(&normalized)));
        assert!(
            !seen.insert(trace_reference_dedup_key(&normalize_trace_reference(
                &canonical
            )))
        );
        assert_eq!(normalized.file, PathBuf::from("src/lib.rs"));
        assert_eq!(normalized.symbols, vec!["trace_symbol".to_string()]);
        assert_eq!(normalized.doc_contains, vec!["REQ-1".to_string()]);
    }

    #[test]
    fn normalize_delivery_status_accepts_typos_and_known_values() {
        assert_eq!(
            normalize_delivery_status(" planed "),
            Some(DeliveryStatus::Planned)
        );
        assert_eq!(
            normalize_delivery_status("implemented"),
            Some(DeliveryStatus::Implemented)
        );
        assert_eq!(normalize_delivery_status("unknown"), None);
    }

    #[test]
    fn load_feature_documents_for_autofix_loads_only_feature_documents() {
        let tempdir = tempdir().expect("tempdir should exist");
        let feature_root = tempdir.path().join("docs/syu/features");
        fs::create_dir_all(feature_root.join("core")).expect("feature dirs");
        fs::write(
            feature_root.join("features.yaml"),
            "version: \"1\"\nfiles:\n  - kind: core\n    file: core.yaml\n",
        )
        .expect("registry");
        fs::write(
            feature_root.join("core.yaml"),
            "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-1\n    title: Example\n    summary: Keep it simple.\n    status: implemented\n    linked_requirements: []\n    implementations: {}\n",
        )
        .expect("feature document");
        fs::write(
            feature_root.join("notes.yaml"),
            "category: Notes\nversion: 1\nnotes:\n  - ignore me\n",
        )
        .expect("non-feature document");

        let documents = load_feature_documents_for_autofix(&feature_root)
            .expect("feature documents should load");

        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].path, feature_root.join("core.yaml"));
    }

    #[test]
    fn write_or_plan_autofix_change_records_apply_and_plan_modes() {
        let tempdir = tempdir().expect("tempdir should exist");
        let root = tempdir.path();
        let path = root.join("trace.rs");
        fs::write(&path, "original").expect("file should exist");

        let mut apply_run = AutofixRun::default();
        let mut apply_transaction = AutofixTransaction::default();
        write_or_plan_autofix_change(AutofixChangeRequest {
            root,
            path: &path,
            transaction: &mut apply_transaction,
            run: &mut apply_run,
            mode: AutofixMode::Apply,
            rules: vec!["SYU-trace-file-003"],
            summary_text: "update trace path".to_string(),
            contents: Some("updated".to_string()),
        })
        .expect("apply mode should write");

        assert_eq!(
            fs::read_to_string(&path).expect("updated file should exist"),
            "updated"
        );
        assert_eq!(apply_run.summary.symbol_updates, 1);
        assert!(
            apply_run
                .summary
                .updated_files
                .contains(Path::new("trace.rs"))
        );

        let mut plan_run = AutofixRun {
            plan: Some(AutofixPlan::default()),
            ..Default::default()
        };
        let mut plan_transaction = AutofixTransaction::default();
        write_or_plan_autofix_change(AutofixChangeRequest {
            root,
            path: &path,
            transaction: &mut plan_transaction,
            run: &mut plan_run,
            mode: AutofixMode::Plan,
            rules: vec!["SYU-trace-file-003"],
            summary_text: "update trace path".to_string(),
            contents: None,
        })
        .expect("plan mode should record change");

        let plan = plan_run.plan.expect("plan should be initialized");
        assert_eq!(plan.planned_updates, 1);
        assert!(plan.updated_files.contains(Path::new("trace.rs")));
        assert_eq!(plan.changes.len(), 1);
    }

    #[test]
    fn describe_trace_reference_includes_openapi_selector_details() {
        let reference = TraceReference {
            file: PathBuf::from("api/openapi.yaml"),
            symbols: Vec::new(),
            doc_contains: Vec::new(),
            method: Some("get".to_string()),
            path: Some("/pets/{petId}".to_string()),
        };

        assert_eq!(
            describe_trace_reference(&reference),
            "file=`api/openapi.yaml` symbols=[] doc_contains=[] method=`get` path=`/pets/{petId}`"
        );
    }

    #[test]
    fn validate_openapi_trace_reference_covers_selector_and_document_failures() {
        let tempdir = tempdir().expect("tempdir should exist");
        let display_path = tempdir.path().join("api/openapi.yaml");
        fs::create_dir_all(display_path.parent().expect("api dir should exist")).expect("api dir");

        let reference = TraceReference {
            file: PathBuf::from("api/openapi.yaml"),
            symbols: vec!["unexpected".to_string()],
            doc_contains: vec!["unexpected".to_string()],
            method: Some("get".to_string()),
            path: Some("/pets/{petId}".to_string()),
        };
        let openapi_reference = TraceReference {
            file: PathBuf::from("api/openapi.yaml"),
            symbols: Vec::new(),
            doc_contains: Vec::new(),
            method: Some("get".to_string()),
            path: Some("/pets/{petId}".to_string()),
        };

        let mut issues = Vec::new();
        assert!(!validate_openapi_trace_reference(
            "FEAT-API-001",
            TraceRole::FeatureImplementation,
            &display_path,
            &reference,
            "openapi: 3.0.3\npaths:\n  /pets/{petId}:\n    get:\n      responses:\n        '200':\n          description: ok\n",
            &mut issues,
        ));
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-trace-openapi-001")
        );

        issues.clear();
        assert!(!validate_openapi_trace_reference(
            "FEAT-API-001",
            TraceRole::FeatureImplementation,
            &display_path,
            &TraceReference {
                method: Some("   ".to_string()),
                path: Some("/pets/{petId}".to_string()),
                ..reference.clone()
            },
            "openapi: 3.0.3\npaths:\n  /pets/{petId}:\n    get:\n      responses:\n        '200':\n          description: ok\n",
            &mut issues,
        ));
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-trace-openapi-001")
        );

        issues.clear();
        assert!(!validate_openapi_trace_reference(
            "FEAT-API-001",
            TraceRole::FeatureImplementation,
            &display_path,
            &TraceReference {
                method: Some("get".to_string()),
                path: Some("   ".to_string()),
                ..reference.clone()
            },
            "openapi: 3.0.3\npaths:\n  /pets/{petId}:\n    get:\n      responses:\n        '200':\n          description: ok\n",
            &mut issues,
        ));
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-trace-openapi-001")
        );

        issues.clear();
        assert!(!validate_openapi_trace_reference(
            "FEAT-API-001",
            TraceRole::FeatureImplementation,
            &display_path,
            &TraceReference {
                method: Some("propfind".to_string()),
                path: Some("/pets/{petId}".to_string()),
                ..reference.clone()
            },
            "openapi: 3.0.3\npaths:\n  /pets/{petId}:\n    get:\n      responses:\n        '200':\n          description: ok\n",
            &mut issues,
        ));
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-trace-openapi-001")
        );

        issues.clear();
        assert!(!validate_openapi_trace_reference(
            "FEAT-API-001",
            TraceRole::FeatureImplementation,
            &display_path,
            &TraceReference {
                method: Some("get".to_string()),
                path: Some("   ".to_string()),
                ..reference.clone()
            },
            "openapi: 3.0.3\n",
            &mut issues,
        ));
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-trace-openapi-001")
        );

        issues.clear();
        assert!(!validate_openapi_trace_reference(
            "FEAT-API-001",
            TraceRole::FeatureImplementation,
            &display_path,
            &reference,
            "{",
            &mut issues,
        ));
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-trace-openapi-002")
        );

        issues.clear();
        assert!(!validate_openapi_trace_reference(
            "FEAT-API-001",
            TraceRole::FeatureImplementation,
            &display_path,
            &reference,
            "paths:\n  /pets/{petId}:\n    get:\n      responses:\n        '200':\n          description: ok\n",
            &mut issues,
        ));
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-trace-openapi-002")
        );

        issues.clear();
        assert!(!validate_openapi_trace_reference(
            "FEAT-API-001",
            TraceRole::FeatureImplementation,
            &display_path,
            &reference,
            "openapi: 3.0.3",
            &mut issues,
        ));
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-trace-openapi-002")
        );

        issues.clear();
        assert!(!validate_openapi_trace_reference(
            "FEAT-API-001",
            TraceRole::FeatureImplementation,
            &display_path,
            &reference,
            "openapi: 3.0.3\npaths:\n  /pets/{petId}: not-a-map\n",
            &mut issues,
        ));
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-trace-openapi-002")
        );

        issues.clear();
        assert!(!validate_openapi_trace_reference(
            "FEAT-API-001",
            TraceRole::FeatureImplementation,
            &display_path,
            &reference,
            "openapi: 3.0.3\npaths:\n  /pets/{petId}:\n    get: not-an-operation\n",
            &mut issues,
        ));
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-trace-openapi-002")
        );

        issues.clear();
        assert!(!validate_openapi_trace_reference(
            "FEAT-API-001",
            TraceRole::FeatureImplementation,
            &display_path,
            &reference,
            "openapi: 3.0.3\npaths:\n  /pets/{petId}:\n    post:\n      responses:\n        '200':\n          description: ok\n",
            &mut issues,
        ));
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-trace-openapi-002")
        );

        issues.clear();
        assert!(!validate_openapi_trace_reference(
            "FEAT-API-001",
            TraceRole::FeatureImplementation,
            &display_path,
            &reference,
            "openapi: 3.0.3\npaths:\n  /other:\n    get:\n      responses:\n        '200':\n          description: ok\n",
            &mut issues,
        ));
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-trace-openapi-002")
        );

        issues.clear();
        assert!(validate_openapi_trace_reference(
            "FEAT-API-001",
            TraceRole::FeatureImplementation,
            &display_path,
            &openapi_reference,
            "openapi: 3.0.3\npaths:\n  /pets/{petId}:\n    $ref: '#/components/pathItems/Pets'\ncomponents:\n  pathItems:\n    Pets:\n      get:\n        responses:\n          '200':\n            description: ok\n",
            &mut issues,
        ));
        assert!(issues.is_empty(), "resolved path-item refs should validate");
    }

    #[test]
    fn validate_openapi_trace_reference_rejects_non_mapping_documents() {
        let tempdir = tempdir().expect("tempdir should exist");
        let display_path = tempdir.path().join("api/openapi.yaml");
        fs::create_dir_all(display_path.parent().expect("api dir should exist")).expect("api dir");

        let reference = TraceReference {
            file: PathBuf::from("api/openapi.yaml"),
            symbols: Vec::new(),
            doc_contains: Vec::new(),
            method: Some("get".to_string()),
            path: Some("/pets/{petId}".to_string()),
        };

        let mut issues = Vec::new();
        assert!(!validate_openapi_trace_reference(
            "FEAT-API-001",
            TraceRole::FeatureImplementation,
            &display_path,
            &reference,
            "[]",
            &mut issues,
        ));
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SYU-trace-openapi-002")
        );
    }

    #[test]
    fn resolve_openapi_path_item_handles_nested_refs_and_invalid_targets() {
        let document: serde_yaml::Value = serde_yaml::from_str(
            "openapi: 3.0.3\ncomponents:\n  pathItems:\n    Loop:\n      $ref: '#/components/pathItems/Loop'\n    Valid:\n      get:\n        responses:\n          '200':\n            description: ok\n  sequences:\n    - first\n    - second\n  scalar: hello\n",
        )
        .expect("document should parse");

        let document_map = document.as_mapping().expect("document should be a mapping");
        let components = document_map
            .get(serde_yaml::Value::String("components".to_string()))
            .and_then(serde_yaml::Value::as_mapping)
            .expect("components should be a mapping");
        let path_items = components
            .get(serde_yaml::Value::String("pathItems".to_string()))
            .and_then(serde_yaml::Value::as_mapping)
            .expect("pathItems should be a mapping");

        let valid = path_items
            .get(serde_yaml::Value::String("Valid".to_string()))
            .expect("valid path item should exist");
        let loop_ref = path_items
            .get(serde_yaml::Value::String("Loop".to_string()))
            .expect("loop path item should exist");

        assert!(resolve_openapi_path_item(&document, valid).is_some());
        assert!(resolve_openapi_path_item(&document, loop_ref).is_none());

        let sequences = components
            .get(serde_yaml::Value::String("sequences".to_string()))
            .expect("sequence container should exist");
        let sequence_ref =
            serde_yaml::from_str::<serde_yaml::Value>("$ref: '#/components/sequences/1'\n")
                .expect("sequence ref should parse");
        assert!(resolve_openapi_path_item(&document, &sequence_ref).is_some());
        assert!(
            resolve_openapi_path_item(
                &document,
                &serde_yaml::from_str::<serde_yaml::Value>(
                    "$ref: 'https://example.com/spec.yaml'\n"
                )
                .expect("external ref should parse"),
            )
            .is_none()
        );
        assert!(
            resolve_openapi_path_item(
                &document,
                &serde_yaml::from_str::<serde_yaml::Value>("$ref: '#/components/scalar/0'\n")
                    .expect("scalar ref should parse"),
            )
            .is_none()
        );
        assert!(sequences.is_sequence());
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
        validate_philosophy(&entry, &HashMap::new(), &SyuConfig::default(), &mut issues);

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
        validate_philosophy(&entry, &HashMap::new(), &SyuConfig::default(), &mut issues);
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
        validate_philosophy(&entry, &policy_map, &SyuConfig::default(), &mut issues);
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
        validate_policy(
            &entry,
            &philosophies,
            &requirements,
            &SyuConfig::default(),
            &mut issues,
        );

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
        validate_policy(
            &entry,
            &HashMap::new(),
            &HashMap::new(),
            &SyuConfig::default(),
            &mut issues,
        );
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
        validate_policy(
            &entry,
            &HashMap::new(),
            &HashMap::new(),
            &SyuConfig::default(),
            &mut issues,
        );

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
            requirement_validation_index(&HashMap::new(), &HashMap::new()),
            validation_resources(&SyuConfig::default()),
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
            requirement_validation_index(&policies, &features),
            validation_resources(&SyuConfig::default()),
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
            validation_resources(&SyuConfig::default()),
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
                method: None,
                path: None,
            }],
        );

        let mut issues = Vec::new();
        let mut trace_count = Default::default();
        validate_requirement(
            &entry,
            requirement_validation_index(&HashMap::new(), &HashMap::new()),
            validation_resources(&SyuConfig::default()),
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
            requirement_validation_index(&HashMap::new(), &HashMap::new()),
            validation_resources(&SyuConfig::default()),
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
            requirement_validation_index(&HashMap::new(), &features),
            validation_resources(&SyuConfig::default()),
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
            requirement_validation_index(&HashMap::new(), &features),
            validation_resources(&SyuConfig::default()),
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
            "swift".to_string(),
            vec![TraceReference {
                file: PathBuf::from("Trace.swift"),
                symbols: vec!["trace".to_string()],
                doc_contains: Vec::new(),
                method: None,
                path: None,
            }],
        );

        let mut issues = Vec::new();
        let mut trace_count = Default::default();
        validate_requirement(
            &entry,
            requirement_validation_index(&HashMap::new(), &HashMap::new()),
            validation_resources(&SyuConfig::default()),
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
            validation_resources(&config),
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
            validation_resources(&SyuConfig::default()),
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
            validation_resources(&SyuConfig::default()),
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
            validation_resources(&SyuConfig::default()),
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
            validation_resources(&SyuConfig::default()),
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
            file: PathBuf::from("Trace.swift"),
            symbols: vec!["main".to_string()],
            doc_contains: Vec::new(),
            method: None,
            path: None,
        };
        let mut issues = Vec::new();
        assert!(!verify_trace_reference(
            Path::new("."),
            &SyuConfig::default(),
            "REQ-1",
            TraceRole::RequirementTest,
            "swift",
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
            method: None,
            path: None,
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
            method: None,
            path: None,
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
    fn verify_trace_reference_accepts_valid_openapi_yaml_operation() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("api/openapi.yaml");
        fs::create_dir_all(path.parent().expect("api dir should exist")).expect("api dir");
        fs::write(
            &path,
            "openapi: 3.0.3\npaths:\n  /pets/{petId}:\n    get:\n      responses:\n        '200':\n          description: ok\n",
        )
        .expect("openapi yaml should exist");

        let reference = TraceReference {
            file: PathBuf::from("api/openapi.yaml"),
            symbols: Vec::new(),
            doc_contains: Vec::new(),
            method: Some("get".to_string()),
            path: Some("/pets/{petId}".to_string()),
        };
        let mut issues = Vec::new();
        assert!(verify_trace_reference(
            tempdir.path(),
            &SyuConfig::default(),
            "FEAT-API-001",
            TraceRole::FeatureImplementation,
            "openapi",
            &reference,
            &mut issues,
        ));
        assert!(issues.is_empty(), "issues: {issues:#?}");
    }

    #[test]
    fn verify_trace_reference_accepts_valid_openapi_json_operation() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("api/openapi.json");
        fs::create_dir_all(path.parent().expect("api dir should exist")).expect("api dir");
        fs::write(
            &path,
            "{\n  \"openapi\": \"3.0.3\",\n  \"paths\": {\n    \"/pets/{petId}\": {\n      \"post\": {\n        \"responses\": {\n          \"200\": {\n            \"description\": \"ok\"\n          }\n        }\n      }\n    }\n  }\n}\n",
        )
        .expect("openapi json should exist");

        let reference = TraceReference {
            file: PathBuf::from("api/openapi.json"),
            symbols: Vec::new(),
            doc_contains: Vec::new(),
            method: Some("post".to_string()),
            path: Some("/pets/{petId}".to_string()),
        };
        let mut issues = Vec::new();
        assert!(verify_trace_reference(
            tempdir.path(),
            &SyuConfig::default(),
            "FEAT-API-001",
            TraceRole::FeatureImplementation,
            "openapi",
            &reference,
            &mut issues,
        ));
        assert!(issues.is_empty(), "issues: {issues:#?}");
    }

    #[test]
    fn verify_trace_reference_reports_missing_openapi_method_or_path() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("api/openapi.yaml");
        fs::create_dir_all(path.parent().expect("api dir should exist")).expect("api dir");
        fs::write(
            &path,
            "openapi: 3.0.3\npaths:\n  /pets/{petId}:\n    get:\n      responses:\n        '200':\n          description: ok\n",
        )
        .expect("openapi yaml should exist");

        let missing_method = TraceReference {
            file: PathBuf::from("api/openapi.yaml"),
            symbols: Vec::new(),
            doc_contains: Vec::new(),
            method: None,
            path: Some("/pets/{petId}".to_string()),
        };
        let missing_path = TraceReference {
            file: PathBuf::from("api/openapi.yaml"),
            symbols: Vec::new(),
            doc_contains: Vec::new(),
            method: Some("get".to_string()),
            path: None,
        };
        let mut issues = Vec::new();

        assert!(!verify_trace_reference(
            tempdir.path(),
            &SyuConfig::default(),
            "FEAT-API-001",
            TraceRole::FeatureImplementation,
            "openapi",
            &missing_method,
            &mut issues,
        ));
        assert!(!verify_trace_reference(
            tempdir.path(),
            &SyuConfig::default(),
            "FEAT-API-001",
            TraceRole::FeatureImplementation,
            "openapi",
            &missing_path,
            &mut issues,
        ));

        assert_eq!(
            issues
                .iter()
                .filter(|issue| issue.code == "SYU-trace-openapi-001")
                .count(),
            2
        );
    }

    #[test]
    fn verify_trace_reference_warns_for_non_canonical_relative_paths() {
        let tempdir = tempdir().expect("tempdir should exist");
        let source = tempdir.path().join("src");
        fs::create_dir_all(&source).expect("src dir should exist");
        fs::write(source.join("trace.rs"), "/// REQ-1\npub fn expected() {}\n")
            .expect("trace file should exist");

        let reference = TraceReference {
            file: PathBuf::from("./src/../src/trace.rs"),
            symbols: vec!["expected".to_string()],
            doc_contains: Vec::new(),
            method: None,
            path: None,
        };
        let mut issues = Vec::new();
        assert!(verify_trace_reference(
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
                .any(|issue| issue.code == "SYU-trace-file-003")
        );
        assert!(
            issues
                .iter()
                .all(|issue| issue.code != "SYU-trace-file-002")
        );
    }

    #[test]
    fn verify_trace_reference_warns_for_backslash_path_spellings() {
        let tempdir = tempdir().expect("tempdir should exist");
        let source = tempdir.path().join("src");
        fs::create_dir_all(&source).expect("src dir should exist");
        fs::write(source.join("trace.rs"), "/// REQ-1\npub fn expected() {}\n")
            .expect("trace file should exist");

        let reference = TraceReference {
            file: PathBuf::from(r"src\trace.rs"),
            symbols: vec!["expected".to_string()],
            doc_contains: Vec::new(),
            method: None,
            path: None,
        };
        let mut issues = Vec::new();
        assert!(verify_trace_reference(
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
                .any(|issue| issue.code == "SYU-trace-file-003")
        );
    }

    #[test]
    fn preferred_trace_file_path_rejects_paths_that_escape_the_workspace() {
        assert!(preferred_trace_file_path(Path::new("../trace.rs")).is_none());
    }

    #[test]
    fn verify_trace_reference_reports_extension_and_blank_symbol_errors() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("trace.txt");
        fs::write(&path, "fn unrelated() {}\n").expect("file should exist");

        let reference = TraceReference {
            file: PathBuf::from("trace.txt"),
            symbols: vec![String::new()],
            doc_contains: Vec::new(),
            method: None,
            path: None,
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
        assert!(
            issues
                .iter()
                .filter(|issue| issue.code == "SYU-trace-symbol-002")
                .count()
                >= 1
        );
    }

    #[test]
    fn trace_role_labels_cover_both_variants() {
        assert_eq!(TraceRole::RequirementTest.label(), "test");
        assert_eq!(TraceRole::FeatureImplementation.label(), "implementation");
    }

    #[test]
    fn verify_trace_reference_accepts_files_without_owner_id_mentions() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("trace.rs");
        fs::write(&path, "fn expected_symbol() {}\n").expect("file should exist");

        let reference = TraceReference {
            file: PathBuf::from("trace.rs"),
            symbols: vec!["expected_symbol".to_string()],
            doc_contains: Vec::new(),
            method: None,
            path: None,
        };
        let mut issues = Vec::new();
        assert!(verify_trace_reference(
            tempdir.path(),
            &SyuConfig::default(),
            "REQ-1",
            TraceRole::RequirementTest,
            "rust",
            &reference,
            &mut issues,
        ));
        assert!(issues.is_empty(), "issues: {issues:#?}");
    }

    #[test]
    fn verify_trace_reference_reports_missing_owner_id_in_inline_mode() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("trace.rs");
        fs::write(&path, "fn expected_symbol() {}\n").expect("file should exist");

        let reference = TraceReference {
            file: PathBuf::from("trace.rs"),
            symbols: vec!["expected_symbol".to_string()],
            doc_contains: Vec::new(),
            method: None,
            path: None,
        };
        let mut config = SyuConfig::default();
        config.validate.trace_ownership_mode = TraceOwnershipMode::Inline;
        let mut issues = Vec::new();
        assert!(!verify_trace_reference(
            tempdir.path(),
            &config,
            "REQ-1",
            TraceRole::RequirementTest,
            "rust",
            &reference,
            &mut issues,
        ));
        let issue = issues
            .iter()
            .find(|issue| issue.code == "SYU-trace-id-001")
            .expect("inline ownership issue");
        assert!(issue.message.contains("does not mention `REQ-1`"));
        assert_eq!(
            issue.suggestion.as_deref(),
            Some("Add `REQ-1` to `trace.rs` so the test remains explicitly traceable.")
        );
    }

    #[test]
    fn verify_trace_reference_accepts_owner_id_in_inline_mode() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("trace.rs");
        fs::write(&path, "// REQ-1\nfn expected_symbol() {}\n").expect("file should exist");

        let reference = TraceReference {
            file: PathBuf::from("trace.rs"),
            symbols: vec!["expected_symbol".to_string()],
            doc_contains: Vec::new(),
            method: None,
            path: None,
        };
        let mut config = SyuConfig::default();
        config.validate.trace_ownership_mode = TraceOwnershipMode::Inline;
        let mut issues = Vec::new();
        assert!(verify_trace_reference(
            tempdir.path(),
            &config,
            "REQ-1",
            TraceRole::RequirementTest,
            "rust",
            &reference,
            &mut issues,
        ));
        assert!(issues.is_empty(), "issues: {issues:#?}");
    }

    #[test]
    fn verify_trace_reference_skips_inline_ownership_for_ignored_generated_paths() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("app/dist/assets/generated.js");
        fs::create_dir_all(path.parent().expect("generated dir")).expect("generated dir");
        fs::write(
            &path,
            "export function generatedBundle() { return true; }\n",
        )
        .expect("generated file should exist");

        let reference = TraceReference {
            file: PathBuf::from("app/dist/assets/generated.js"),
            symbols: vec!["generatedBundle".to_string()],
            doc_contains: Vec::new(),
            method: None,
            path: None,
        };
        let mut config = SyuConfig::default();
        config.validate.trace_ownership_mode = TraceOwnershipMode::Inline;
        let mut issues = Vec::new();

        assert!(verify_trace_reference(
            tempdir.path(),
            &config,
            "FEAT-1",
            TraceRole::FeatureImplementation,
            "typescript",
            &reference,
            &mut issues,
        ));
        assert!(
            issues.iter().all(|issue| issue.code != "SYU-trace-id-001"),
            "issues: {issues:#?}"
        );
    }

    #[test]
    fn verify_trace_reference_can_opt_generated_paths_back_into_inline_ownership() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("app/dist/assets/generated.js");
        fs::create_dir_all(path.parent().expect("generated dir")).expect("generated dir");
        fs::write(
            &path,
            "export function generatedBundle() { return true; }\n",
        )
        .expect("generated file should exist");

        let reference = TraceReference {
            file: PathBuf::from("app/dist/assets/generated.js"),
            symbols: vec!["generatedBundle".to_string()],
            doc_contains: Vec::new(),
            method: None,
            path: None,
        };
        let mut config = SyuConfig::default();
        config.validate.trace_ownership_mode = TraceOwnershipMode::Inline;
        config.validate.symbol_trace_coverage_ignored_paths.clear();
        let mut issues = Vec::new();

        assert!(!verify_trace_reference(
            tempdir.path(),
            &config,
            "FEAT-1",
            TraceRole::FeatureImplementation,
            "typescript",
            &reference,
            &mut issues,
        ));
        assert!(
            issues.iter().any(|issue| issue.code == "SYU-trace-id-001"),
            "issues: {issues:#?}"
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
            method: None,
            path: None,
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
            method: None,
            path: None,
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
            method: None,
            path: None,
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
    fn verify_trace_reference_accepts_valid_java_files() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("TraceService.java");
        fs::write(
            &path,
            "// FEAT-TRACE-004\npublic class TraceService {\n    public void featureTraceJava() {}\n}\n",
        )
        .expect("file should exist");

        let reference = TraceReference {
            file: PathBuf::from("TraceService.java"),
            symbols: vec!["featureTraceJava".to_string()],
            doc_contains: Vec::new(),
            method: None,
            path: None,
        };
        let mut issues = Vec::new();
        assert!(verify_trace_reference(
            tempdir.path(),
            &SyuConfig::default(),
            "FEAT-TRACE-004",
            TraceRole::FeatureImplementation,
            "java",
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
            method: None,
            path: None,
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
    fn verify_trace_reference_accepts_sidecar_ownership_manifests() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("trace.rs");
        fs::write(&path, "pub fn expected() {}\n").expect("file should exist");
        fs::write(
            tempdir.path().join("trace.rs.syu-ownership.yaml"),
            "version: 1\nowners:\n  - id: REQ-1\n    symbols:\n      - expected\n",
        )
        .expect("ownership manifest should exist");

        let reference = TraceReference {
            file: PathBuf::from("trace.rs"),
            symbols: vec!["expected".to_string()],
            doc_contains: Vec::new(),
            method: None,
            path: None,
        };
        let mut config = SyuConfig::default();
        config.validate.trace_ownership_mode = TraceOwnershipMode::Sidecar;
        let mut issues = Vec::new();

        assert!(verify_trace_reference(
            tempdir.path(),
            &config,
            "REQ-1",
            TraceRole::RequirementTest,
            "rust",
            &reference,
            &mut issues,
        ));
        assert!(issues.is_empty());
    }

    #[test]
    fn verify_trace_reference_skips_sidecar_ownership_for_ignored_generated_paths() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("app/dist/assets/generated.js");
        fs::create_dir_all(path.parent().expect("generated dir")).expect("generated dir");
        fs::write(
            &path,
            "export function generatedBundle() { return true; }\n",
        )
        .expect("generated file should exist");

        let mut config = SyuConfig::default();
        config.validate.trace_ownership_mode = TraceOwnershipMode::Sidecar;
        let mut issues = Vec::new();

        assert!(verify_trace_reference(
            tempdir.path(),
            &config,
            "FEAT-1",
            TraceRole::FeatureImplementation,
            "typescript",
            &TraceReference {
                file: PathBuf::from("app/dist/assets/generated.js"),
                symbols: vec!["generatedBundle".to_string()],
                doc_contains: Vec::new(),
                method: None,
                path: None,
            },
            &mut issues,
        ));
        assert!(
            issues.iter().all(|issue| issue.code != "SYU-trace-id-001"),
            "issues: {issues:#?}"
        );
    }

    #[test]
    fn sidecar_ownership_helpers_cover_empty_and_wildcard_symbols() {
        let wildcard_entry = OwnershipEntry {
            id: "REQ-1".to_string(),
            symbols: vec!["*".to_string()],
        };
        assert!(entry_covers_symbols(
            &wildcard_entry,
            &["expected".to_string()]
        ));

        let wildcard_reference = TraceReference {
            file: PathBuf::from("trace.rs"),
            symbols: vec!["*".to_string()],
            doc_contains: Vec::new(),
            method: None,
            path: None,
        };
        assert_eq!(required_ownership_symbols(&wildcard_reference), vec!["*"]);

        let empty_reference = TraceReference {
            file: PathBuf::from("trace.rs"),
            symbols: Vec::new(),
            doc_contains: Vec::new(),
            method: None,
            path: None,
        };
        assert_eq!(
            ownership_symbols_hint(&empty_reference),
            "the traced symbols"
        );

        let mut manifest = OwnershipManifest {
            version: 1,
            owners: vec![wildcard_entry],
        };
        assert!(!merge_ownership_entry(
            &mut manifest,
            "REQ-1",
            &empty_reference
        ));
        assert!(!merge_ownership_entry(
            &mut manifest,
            "REQ-1",
            &TraceReference {
                file: PathBuf::from("trace.rs"),
                symbols: vec!["expected".to_string()],
                doc_contains: Vec::new(),
                method: None,
                path: None,
            }
        ));
    }

    #[test]
    fn merge_ownership_entry_updates_existing_sidecar_entries() {
        let mut manifest = OwnershipManifest {
            version: 1,
            owners: vec![
                OwnershipEntry {
                    id: "REQ-2".to_string(),
                    symbols: vec!["covered".to_string()],
                },
                OwnershipEntry {
                    id: "REQ-1".to_string(),
                    symbols: vec!["covered".to_string()],
                },
            ],
        };

        assert!(merge_ownership_entry(
            &mut manifest,
            "REQ-1",
            &TraceReference {
                file: PathBuf::from("trace.rs"),
                symbols: vec!["added".to_string()],
                doc_contains: Vec::new(),
                method: None,
                path: None,
            },
        ));
        assert_eq!(manifest.owners[0].id, "REQ-1");
        assert_eq!(
            manifest.owners[0].symbols,
            vec!["added".to_string(), "covered".to_string()]
        );

        assert!(merge_ownership_entry(
            &mut manifest,
            "REQ-2",
            &TraceReference {
                file: PathBuf::from("trace.rs"),
                symbols: vec!["*".to_string()],
                doc_contains: Vec::new(),
                method: None,
                path: None,
            },
        ));
        assert_eq!(manifest.owners[1].symbols, vec!["*".to_string()]);
    }

    #[test]
    fn merge_ownership_entry_normalizes_duplicate_sidecar_symbols() {
        let mut manifest = OwnershipManifest {
            version: 1,
            owners: vec![OwnershipEntry {
                id: "REQ-1".to_string(),
                symbols: vec![
                    "covered".to_string(),
                    "covered".to_string(),
                    " extra ".to_string(),
                ],
            }],
        };

        assert!(merge_ownership_entry(
            &mut manifest,
            "REQ-1",
            &TraceReference {
                file: PathBuf::from("trace.rs"),
                symbols: vec!["covered".to_string()],
                doc_contains: Vec::new(),
                method: None,
                path: None,
            },
        ));
        assert_eq!(
            manifest.owners[0].symbols,
            vec!["covered".to_string(), "extra".to_string()]
        );
    }

    #[test]
    fn verify_trace_reference_reports_missing_owner_in_sidecar_manifest() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("trace.rs");
        fs::write(&path, "pub fn expected() {}\n").expect("file should exist");
        fs::write(
            tempdir.path().join("trace.rs.syu-ownership.yaml"),
            "version: 1\nowners:\n  - id: REQ-OTHER\n    symbols:\n      - expected\n",
        )
        .expect("ownership manifest should exist");

        let mut config = SyuConfig::default();
        config.validate.trace_ownership_mode = TraceOwnershipMode::Sidecar;
        let mut issues = Vec::new();

        assert!(!verify_trace_reference(
            tempdir.path(),
            &config,
            "REQ-1",
            TraceRole::RequirementTest,
            "rust",
            &TraceReference {
                file: PathBuf::from("trace.rs"),
                symbols: vec!["expected".to_string()],
                doc_contains: Vec::new(),
                method: None,
                path: None,
            },
            &mut issues,
        ));
        let issue = issues
            .iter()
            .find(|issue| issue.code == "SYU-trace-id-001")
            .expect("sidecar ownership issue");
        assert!(issue.message.contains("does not declare ownership"));
        assert!(issue.message.contains("trace.rs.syu-ownership.yaml"));
    }

    #[test]
    fn verify_trace_reference_reports_invalid_sidecar_manifests() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("trace.rs");
        fs::write(&path, "pub fn expected() {}\n").expect("file should exist");
        fs::write(
            tempdir.path().join("trace.rs.syu-ownership.yaml"),
            "version: 2\nowners: []\n",
        )
        .expect("ownership manifest should exist");

        let mut config = SyuConfig::default();
        config.validate.trace_ownership_mode = TraceOwnershipMode::Sidecar;
        let mut issues = Vec::new();

        assert!(!verify_trace_reference(
            tempdir.path(),
            &config,
            "REQ-1",
            TraceRole::RequirementTest,
            "rust",
            &TraceReference {
                file: PathBuf::from("trace.rs"),
                symbols: vec!["expected".to_string()],
                doc_contains: Vec::new(),
                method: None,
                path: None,
            },
            &mut issues,
        ));
        let issue = issues
            .iter()
            .find(|issue| issue.code == "SYU-trace-id-001")
            .expect("invalid sidecar issue");
        assert!(issue.message.contains("is invalid"));
        assert!(
            issue
                .message
                .contains("unsupported ownership manifest version `2`")
        );
        assert_eq!(
            issue.suggestion.as_deref(),
            Some(
                "Fix `trace.rs.syu-ownership.yaml` or switch `validate.trace_ownership_mode` to `mapping` or `inline`."
            )
        );
    }

    #[test]
    fn verify_trace_reference_reports_duplicate_owner_entries_in_sidecar_manifests() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("trace.rs");
        fs::write(&path, "pub fn expected() {}\n").expect("file should exist");
        fs::write(
            tempdir.path().join("trace.rs.syu-ownership.yaml"),
            "version: 1\nowners:\n  - id: REQ-1\n    symbols:\n      - expected\n  - id: REQ-1\n    symbols:\n      - other\n",
        )
        .expect("ownership manifest should exist");

        let mut config = SyuConfig::default();
        config.validate.trace_ownership_mode = TraceOwnershipMode::Sidecar;
        let mut issues = Vec::new();

        assert!(!verify_trace_reference(
            tempdir.path(),
            &config,
            "REQ-1",
            TraceRole::RequirementTest,
            "rust",
            &TraceReference {
                file: PathBuf::from("trace.rs"),
                symbols: vec!["expected".to_string()],
                doc_contains: Vec::new(),
                method: None,
                path: None,
            },
            &mut issues,
        ));
        let issue = issues
            .iter()
            .find(|issue| issue.code == "SYU-trace-id-001")
            .expect("duplicate sidecar issue");
        assert!(issue.message.contains("is invalid"));
        assert!(issue.message.contains("duplicate ownership entry `REQ-1`"));
    }

    #[test]
    fn apply_autofix_for_reference_leaves_existing_sidecar_entries_unchanged() {
        let tempdir = tempdir().expect("tempdir should exist");
        let root = tempdir.path();
        fs::write(root.join("trace.rs"), "pub fn expected() {}\n")
            .expect("trace file should exist");
        fs::write(
            root.join("trace.rs.syu-ownership.yaml"),
            "version: 1\nowners:\n  - id: REQ-1\n    symbols:\n      - expected\n",
        )
        .expect("ownership manifest should exist");

        let mut config = SyuConfig::default();
        config.validate.trace_ownership_mode = TraceOwnershipMode::Sidecar;

        let mut summary = super::AutofixSummary::default();
        apply_autofix_for_reference(
            root,
            &config,
            "REQ-1",
            "rust",
            &TraceReference {
                file: PathBuf::from("trace.rs"),
                symbols: vec!["expected".to_string()],
                doc_contains: Vec::new(),
                method: None,
                path: None,
            },
            &mut summary,
        )
        .expect("existing sidecar manifest should remain valid");

        assert!(summary.updated_files.is_empty());
        assert_eq!(summary.symbol_updates, 0);
    }

    #[test]
    fn apply_autofix_for_trace_map_dedupes_normalized_references() {
        let tempdir = tempdir().expect("tempdir should exist");
        let root = tempdir.path();
        fs::write(root.join("trace.rs"), "pub fn expected() {}\n")
            .expect("trace file should exist");

        let mut config = SyuConfig::default();
        config.validate.trace_ownership_mode = TraceOwnershipMode::Inline;

        let mut references_by_language = BTreeMap::new();
        references_by_language.insert(
            "rust".to_string(),
            vec![
                TraceReference {
                    file: PathBuf::from("trace.rs"),
                    symbols: vec!["expected".to_string()],
                    doc_contains: Vec::new(),
                    method: None,
                    path: None,
                },
                TraceReference {
                    file: PathBuf::from("./trace.rs"),
                    symbols: vec!["expected".to_string()],
                    doc_contains: Vec::new(),
                    method: None,
                    path: None,
                },
            ],
        );

        let mut run = super::AutofixRun::default();
        let mut transaction = super::AutofixTransaction::default();
        super::apply_autofix_for_trace_map_with_transaction(
            root,
            &config,
            "REQ-1",
            &references_by_language,
            &mut run,
            super::AutofixMode::Apply,
            &mut transaction,
        )
        .expect("normalized references should be processed once");

        assert_eq!(run.summary.symbol_updates, 1);
        assert_eq!(run.summary.updated_files.len(), 1);
        assert!(
            run.summary
                .updated_files
                .contains(&PathBuf::from("trace.rs"))
        );
    }

    #[test]
    fn apply_autofix_for_reference_reports_invalid_sidecar_manifests() {
        let tempdir = tempdir().expect("tempdir should exist");
        let root = tempdir.path();
        fs::write(root.join("trace.rs"), "pub fn expected() {}\n")
            .expect("trace file should exist");
        fs::write(
            root.join("trace.rs.syu-ownership.yaml"),
            "version: 2\nowners: []\n",
        )
        .expect("ownership manifest should exist");

        let mut config = SyuConfig::default();
        config.validate.trace_ownership_mode = TraceOwnershipMode::Sidecar;

        let mut summary = super::AutofixSummary::default();
        let error = apply_autofix_for_reference(
            root,
            &config,
            "REQ-1",
            "rust",
            &TraceReference {
                file: PathBuf::from("trace.rs"),
                symbols: vec!["expected".to_string()],
                doc_contains: Vec::new(),
                method: None,
                path: None,
            },
            &mut summary,
        )
        .expect_err("invalid sidecar manifests should fail autofix");
        assert!(error.to_string().contains("failed to load"));
        assert!(error.to_string().contains("trace.rs.syu-ownership.yaml"));
    }

    #[test]
    fn apply_autofix_for_reference_reports_duplicate_owner_entries_in_sidecar_manifests() {
        let tempdir = tempdir().expect("tempdir should exist");
        let root = tempdir.path();
        fs::write(root.join("trace.rs"), "pub fn expected() {}\n")
            .expect("trace file should exist");
        fs::write(
            root.join("trace.rs.syu-ownership.yaml"),
            "version: 1\nowners:\n  - id: REQ-1\n    symbols:\n      - expected\n  - id: REQ-1\n    symbols:\n      - other\n",
        )
        .expect("ownership manifest should exist");

        let mut config = SyuConfig::default();
        config.validate.trace_ownership_mode = TraceOwnershipMode::Sidecar;

        let mut summary = super::AutofixSummary::default();
        let error = apply_autofix_for_reference(
            root,
            &config,
            "REQ-1",
            "rust",
            &TraceReference {
                file: PathBuf::from("trace.rs"),
                symbols: vec!["expected".to_string()],
                doc_contains: Vec::new(),
                method: None,
                path: None,
            },
            &mut summary,
        )
        .expect_err("duplicate owner manifests should fail autofix");
        assert!(error.to_string().contains("failed to load"));
        assert!(
            error
                .to_string()
                .contains("duplicate ownership entry `REQ-1`")
        );
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
                method: None,
                path: None,
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
                method: None,
                path: None,
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

        let go_path = tempdir.path().join("trace.go");
        fs::write(&go_path, "// Explain expected\nfunc expected() {}\n")
            .expect("go file should exist");
        let mut go_issues = Vec::new();
        assert!(verify_trace_reference(
            tempdir.path(),
            &SyuConfig::default(),
            "REQ-1",
            TraceRole::RequirementTest,
            "go",
            &TraceReference {
                file: PathBuf::from("trace.go"),
                symbols: vec!["expected".to_string()],
                doc_contains: vec!["Explain expected".to_string()],
                method: None,
                path: None,
            },
            &mut go_issues,
        ));
        assert!(go_issues.is_empty());

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
                method: None,
                path: None,
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
                method: None,
                path: None,
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
            "brainfuck",
            &TraceReference {
                file: PathBuf::from("Trace.kt"),
                symbols: vec!["expected".to_string()],
                doc_contains: vec!["Explain expected".to_string()],
                method: None,
                path: None,
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
                method: None,
                path: None,
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
                method: None,
                path: None,
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
                method: None,
                path: None,
            },
            &mut summary,
        )
        .expect("directories should be ignored");

        let no_update_path = root.join("trace.rs");
        fs::write(&no_update_path, "pub fn expected() {}\n").expect("trace file should exist");
        apply_autofix_for_reference(
            root,
            &SyuConfig::default(),
            "REQ-1",
            "rust",
            &TraceReference {
                file: PathBuf::from("trace.rs"),
                symbols: vec!["expected".to_string()],
                doc_contains: Vec::new(),
                method: None,
                path: None,
            },
            &mut summary,
        )
        .expect("already traced symbols should be left alone");

        assert!(summary.updated_files.is_empty());
        assert_eq!(summary.symbol_updates, 0);
    }

    #[test]
    fn apply_autofix_for_reference_writes_sidecar_ownership_manifest() {
        let tempdir = tempdir().expect("tempdir should exist");
        let root = tempdir.path();
        let source_path = root.join("trace.rs");
        fs::write(&source_path, "pub fn expected() {}\n").expect("trace file should exist");

        let mut config = SyuConfig::default();
        config.validate.trace_ownership_mode = TraceOwnershipMode::Sidecar;

        let mut summary = super::AutofixSummary::default();
        apply_autofix_for_reference(
            root,
            &config,
            "REQ-1",
            "rust",
            &TraceReference {
                file: PathBuf::from("trace.rs"),
                symbols: vec!["expected".to_string()],
                doc_contains: Vec::new(),
                method: None,
                path: None,
            },
            &mut summary,
        )
        .expect("sidecar ownership autofix should succeed");

        let source_contents = fs::read_to_string(&source_path).expect("source contents");
        assert_eq!(source_contents, "pub fn expected() {}\n");

        let manifest_contents =
            fs::read_to_string(root.join("trace.rs.syu-ownership.yaml")).expect("manifest");
        assert!(manifest_contents.contains("version: 1"));
        assert!(manifest_contents.contains("id: REQ-1"));
        assert!(manifest_contents.contains("- expected"));
        assert_eq!(summary.symbol_updates, 1);
        assert!(
            summary
                .updated_files
                .contains(Path::new("trace.rs.syu-ownership.yaml"))
        );
    }

    #[test]
    fn apply_autofix_for_reference_inserts_inline_owner_id_when_configured() {
        let tempdir = tempdir().expect("tempdir should exist");
        let root = tempdir.path();
        let source_path = root.join("trace.rs");
        fs::write(&source_path, "pub fn expected() {}\n").expect("trace file should exist");

        let mut config = SyuConfig::default();
        config.validate.trace_ownership_mode = TraceOwnershipMode::Inline;

        let mut summary = super::AutofixSummary::default();
        apply_autofix_for_reference(
            root,
            &config,
            "REQ-1",
            "rust",
            &TraceReference {
                file: PathBuf::from("trace.rs"),
                symbols: vec!["expected".to_string()],
                doc_contains: Vec::new(),
                method: None,
                path: None,
            },
            &mut summary,
        )
        .expect("inline ownership autofix should succeed");

        let source_contents = fs::read_to_string(&source_path).expect("source contents");
        assert!(source_contents.contains("REQ-1"));
        assert_eq!(summary.symbol_updates, 1);
    }

    #[test]
    fn apply_autofix_for_reference_records_planned_changes() {
        let tempdir = tempdir().expect("tempdir should exist");
        let root = tempdir.path();
        let source_path = root.join("trace.rs");
        fs::write(&source_path, "pub fn expected() {}\n").expect("trace file should exist");

        let mut summary = super::AutofixRun {
            summary: Default::default(),
            plan: Some(Default::default()),
        };
        super::apply_autofix_for_reference(
            root,
            &SyuConfig::default(),
            "REQ-1",
            "rust",
            &TraceReference {
                file: PathBuf::from("trace.rs"),
                symbols: vec!["expected".to_string()],
                doc_contains: vec!["Explain expected".to_string()],
                method: None,
                path: None,
            },
            &mut summary,
            super::AutofixMode::Plan,
        )
        .expect("plan mode should succeed");

        assert_eq!(summary.summary.symbol_updates, 1);
        let plan = summary.plan.as_ref().expect("plan should exist");
        assert_eq!(plan.planned_updates, 1);
        assert!(plan.updated_files.contains(Path::new("trace.rs")));
        assert!(source_path.exists());
        assert_eq!(
            fs::read_to_string(&source_path).expect("source contents"),
            "pub fn expected() {}\n"
        );
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
                method: None,
                path: None,
            }],
        );

        let mut feat = feature("FEAT-1");
        feat.implementations.insert(
            "rust".to_string(),
            vec![TraceReference {
                file: PathBuf::from("feature.rs"),
                symbols: vec!["feature_impl".to_string()],
                doc_contains: vec!["Feature docs".to_string()],
                method: None,
                path: None,
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
        assert!(summary.updated_files.contains(Path::new("requirement.rs")));
        assert!(summary.updated_files.contains(Path::new("feature.rs")));
        let requirement_contents =
            fs::read_to_string(root.join("requirement.rs")).expect("requirement contents");
        assert!(requirement_contents.contains("Requirement docs"));
        assert!(!requirement_contents.contains("REQ-1"));
        let feature_contents =
            fs::read_to_string(root.join("feature.rs")).expect("feature contents");
        assert!(feature_contents.contains("Feature docs"));
        assert!(!feature_contents.contains("FEAT-1"));
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
                method: None,
                path: None,
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
                method: None,
                path: None,
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
    fn apply_autofix_bootstraps_missing_feature_registry() {
        let tempdir = tempdir().expect("tempdir should exist");
        write_valid_planned_workspace(tempdir.path());
        fs::remove_file(tempdir.path().join("docs/syu/features/features.yaml"))
            .expect("feature registry should be removable");

        let workspace =
            crate::workspace::load_workspace_allowing_missing_feature_registry(tempdir.path())
                .expect("workspace should still load");
        let summary = apply_autofix(&workspace).expect("autofix should succeed");

        let registry_contents =
            fs::read_to_string(tempdir.path().join("docs/syu/features/features.yaml"))
                .expect("feature registry should be written");
        assert!(registry_contents.contains("version:"));
        assert!(registry_contents.contains("files:"));
        assert!(registry_contents.contains("core.yaml"));
        assert_eq!(summary.symbol_updates, 1);
        assert!(
            summary
                .updated_files
                .contains(Path::new("docs/syu/features/features.yaml"))
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
                method: None,
                path: None,
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
        assert!(
            error
                .to_string()
                .contains("autofix failed before applying changes")
        );
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
                method: None,
                path: None,
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

        assert!(
            error
                .to_string()
                .contains("autofix failed before applying changes")
        );
    }

    #[cfg(unix)]
    #[test]
    fn autofix_transaction_reports_backup_read_errors() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("trace.rs");
        fs::write(&path, "pub fn expected() {}\n").expect("trace file should exist");

        let mut permissions = fs::metadata(&path).expect("metadata").permissions();
        permissions.set_mode(0o000);
        fs::set_permissions(&path, permissions).expect("permissions should update");

        let mut transaction = AutofixTransaction::default();
        let result = transaction.write(&path, "pub fn updated() {}\n");

        let mut restore = fs::metadata(&path).expect("metadata").permissions();
        restore.set_mode(0o644);
        fs::set_permissions(&path, restore).expect("permissions should restore");

        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn autofix_transaction_rolls_back_deleted_files() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("trace.rs");
        fs::write(&path, "pub fn expected() {}\n").expect("trace file should exist");

        let mut transaction = AutofixTransaction::default();
        transaction.backups.insert(path.clone(), None);
        transaction
            .rollback()
            .expect("rollback should delete newly created file");

        assert!(!path.exists());
    }

    #[test]
    fn autofix_transaction_reports_delete_failures() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("trace.rs");
        fs::write(&path, "pub fn expected() {}\n").expect("trace file should exist");

        let mut transaction = AutofixTransaction::default();
        transaction.backups.insert(path.clone(), None);

        fs::remove_file(&path).expect("file should be removable");
        fs::create_dir(&path).expect("directory should be creatable");

        let error = transaction.rollback().expect_err("rollback should fail");
        assert!(error.downcast_ref::<std::io::Error>().is_some());
    }

    #[cfg(unix)]
    #[test]
    fn finalize_autofix_error_reports_rollback_failures() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("trace.rs");
        fs::write(&path, "pub fn expected() {}\n").expect("trace file should exist");

        let mut permissions = fs::metadata(&path).expect("metadata").permissions();
        permissions.set_mode(0o400);
        fs::set_permissions(&path, permissions).expect("permissions should update");

        let mut transaction = AutofixTransaction::default();
        transaction
            .backups
            .insert(path.clone(), Some("pub fn updated() {}\n".to_string()));
        transaction.writes_committed = 1;

        let error = finalize_autofix_error(&transaction, anyhow::anyhow!("boom"));

        let mut restore = fs::metadata(&path).expect("metadata").permissions();
        restore.set_mode(0o644);
        fs::set_permissions(&path, restore).expect("permissions should restore");

        assert!(
            error
                .to_string()
                .contains("failed to roll back autofix changes after boom")
        );
    }

    #[cfg(unix)]
    #[test]
    fn apply_autofix_rolls_back_prior_writes_on_failure() {
        let tempdir = tempdir().expect("tempdir should exist");
        let root = tempdir.path().to_path_buf();

        let go_path = root.join("first.go");
        fs::write(&go_path, "func first() {}\n").expect("go file should exist");

        let python_path = root.join("second.py");
        fs::write(&python_path, "def second():\n    return 1\n").expect("python file should exist");

        let mut req = requirement("REQ-1");
        req.tests.insert(
            "go".to_string(),
            vec![TraceReference {
                file: PathBuf::from("first.go"),
                symbols: vec!["first".to_string()],
                doc_contains: vec!["First docs".to_string()],
                method: None,
                path: None,
            }],
        );
        req.tests.insert(
            "python".to_string(),
            vec![TraceReference {
                file: PathBuf::from("second.py"),
                symbols: vec!["second".to_string()],
                doc_contains: vec!["Second docs".to_string()],
                method: None,
                path: None,
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

        let error = apply_autofix(&workspace).expect_err("write failure should bubble up");

        let mut go_restore = fs::metadata(&go_path).expect("metadata").permissions();
        go_restore.set_mode(0o644);
        fs::set_permissions(&go_path, go_restore).expect("permissions should restore");

        assert!(
            error
                .to_string()
                .contains("autofix rolled back partial changes")
        );
        assert_eq!(
            fs::read_to_string(&go_path).expect("go contents"),
            "func first() {}\n"
        );
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
                method: None,
                path: None,
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
                method: None,
                path: None,
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
            method: None,
            path: None,
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
            method: None,
            path: None,
        };
        assert_eq!(
            format_reference_location("rust", &reference),
            "rust:src/lib.rs"
        );
    }

    #[test]
    fn graph_autofix_skips_missing_targets_without_changes() {
        let philosophies = vec![MutableLoadedDocument {
            path: PathBuf::from("philosophy.yaml"),
            document: PhilosophyDocument {
                category: "Philosophy".to_string(),
                version: 1,
                language: Some("en".to_string()),
                philosophies: vec![Philosophy {
                    id: "PHIL-1".to_string(),
                    title: "Title".to_string(),
                    product_design_principle: "Principle".to_string(),
                    coding_guideline: "Guideline".to_string(),
                    linked_policies: vec!["POL-missing".to_string()],
                }],
            },
            changed: false,
        }];
        let mut policies = vec![MutableLoadedDocument {
            path: PathBuf::from("policy.yaml"),
            document: PolicyDocument {
                category: "Policies".to_string(),
                version: 1,
                language: Some("en".to_string()),
                policies: vec![Policy {
                    id: "POL-1".to_string(),
                    title: "Title".to_string(),
                    summary: "Summary".to_string(),
                    description: "Description".to_string(),
                    linked_philosophies: vec!["PHIL-missing".to_string()],
                    linked_requirements: vec!["REQ-missing".to_string()],
                }],
            },
            changed: false,
        }];
        let mut requirements = vec![MutableLoadedDocument {
            path: PathBuf::from("requirement.yaml"),
            document: RequirementDocument {
                category: "Core Requirements".to_string(),
                prefix: "REQ".to_string(),
                requirements: vec![Requirement {
                    id: "REQ-1".to_string(),
                    title: "Title".to_string(),
                    description: "Description".to_string(),
                    priority: "high".to_string(),
                    status: "planned".to_string(),
                    linked_policies: vec!["POL-missing".to_string()],
                    linked_features: vec!["FEAT-missing".to_string()],
                    tests: BTreeMap::new(),
                }],
            },
            changed: false,
        }];
        let mut features = vec![MutableLoadedDocument {
            path: PathBuf::from("feature.yaml"),
            document: FeatureDocument {
                category: "Core Features".to_string(),
                version: 1,
                features: vec![Feature {
                    id: "FEAT-1".to_string(),
                    title: "Title".to_string(),
                    summary: "Summary".to_string(),
                    status: "planned".to_string(),
                    linked_requirements: vec!["REQ-missing".to_string()],
                    implementations: BTreeMap::new(),
                }],
            },
            changed: false,
        }];

        let empty = HashMap::new();
        let mut empty_philosophies: Vec<MutableLoadedDocument<PhilosophyDocument>> = Vec::new();
        apply_philosophy_reciprocals(&philosophies, &mut policies, &empty);
        apply_policy_reciprocals(
            &policies,
            &mut empty_philosophies,
            &empty,
            &mut requirements,
            &empty,
        );
        apply_requirement_reciprocals(&requirements, &mut features, &empty);
        apply_feature_reciprocals(&features, &mut requirements, &empty);

        assert!(!policies[0].changed);
        assert!(!requirements[0].changed);
        assert!(!features[0].changed);
    }

    #[test]
    fn graph_autofix_adds_missing_reciprocals_and_falls_back_for_registry_kind() {
        let mut philosophies = vec![MutableLoadedDocument {
            path: PathBuf::from("philosophy.yaml"),
            document: PhilosophyDocument {
                category: "Philosophy".to_string(),
                version: 1,
                language: Some("en".to_string()),
                philosophies: vec![Philosophy {
                    id: "PHIL-1".to_string(),
                    title: "Title".to_string(),
                    product_design_principle: "Principle".to_string(),
                    coding_guideline: "Guideline".to_string(),
                    linked_policies: Vec::new(),
                }],
            },
            changed: false,
        }];
        let policies = vec![MutableLoadedDocument {
            path: PathBuf::from("policy.yaml"),
            document: PolicyDocument {
                category: "Policies".to_string(),
                version: 1,
                language: Some("en".to_string()),
                policies: vec![Policy {
                    id: "POL-1".to_string(),
                    title: "Title".to_string(),
                    summary: "Summary".to_string(),
                    description: "Description".to_string(),
                    linked_philosophies: vec!["PHIL-1".to_string()],
                    linked_requirements: Vec::new(),
                }],
            },
            changed: false,
        }];
        let mut requirements = vec![MutableLoadedDocument {
            path: PathBuf::from("requirement.yaml"),
            document: RequirementDocument {
                category: "Core Requirements".to_string(),
                prefix: "REQ".to_string(),
                requirements: vec![Requirement {
                    id: "REQ-1".to_string(),
                    title: "Title".to_string(),
                    description: "Description".to_string(),
                    priority: "high".to_string(),
                    status: "planned".to_string(),
                    linked_policies: Vec::new(),
                    linked_features: Vec::new(),
                    tests: BTreeMap::new(),
                }],
            },
            changed: false,
        }];
        let features = vec![MutableLoadedDocument {
            path: PathBuf::from("feature.yaml"),
            document: FeatureDocument {
                category: "Core Features".to_string(),
                version: 1,
                features: vec![Feature {
                    id: "FEAT-1".to_string(),
                    title: "Title".to_string(),
                    summary: "Summary".to_string(),
                    status: "planned".to_string(),
                    linked_requirements: vec!["REQ-1".to_string()],
                    implementations: BTreeMap::new(),
                }],
            },
            changed: false,
        }];

        let philosophy_index = HashMap::from([(
            "PHIL-1".to_string(),
            ItemLocation {
                doc_index: 0,
                item_index: 0,
            },
        )]);
        let requirement_index = HashMap::from([(
            "REQ-1".to_string(),
            ItemLocation {
                doc_index: 0,
                item_index: 0,
            },
        )]);

        apply_policy_reciprocals(
            &policies,
            &mut philosophies,
            &philosophy_index,
            &mut requirements,
            &requirement_index,
        );
        apply_feature_reciprocals(&features, &mut requirements, &requirement_index);

        assert!(philosophies[0].changed);
        assert!(requirements[0].changed);
        assert_eq!(
            philosophies[0].document.philosophies[0].linked_policies,
            vec!["POL-1"]
        );
        assert_eq!(
            requirements[0].document.requirements[0].linked_features,
            vec!["FEAT-1"]
        );
        assert_eq!(
            feature_registry_kind(Path::new("features"), Path::new("features/../core.yaml")),
            "core"
        );
    }
    #[test]
    fn render_autofix_plan_returns_empty_output_for_empty_plan() {
        assert!(render_autofix_plan(&AutofixPlan::default()).is_empty());
    }

    #[test]
    fn render_autofix_plan_labels_changes_without_rule_matches() {
        let mut plan = AutofixPlan::default();
        plan.changes.push(AutofixPlanChange {
            path: PathBuf::from("trace.rs"),
            rules: Vec::new(),
            summary: "update trace.rs".to_string(),
        });

        let rendered = render_autofix_plan(&plan);
        assert!(rendered.contains("no direct rule match"));
        assert!(rendered.contains("trace.rs"));
    }
}
