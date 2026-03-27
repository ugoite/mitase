// FEAT-BROWSE-001
// FEAT-BROWSE-002
// REQ-CORE-015

use std::io::{self, Write};

use anyhow::Result;

use crate::{
    cli::{BrowseArgs, LookupKind as EntityKind},
    command::check::collect_check_result,
    model::{CheckResult, Feature, Philosophy, Policy, Requirement},
    rules::rule_by_code,
    workspace::{Workspace, load_workspace},
};

use super::lookup::WorkspaceLookup;

#[derive(Debug, Clone, Copy)]
enum TopLevelSection {
    Philosophy,
    Policy,
    Feature,
    Requirement,
    Errors,
}

#[derive(Debug, Clone)]
enum View {
    Menu,
    List(EntityKind),
    Detail(EntityRef),
    Errors,
    ErrorDetail(usize),
}

#[derive(Debug, Clone)]
struct EntityRef {
    kind: EntityKind,
    id: String,
}

pub fn run_browse_command(args: &BrowseArgs) -> Result<i32> {
    let result = collect_check_result(&args.workspace);
    let workspace = load_workspace(&args.workspace).ok();
    let state = BrowseState { workspace, result };

    if args.non_interactive {
        state.print_non_interactive();
        return Ok(0);
    }

    state.run()
}

struct BrowseState {
    workspace: Option<Workspace>,
    result: CheckResult,
}

