// FEAT-LIST-001
// FEAT-LIST-002
// REQ-CORE-018

use std::path::PathBuf;

use anyhow::{Result, anyhow};
use clap::ValueEnum as _;
use serde::Serialize;

use crate::{
    cli::{ListArgs, LookupKind, OutputFormat},
    workspace::load_workspace,
};

use super::lookup::{EntitySummary, WorkspaceLookup};

#[derive(Debug, Serialize)]
struct JsonListOutput {
    kind: &'static str,
    items: Vec<EntitySummary>,
}

#[derive(Debug, Serialize)]
struct JsonAllKindsOutput {
    philosophy: Vec<EntitySummary>,
    policy: Vec<EntitySummary>,
    requirement: Vec<EntitySummary>,
    feature: Vec<EntitySummary>,
}

pub fn run_list_command(args: &ListArgs) -> Result<i32> {
    let (kind, workspace) = parse_list_positionals(&args.positional)?;
    let workspace = load_workspace(&workspace)?;
    let lookup = WorkspaceLookup::new(&workspace);

    match kind {
        Some(k) => {
            let items = lookup.entries(k);
            match args.format {
                OutputFormat::Text => print_text_list(&items),
                OutputFormat::Json => println!(
                    "{}",
                    serde_json::to_string_pretty(&JsonListOutput {
                        kind: k.label(),
                        items,
                    })
                    .expect("serializing list output to JSON should succeed")
                ),
            }
        }
        None => {
            let philosophies = lookup.entries(LookupKind::Philosophy);
            let policies = lookup.entries(LookupKind::Policy);
            let requirements = lookup.entries(LookupKind::Requirement);
            let features = lookup.entries(LookupKind::Feature);

            match args.format {
                OutputFormat::Text => {
                    print_section_list("philosophy", &philosophies);
                    print_section_list("policy", &policies);
                    print_section_list("requirement", &requirements);
                    print_section_list("feature", &features);
                }
                OutputFormat::Json => println!(
                    "{}",
                    serde_json::to_string_pretty(&JsonAllKindsOutput {
                        philosophy: philosophies,
                        policy: policies,
                        requirement: requirements,
                        feature: features,
                    })
                    .expect("serializing all-kinds output to JSON should succeed")
                ),
            }
        }
    }

    Ok(0)
}

fn parse_list_positionals(positional: &[String]) -> Result<(Option<LookupKind>, PathBuf)> {
    match positional {
        [] => Ok((None, PathBuf::from("."))),
        [first] => {
            if let Ok(kind) = LookupKind::from_str(first, true) {
                Ok((Some(kind), PathBuf::from(".")))
            } else if PathBuf::from(first).exists() {
                Ok((None, PathBuf::from(first)))
            } else if let Some(suggestion) = suggested_lookup_kind(first) {
                Err(invalid_kind_error(first, None, Some(suggestion)))
            } else {
                Ok((None, PathBuf::from(first)))
            }
        }
        [kind_str, workspace_str, ..] => {
            let kind = LookupKind::from_str(kind_str, true).map_err(|_| {
                invalid_kind_error(
                    kind_str,
                    Some(workspace_str),
                    suggested_lookup_kind(kind_str),
                )
            })?;
            Ok((Some(kind), PathBuf::from(workspace_str)))
        }
    }
}

fn invalid_kind_error(
    value: &str,
    workspace_hint: Option<&str>,
    suggestion: Option<&'static str>,
) -> anyhow::Error {
    let mut message = format!(
        "invalid value '{}' for KIND\n  possible values: philosophy, policy, requirement, feature",
        value
    );
    if let Some(suggestion) = suggestion {
        message.push_str(&format!("\n  did you mean `{suggestion}`?"));
    }
    if let Some(workspace_hint) = workspace_hint {
        message.push_str(&format!(
            "\n  Hint: to list all kinds in a workspace, run `syu list {workspace_hint}`"
        ));
    } else {
        message.push_str(
            "\n  Hint: use one of the layer kinds above, pass a workspace path, or run `syu list --help`.",
        );
    }
    anyhow!(message)
}

