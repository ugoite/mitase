#![forbid(unsafe_code)]
use anyhow::{Result, bail};
use std::collections::BTreeSet;
use syu_diagnostics::Diagnostic;
use syu_spec_model::{ArtifactBinding, BoundTargetRef, LocalAnchorKind, SpecAnchor};
use syu_work_model::*;
use syu_workspace::{AnchorValue, SpecIndex, SpecWorkspace, resolve_target_with_adapters};

pub fn plan(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    revision: &str,
) -> Result<WorkPlan> {
    if request.schema != WORK_REQUEST_SCHEMA {
        bail!("request schema must be {WORK_REQUEST_SCHEMA}");
    }
    if request.seeds.is_empty() && request.requested_targets.is_empty() {
        return Ok(blocked_plan(
            request,
            workspace,
            revision,
            "SYU-WORK-001",
            "an exact seed is required",
        ));
    }
    let mut criteria = BTreeSet::new();
    for requested in &request.requested_targets {
        if index.target(requested).is_none() {
            return Ok(blocked_plan(
                request,
                workspace,
                revision,
                "SYU-WORK-001",
                format!("requested target {requested} does not resolve"),
            ));
        }
        expand_seed(index, &requested.binding, &mut criteria);
    }
    for seed in &request.seeds {
        match seed {
            WorkSeed::Anchor(a) => {
                if index.anchor(a).is_none() {
                    return Ok(blocked_plan(
                        request,
                        workspace,
                        revision,
                        "SYU-WORK-001",
                        format!("seed {a} does not resolve"),
                    ));
                }
                expand_seed(index, a, &mut criteria);
            }
            WorkSeed::Item(item) => {
                if let Some(anchors) = index.item_anchors.get(&item.0) {
                    for a in anchors {
                        if a.kind == LocalAnchorKind::Criterion {
                            criteria.insert(a.clone());
                        }
                    }
                }
            }
        }
    }
    if criteria.is_empty() {
        return Ok(blocked_plan(
            request,
            workspace,
            revision,
            "SYU-WORK-001",
            "seed does not select a criterion",
        ));
    }
    let mut slices = Vec::new();
    for criterion in criteria {
        let implementations = index
            .criteria_to_implementations
            .get(&criterion)
            .cloned()
            .unwrap_or_default();
        for implementation in implementations {
            if !request.constraints.include_facets.is_empty()
                && index.bindings.get(&implementation).is_some_and(|binding| {
                    !request.constraints.include_facets.contains(&binding.facet)
                })
            {
                continue;
            }
            slices.push(build_slice(
                request,
                workspace,
                index,
                &criterion,
                &implementation,
            )?);
        }
    }
    slices.sort_by(|a, b| a.id.cmp(&b.id));
    if let Some(max) = request.constraints.max_slices
        && slices.len() > max
    {
        let d = Diagnostic::error(
            "SYU-WORK-003",
            format!("{} slices exceed requested maximum {max}", slices.len()),
            "work-request",
        );
        return Ok(WorkPlan {
            schema: WORK_PLAN_SCHEMA.into(),
            id: plan_id(request, revision),
            basis: basis(workspace, revision),
            request: embed(request),
            status: PlanStatus::Blocked,
            slices,
            diagnostics: vec![d],
        });
    }
    if slices.is_empty() {
        return Ok(blocked_plan(
            request,
            workspace,
            revision,
            "SYU-WORK-002",
            "request produced no execution slices",
        ));
    }
    let status = if slices.iter().any(|s| !s.blockers.is_empty()) {
        PlanStatus::Blocked
    } else {
        PlanStatus::Ready
    };
    Ok(WorkPlan {
        schema: WORK_PLAN_SCHEMA.into(),
        id: plan_id(request, revision),
        basis: basis(workspace, revision),
        request: embed(request),
        status,
        slices,
        diagnostics: vec![],
    })
}

