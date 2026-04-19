// FEAT-INIT-006
// REQ-CORE-009

use anyhow::Result;
use serde::Serialize;

use crate::cli::{OutputFormat, TemplatesArgs};

use super::init::starter_template_catalog as shared_starter_template_catalog;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
enum TemplateRelationship {
    StarterOnly,
    TemplateAndExample,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct TemplateCatalogEntry {
    name: &'static str,
    description: &'static str,
    relationship: TemplateRelationship,
    #[serde(skip_serializing_if = "Option::is_none")]
    related_example: Option<&'static str>,
}

#[derive(Debug, Serialize)]
struct JsonTemplatesOutput {
    templates: Vec<TemplateCatalogEntry>,
}

pub fn run_templates_command(args: &TemplatesArgs) -> Result<i32> {
    let templates = template_catalog_entries();

    match args.format {
        OutputFormat::Text => print_text_catalog(&templates),
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&JsonTemplatesOutput { templates })
                .expect("serializing templates output to JSON should succeed")
        ),
    }

    Ok(0)
}

fn template_catalog_entries() -> Vec<TemplateCatalogEntry> {
    shared_starter_template_catalog()
        .iter()
        .map(|template| TemplateCatalogEntry {
            name: template.name,
            description: template.description,
            relationship: match template.related_example {
                Some(_) => TemplateRelationship::TemplateAndExample,
                None => TemplateRelationship::StarterOnly,
            },
            related_example: template.related_example,
        })
        .collect()
}

fn print_text_catalog(templates: &[TemplateCatalogEntry]) {
    println!("name\trelationship\trelated_example\tdescription");
    for template in templates {
        match template.related_example {
            Some(example) => println!(
                "{}\t{}\t{}\t{}",
                template.name,
                template.relationship_label(),
                example,
                template.description
            ),
            None => println!(
                "{}\t{}\t-\t{}",
                template.name,
                template.relationship_label(),
                template.description
            ),
        }
    }
}

impl TemplateCatalogEntry {
    const fn relationship_label(self) -> &'static str {
        match self.relationship {
            TemplateRelationship::StarterOnly => "starter-only",
            TemplateRelationship::TemplateAndExample => "template-and-example",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::template_catalog_entries;

    #[test]
    fn starter_template_catalog_lists_every_supported_template() {
        let templates = template_catalog_entries();
        assert_eq!(templates.len(), 5);
        assert_eq!(templates[0].name, "generic");
        assert_eq!(templates[1].name, "rust-only");
        assert_eq!(templates[2].name, "python-only");
        assert_eq!(templates[3].name, "go-only");
        assert_eq!(templates[4].name, "polyglot");
    }

    #[test]
    fn starter_template_catalog_marks_example_backed_templates() {
        let templates = template_catalog_entries();
        assert_eq!(templates[0].relationship_label(), "starter-only");
        assert_eq!(templates[1].relationship_label(), "template-and-example");
        assert_eq!(templates[0].related_example, None);
        assert_eq!(templates[1].related_example, Some("examples/rust-only"));
        assert_eq!(templates[2].related_example, Some("examples/python-only"));
        assert_eq!(templates[3].related_example, Some("examples/go-only"));
        assert_eq!(templates[4].related_example, Some("examples/polyglot"));
    }
}
