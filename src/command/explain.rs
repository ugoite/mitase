// FEAT-EXPLAIN-001

use std::fmt::Write as _;

use anyhow::Result;
use serde::Serialize;

use crate::{
    cli::{ExplainArgs, OutputFormat},
    workspace::load_workspace,
};

use super::relate::{
    DirectMatches, Gap, JsonRelateOutput, RelatedNode, RelatedTrace, SelectionSummary,
    build_relation_report,
};

#[derive(Debug, Clone, Serialize)]
struct ExplainOutput {
    selection: SelectionSummary,
    assessment: ExplainAssessment,
    direct_matches: DirectMatches,
    chain: ExplainChain,
    traces: Vec<RelatedTrace>,
    gaps: Vec<Gap>,
}

#[derive(Debug, Clone, Serialize)]
struct ExplainChain {
    philosophies: Vec<RelatedNode>,
    policies: Vec<RelatedNode>,
    requirements: Vec<RelatedNode>,
    features: Vec<RelatedNode>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ExplainAssessment {
    Aligned,
    Ambiguous,
    NeedsAttention,
}

impl ExplainAssessment {
    const fn label(self) -> &'static str {
        match self {
            Self::Aligned => "aligned",
            Self::Ambiguous => "ambiguous",
            Self::NeedsAttention => "needs-attention",
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::Aligned => {
                "The connected philosophy, policy, requirement, and feature chain is present with no obvious graph gaps."
            }
            Self::Ambiguous => {
                "The selector matched multiple direct candidates, so the connected chain still needs human review."
            }
            Self::NeedsAttention => {
                "The connected chain is present, but at least one obvious gap or mismatch still needs review."
            }
        }
    }
}

pub fn run_explain_command(args: &ExplainArgs) -> Result<i32> {
    let workspace = load_workspace(&args.workspace)?;
    let relation = build_relation_report(&workspace, &args.selector)?;
    let output = build_explain_output(relation);

    match args.format {
        OutputFormat::Text => print!("{}", render_explain_text(&output)),
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&output)
                .expect("serializing explain output to JSON should succeed")
        ),
    }

    Ok(0)
}

fn build_explain_output(relation: JsonRelateOutput) -> ExplainOutput {
    let assessment = if selector_is_ambiguous(&relation) {
        ExplainAssessment::Ambiguous
    } else if relation.gaps.is_empty() {
        ExplainAssessment::Aligned
    } else {
        ExplainAssessment::NeedsAttention
    };

    ExplainOutput {
        selection: relation.selection,
        assessment,
        direct_matches: relation.direct_matches,
        chain: ExplainChain {
            philosophies: relation.philosophies,
            policies: relation.policies,
            requirements: relation.requirements,
            features: relation.features,
        },
        traces: relation.traces,
        gaps: relation.gaps,
    }
}

fn selector_is_ambiguous(relation: &JsonRelateOutput) -> bool {
    relation.selection.kind != "definition"
        && relation.direct_matches.definitions.len() + relation.direct_matches.traces.len() > 1
}