fn expand_seed(index: &SpecIndex, seed: &SpecAnchor, criteria: &mut BTreeSet<SpecAnchor>) {
    match seed.kind {
        LocalAnchorKind::Criterion => {
            criteria.insert(seed.clone());
        }
        LocalAnchorKind::Binding => {
            if let Some(b) = index.bindings.get(seed) {
                criteria.extend(b.satisfies.iter().chain(&b.verifies).cloned());
            }
        }
        LocalAnchorKind::Contract => {
            if let Some(c) = index.contracts.get(seed) {
                for p in &c.participants {
                    if let Some(b) = index.bindings.get(&p.binding) {
                        criteria.extend(b.satisfies.iter().cloned());
                    }
                }
            }
        }
        _ => {}
    }
}
fn build_slice(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    criterion: &SpecAnchor,
    implementation: &SpecAnchor,
) -> Result<ExecutionSlice> {
    let binding = index.bindings.get(implementation).expect("indexed binding");
    let mut blockers = vec![];
    let mut editable = targets(
        workspace,
        implementation,
        binding,
        "Primary implementation satisfying the selected criterion.",
        &mut blockers,
    );
    if !request.requested_targets.is_empty() {
        editable.retain(|target| request.requested_targets.contains(&target.reference));
    }
    let mut verification = vec![];
    for anchor in index
        .criteria_to_verifications
        .get(criterion)
        .into_iter()
        .flatten()
    {
        if let Some(b) = index.bindings.get(anchor) {
            verification.extend(targets(
                workspace,
                anchor,
                b,
                "Direct verification of the selected criterion.",
                &mut blockers,
            ));
        }
    }
    let mut readonly = vec![];
    let mut contracts = vec![];
    for contract_anchor in index
        .binding_to_contracts
        .get(implementation)
        .into_iter()
        .flatten()
    {
        contracts.push(contract_anchor.clone());
        if let Some(contract) = index.contracts.get(contract_anchor) {
            if let Some(target) = index.target(&contract.source)
                && let Some(source_binding) = index.bindings.get(&contract.source.binding)
            {
                readonly.extend(one_target(
                    workspace,
                    &contract.source,
                    source_binding,
                    target,
                    "Contract source constraining this implementation.",
                    &mut blockers,
                ));
            }
            for participant in &contract.participants {
                if &participant.binding != implementation
                    && let Some(other) = index.bindings.get(&participant.binding)
                {
                    readonly.extend(targets(
                        workspace,
                        &participant.binding,
                        other,
                        "Contract counterpart; readonly in this slice.",
                        &mut blockers,
                    ));
                }
            }
        }
    }
    dedup(&mut editable);
    dedup(&mut verification);
    dedup(&mut readonly);
    for values in [&mut editable, &mut verification, &mut readonly] {
        values.retain(|target| {
            !request
                .constraints
                .exclude_paths
                .iter()
                .any(|pattern| path_matches(pattern, &target.resolved_path))
        });
    }
    if request.operation == WorkOperation::Investigate {
        readonly.append(&mut editable);
        dedup(&mut readonly);
    }
    let criterion_value = match index.anchor(criterion) {
        Some(AnchorValue::Criterion(c)) => c,
        _ => unreachable!(),
    };
    let mut anchors = vec![criterion.clone(), implementation.clone()];
    anchors.extend(contracts.clone());
    if workspace.config.work.context.include_parent_rules {
        for rule in index.criteria_to_rules.get(criterion).into_iter().flatten() {
            anchors.push(rule.clone());
            if workspace.config.work.context.include_parent_principles {
                anchors.extend(
                    index
                        .rules_to_principles
                        .get(rule)
                        .into_iter()
                        .flatten()
                        .cloned(),
                );
            }
        }
    }
    anchors.sort();
    anchors.dedup();
    let editable_files = editable
        .iter()
        .map(|t| &t.resolved_path)
        .collect::<BTreeSet<_>>()
        .len();
    let editable_symbols = editable
        .iter()
        .map(|t| t.resolved_selector.symbols.len())
        .sum();
    let total_bytes = editable
        .iter()
        .chain(&verification)
        .chain(&readonly)
        .map(|t| t.byte_end.saturating_sub(t.byte_start))
        .sum();
    let budget = SliceBudgetUsage {
        editable_files,
        editable_symbols,
        verification_targets: verification.len(),
        readonly_targets: readonly.len(),
        total_bytes,
    };
    let limits = &workspace.config.work.slicing;
    if editable_files > limits.max_editable_files
        || editable_symbols > limits.max_editable_symbols
        || verification.len() > limits.max_verification_targets
        || readonly.len() > limits.max_readonly_targets
        || total_bytes > limits.max_total_bytes
    {
        blockers.push(Diagnostic::error(
            "SYU-WORK-003",
            "slice exceeds configured budget",
            "work-plan",
        ));
    }
    let id = format!("{}-{}", criterion.local_id, implementation.local_id);
    let completion = completion_checks(&verification);
    Ok(ExecutionSlice {
        id,
        goal: format!("{}: {}", request.summary, binding.responsibility),
        anchors,
        editable_targets: editable,
        verification_targets: verification,
        readonly_context: readonly,
        acceptance: vec![AcceptanceRef {
            anchor: criterion.clone(),
            statement: criterion_value.statement.clone(),
        }],
        contracts,
        non_goals: vec![
            "Do not modify readonly contract counterparts or unrelated sibling bindings.".into(),
        ],
        completion,
        budget,
        confidence: PlanConfidence::Exact,
        blockers,
    })
}
fn completion_checks(verification: &[PlannedTarget]) -> Vec<CompletionCheck> {
    let mut checks = verification
        .iter()
        .filter_map(|target| {
            target.resolved_selector.symbols.first().and_then(|symbol| {
                if !symbol
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                {
                    return None;
                }
                let command = match target.adapter.as_str() {
                    "rust" => format!("cargo test {symbol}"),
                    "typescript" => format!("npm test -- {symbol}"),
                    "python" => format!("pytest -k {symbol}"),
                    "go" => format!("go test ./... -run {symbol}"),
                    "shell" => format!("bash -n {}", target.resolved_path),
                    _ => return None,
                };
                Some(CompletionCheck::Command { command })
            })
        })
        .collect::<Vec<_>>();
    checks.push(CompletionCheck::Validate {
        preset: "agent-ready".into(),
    });
    checks
}
fn path_matches(pattern: &str, path: &str) -> bool {
    pattern
        .strip_suffix("/**")
        .map_or(pattern == path, |prefix| {
            path == prefix || path.starts_with(&format!("{prefix}/"))
        })
}
fn targets(
    workspace: &SpecWorkspace,
    anchor: &SpecAnchor,
    binding: &ArtifactBinding,
    reason: &str,
    blockers: &mut Vec<Diagnostic>,
) -> Vec<PlannedTarget> {
    binding
        .targets
        .iter()
        .flat_map(|t| {
            one_target(
                workspace,
                &BoundTargetRef {
                    binding: anchor.clone(),
                    target_id: t.id.clone(),
                },
                binding,
                t,
                reason,
                blockers,
            )
        })
        .collect()
}
fn one_target(
    workspace: &SpecWorkspace,
    reference: &BoundTargetRef,
    binding: &ArtifactBinding,
    target: &syu_spec_model::ArtifactTarget,
    reason: &str,
    blockers: &mut Vec<Diagnostic>,
) -> Vec<PlannedTarget> {
    match resolve_target_with_adapters(&workspace.root, target, &workspace.config.adapters.enabled)
    {
        Ok(r) => vec![PlannedTarget {
            reference: reference.clone(),
            resolved_path: r.path.to_string_lossy().into_owned(),
            resolved_selector: ResolvedSelector {
                description: r.description,
                symbols: r.symbols,
            },
            content_hash: r.content_hash,
            excerpt_hash: r.excerpt_hash,
            adapter: target.adapter.clone(),
            facet: binding.facet.clone(),
            role: binding.role,
            byte_start: r.byte_start,
            byte_end: r.byte_end,
            line_start: r.line_start,
            line_end: r.line_end,
            reason: reason.into(),
        }],
        Err(e) => {
            let mut d = Diagnostic::error(
                "SYU-TARGET-002",
                e.to_string(),
                target.path.to_string_lossy(),
            );
            d.target = Some(reference.clone());
            blockers.push(d);
            vec![]
        }
    }
}
fn dedup(values: &mut Vec<PlannedTarget>) {
    values.sort_by(|a, b| a.reference.cmp(&b.reference));
    values.dedup_by(|a, b| a.reference == b.reference);
}
fn basis(workspace: &SpecWorkspace, revision: &str) -> PlanBasis {
    PlanBasis {
        revision: revision.into(),
        workspace_fingerprint: workspace.fingerprint(),
    }
}
fn embed(r: &WorkRequest) -> EmbeddedRequest {
    EmbeddedRequest {
        id: r.id.clone(),
        summary: r.summary.clone(),
        operation: r.operation,
        seeds: r.seeds.clone(),
    }
}
fn plan_id(r: &WorkRequest, revision: &str) -> String {
    format!(
        "PLAN-{}-{}",
        r.id.trim_start_matches("WORK-"),
        &revision.chars().take(8).collect::<String>()
    )
}
fn blocked_plan(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    revision: &str,
    rule: &str,
    message: impl Into<String>,
) -> WorkPlan {
    WorkPlan {
        schema: WORK_PLAN_SCHEMA.into(),
        id: plan_id(request, revision),
        basis: basis(workspace, revision),
        request: embed(request),
        status: PlanStatus::Blocked,
        slices: vec![],
        diagnostics: vec![Diagnostic::error(rule, message, "work-request")],
    }
}

