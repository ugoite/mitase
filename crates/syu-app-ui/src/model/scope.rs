use std::collections::BTreeMap;
use syu_workbench::{BranchScopeConfidence, BranchScopeReport, OwnershipStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceSource {
    BranchDiff,
    ActiveGoal,
    ItemDriven,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplementationSlice {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub rationale: String,
    pub source: SliceSource,
    pub confidence: BranchScopeConfidence,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub files: Vec<String>,
    pub symbols: Vec<String>,
    pub tests: Vec<String>,
    pub spec_ids: Vec<String>,
    pub ownership: OwnershipStatus,
    pub evidence: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Default)]
struct SliceGroup {
    title: String,
    files: Vec<String>,
    symbols: Vec<String>,
    spec_ids: Vec<String>,
    statuses: Vec<OwnershipStatus>,
}

pub fn implementation_slices(report: &BranchScopeReport) -> Vec<ImplementationSlice> {
    let mut groups = BTreeMap::<String, SliceGroup>::new();
    for file in &report.changed_files {
        let primary_owner = file.owners.first();
        let key = primary_owner
            .map(|owner| owner.id.clone())
            .unwrap_or_else(|| component_key(&file.file));
        let title = primary_owner
            .map(|owner| owner.title.clone())
            .unwrap_or_else(|| human_component_title(&key));
        let group = groups.entry(key).or_default();
        if group.title.is_empty() {
            group.title = title;
        }
        group.files.push(file.file.clone());
        group.symbols.extend(file.symbols.clone());
        group
            .spec_ids
            .extend(file.owners.iter().map(|owner| owner.id.clone()));
        group.statuses.push(file.status);
    }

    let mut slices = groups.into_values().map(|mut group| {
        group.files.sort();
        group.files.dedup();
        group.symbols.sort();
        group.symbols.dedup();
        group.spec_ids.sort();
        group.spec_ids.dedup();
        let ownership = if group.statuses.contains(&OwnershipStatus::Unowned) {
            OwnershipStatus::Unowned
        } else if group.statuses.contains(&OwnershipStatus::Partial) {
            OwnershipStatus::Partial
        } else {
            OwnershipStatus::Owned
        };
        let rationale = if group.spec_ids.is_empty() {
            format!(
                "These {} changed files share one repository component, but trace ownership is not yet verified.",
                group.files.len()
            )
        } else {
            format!(
                "These {} changed files implement the same traced specification boundary: {}.",
                group.files.len(),
                group.spec_ids.join(", ")
            )
        };
        let mut warnings = report.warnings.clone();
        if ownership != OwnershipStatus::Owned {
            warnings.push("Ownership must be reviewed before assignment.".to_string());
        }
        ImplementationSlice {
            id: String::new(),
            title: group.title,
            summary: format!(
                "Implement one coherent change across {} related file{}.",
                group.files.len(),
                if group.files.len() == 1 { "" } else { "s" }
            ),
            rationale,
            source: SliceSource::BranchDiff,
            confidence: report.confidence,
            include: group.files.clone(),
            exclude: report.suggested_goal_split.exclude.clone(),
            files: group.files,
            symbols: group.symbols,
            tests: report.test_inventory.required_tests.iter()
                .chain(report.test_inventory.linked_tests.iter()).cloned().collect(),
            spec_ids: group.spec_ids,
            ownership,
            evidence: vec![
                format!("branch range {}", report.range),
                format!("suggested split confidence {}", report.suggested_goal_split.confidence),
            ],
            warnings,
        }
    }).collect::<Vec<_>>();
    slices.sort_by(|left, right| {
        (!left.spec_ids.is_empty(), left.files.len(), &left.title)
            .cmp(&(!right.spec_ids.is_empty(), right.files.len(), &right.title))
            .reverse()
    });
    for (index, slice) in slices.iter_mut().enumerate() {
        slice.id = format!("slice-{:02}", index + 1);
    }
    slices
}

fn component_key(path: &str) -> String {
    let parts = path.split('/').collect::<Vec<_>>();
    match parts.as_slice() {
        ["crates", name, ..] => format!("crates/{name}"),
        [first, second, ..] => format!("{first}/{second}"),
        [only] => (*only).to_string(),
        [] => "repository".to_string(),
    }
}

fn human_component_title(component: &str) -> String {
    format!(
        "{} implementation changes",
        component.replace(['/', '_', '-'], " ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use syu_workbench::{BranchScopeEvidence, ChangedFileReport};

    #[test]
    fn groups_files_by_human_change_boundary_instead_of_raw_file() {
        let report = BranchScopeReport::from_evidence(BranchScopeEvidence {
            range: "main...HEAD".to_string(),
            changed_files: vec![
                ChangedFileReport {
                    file: "crates/ui/src/a.rs".to_string(),
                    symbols: vec!["a".to_string()],
                    owners: Vec::new(),
                    status: OwnershipStatus::Unowned,
                    is_spec_file: false,
                },
                ChangedFileReport {
                    file: "crates/ui/src/b.rs".to_string(),
                    symbols: vec!["b".to_string()],
                    owners: Vec::new(),
                    status: OwnershipStatus::Unowned,
                    is_spec_file: false,
                },
            ],
            trace_ownership: Vec::new(),
            spec_items: Vec::new(),
            required_tests: Vec::new(),
            linked_tests: Vec::new(),
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            allowed_ids: Vec::new(),
            unowned_files: vec![
                "crates/ui/src/a.rs".to_string(),
                "crates/ui/src/b.rs".to_string(),
            ],
            ambiguous_files: Vec::new(),
            spec_files: Vec::new(),
            out_of_scope_changes: Vec::new(),
            direct_items: Vec::new(),
            related_items: Vec::new(),
            has_planned_features: false,
        });
        let slices = implementation_slices(&report);
        assert_eq!(slices.len(), 1);
        assert_eq!(slices[0].files.len(), 2);
        assert!(!slices[0].rationale.is_empty());
        assert_eq!(slices[0].confidence, report.confidence);
    }
}