fn render_explain_text(output: &ExplainOutput) -> String {
    let mut rendered = String::new();
    writeln!(
        rendered,
        "Selection: {} {}",
        output.selection.kind, output.selection.query
    )
    .expect("write to string");
    writeln!(rendered, "Assessment: {}", output.assessment.label()).expect("write to string");
    writeln!(rendered, "{}", output.assessment.summary()).expect("write to string");
    writeln!(rendered).expect("write to string");

    writeln!(rendered, "Connected chain:").expect("write to string");
    render_nodes(&mut rendered, "Philosophies", &output.chain.philosophies);
    render_nodes(&mut rendered, "Policies", &output.chain.policies);
    render_nodes(&mut rendered, "Requirements", &output.chain.requirements);
    render_nodes(&mut rendered, "Features", &output.chain.features);
    writeln!(rendered).expect("write to string");

    writeln!(rendered, "Direct matches:").expect("write to string");
    if output.direct_matches.definitions.is_empty() && output.direct_matches.traces.is_empty() {
        writeln!(rendered, "- none").expect("write to string");
    } else {
        for node in &output.direct_matches.definitions {
            writeln!(rendered, "- {} {} — {}", node.kind, node.id, node.title)
                .expect("write to string");
        }
        for trace in &output.direct_matches.traces {
            let symbols = if trace.symbols.is_empty() {
                "file-only".to_string()
            } else {
                trace.symbols.join(", ")
            };
            writeln!(
                rendered,
                "- {} {} {} {}\t{} ({symbols})",
                trace.owner_kind, trace.owner_id, trace.relation_kind, trace.language, trace.file
            )
            .expect("write to string");
        }
    }
    writeln!(rendered).expect("write to string");

    writeln!(rendered, "Traces in scope:").expect("write to string");
    if output.traces.is_empty() {
        writeln!(rendered, "- none").expect("write to string");
    } else {
        for trace in &output.traces {
            let symbols = if trace.symbols.is_empty() {
                "file-only".to_string()
            } else {
                trace.symbols.join(", ")
            };
            let direct = if trace.direct_match {
                " (direct match)"
            } else {
                ""
            };
            writeln!(
                rendered,
                "- {} {} {} {}\t{} ({symbols}){direct}",
                trace.owner_kind, trace.owner_id, trace.relation_kind, trace.language, trace.file
            )
            .expect("write to string");
        }
    }
    writeln!(rendered).expect("write to string");

    writeln!(rendered, "Obvious gaps:").expect("write to string");
    if output.gaps.is_empty() {
        writeln!(rendered, "- none").expect("write to string");
    } else {
        for gap in &output.gaps {
            writeln!(rendered, "- {} {} — {}", gap.kind, gap.id, gap.message)
                .expect("write to string");
        }
    }

    rendered
}