pub fn export_context(
    plan: &WorkPlan,
    slice: &ExecutionSlice,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
) -> ContextPack {
    let mut spec_context = Vec::new();
    for anchor in &slice.anchors {
        let text = match index.anchor(anchor) {
            Some(AnchorValue::Principle(v)) => &v.statement,
            Some(AnchorValue::Rule(v)) => &v.statement,
            Some(AnchorValue::Criterion(v)) => &v.statement,
            Some(AnchorValue::Binding(v)) => &v.responsibility,
            Some(AnchorValue::Contract(_)) | None => continue,
        };
        spec_context.push(SpecExcerpt {
            anchor: anchor.clone(),
            text: text.clone(),
        });
    }
    let mut artifact_context = Vec::new();
    let mut included = BTreeSet::new();
    for (mode, targets) in [
        (ContextMode::Editable, &slice.editable_targets),
        (ContextMode::Verification, &slice.verification_targets),
        (ContextMode::Readonly, &slice.readonly_context),
    ] {
        for target in targets {
            if !included.insert(target.reference.clone()) {
                continue;
            }
            let excerpt = index
                .target(&target.reference)
                .and_then(|declared| {
                    resolve_target_with_adapters(
                        &workspace.root,
                        declared,
                        &workspace.config.adapters.enabled,
                    )
                    .ok()
                })
                .map(|resolved| resolved.excerpt)
                .unwrap_or_default();
            artifact_context.push(ArtifactExcerpt {
                reference: target.reference.clone(),
                mode,
                excerpt,
            });
        }
    }
    ContextPack {
        schema: CONTEXT_PACK_SCHEMA.into(),
        plan: plan.id.clone(),
        slice: slice.id.clone(),
        basis: plan.basis.clone(),
        instructions: ContextInstructions {
            goal: slice.goal.clone(),
            non_goals: slice.non_goals.clone(),
        },
        spec_context,
        artifact_context,
        completion: slice.completion.clone(),
    }
}
