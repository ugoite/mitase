// FEAT-INIT-006
// REQ-CORE-009

use anyhow::Result;
use serde::Serialize;

use crate::cli::{OutputFormat, StarterTemplate, TemplatesArgs};

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
    let templates = starter_template_catalog();

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

fn starter_template_catalog() -> Vec<TemplateCatalogEntry> {
    [
        StarterTemplate::Generic,
        StarterTemplate::RustOnly,
        StarterTemplate::PythonOnly,
        StarterTemplate::Polyglot,
    ]
    .into_iter()
    .map(template_catalog_entry)
    .collect()
}

fn template_catalog_entry(template: StarterTemplate) -> TemplateCatalogEntry {
    match template {
        StarterTemplate::Generic => TemplateCatalogEntry {
            name: "generic",
            description: "Starter with minimal four-layer files, neutral IDs, and core file names.",
            relationship: TemplateRelationship::StarterOnly,
            related_example: None,
        },
        StarterTemplate::RustOnly => TemplateCatalogEntry {
            name: "rust-only",
            description: "Starter for Rust-first repos with Rust-oriented IDs plus requirement and feature files.",
            relationship: TemplateRelationship::TemplateAndExample,
            related_example: Some("examples/rust-only"),
        },
        StarterTemplate::PythonOnly => TemplateCatalogEntry {
            name: "python-only",
            description: "Starter for Python-first repos with Python-oriented IDs plus requirement and feature files.",
            relationship: TemplateRelationship::TemplateAndExample,
            related_example: Some("examples/python-only"),
        },
        StarterTemplate::Polyglot => TemplateCatalogEntry {
            name: "polyglot",
            description: "Starter for mixed-language repos with the same four layers and a polyglot first spec.",
            relationship: TemplateRelationship::TemplateAndExample,
            related_example: Some("examples/polyglot"),
        },
    }
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
    use super::starter_template_catalog;

    #[test]
    fn starter_template_catalog_lists_every_supported_template() {
        let templates = starter_template_catalog();
        assert_eq!(templates.len(), 4);
        assert_eq!(templates[0].name, "generic");
        assert_eq!(templates[1].name, "rust-only");
        assert_eq!(templates[2].name, "python-only");
        assert_eq!(templates[3].name, "polyglot");
    }

    #[test]
    fn starter_template_catalog_marks_example_backed_templates() {
        let templates = starter_template_catalog();
        assert_eq!(templates[0].relationship_label(), "starter-only");
        assert_eq!(templates[1].relationship_label(), "template-and-example");
        assert_eq!(templates[0].related_example, None);
        assert_eq!(templates[1].related_example, Some("examples/rust-only"));
        assert_eq!(templates[2].related_example, Some("examples/python-only"));
        assert_eq!(templates[3].related_example, Some("examples/polyglot"));
    }
}