fn render_nodes(rendered: &mut String, label: &str, nodes: &[RelatedNode]) {
    writeln!(rendered, "{label}:").expect("write to string");
    if nodes.is_empty() {
        writeln!(rendered, "- none").expect("write to string");
        return;
    }

    for node in nodes {
        writeln!(
            rendered,
            "- {} {} — {} ({})",
            node.kind, node.id, node.title, node.document_path
        )
        .expect("write to string");
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DirectMatches, ExplainAssessment, Gap, JsonRelateOutput, RelatedNode, RelatedTrace,
        SelectionSummary, build_explain_output, render_explain_text,
    };

    #[test]
    fn assessment_labels_stay_stable() {
        assert_eq!(ExplainAssessment::Aligned.label(), "aligned");
        assert_eq!(ExplainAssessment::Ambiguous.label(), "ambiguous");
        assert_eq!(ExplainAssessment::NeedsAttention.label(), "needs-attention");
        assert!(
            ExplainAssessment::NeedsAttention
                .summary()
                .contains("needs review")
        );
    }

    #[test]
    fn explain_marks_ambiguous_non_definition_selectors() {
        let output = build_explain_output(JsonRelateOutput {
            selection: SelectionSummary {
                kind: "symbol",
                query: "shared_selector".to_string(),
            },
            direct_matches: DirectMatches {
                definitions: Vec::new(),
                traces: vec![
                    RelatedTrace {
                        owner_kind: "requirement",
                        owner_id: "REQ-TRACE-001".to_string(),
                        relation_kind: "test",
                        language: "rust".to_string(),
                        file: "src/rust_trace_tests.rs".to_string(),
                        symbols: vec!["shared_selector".to_string()],
                        direct_match: true,
                    },
                    RelatedTrace {
                        owner_kind: "feature",
                        owner_id: "FEAT-TRACE-001".to_string(),
                        relation_kind: "implementation",
                        language: "rust".to_string(),
                        file: "src/rust_feature.rs".to_string(),
                        symbols: vec!["shared_selector".to_string()],
                        direct_match: true,
                    },
                ],
            },
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
            traces: Vec::new(),
            gaps: Vec::new(),
        });

        let rendered = render_explain_text(&output);
        assert_eq!(output.assessment.label(), "ambiguous");
        assert!(rendered.contains("Assessment: ambiguous"));
        assert!(rendered.contains("multiple direct candidates"));
    }

    #[test]
    fn explain_text_handles_gaps_and_empty_sections() {
        let output = build_explain_output(JsonRelateOutput {
            selection: SelectionSummary {
                kind: "symbol",
                query: "missing_selector".to_string(),
            },
            direct_matches: DirectMatches::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
            traces: Vec::new(),
            gaps: vec![Gap {
                kind: "requirement",
                id: "REQ-GAP-001".to_string(),
                message: "missing trace coverage".to_string(),
            }],
        });

        let rendered = render_explain_text(&output);
        assert_eq!(output.assessment.label(), "needs-attention");
        assert!(rendered.contains("Assessment: needs-attention"));
        assert!(rendered.contains("Direct matches:\n- none"));
        assert!(rendered.contains("Traces in scope:\n- none"));
        assert!(
            rendered.contains("Obvious gaps:\n- requirement REQ-GAP-001 — missing trace coverage")
        );
    }

    #[test]
    fn explain_text_renders_file_only_traces_and_definition_matches() {
        let output = build_explain_output(JsonRelateOutput {
            selection: SelectionSummary {
                kind: "path",
                query: "README.md".to_string(),
            },
            direct_matches: DirectMatches {
                definitions: vec![RelatedNode {
                    kind: "feature",
                    id: "FEAT-DOCS-001".to_string(),
                    title: "Docs feature".to_string(),
                    document_path: "docs/syu/features/docs.yaml".to_string(),
                }],
                traces: vec![RelatedTrace {
                    owner_kind: "feature",
                    owner_id: "FEAT-DOCS-001".to_string(),
                    relation_kind: "implementation",
                    language: "markdown".to_string(),
                    file: "README.md".to_string(),
                    symbols: Vec::new(),
                    direct_match: true,
                }],
            },
            philosophies: vec![RelatedNode {
                kind: "philosophy",
                id: "PHIL-001".to_string(),
                title: "Stay explicit".to_string(),
                document_path: "docs/syu/philosophy/foundation.yaml".to_string(),
            }],
            policies: Vec::new(),
            requirements: Vec::new(),
            features: vec![RelatedNode {
                kind: "feature",
                id: "FEAT-DOCS-001".to_string(),
                title: "Docs feature".to_string(),
                document_path: "docs/syu/features/docs.yaml".to_string(),
            }],
            traces: vec![RelatedTrace {
                owner_kind: "feature",
                owner_id: "FEAT-DOCS-001".to_string(),
                relation_kind: "implementation",
                language: "markdown".to_string(),
                file: "README.md".to_string(),
                symbols: Vec::new(),
                direct_match: true,
            }],
            gaps: Vec::new(),
        });

        let rendered = render_explain_text(&output);
        assert!(rendered.contains("feature FEAT-DOCS-001 — Docs feature"));
        assert!(rendered.contains("README.md (file-only)"));
        assert!(rendered.contains("(direct match)"));
    }

    #[test]
    fn explain_text_renders_direct_trace_symbols() {
        let output = build_explain_output(JsonRelateOutput {
            selection: SelectionSummary {
                kind: "symbol",
                query: "run_check_command".to_string(),
            },
            direct_matches: DirectMatches {
                definitions: Vec::new(),
                traces: vec![RelatedTrace {
                    owner_kind: "feature",
                    owner_id: "FEAT-CHECK-001".to_string(),
                    relation_kind: "implementation",
                    language: "rust".to_string(),
                    file: "src/command/check.rs".to_string(),
                    symbols: vec!["run_check_command".to_string()],
                    direct_match: true,
                }],
            },
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
            traces: Vec::new(),
            gaps: Vec::new(),
        });

        let rendered = render_explain_text(&output);
        assert!(rendered.contains("run_check_command"));
        assert!(rendered.contains("src/command/check.rs"));
    }
}
