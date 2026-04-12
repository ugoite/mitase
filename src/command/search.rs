// FEAT-SEARCH-001
// REQ-CORE-019

use anyhow::{Result, bail};
use serde::Serialize;

use crate::{
    cli::{OutputFormat, SearchArgs},
    workspace::load_workspace,
};

use super::lookup::{SearchResult, WorkspaceLookup};

#[derive(Debug, Serialize)]
struct JsonSearchOutput {
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<&'static str>,
    items: Vec<SearchResult>,
}

pub fn run_search_command(args: &SearchArgs) -> Result<i32> {
    let query = args.query.trim();
    if query.is_empty() {
        bail!("search query must not be empty or whitespace");
    }

    let workspace = load_workspace(&args.workspace)?;
    let items = WorkspaceLookup::new(&workspace).search(query, args.kind);

    match args.format {
        OutputFormat::Text => print_text_results(query, &items),
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&JsonSearchOutput {
                query: query.to_string(),
                kind: args.kind.map(|kind| kind.label()),
                items,
            })
            .expect("serializing search output to JSON should succeed")
        ),
    }

    Ok(0)
}

fn print_text_results(query: &str, items: &[SearchResult]) {
    if items.is_empty() {
        println!("no matches for `{query}`");
        return;
    }

    for item in items {
        println!("{}\t{}\t{}", item.id, item.kind, item.title);
    }
}
