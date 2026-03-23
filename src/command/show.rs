// FEAT-SHOW-001
// REQ-CORE-018

use std::{collections::BTreeMap, fmt::Write};

use anyhow::{Result, bail};
use serde::Serialize;

use crate::{
    cli::{OutputFormat, ShowArgs},
    model::{Feature, Philosophy, Policy, Requirement, TraceReference},
    workspace::load_workspace,
};

use super::lookup::{WorkspaceEntity, WorkspaceLookup};

#[derive(Debug, Serialize)]
struct JsonShowOutput<'a, T> {
    kind: &'a str,
    item: &'a T,
}

pub fn run_show_command(args: &ShowArgs) -> Result<i32> {
    let workspace = load_workspace(&args.workspace)?;
    let lookup = WorkspaceLookup::new(&workspace);
    let Some(entity) = lookup.find(&args.id) else {
        bail!(
            "definition `{}` was not found in `{}`",
            args.id,
            workspace.root.display()
        );
    };

    match args.format {
        OutputFormat::Text => print!("{}", render_entity_text(lookup, entity)),
        OutputFormat::Json => print_entity_json(entity),
    }

    Ok(0)
}

fn print_entity_json(entity: WorkspaceEntity<'_>) {
    match entity {
        WorkspaceEntity::Philosophy(item) => print_json_output("philosophy", item),
        WorkspaceEntity::Policy(item) => print_json_output("policy", item),
        WorkspaceEntity::Requirement(item) => print_json_output("requirement", item),
        WorkspaceEntity::Feature(item) => print_json_output("feature", item),
    }
}

fn print_json_output<T: Serialize>(kind: &str, item: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(&JsonShowOutput { kind, item })
            .expect("serializing show output to JSON should succeed")
    );
}

fn render_entity_text(lookup: WorkspaceLookup<'_>, entity: WorkspaceEntity<'_>) -> String {
    let mut output = String::new();

    match entity {
        WorkspaceEntity::Philosophy(item) => render_philosophy(&mut output, lookup, item),
        WorkspaceEntity::Policy(item) => render_policy(&mut output, lookup, item),
        WorkspaceEntity::Requirement(item) => render_requirement(&mut output, lookup, item),
        WorkspaceEntity::Feature(item) => render_feature(&mut output, lookup, item),
    }

    output
}

fn render_philosophy(output: &mut String, lookup: WorkspaceLookup<'_>, item: &Philosophy) {
    write_common_header(output, "philosophy", &item.id, &item.title);
    writeln!(
        output,
        "Product design principle: {}",
        collapse_whitespace(&item.product_design_principle)
    )
    .expect("writing to String must succeed");
    writeln!(
        output,
        "Coding guideline: {}",
        collapse_whitespace(&item.coding_guideline)
    )
    .expect("writing to String must succeed");
    write_links(
        output,
        "Linked policies",
        lookup,
        crate::cli::LookupKind::Policy,
        &item.linked_policies,
    );
}

fn render_policy(output: &mut String, lookup: WorkspaceLookup<'_>, item: &Policy) {
    write_common_header(output, "policy", &item.id, &item.title);
    writeln!(output, "Summary: {}", collapse_whitespace(&item.summary))
        .expect("writing to String must succeed");
    writeln!(
        output,
        "Description: {}",
        collapse_whitespace(&item.description)
    )
    .expect("writing to String must succeed");
    write_links(
        output,
        "Linked philosophies",
        lookup,
        crate::cli::LookupKind::Philosophy,
        &item.linked_philosophies,
    );
    write_links(
        output,
        "Linked requirements",
        lookup,
        crate::cli::LookupKind::Requirement,
        &item.linked_requirements,
    );
}

fn render_requirement(output: &mut String, lookup: WorkspaceLookup<'_>, item: &Requirement) {
    write_common_header(output, "requirement", &item.id, &item.title);
    writeln!(output, "Priority: {}", item.priority).expect("writing to String must succeed");
    writeln!(output, "Status: {}", item.status).expect("writing to String must succeed");
    writeln!(
        output,
        "Description: {}",
        collapse_whitespace(&item.description)
    )
    .expect("writing to String must succeed");
    write_links(
        output,
        "Linked policies",
        lookup,
        crate::cli::LookupKind::Policy,
        &item.linked_policies,
    );
    write_links(
        output,
        "Linked features",
        lookup,
        crate::cli::LookupKind::Feature,
        &item.linked_features,
    );
    write_trace_map(output, "Declared tests", &item.tests);
}

fn render_feature(output: &mut String, lookup: WorkspaceLookup<'_>, item: &Feature) {
    write_common_header(output, "feature", &item.id, &item.title);
    writeln!(output, "Status: {}", item.status).expect("writing to String must succeed");
    writeln!(output, "Summary: {}", collapse_whitespace(&item.summary))
        .expect("writing to String must succeed");
    write_links(
        output,
        "Linked requirements",
        lookup,
        crate::cli::LookupKind::Requirement,
        &item.linked_requirements,
    );
    write_trace_map(output, "Declared implementations", &item.implementations);
}

fn write_common_header(output: &mut String, kind: &str, id: &str, title: &str) {
    writeln!(output, "Kind: {kind}").expect("writing to String must succeed");
    writeln!(output, "ID: {id}").expect("writing to String must succeed");
    writeln!(output, "Title: {title}").expect("writing to String must succeed");
}

fn write_links(
    output: &mut String,
    heading: &str,
    lookup: WorkspaceLookup<'_>,
    kind: crate::cli::LookupKind,
    ids: &[String],
) {
    writeln!(output, "{heading}:").expect("writing to String must succeed");
    if ids.is_empty() {
        writeln!(output, "- none").expect("writing to String must succeed");
        return;
    }

    for id in ids {
        let title = lookup.title_for(kind, id).unwrap_or("missing");
        writeln!(output, "- {id}\t{title}").expect("writing to String must succeed");
    }
}

fn write_trace_map(
    output: &mut String,
    heading: &str,
    references: &BTreeMap<String, Vec<TraceReference>>,
) {
    writeln!(output, "{heading}:").expect("writing to String must succeed");
    if references.is_empty() {
        writeln!(output, "- none").expect("writing to String must succeed");
        return;
    }

    for (language, items) in references {
        writeln!(output, "- {language}:").expect("writing to String must succeed");
        for item in items {
            writeln!(output, "  - file: {}", item.file.display())
                .expect("writing to String must succeed");
            writeln!(
                output,
                "    symbols: {}",
                if item.symbols.is_empty() {
                    "-".to_string()
                } else {
                    item.symbols.join(", ")
                }
            )
            .expect("writing to String must succeed");
            if !item.doc_contains.is_empty() {
                writeln!(output, "    doc_contains: {}", item.doc_contains.join(", "))
                    .expect("writing to String must succeed");
            }
        }
    }
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