impl BrowseState {
    fn lookup(&self) -> Option<WorkspaceLookup<'_>> {
        self.workspace.as_ref().map(WorkspaceLookup::new)
    }

    fn run(&self) -> Result<i32> {
        let mut view = View::Menu;

        loop {
            view = match view {
                View::Menu => match self.show_top_level_menu()? {
                    Some(section) => match section {
                        TopLevelSection::Philosophy => View::List(EntityKind::Philosophy),
                        TopLevelSection::Policy => View::List(EntityKind::Policy),
                        TopLevelSection::Feature => View::List(EntityKind::Feature),
                        TopLevelSection::Requirement => View::List(EntityKind::Requirement),
                        TopLevelSection::Errors => View::Errors,
                    },
                    None => return Ok(0),
                },
                View::List(kind) => match self.show_entity_list(kind)? {
                    Some(entity) => View::Detail(entity),
                    None => View::Menu,
                },
                View::Detail(entity) => match self.show_entity_detail(&entity)? {
                    Some(next) => View::Detail(next),
                    None => View::List(entity.kind),
                },
                View::Errors => match self.show_error_list()? {
                    Some(index) => View::ErrorDetail(index),
                    None => View::Menu,
                },
                View::ErrorDetail(index) => {
                    self.show_error_detail(index)?;
                    View::Errors
                }
            };
        }
    }

    fn show_top_level_menu(&self) -> Result<Option<TopLevelSection>> {
        const SECTIONS: [TopLevelSection; 5] = [
            TopLevelSection::Philosophy,
            TopLevelSection::Policy,
            TopLevelSection::Feature,
            TopLevelSection::Requirement,
            TopLevelSection::Errors,
        ];

        self.print_summary("syu interactive browser");
        println!(
            "1. philosophy ({})",
            self.result.definition_counts.philosophies
        );
        println!("2. policy ({})", self.result.definition_counts.policies);
        println!("3. feature ({})", self.result.definition_counts.features);
        println!(
            "4. requirement ({})",
            self.result.definition_counts.requirements
        );
        println!("5. errors ({})", self.result.issues.len());
        println!("0. exit");

        match prompt_number("Select a section", 5)? {
            Some(0) => Ok(None),
            None => Ok(None),
            Some(choice) => Ok(SECTIONS.get(choice - 1).copied()),
        }
    }

    fn show_entity_list(&self, kind: EntityKind) -> Result<Option<EntityRef>> {
        self.print_summary(kind.label());
        let entries = self.entity_entries(kind);
        if entries.is_empty() {
            println!("No {} entries are currently available.", kind.label());
            wait_for_enter("Press Enter to go back")?;
            return Ok(None);
        }

        for (index, (id, title)) in entries.iter().enumerate() {
            println!("{}. {} — {}", index + 1, id, title);
        }
        println!("0. back");

        match prompt_number("Select an entry", entries.len())? {
            Some(0) | None => Ok(None),
            Some(choice) => {
                let (id, _) = &entries[choice - 1];
                Ok(Some(EntityRef {
                    kind,
                    id: id.clone(),
                }))
            }
        }
    }

    fn show_entity_detail(&self, entity: &EntityRef) -> Result<Option<EntityRef>> {
        self.print_summary(&format!("{} detail", entity.kind.label()));
        let linked = match entity.kind {
            EntityKind::Philosophy => self.print_philosophy_detail(&entity.id),
            EntityKind::Policy => self.print_policy_detail(&entity.id),
            EntityKind::Feature => self.print_feature_detail(&entity.id),
            EntityKind::Requirement => self.print_requirement_detail(&entity.id),
        };

        println!("0. back");
        match prompt_number("Choose an entry", linked.len())? {
            Some(0) | None => Ok(None),
            Some(choice) => Ok(Some(linked[choice - 1].clone())),
        }
    }

    fn show_error_list(&self) -> Result<Option<usize>> {
        self.print_summary("validation errors");
        if self.result.issues.is_empty() {
            println!("No validation issues are currently reported.");
            wait_for_enter("Press Enter to go back")?;
            return Ok(None);
        }

        for (index, issue) in self.result.issues.iter().enumerate() {
            let rule_title = rule_by_code(&issue.code)
                .map(|rule| rule.title.as_str())
                .unwrap_or("Unknown rule");
            println!(
                "{}. [{:?}] {} — {}",
                index + 1,
                issue.severity,
                issue.code,
                rule_title
            );
        }
        println!("0. back");

        match prompt_number("Select an error", self.result.issues.len())? {
            Some(0) | None => Ok(None),
            Some(choice) => Ok(Some(choice - 1)),
        }
    }

    fn show_error_detail(&self, index: usize) -> Result<()> {
        self.print_summary("error detail");
        let issue = &self.result.issues[index];
        println!("Code: {}", issue.code);
        println!("Severity: {:?}", issue.severity);
        println!("Subject: {}", issue.subject);
        println!("Location: {}", issue.location.as_deref().unwrap_or("-"));
        println!("Message: {}", issue.message);
        if let Some(suggestion) = &issue.suggestion {
            println!("Suggestion: {suggestion}");
        }

        if let Some(rule) = rule_by_code(&issue.code) {
            println!("Rule genre: {}", rule.genre);
            println!("Rule title: {}", rule.title);
            println!("Rule summary: {}", rule.summary);
            println!(
                "Rule description: {}",
                collapse_whitespace(&rule.description)
            );
        }

        wait_for_enter("Press Enter to go back")?;
        Ok(())
    }

    fn print_philosophy_detail(&self, id: &str) -> Vec<EntityRef> {
        let Some(item) = self.philosophy(id) else {
            println!("Philosophy `{id}` is not available.");
            return Vec::new();
        };

        println!("ID: {}", item.id);
        println!("Title: {}", item.title);
        println!(
            "Product design principle: {}",
            collapse_whitespace(&item.product_design_principle)
        );
        println!(
            "Coding guideline: {}",
            collapse_whitespace(&item.coding_guideline)
        );
        println!("Linked policies:");
        self.print_links(EntityKind::Policy, &item.linked_policies, 1, |policy_id| {
            self.policy(policy_id).map(|policy| policy.title.clone())
        })
    }

    fn print_policy_detail(&self, id: &str) -> Vec<EntityRef> {
        let Some(item) = self.policy(id) else {
            println!("Policy `{id}` is not available.");
            return Vec::new();
        };

        println!("ID: {}", item.id);
        println!("Title: {}", item.title);
        println!("Summary: {}", collapse_whitespace(&item.summary));
        println!("Description: {}", collapse_whitespace(&item.description));
        println!("Linked philosophies:");
        let mut linked = self.print_links(
            EntityKind::Philosophy,
            &item.linked_philosophies,
            1,
            |philosophy_id| {
                self.philosophy(philosophy_id)
                    .map(|philosophy| philosophy.title.clone())
            },
        );
        println!("Linked requirements:");
        linked.extend(self.print_links(
            EntityKind::Requirement,
            &item.linked_requirements,
            linked.len() + 1,
            |requirement_id| {
                self.requirement(requirement_id)
                    .map(|requirement| requirement.title.clone())
            },
        ));
        linked
    }

    fn print_requirement_detail(&self, id: &str) -> Vec<EntityRef> {
        let Some(item) = self.requirement(id) else {
            println!("Requirement `{id}` is not available.");
            return Vec::new();
        };

        println!("ID: {}", item.id);
        println!("Title: {}", item.title);
        println!("Priority: {}", item.priority);
        println!("Status: {}", item.status);
        println!("Description: {}", collapse_whitespace(&item.description));
        println!("Linked policies:");
        let mut linked =
            self.print_links(EntityKind::Policy, &item.linked_policies, 1, |policy_id| {
                self.policy(policy_id).map(|policy| policy.title.clone())
            });
        println!("Linked features:");
        linked.extend(self.print_links(
            EntityKind::Feature,
            &item.linked_features,
            linked.len() + 1,
            |feature_id| {
                self.feature(feature_id)
                    .map(|feature| feature.title.clone())
            },
        ));
        println!("Declared tests:");
        print_trace_summary(&item.tests);
        linked
    }

    fn print_feature_detail(&self, id: &str) -> Vec<EntityRef> {
        let Some(item) = self.feature(id) else {
            println!("Feature `{id}` is not available.");
            return Vec::new();
        };

        println!("ID: {}", item.id);
        println!("Title: {}", item.title);
        println!("Status: {}", item.status);
        println!("Summary: {}", collapse_whitespace(&item.summary));
        println!("Linked requirements:");
        let linked = self.print_links(
            EntityKind::Requirement,
            &item.linked_requirements,
            1,
            |requirement_id| {
                self.requirement(requirement_id)
                    .map(|requirement| requirement.title.clone())
            },
        );
        println!("Declared implementations:");
        print_trace_summary(&item.implementations);
        linked
    }

    fn print_summary(&self, heading: &str) {
        println!();
        println!("=== {heading} ===");
        println!("workspace: {}", self.result.workspace_root.display());
        println!(
            "philosophy={} policy={} feature={} requirement={} errors={}",
            self.result.definition_counts.philosophies,
            self.result.definition_counts.policies,
            self.result.definition_counts.features,
            self.result.definition_counts.requirements,
            self.result.issues.len()
        );
        println!();
    }

    fn print_non_interactive(&self) {
        self.print_summary("syu spec tree");

        for (heading, kind) in [
            ("philosophy", EntityKind::Philosophy),
            ("policy", EntityKind::Policy),
            ("requirement", EntityKind::Requirement),
            ("feature", EntityKind::Feature),
        ] {
            let entries = self.entity_entries(kind);
            println!("=== {} ({}) ===", heading, entries.len());
            for (id, title) in &entries {
                println!("  {id}\t{title}");
            }
            println!();
        }

        if !self.result.issues.is_empty() {
            println!("=== errors ({}) ===", self.result.issues.len());
            for issue in &self.result.issues {
                println!("  [{}] {}", issue.code, issue.subject);
            }
            println!();
        }
    }

    fn print_links<F>(
        &self,
        kind: EntityKind,
        ids: &[String],
        start_index: usize,
        title_for: F,
    ) -> Vec<EntityRef>
    where
        F: Fn(&str) -> Option<String>,
    {
        if ids.is_empty() {
            println!("- none");
            return Vec::new();
        }

        let mut linked = Vec::new();
        for (index, id) in ids.iter().enumerate() {
            let title = title_for(id).unwrap_or_else(|| "missing".to_string());
            println!("{}. {} — {}", start_index + index, id, title);
            linked.push(EntityRef {
                kind,
                id: id.clone(),
            });
        }
        linked
    }

    fn entity_entries(&self, kind: EntityKind) -> Vec<(String, String)> {
        self.lookup()
            .map(|lookup| {
                lookup
                    .entries(kind)
                    .into_iter()
                    .map(|item| (item.id, item.title))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn philosophy(&self, id: &str) -> Option<&Philosophy> {
        self.lookup()?.philosophy(id)
    }

    fn policy(&self, id: &str) -> Option<&Policy> {
        self.lookup()?.policy(id)
    }

    fn requirement(&self, id: &str) -> Option<&Requirement> {
        self.lookup()?.requirement(id)
    }

    fn feature(&self, id: &str) -> Option<&Feature> {
        self.lookup()?.feature(id)
    }
}

fn print_trace_summary(
    references: &std::collections::BTreeMap<String, Vec<crate::model::TraceReference>>,
) {
    if references.is_empty() {
        println!("- none");
        return;
    }

    for (language, items) in references {
        println!("- {language}:");
        for item in items {
            println!(
                "  - {} [{}]",
                item.file.display(),
                if item.symbols.is_empty() {
                    "-".to_string()
                } else {
                    item.symbols.join(", ")
                }
            );
        }
    }
}

fn prompt_number(prompt: &str, max: usize) -> Result<Option<usize>> {
    loop {
        print!("{prompt} [0-{max}]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        if io::stdin().read_line(&mut input)? == 0 {
            println!();
            return Ok(None);
        }
        let trimmed = input.trim();
        if trimmed.eq_ignore_ascii_case("q") || trimmed.eq_ignore_ascii_case("quit") {
            return Ok(Some(0));
        }
        if trimmed.is_empty() {
            println!("Please enter a number between 0 and {max}.");
            continue;
        }

        match trimmed.parse::<usize>() {
            Ok(value) if value <= max => return Ok(Some(value)),
            _ => println!("Please enter a number between 0 and {max}."),
        }
    }
}

fn wait_for_enter(prompt: &str) -> Result<()> {
    print!("{prompt}: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(())
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::PathBuf};

    use crate::{
        command::check::collect_check_result,
        model::{
            CheckResult, DefinitionCounts, Feature, Philosophy, Policy, Requirement,
            TraceReference, TraceSummary,
        },
        workspace::{Workspace, load_workspace},
    };

    use super::{BrowseState, EntityKind, collapse_whitespace, print_trace_summary};

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/workspaces")
            .join(name)
    }

    fn state_for_fixture(name: &str) -> BrowseState {
        let root = fixture_path(name);
        BrowseState {
            workspace: load_workspace(&root).ok(),
            result: collect_check_result(&root),
        }
    }

    #[test]
    fn entity_entries_follow_workspace_contents() {
        let workspace = Workspace {
            root: PathBuf::from("."),
            spec_root: PathBuf::from("docs/syu"),
            config: crate::config::SyuConfig::default(),
            philosophies: vec![Philosophy {
                id: "PHIL-001".to_string(),
                title: "Philosophy".to_string(),
                product_design_principle: "Principle".to_string(),
                coding_guideline: "Guideline".to_string(),
                linked_policies: vec!["POL-001".to_string()],
            }],
            policies: vec![Policy {
                id: "POL-001".to_string(),
                title: "Policy".to_string(),
                summary: "Summary".to_string(),
                description: "Description".to_string(),
                linked_philosophies: vec!["PHIL-001".to_string()],
                linked_requirements: vec!["REQ-001".to_string()],
            }],
            requirements: vec![Requirement {
                id: "REQ-001".to_string(),
                title: "Requirement".to_string(),
                description: "Description".to_string(),
                priority: "high".to_string(),
                status: "implemented".to_string(),
                linked_policies: vec!["POL-001".to_string()],
                linked_features: vec!["FEAT-001".to_string()],
                tests: Default::default(),
            }],
            features: vec![Feature {
                id: "FEAT-001".to_string(),
                title: "Feature".to_string(),
                summary: "Summary".to_string(),
                status: "implemented".to_string(),
                linked_requirements: vec!["REQ-001".to_string()],
                implementations: Default::default(),
            }],
        };
        let state = BrowseState {
            workspace: Some(workspace),
            result: CheckResult {
                workspace_root: PathBuf::from("."),
                definition_counts: DefinitionCounts {
                    philosophies: 1,
                    policies: 1,
                    requirements: 1,
                    features: 1,
                },
                trace_summary: TraceSummary::default(),
                issues: Vec::new(),
                referenced_rules: Vec::new(),
            },
        };

        assert_eq!(state.entity_entries(EntityKind::Philosophy).len(), 1);
        assert_eq!(state.entity_entries(EntityKind::Policy).len(), 1);
        assert_eq!(state.entity_entries(EntityKind::Requirement).len(), 1);
        assert_eq!(state.entity_entries(EntityKind::Feature).len(), 1);
    }

    #[test]
    fn browse_helpers_cover_labels_and_entity_lookup() {
        let state = state_for_fixture("passing");

        assert_eq!(EntityKind::Philosophy.label(), "philosophy");
        assert_eq!(EntityKind::Policy.label(), "policy");
        assert_eq!(EntityKind::Feature.label(), "feature");
        assert_eq!(EntityKind::Requirement.label(), "requirement");

        let workspace = state.workspace.as_ref().expect("fixture should load");
        let philosophy_id = workspace.philosophies[0].id.clone();
        let policy_id = workspace.policies[0].id.clone();
        let requirement_id = workspace.requirements[0].id.clone();
        let feature_id = workspace.features[0].id.clone();

        assert!(state.philosophy(&philosophy_id).is_some());
        assert!(state.policy(&policy_id).is_some());
        assert!(state.requirement(&requirement_id).is_some());
        assert!(state.feature(&feature_id).is_some());

        assert!(state.philosophy("PHIL-MISSING").is_none());
        assert!(state.policy("POL-MISSING").is_none());
        assert!(state.requirement("REQ-MISSING").is_none());
        assert!(state.feature("FEAT-MISSING").is_none());
    }

    #[test]
    fn detail_helpers_return_linked_entities_and_handle_missing_items() {
        let state = state_for_fixture("passing");
        let workspace = state.workspace.as_ref().expect("fixture should load");

        let philosophy_links = state.print_philosophy_detail(&workspace.philosophies[0].id);
        let policy_links = state.print_policy_detail(&workspace.policies[0].id);
        let requirement_links = state.print_requirement_detail(&workspace.requirements[0].id);
        let feature_links = state.print_feature_detail(&workspace.features[0].id);

        assert!(!philosophy_links.is_empty());
        assert!(!policy_links.is_empty());
        assert!(!requirement_links.is_empty());
        assert!(!feature_links.is_empty());

        assert!(state.print_philosophy_detail("PHIL-MISSING").is_empty());
        assert!(state.print_policy_detail("POL-MISSING").is_empty());
        assert!(state.print_requirement_detail("REQ-MISSING").is_empty());
        assert!(state.print_feature_detail("FEAT-MISSING").is_empty());
    }

    #[test]
    fn helper_rendering_handles_empty_and_non_empty_values() {
        let state = state_for_fixture("passing");

        assert!(
            state
                .print_links(EntityKind::Policy, &[], 1, |_| None)
                .is_empty()
        );

        print_trace_summary(&BTreeMap::new());
        print_trace_summary(&BTreeMap::from([(
            "rust".to_string(),
            vec![
                TraceReference {
                    file: PathBuf::from("src/lib.rs"),
                    symbols: Vec::new(),
                    doc_contains: Vec::new(),
                },
                TraceReference {
                    file: PathBuf::from("src/main.rs"),
                    symbols: vec!["public_api".to_string()],
                    doc_contains: Vec::new(),
                },
            ],
        )]));

        assert_eq!(
            collapse_whitespace("browse\n  keeps   context"),
            "browse keeps context"
        );
    }
}
