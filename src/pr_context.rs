use mitase_validation::pr_context::{ChangeKind, DisplayReason, PrContextReport};

pub(crate) fn changed_paths(
    repo: &std::path::Path,
    base: &str,
    head: &str,
) -> anyhow::Result<Vec<mitase_validation::pr_context::ChangedPath>> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-status", "-M", "-z", base, head, "--"])
        .current_dir(repo)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "git diff failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let fields = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let mut changes = Vec::new();
    let mut index = 0;
    while index < fields.len() {
        let status = String::from_utf8_lossy(fields[index]).to_string();
        index += 1;
        let status_char = status.chars().next().unwrap_or('?');
        let path = |bytes: &[u8]| String::from_utf8_lossy(bytes).replace('\\', "/");
        if matches!(status_char, 'R' | 'C') && index + 1 < fields.len() {
            let old_path = path(fields[index]);
            let new_path = path(fields[index + 1]);
            index += 2;
            changes.push(mitase_validation::pr_context::ChangedPath {
                path: new_path,
                old_path: Some(old_path),
                status: ChangeKind::Renamed,
            });
        } else if index < fields.len() {
            let path = path(fields[index]);
            index += 1;
            let status = match status_char {
                'A' => ChangeKind::Added,
                'D' => ChangeKind::Deleted,
                'M' | 'T' => ChangeKind::Modified,
                _ => ChangeKind::Unknown,
            };
            changes.push(mitase_validation::pr_context::ChangedPath {
                path,
                old_path: None,
                status,
            });
        }
    }
    Ok(changes)
}

pub(crate) fn render_markdown(report: &PrContextReport) -> String {
    let mut out = String::new();
    out.push_str("# PR Context Report\n\n");
    out.push_str(&format!(
        "Base: `{}`  \nHead: `{}`  \nInventory profile: `{}`\n\n",
        report.revision.base_sha, report.revision.head_sha, report.revision.inventory_profile
    ));
    out.push_str("## Change Summary\n\n");
    if report.changed_artifacts.is_empty() {
        out.push_str("No changed artifacts.\n\n");
    }
    for item in &report.changed_artifacts {
        let label = item
            .symbol
            .as_deref()
            .map(|s| format!(" `{s}`"))
            .unwrap_or_default();
        out.push_str(&format!("- `{}`{} — {:?}\n", item.path, label, item.status));
        if let Some(old) = &item.old_path {
            out.push_str(&format!("  - Previous path or identity: `{old}`\n"));
        }
        if let Some(identity) = &item.base_identity {
            out.push_str(&format!("  - Base identity: `{identity}`\n"));
        }
        if let Some(identity) = &item.head_identity {
            out.push_str(&format!("  - Head identity: `{identity}`\n"));
        }
        out.push_str(&format!("  - Binding: {}\n", item.binding_state));
    }
    section(&mut out, "Direct Impact", report, DisplayReason::Direct);
    section(
        &mut out,
        "Upstream Specifications",
        report,
        DisplayReason::Upstream,
    );
    section(&mut out, "Always Review", report, DisplayReason::Always);
    out.push_str("## Verification Evidence\n\n");
    if report.verification_evidence.is_empty() {
        out.push_str("No verification claims were found for affected criteria.\n\n");
    }
    for item in &report.verification_evidence {
        let assessment_label = |assessment: &Option<mitase_validation::VerificationAssessment>| {
            assessment.as_ref().map_or_else(
                || "missing".to_owned(),
                |assessment| format!("{:?} ({:?})", assessment.status, assessment.reason),
            )
        };
        out.push_str(&format!(
            "- Criterion `{}` — claim `{}`: base `{}`, head `{}`{}\n",
            item.criterion,
            serde_json::to_string(&item.claim).unwrap_or_default(),
            assessment_label(&item.base_assessment),
            assessment_label(&item.head_assessment),
            if item.evidence_lost {
                " (**evidence lost**)"
            } else {
                ""
            }
        ));
        if let Some(runner) = &item.runner {
            out.push_str(&format!(
                "  - Runner: `{}`; claim arguments `{}`; base config `{}`; head config `{}`\n",
                runner.id,
                serde_json::to_string(&runner.claim_arguments).unwrap_or_default(),
                serde_json::to_string(&runner.base).unwrap_or_default(),
                serde_json::to_string(&runner.head).unwrap_or_default(),
            ));
        }
    }
    out.push_str("\n> A valid declaration describes resolvable evidence; it does not mean a test ran or passed.\n\n");
    out.push_str("## Evidence Gaps\n\n");
    if report.evidence_gaps.is_empty() {
        out.push_str("No evidence gaps were identified.\n");
    }
    for gap in &report.evidence_gaps {
        let target = gap
            .path
            .as_deref()
            .or(gap.id.as_deref())
            .unwrap_or("repository");
        out.push_str(&format!(
            "- **{}** `{}` — {}\n",
            gap.kind, target, gap.message
        ));
    }
    out
}

fn section(out: &mut String, title: &str, report: &PrContextReport, reason: DisplayReason) {
    out.push_str(&format!("## {title}\n\n"));
    let selected = report
        .related_specifications
        .iter()
        .filter(|item| item.reasons.contains(&reason))
        .collect::<Vec<_>>();
    if selected.is_empty() {
        out.push_str("None.\n\n");
        return;
    }
    for item in selected {
        out.push_str(&format!(
            "- **{}** `{}` — {}\n",
            item.title, item.id, item.kind
        ));
        for evidence in &item.evidence {
            if evidence
                .relation
                .first()
                .is_some_and(|first| first.starts_with("review:"))
                && reason != DisplayReason::Always
            {
                continue;
            }
            if reason == DisplayReason::Always && evidence.rule_id.is_none() {
                continue;
            }
            out.push_str(&format!("  - via `{}`", evidence.changed_path));
            if !evidence.relation.is_empty() {
                out.push_str(&format!(" → {}", evidence.relation.join(" → ")));
            }
            if let Some(rule) = &evidence.rule_id {
                out.push_str(&format!(" (rule `{rule}`)"));
            }
            out.push('\n');
        }
    }
    out.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;
    use mitase_validation::pr_context::{PrContextReport, RevisionContext};

    #[test]
    fn markdown_sections_are_rendered_from_typed_report() {
        let report = PrContextReport {
            schema_version: "mitase/cli/v1".into(),
            contract_version: "mitase/pr-context-report/v1".into(),
            revision: RevisionContext {
                base_sha: "base".into(),
                head_sha: "head".into(),
                inventory_profile: "default".into(),
            },
            changed_artifacts: vec![],
            related_specifications: vec![],
            verification_evidence: vec![],
            evidence_gaps: vec![],
        };
        let markdown = render_markdown(&report);
        for heading in [
            "Change Summary",
            "Direct Impact",
            "Upstream Specifications",
            "Always Review",
            "Verification Evidence",
            "Evidence Gaps",
        ] {
            assert!(markdown.contains(&format!("## {heading}")));
        }
    }
}
