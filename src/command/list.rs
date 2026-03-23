// FEAT-LIST-001
// REQ-CORE-018

use anyhow::Result;
use serde::Serialize;

use crate::{
    cli::{ListArgs, OutputFormat},
    workspace::load_workspace,
};

use super::lookup::{EntitySummary, WorkspaceLookup};

#[derive(Debug, Serialize)]
struct JsonListOutput {
    kind: &'static str,
    items: Vec<EntitySummary>,
}

pub fn run_list_command(args: &ListArgs) -> Result<i32> {
    let workspace = load_workspace(&args.workspace)?;
    let items = WorkspaceLookup::new(&workspace).entries(args.kind);

    match args.format {
        OutputFormat::Text => print_text_list(&items),
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&JsonListOutput {
                kind: args.kind.label(),
                items,
            })
            .expect("serializing list output to JSON should succeed")
        ),
    }

    Ok(0)
}

fn print_text_list(items: &[EntitySummary]) {
    for item in items {
        println!("{}\t{}", item.id, item.title);
    }
}
