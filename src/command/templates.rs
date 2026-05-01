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
            relationship: template_relationship(template.related_example),
            related_example: template.related_example,
        })
        .collect()
}

fn template_relationship(related_example: Option<&'static str>) -> TemplateRelationship {
    match related_example {
        Some(_) => TemplateRelationship::TemplateAndExample,
        None => TemplateRelationship::StarterOnly,
    }
}

fn print_text_catalog(templates: &[TemplateCatalogEntry]) {
    print!("{}", render_text_catalog(templates));
}

fn render_text_catalog(templates: &[TemplateCatalogEntry]) -> String {
    let mut output = String::from("name\trelationship\trelated_example\tdescription\n");
    for template in templates {
        match template.related_example {
            Some(example) => {
                output.push_str(&format!(
                    "{}\t{}\t{}\t{}\n",
                    template.name,
                    template.relationship_label(),
                    example,
                    template.description
                ));
            }
            None => {
                output.push_str(&format!(
                    "{}\t{}\t-\t{}\n",
                    template.name,
                    template.relationship_label(),
                    template.description
                ));
            }
        }
    }
    output
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
    use super::{
        TemplateCatalogEntry, TemplateRelationship, render_text_catalog, template_relationship,
    };

    #[test]
    fn starter_template_catalog_lists_every_supported_template() {
        let templates = template_catalog_entries();
        assert_eq!(templates.len(), 9);
        assert_eq!(templates[0].name, "generic");
        assert_eq!(templates[1].name, "docs-first");
        assert_eq!(templates[2].name, "rust-only");
        assert_eq!(templates[3].name, "python-only");
        assert_eq!(templates[4].name, "ruby-only");
        assert_eq!(templates[5].name, "go-only");
        assert_eq!(templates[6].name, "java-only");
        assert_eq!(templates[7].name, "typescript-only");
        assert_eq!(templates[8].name, "polyglot");
    }

    #[test]
    fn starter_template_catalog_marks_example_backed_templates() {
        let templates = template_catalog_entries();
        assert_eq!(templates[0].relationship_label(), "template-and-example");
        assert_eq!(templates[1].relationship_label(), "template-and-example");
        assert_eq!(templates[0].related_example, Some("examples/generic"));
        assert_eq!(templates[1].related_example, Some("examples/docs-first"));
        assert_eq!(templates[2].related_example, Some("examples/rust-only"));
        assert_eq!(templates[3].related_example, Some("examples/python-only"));
        assert_eq!(templates[4].related_example, Some("examples/ruby-only"));
        assert_eq!(templates[5].related_example, Some("examples/go-only"));
        assert_eq!(templates[6].related_example, Some("examples/java-only"));
        assert_eq!(
            templates[7].related_example,
            Some("examples/typescript-only")
        );
        assert_eq!(templates[8].related_example, Some("examples/polyglot"));
    }

    #[test]
    fn starter_only_template_catalog_renders_without_example() {
        let output = render_text_catalog(&[TemplateCatalogEntry {
            name: "starter",
            description: "starter description",
            relationship: TemplateRelationship::StarterOnly,
            related_example: None,
        }]);

        assert!(output.contains("starter-only"));
        assert!(output.contains("\t-\t"));
    }

    #[test]
    fn template_relationship_distinguishes_example_backed_templates() {
        assert!(matches!(
            template_relationship(None),
            TemplateRelationship::StarterOnly
        ));
        assert!(matches!(
            template_relationship(Some("examples/generic")),
            TemplateRelationship::TemplateAndExample
        ));
    }
}
