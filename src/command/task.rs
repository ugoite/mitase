// FEAT-TASK-001
// REQ-CORE-028

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    cli::{OutputFormat, TaskArgs, TaskClassifyArgs, TaskCommands},
    workspace::load_workspace,
};

use super::lookup::{SearchResult, WorkspaceEntity, WorkspaceLookup};

const REQUEST_ARTIFACT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum RequirementAction {
    Create,
    Change,
    Delete,
}

impl RequirementAction {
    const fn label(self) -> &'static str {
        match self {
            Self::Create => "requirement_create",
            Self::Change => "requirement_change",
            Self::Delete => "requirement_delete",
        }
    }
}

#[derive(Debug, Deserialize)]
struct RequestArtifact {
    version: u32,
    request: String,
    #[serde(default)]
    context: RequestArtifactContext,
}

#[derive(Debug, Deserialize, Default)]
struct RequestArtifactContext {
    #[serde(default)]
    affected_area: Option<String>,
    #[serde(default)]
    repository_constraints: Vec<String>,
    #[serde(default)]
    linked_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct JsonTaskClassifyOutput {
    request_path: String,
    request: String,
    classification: String,
    reasons: Vec<String>,
    explicit_items: Vec<SearchResult>,
    related_items: Vec<SearchResult>,
    context: JsonRequestArtifactContext,
}

#[derive(Debug, Serialize)]
struct JsonRequestArtifactContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    affected_area: Option<String>,
    repository_constraints: Vec<String>,
    linked_ids: Vec<String>,
}

#[derive(Debug)]
struct ClassificationOutcome {
    classification: RequirementAction,
    reasons: Vec<String>,
    explicit_items: Vec<SearchResult>,
    related_items: Vec<SearchResult>,
    request: String,
    context: RequestArtifactContext,
}

pub fn run_task_command(args: &TaskArgs) -> Result<i32> {
    match &args.command {
        TaskCommands::Classify(classify) => run_task_classify_command(classify),
    }
}

pub fn run_task_classify_command(args: &TaskClassifyArgs) -> Result<i32> {
    let workspace = load_workspace(&args.workspace)?;
    let request_artifact = load_request_artifact(&args.request)?;
    let outcome = classify_request(&workspace, request_artifact)?;

    match args.format {
        OutputFormat::Text => print_text_output(&args.request, &outcome),
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&JsonTaskClassifyOutput {
                request_path: args.request.display().to_string(),
                request: outcome.request,
                classification: outcome.classification.label().to_string(),
                reasons: outcome.reasons,
                explicit_items: outcome.explicit_items,
                related_items: outcome.related_items,
                context: JsonRequestArtifactContext {
                    affected_area: outcome.context.affected_area,
                    repository_constraints: outcome.context.repository_constraints,
                    linked_ids: outcome.context.linked_ids,
                },
            })
            .expect("serializing task classification output to JSON should succeed")
        ),
    }

    Ok(0)
}

fn load_request_artifact(path: &PathBuf) -> Result<RequestArtifact> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read request artifact `{}`", path.display()))?;
    let artifact: RequestArtifact = serde_yaml::from_str(&raw)
        .with_context(|| format!("failed to parse request artifact `{}`", path.display()))?;
    if artifact.version != REQUEST_ARTIFACT_VERSION {
        bail!(
            "unsupported request artifact version `{}` in `{}`",
            artifact.version,
            path.display()
        );
    }
    Ok(artifact)
}