fn suggested_lookup_kind(value: &str) -> Option<&'static str> {
    const KINDS: [&str; 4] = ["philosophy", "policy", "requirement", "feature"];

    let normalized = value.to_ascii_lowercase();
    KINDS
        .into_iter()
        .map(|candidate| (candidate, levenshtein_distance(&normalized, candidate)))
        .filter(|(_, distance)| *distance <= 2)
        .min_by_key(|(_, distance)| *distance)
        .map(|(candidate, _)| candidate)
}

fn levenshtein_distance(left: &str, right: &str) -> usize {
    let left: Vec<_> = left.chars().collect();
    let right: Vec<_> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0; right.len() + 1];

    for (left_index, left_char) in left.iter().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_char) in right.iter().enumerate() {
            let cost = usize::from(left_char != right_char);
            current[right_index + 1] = (current[right_index] + 1)
                .min(previous[right_index + 1] + 1)
                .min(previous[right_index] + cost);
        }
        previous.clone_from(&current);
    }

    previous[right.len()]
}

fn print_text_list(items: &[EntitySummary]) {
    for item in items {
        println!("{}\t{}", item.id, item.title);
    }
}

fn print_section_list(heading: &str, items: &[EntitySummary]) {
    println!("=== {} ({}) ===", heading, items.len());
    for item in items {
        println!("{}\t{}", item.id, item.title);
    }
    println!();
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::cli::LookupKind;

    use super::parse_list_positionals;

    #[test]
    // FEAT-LIST-002
    fn parse_list_positionals_empty_returns_all_kinds_in_cwd() {
        let (kind, workspace) = parse_list_positionals(&[]).expect("should succeed");
        assert!(kind.is_none());
        assert_eq!(workspace, PathBuf::from("."));
    }

    #[test]
    // FEAT-LIST-002
    fn parse_list_positionals_path_returns_all_kinds_in_given_workspace() {
        let (kind, workspace) =
            parse_list_positionals(&["/tmp/myproject".to_string()]).expect("should succeed");
        assert!(kind.is_none());
        assert_eq!(workspace, PathBuf::from("/tmp/myproject"));
    }

    #[test]
    // FEAT-LIST-002
    fn parse_list_positionals_dot_returns_all_kinds_in_cwd() {
        let (kind, workspace) = parse_list_positionals(&[".".to_string()]).expect("should succeed");
        assert!(kind.is_none());
        assert_eq!(workspace, PathBuf::from("."));
    }

    #[test]
    // FEAT-LIST-001
    fn parse_list_positionals_kind_only_returns_kind_with_cwd() {
        let (kind, workspace) =
            parse_list_positionals(&["requirement".to_string()]).expect("should succeed");
        assert_eq!(kind, Some(LookupKind::Requirement));
        assert_eq!(workspace, PathBuf::from("."));
    }

    #[test]
    // FEAT-LIST-001
    fn parse_list_positionals_kind_and_workspace_returns_both() {
        let (kind, workspace) =
            parse_list_positionals(&["feature".to_string(), "/tmp/ws".to_string()])
                .expect("should succeed");
        assert_eq!(kind, Some(LookupKind::Feature));
        assert_eq!(workspace, PathBuf::from("/tmp/ws"));
    }

    #[test]
    // FEAT-LIST-001
    fn parse_list_positionals_kind_plural_alias_works() {
        let (kind, _workspace) =
            parse_list_positionals(&["requirements".to_string()]).expect("should succeed");
        assert_eq!(kind, Some(LookupKind::Requirement));
    }

    #[test]
    // FEAT-LIST-002
    fn parse_list_positionals_two_args_invalid_kind_returns_error() {
        let result = parse_list_positionals(&["notakind".to_string(), ".".to_string()]);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("invalid value"), "error: {msg}");
        assert!(msg.contains("syu list ."), "hint missing: {msg}");
    }

    #[test]
    // FEAT-LIST-001
    fn parse_list_positionals_single_arg_kind_typo_returns_error() {
        let result = parse_list_positionals(&["philsophy".to_string()]);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("did you mean `philosophy`"), "error: {msg}");
        assert!(msg.contains("syu list --help"), "hint missing: {msg}");
    }

    #[test]
    // FEAT-LIST-001
    fn parse_list_positionals_two_arg_kind_typo_returns_suggestion() {
        let result = parse_list_positionals(&["philsophy".to_string(), ".".to_string()]);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("did you mean `philosophy`"), "error: {msg}");
        assert!(msg.contains("syu list ."), "hint missing: {msg}");
    }
}