fn classify_request(
    workspace: &crate::workspace::Workspace,
    artifact: RequestArtifact,
) -> Result<ClassificationOutcome> {
    let lookup = WorkspaceLookup::new(workspace);
    let analysis_text = artifact.analysis_text();
    let lower = analysis_text.to_lowercase();
    let delete_hits = count_keyword_hits(&lower, DELETE_KEYWORDS);
    let change_hits = count_keyword_hits(&lower, CHANGE_KEYWORDS);
    let create_hits = count_keyword_hits(&lower, CREATE_KEYWORDS);

    let explicit_ids = artifact.explicit_ids();
    let explicit_items = collect_explicit_items(&lookup, &explicit_ids);
    let mut related_items = collect_related_items(&lookup, &artifact.request);
    merge_related_items(&mut related_items, lookup.search(&analysis_text, None));
    related_items.truncate(5);

    let mut reasons = Vec::new();
    if delete_hits > 0 {
        reasons.push(format!(
            "request uses delete-oriented language: {}",
            describe_keyword_hits(&lower, DELETE_KEYWORDS)
        ));
    }
    if change_hits > 0 {
        reasons.push(format!(
            "request uses change-oriented language: {}",
            describe_keyword_hits(&lower, CHANGE_KEYWORDS)
        ));
    }
    if create_hits > 0 {
        reasons.push(format!(
            "request uses create-oriented language: {}",
            describe_keyword_hits(&lower, CREATE_KEYWORDS)
        ));
    }
    if !explicit_items.is_empty() {
        reasons.push(format!(
            "request names existing spec items: {}",
            explicit_items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if explicit_items.is_empty() && !related_items.is_empty() {
        reasons.push(format!(
            "closest spec graph matches are {}",
            related_items
                .iter()
                .map(|item| format!("{} {}", item.kind, item.id))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if delete_hits == 0 && change_hits == 0 && create_hits == 0 {
        reasons.push(
            "request does not use a strong create/change/delete verb, so the graph match and linked IDs carry the decision"
                .to_string(),
        );
    }

    let classification = if delete_hits > 0 {
        RequirementAction::Delete
    } else if change_hits > 0 || !explicit_items.is_empty() {
        RequirementAction::Change
    } else {
        RequirementAction::Create
    };

    if matches!(classification, RequirementAction::Create) {
        if create_hits > 0 {
            reasons.push(
                "request uses create-oriented language and does not name an existing spec item"
                    .to_string(),
            );
        } else {
            reasons.push(
                "no existing spec item was named and the request reads like new work".to_string(),
            );
        }
    }

    Ok(ClassificationOutcome {
        classification,
        reasons,
        explicit_items,
        related_items,
        request: artifact.request,
        context: artifact.context,
    })
}

impl RequestArtifact {
    fn analysis_text(&self) -> String {
        let mut text = String::new();
        text.push_str(&self.request);
        if let Some(affected_area) = &self.context.affected_area {
            text.push('\n');
            text.push_str(affected_area);
        }
        for constraint in &self.context.repository_constraints {
            text.push('\n');
            text.push_str(constraint);
        }
        for id in &self.context.linked_ids {
            text.push('\n');
            text.push_str(id);
        }
        text
    }

    fn explicit_ids(&self) -> Vec<String> {
        let mut ids = self.context.linked_ids.clone();
        ids.extend(extract_spec_ids(&self.request));
        if let Some(affected_area) = &self.context.affected_area {
            ids.extend(extract_spec_ids(affected_area));
        }
        ids.sort();
        ids.dedup();
        ids
    }
}

fn collect_explicit_items(lookup: &WorkspaceLookup<'_>, ids: &[String]) -> Vec<SearchResult> {
    let mut items = BTreeMap::<String, SearchResult>::new();
    for id in ids {
        if let Some(item) = lookup.find(id) {
            items.insert(id.clone(), item_to_search_result(item));
        }
    }
    items.into_values().collect()
}

fn collect_related_items(lookup: &WorkspaceLookup<'_>, request: &str) -> Vec<SearchResult> {
    let mut items = BTreeMap::<String, SearchResult>::new();
    for result in lookup.search(request, None) {
        items.insert(result.id.clone(), result);
    }
    items.into_values().collect()
}

fn merge_related_items(related_items: &mut Vec<SearchResult>, additional_items: Vec<SearchResult>) {
    let mut merged = BTreeMap::<String, SearchResult>::new();
    for item in related_items.drain(..) {
        merged.insert(item.id.clone(), item);
    }
    for item in additional_items {
        merged.insert(item.id.clone(), item);
    }
    *related_items = merged.into_values().collect();
}

fn item_to_search_result(item: WorkspaceEntity<'_>) -> SearchResult {
    match item {
        WorkspaceEntity::Philosophy(item) => SearchResult {
            id: item.id.clone(),
            kind: "philosophy",
            title: item.title.clone(),
        },
        WorkspaceEntity::Policy(item) => SearchResult {
            id: item.id.clone(),
            kind: "policy",
            title: item.title.clone(),
        },
        WorkspaceEntity::Requirement(item) => SearchResult {
            id: item.id.clone(),
            kind: "requirement",
            title: item.title.clone(),
        },
        WorkspaceEntity::Feature(item) => SearchResult {
            id: item.id.clone(),
            kind: "feature",
            title: item.title.clone(),
        },
    }
}

fn print_text_output(request_path: &Path, outcome: &ClassificationOutcome) {
    println!("request: {}", request_path.display());
    println!("classification: {}", outcome.classification.label());
    println!();
    println!("request text:");
    println!("{}", outcome.request.trim());
    println!();
    print_items("explicit items", &outcome.explicit_items);
    print_items("related items", &outcome.related_items);
    println!("reasons:");
    if outcome.reasons.is_empty() {
        println!("- none");
    } else {
        for reason in &outcome.reasons {
            println!("- {reason}");
        }
    }
}

fn print_items(heading: &str, items: &[SearchResult]) {
    println!("{heading}:");
    if items.is_empty() {
        println!("- none");
        return;
    }

    for item in items {
        println!("- {}\t{}\t{}", item.id, item.kind, item.title);
    }
}

const DELETE_KEYWORDS: &[&str] = &[
    "delete",
    "remove",
    "drop",
    "retire",
    "deprecate",
    "obsolete",
    "eliminate",
    "no longer valid",
];

const CHANGE_KEYWORDS: &[&str] = &[
    "change", "update", "modify", "refine", "expand", "extend", "revise", "adjust", "replace",
    "rework", "clarify",
];

const CREATE_KEYWORDS: &[&str] = &[
    "create",
    "add",
    "introduce",
    "new",
    "implement",
    "support",
    "build",
];

fn count_keyword_hits(text: &str, keywords: &[&str]) -> usize {
    keywords
        .iter()
        .filter(|keyword| text.contains(**keyword))
        .count()
}

fn describe_keyword_hits(text: &str, keywords: &[&str]) -> String {
    keywords
        .iter()
        .copied()
        .filter(|keyword| text.contains(keyword))
        .collect::<Vec<_>>()
        .join(", ")
}

fn extract_spec_ids(text: &str) -> Vec<String> {
    static SPEC_ID_RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = SPEC_ID_RE.get_or_init(|| {
        Regex::new(r"\b(?:PHIL|POL|REQ|FEAT)-[A-Z0-9][A-Z0-9-]*\b")
            .expect("spec id regex should compile")
    });

    re.find_iter(text).map(|m| m.as_str().to_string()).collect()
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use tempfile::tempdir;

    use crate::cli::{OutputFormat, TaskClassifyArgs, TaskCommands};

    use super::{RequirementAction, classify_request, load_request_artifact, run_task_command};

    fn write_request_artifact(path: &Path, request: &str, linked_ids: &[&str]) {
        let linked_ids_block = if linked_ids.is_empty() {
            "  linked_ids: []\n".to_string()
        } else {
            let list = linked_ids
                .iter()
                .map(|id| format!("    - {id}\n"))
                .collect::<String>();
            format!("  linked_ids:\n{list}")
        };
        fs::write(
            path,
            format!(
                "version: 1\nrequest: >\n  {request}\ncontext:\n  affected_area: core\n  repository_constraints:\n    - keep text and JSON output\n{linked_ids_block}",
            ),
        )
        .expect("request artifact should write");
    }

    fn write_workspace(root: &Path) {
        fs::write(
            root.join("syu.yaml"),
            "version: 1\nspec:\n  root: docs/syu\n",
        )
        .expect("workspace config");
        fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
        fs::create_dir_all(root.join("docs/syu/policies")).expect("policy dir");
        fs::create_dir_all(root.join("docs/syu/requirements/core")).expect("requirements dir");
        fs::create_dir_all(root.join("docs/syu/features/core")).expect("features dir");

        fs::write(
            root.join("docs/syu/philosophy/foundation.yaml"),
            "category: Philosophy\nversion: 1\nlanguage: en\nphilosophies:\n  - id: PHIL-001\n    title: Keep planning explicit\n    product_design_principle: Request artifacts should stay reviewable.\n    coding_guideline: Prefer explicit request classification.\n    linked_policies:\n      - POL-001\n",
        )
        .expect("philosophy doc");
        fs::write(
            root.join("docs/syu/policies/policies.yaml"),
            "category: Policies\nversion: 1\nlanguage: en\npolicies:\n  - id: POL-001\n    title: Keep request workflows visible\n    summary: Keep intake and planning separate.\n    description: Request artifacts should be classified against the current graph.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-CORE-028\n",
        )
        .expect("policy doc");
        fs::write(
            root.join("docs/syu/requirements/core/classify.yaml"),
            "category: Core Workspace\nprefix: REQ-CORE\nrequirements:\n  - id: REQ-CORE-028\n    title: Classify request artifacts into requirement actions\n    description: The task classifier should decide whether a request creates, changes, or deletes a requirement.\n    priority: medium\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-TASK-001\n    tests:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - '*'\n",
        )
        .expect("requirement doc");
        fs::write(
            root.join("docs/syu/features/features.yaml"),
            "version: 1\nupdated: \"2026-05\"\nfiles:\n  - kind: task\n    file: core/task.yaml\n",
        )
        .expect("feature registry");
        fs::write(
            root.join("docs/syu/features/core/task.yaml"),
            "category: Task Planning CLI\nversion: 1\nfeatures:\n  - id: FEAT-TASK-001\n    title: Request artifact classification\n    summary: Classify planned request artifacts into create, change, or delete decisions using the current spec graph and a brief explanation.\n    status: implemented\n    linked_requirements:\n      - REQ-CORE-028\n    implementations:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - run_task_command\n            - run_task_classify_command\n        - file: src/cli.rs\n          symbols:\n            - TaskArgs\n            - TaskClassifyArgs\n",
        )
        .expect("feature doc");
    }

    #[test]
    fn load_request_artifact_rejects_version_mismatch() {
        let tempdir = tempdir().expect("tempdir");
        let request = tempdir.path().join("request.yaml");
        fs::write(
            &request,
            "version: 2\nrequest: Update the requirement\ncontext: {}\n",
        )
        .expect("request");

        let error = load_request_artifact(&request).expect_err("version mismatch should fail");
        assert!(
            error
                .to_string()
                .contains("unsupported request artifact version")
        );
    }

    #[test]
    fn classify_request_prefers_change_for_existing_requirement_ids() {
        let tempdir = tempdir().expect("tempdir");
        write_workspace(tempdir.path());
        let request = tempdir.path().join("request.yaml");
        write_request_artifact(
            &request,
            "Update REQ-CORE-028 so the request classifier stays explainable.",
            &["REQ-CORE-028"],
        );

        let workspace = crate::workspace::load_workspace(tempdir.path()).expect("workspace");
        let artifact = load_request_artifact(&request).expect("request");
        let outcome = classify_request(&workspace, artifact).expect("classification");
        assert_eq!(outcome.classification, RequirementAction::Change);
        assert!(
            outcome
                .reasons
                .iter()
                .any(|reason| reason.contains("REQ-CORE-028"))
        );
    }

    #[test]
    fn classify_request_prefers_create_for_new_requests_without_existing_ids() {
        let tempdir = tempdir().expect("tempdir");
        write_workspace(tempdir.path());
        let request = tempdir.path().join("request.yaml");
        write_request_artifact(
            &request,
            "Create a new request summary for the upcoming planning flow.",
            &[],
        );

        let workspace = crate::workspace::load_workspace(tempdir.path()).expect("workspace");
        let artifact = load_request_artifact(&request).expect("request");
        let outcome = classify_request(&workspace, artifact).expect("classification");
        assert_eq!(outcome.classification, RequirementAction::Create);
        assert!(
            outcome
                .reasons
                .iter()
                .any(|reason| reason.contains("create-oriented language"))
        );
    }

    #[test]
    fn run_task_command_executes_nested_classify_commands() {
        let tempdir = tempdir().expect("tempdir");
        write_workspace(tempdir.path());
        let request = tempdir.path().join("request.yaml");
        write_request_artifact(
            &request,
            "Delete REQ-CORE-028 because the requirement no longer matches the workflow.",
            &["REQ-CORE-028"],
        );

        let code = run_task_command(&crate::cli::TaskArgs {
            command: TaskCommands::Classify(TaskClassifyArgs {
                request: request.clone(),
                workspace: tempdir.path().to_path_buf(),
                format: OutputFormat::Text,
            }),
        })
        .expect("task command should succeed");
        assert_eq!(code, 0);
    }
}
