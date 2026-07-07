#![forbid(unsafe_code)]

use std::fmt::Write;
use syu_diagnostics::{Diagnostic, Severity};
use syu_work_model::{
    ExecutionSlice, PlanStatus, PlannedTarget, TargetAccessMode, TargetTransition,
};
use syu_workbench_server::{ValidationDiagnosticView, ValidationRunState, WorkspaceProjection};

/// Browser-facing read model. Domain parsing, target resolution, planning and
/// validation remain server responsibilities.
pub struct WorkbenchView<'a> {
    projection: &'a WorkspaceProjection,
}

impl<'a> WorkbenchView<'a> {
    pub fn new(projection: &'a WorkspaceProjection) -> Self {
        Self { projection }
    }
    pub fn slices(&self) -> impl Iterator<Item = &'a ExecutionSlice> {
        self.projection
            .plan
            .iter()
            .flat_map(|plan| plan.slices.iter())
    }
    pub fn diagnostics(&self) -> impl Iterator<Item = &'a Diagnostic> {
        self.projection
            .validation
            .diagnostics
            .iter()
            .map(|d| &d.diagnostic)
    }
    pub fn editable(slice: &'a ExecutionSlice) -> impl Iterator<Item = &'a PlannedTarget> {
        slice.editable_targets.iter()
    }
    pub fn verification(slice: &'a ExecutionSlice) -> impl Iterator<Item = &'a PlannedTarget> {
        slice.verification_targets.iter()
    }
    pub fn readonly(slice: &'a ExecutionSlice) -> impl Iterator<Item = &'a PlannedTarget> {
        slice.readonly_context.iter()
    }

    pub fn render_html(&self) -> String {
        let mut out = String::with_capacity(48_000);
        out.push_str(HEAD);
        write!(out, "<body><div class=app><aside class=sidebar><a class=brand href=\"/?page=work\"><b>syu</b><span>WORKBENCH</span></a><nav>{}</nav><div class=sidebar-bottom><a data-nav=settings href=\"/?page=settings\">⚙ <span>Settings</span></a><div class=workspace><small>WORKSPACE</small><b>{}</b><span title=\"{}\">{}</span></div></div></aside>", role_links(), esc(&self.projection.workspace.root), esc(&self.projection.workspace.revision), short(&self.projection.workspace.revision)).unwrap();
        out.push_str("<main><header class=topbar><button class=palette data-open-palette><kbd>⌘ K</kbd><span>Search work, slices, items, targets, validation…</span></button><div class=filters><button data-palette-filter=\"\">All</button><button data-palette-filter=navigate>Navigate</button><button data-palette-filter=plan>Plan</button><button data-palette-filter=validate>Validate</button><button data-palette-filter=configure>Config</button></div></header><div id=page class=content>");
        out.push_str(&self.render_work());
        out.push_str("</div></main></div>");
        out.push_str(PALETTE);
        out.push_str(EDITOR);
        out.push_str(REQUEST_EDITOR);
        out.push_str("<div class=toast role=status aria-live=polite></div>");
        write!(
            out,
            "<script>window.SYU_I18N={{en:{},ja:{}}}</script>",
            include_str!("../assets/locales/en.json"),
            include_str!("../assets/locales/ja.json")
        )
        .unwrap();
        out.push_str(SCRIPT);
        out.push_str("</body></html>");
        out
    }

    fn render_work(&self) -> String {
        let Some(plan) = &self.projection.plan else {
            let mut out = String::from(
                "<section data-page=work><div class=eyebrow>WORKBENCH</div><h1>Work</h1><div class=empty><h2>No work request selected</h2><p>Create Work from an exact Requirement criterion so the planner receives a canonical seed.</p><button class=primary data-action=new-work>New work</button></div></section>",
            );
            out.push_str(&self.render_scope());
            out.push_str(&self.render_items());
            out.push_str(&self.render_diagnostics());
            out.push_str(&self.render_settings());
            return out;
        };
        let status = match plan.status {
            PlanStatus::Ready => "Ready",
            PlanStatus::NeedsReview => "Needs review",
            PlanStatus::Blocked => "Blocked",
        };
        let mut out = String::new();
        write!(out, "<section data-page=work data-request='{}'><div class=eyebrow>WORKBENCH</div><div class=title><h1>Work</h1><span class=status data-status=\"{}\" role=status aria-label=\"Plan status: {}\">● {}</span></div><div class=toolbar><select aria-label=\"Work plan\"><option>{} · {}</option></select><button data-action=edit-request>Edit request</button><button class=primary data-action=replan>Replan</button></div>{}<article class=canvas data-work-panel=overview><h2>{}</h2><p>{}</p><div class=chips><span>{:?}</span><span>{} isolated slices</span><span>basis {}</span></div><div class=summary><section><small>INTENT</small><h3>What this work changes</h3><p>{}</p></section><section><small>EXECUTION</small><h3>Independent slices</h3><p>Each slice has exact editable, verification and readonly targets and must run in an isolated branch or worktree.</p></section></div></article><article class=canvas data-work-panel=slices hidden><h2>Execution slices</h2><p>Each slice is independently executable and has explicit acceptance, non-goals, budgets, and completion checks.</p><div class=slices>", esc(&serde_json::to_string(&plan.request).unwrap_or_default()), status, status, status, esc(&plan.id), esc(&plan.request.summary), work_tabs(plan.slices.len(), plan.diagnostics.len()), esc(&plan.request.summary), esc(&format!("Exact {:?} work projected from canonical specification anchors.", plan.request.operation)), plan.request.operation, plan.slices.len(), short(&plan.basis.revision), esc(&plan.request.summary)).unwrap();
        for slice in &plan.slices {
            out.push_str(&render_slice(slice));
        }
        out.push_str("</div></article>");
        out.push_str(&render_context_panel(plan));
        out.push_str(&render_work_validation_panel(plan));
        out.push_str("</section>");
        out.push_str(&self.render_scope());
        out.push_str(&self.render_items());
        out.push_str(&self.render_diagnostics());
        out.push_str(&self.render_settings());
        out
    }

    fn render_scope(&self) -> String {
        let mut out = String::from(
            "<section data-page=scope hidden><div class=eyebrow>WORKBENCH</div><h1>Scope</h1><p class=lead>Exact PlannedTargets from the selected WorkPlan and ExecutionSlice.</p><div class=tabs><button class=active data-scope-tab=editable>Change</button><button data-scope-tab=verification>Verify</button><button data-scope-tab=readonly>Reference</button><button data-scope-tab=intent>Intent</button></div><div class=target-grid>",
        );
        if let Some(slice) = self.slices().next() {
            for target in slice
                .editable_targets
                .iter()
                .chain(&slice.verification_targets)
                .chain(&slice.readonly_context)
            {
                out.push_str(&render_target(target));
            }
        }
        out.push_str("</div></section>");
        out
    }

    fn render_items(&self) -> String {
        let mut out = String::from(
            "<section data-page=items hidden><div class=eyebrow>WORKBENCH</div><h1>Items</h1><div class=toolbar><input data-item-search placeholder=\"Search specification items, anchors, bindings, or targets\"><button class=primary data-action=new-requirement>+ New requirement</button></div><div class=tabs><button data-item-kind=philosophy>Philosophy</button><button data-item-kind=policy>Policy</button><button class=active data-item-kind=requirement>Requirement</button><button data-item-kind=feature>Feature</button></div><div class=item-list>",
        );
        for item in &self.projection.items {
            write!(out, "<article data-item-id=\"{}\" data-item-path=\"{}\" data-kind=\"{}\"><div><b>{}</b><small>{}</small></div><div class=counts><span>{} criteria</span><span>{} bindings</span><span>{} contracts</span></div><button data-action=edit-item>Edit</button><button data-action=create-work {}>Create work</button></article>", esc(&item.id), esc(&item.path), esc(&item.kind), esc(&item.id), esc(&item.path), item.criteria, item.bindings, item.contracts, if item.criteria > 0 { "" } else { "disabled title=\"Select an exact downstream criterion or binding\"" }).unwrap();
        }
        out.push_str("</div><p class=notice>Edits use structured fields and require server-side diff preview, validation, and source-hash confirmation before apply.</p></section>");
        out
    }

    fn render_diagnostics(&self) -> String {
        let mut out = String::from(
            "<section data-page=diagnostics hidden><div class=eyebrow>WORKBENCH</div><h1>Diagnostics</h1><div class=toolbar><select data-validation-context><option value=workspace>Workspace</option><option value=git-range>Git range</option><option value=work-plan>Work plan</option><option value=slice>Slice</option></select><button class=primary data-action=validate>↻ Validate</button></div><div class=tabs><button class=active data-phase=all>All</button><button data-phase=config>Config &amp; Schema</button><button data-phase=spec>Spec Graph</button><button data-phase=target>Artifact Targets</button><button data-phase=change>Change Scope</button><button data-phase=work>Work Plan</button></div><div class=diagnostics>",
        );
        if matches!(self.projection.validation.state, ValidationRunState::NotRun) {
            out.push_str("<div class=empty data-diagnostic-result><h2>Not run</h2><p>Choose a context and run validation. Issue counts are shown only after a completed run.</p></div>");
        } else if self.projection.validation.diagnostics.is_empty() {
            out.push_str("<div class=empty data-diagnostic-result><span class=ok>●</span><h2>No issues found</h2><p>Validation completed for the selected context. Applicable and skipped phases are reported separately.</p></div>");
        }
        for diagnostic in &self.projection.validation.diagnostics {
            out.push_str(&render_diagnostic(diagnostic));
        }
        out.push_str("</div></section>");
        out
    }

    fn render_settings(&self) -> String {
        let config = &self.projection.config;
        let spec_roots = config
            .workspace
            .spec_roots
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let artifact_roots = config
            .workspace
            .artifact_roots
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let profiles = config.profiles.active.join(", ");
        let adapters = config.adapters.enabled.join(", ");
        let preset = serde_json::to_value(config.validation.preset)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_else(|| "standard".into());
        format!(
            "<section data-page=settings data-config='{}' hidden><div class=eyebrow>UTILITY</div><h1>Settings</h1><div class=layer-tabs role=tablist><button class=active data-settings-layer=application>Application</button><button data-settings-layer=workspace>Workspace</button></div><div class=toolbar data-settings-toolbar=workspace hidden><code>syu/config/v1 · syu.yaml</code><button data-action=view-config>Raw YAML</button><button data-action=preview-config>Preview</button><button class=primary data-action=apply-config>Apply</button></div><div class=settings data-settings-layer-panel=application><nav><button class=active data-settings-tab=language>Language</button><button data-settings-tab=appearance>Appearance</button><button data-settings-tab=accessibility>Accessibility</button></nav><article><div data-settings-panel=language><h2>Language</h2><p>Choose the language used by navigation, controls, help text, empty states, and accessibility labels.</p><label>Interface language<select data-language-select><option value=en>English</option><option value=ja>日本語</option></select></label><p class=notice>Technical identifiers, file paths, schema names, enum values, and user-authored specification text are never machine-translated.</p></div><div data-settings-panel=appearance hidden><h2>Appearance</h2><p>Choose how the Workbench is rendered on this device.</p><div class=theme-choices><button data-theme-choice=system>System</button><button data-theme-choice=light>Light</button><button data-theme-choice=dark>Dark</button></div></div><div data-settings-panel=accessibility hidden><h2>Accessibility</h2><label class=check><input type=checkbox data-preference=reduce-motion> Reduce motion</label><label class=check><input type=checkbox data-preference=focus-visibility> Increase focus visibility</label><button data-reset-preferences>Reset</button></div></article></div><div class=settings data-settings-layer-panel=workspace hidden><nav><button class=active data-settings-tab=general>General</button><button data-settings-tab=profiles>Profiles</button><button data-settings-tab=validation>Validation</button><button data-settings-tab=planning>Planning</button><button data-settings-tab=adapters>Adapters</button><button data-settings-tab=yaml>Raw YAML</button></nav><article><div data-settings-panel=general><h2>General</h2><label>Specification roots<input data-config-field=spec-roots value=\"{}\"></label><label>Artifact roots<input data-config-field=artifact-roots value=\"{}\"></label></div><div data-settings-panel=profiles hidden><h2>Profiles</h2><label>Active profiles<input data-config-field=profiles value=\"{}\"></label></div><div data-settings-panel=validation hidden><h2>Validation</h2><label>Preset<select data-config-field=preset><option value=standard {}>Standard</option><option value=strict {}>Strict</option><option value=agent-ready {}>Agent ready</option></select></label><label class=check><input type=checkbox data-config-field=deny-warnings {}> Deny warnings</label><label class=check><input type=checkbox data-config-field=require-owned {}> Require owned changes</label></div><div data-settings-panel=planning hidden><h2>Planning</h2><div class=form-grid><label>Editable files<input type=number min=1 data-config-field=max-editable-files value=\"{}\"></label><label>Editable symbols<input type=number min=1 data-config-field=max-editable-symbols value=\"{}\"></label><label>Verification targets<input type=number min=1 data-config-field=max-verification value=\"{}\"></label><label>Readonly targets<input type=number min=1 data-config-field=max-readonly value=\"{}\"></label><label>Total bytes<input type=number min=1 data-config-field=max-bytes value=\"{}\"></label></div></div><div data-settings-panel=adapters hidden><h2>Adapters</h2><label>Enabled adapters<input data-config-field=adapters value=\"{}\"></label></div><div data-settings-panel=yaml hidden><textarea data-config-editor></textarea></div><pre data-preview-output hidden></pre></article></div></section>",
            esc(&serde_json::to_string(config).unwrap_or_default()),
            esc(&spec_roots),
            esc(&artifact_roots),
            esc(&profiles),
            selected(&preset, "standard"),
            selected(&preset, "strict"),
            selected(&preset, "agent-ready"),
            checked(config.validation.deny_warnings),
            checked(config.validation.changed.require_owned_changes),
            config.work.slicing.max_editable_files,
            config.work.slicing.max_editable_symbols,
            config.work.slicing.max_verification_targets,
            config.work.slicing.max_readonly_targets,
            config.work.slicing.max_total_bytes,
            esc(&adapters)
        )
    }
}

fn selected(actual: &str, expected: &str) -> &'static str {
    if actual == expected { "selected" } else { "" }
}
fn checked(value: bool) -> &'static str {
    if value { "checked" } else { "" }
}

fn render_slice(slice: &ExecutionSlice) -> String {
    let mut out = String::new();
    write!(out, "<article class=slice><header><span class=ok>●</span><div><b>{}</b><p>{}</p></div><span class=count>{} editable</span></header><div class=chips><span>{:?}</span><span>{} verification</span><span>{} reference</span><span>{} blockers</span></div><details><summary>Acceptance, non-goals and completion</summary><ul>", esc(&slice.id), esc(&slice.goal), slice.editable_targets.len(), slice.confidence, slice.verification_targets.len(), slice.readonly_context.len(), slice.blockers.len()).unwrap();
    for acceptance in &slice.acceptance {
        write!(
            out,
            "<li>{}: {}</li>",
            esc(&acceptance.anchor.to_string()),
            esc(&acceptance.statement)
        )
        .unwrap();
    }
    for non_goal in &slice.non_goals {
        write!(
            out,
            "<li>Non-goal {}: {}</li>",
            esc(&non_goal.code),
            esc(&non_goal.statement)
        )
        .unwrap();
    }
    out.push_str("</ul></details></article>");
    out
}

fn render_context_panel(plan: &syu_work_model::WorkPlan) -> String {
    let mut out = String::from(
        "<article class=canvas data-work-panel=context hidden><h2>Context Pack</h2><p>Export is allowed only for a Ready plan whose revision, fingerprint, and canonical digest still match.</p><div class=context-actions>",
    );
    for slice in &plan.slices {
        write!(out, "<button data-action=export-context data-slice=\"{}\">Preview {}</button><a class=\"button-link primary\" href=\"/api/context/{}\" download=\"{}-context.yaml\">Download</a>", esc(&slice.id), esc(&slice.id), esc(&slice.id), esc(&slice.id))
        .unwrap();
    }
    out.push_str("</div><pre data-context-output hidden></pre></article>");
    out
}

fn render_work_validation_panel(plan: &syu_work_model::WorkPlan) -> String {
    let mut out = String::from(
        "<article class=canvas data-work-panel=validation hidden><h2>Plan validation</h2><button data-action=validate>Validate current plan</button><div class=diagnostics>",
    );
    if plan.diagnostics.is_empty() {
        out.push_str("<p class=notice>No plan diagnostics.</p>");
    }
    for diagnostic in &plan.diagnostics {
        out.push_str(&render_raw_diagnostic(diagnostic, "plan"));
    }
    out.push_str("</div></article>");
    out
}

fn render_target(target: &PlannedTarget) -> String {
    let access = match target.access {
        TargetAccessMode::Editable => "Editable",
        TargetAccessMode::RunOnly => "Verification",
        TargetAccessMode::Readonly => "Readonly",
    };
    let transition = match target.transition {
        TargetTransition::Add => "Add",
        TargetTransition::Modify => "Modify",
        TargetTransition::Remove => "Remove",
        TargetTransition::RunOnly => "Run only",
        TargetTransition::Readonly => "Readonly",
    };
    format!(
        "<article class=target data-access=\"{}\" data-target-path=\"{}\" data-target-ref=\"{}\"><div class=chips><span>{}</span><span>{}</span><span>{}</span></div><h2>{}</h2><code>{} · lines {}-{}</code><p>{}</p><small>{}</small><footer><button data-action=open-target>Open source</button><button data-action=copy-target>Copy locator</button></footer></article>",
        access.to_ascii_lowercase(),
        esc(&target.resolved_path),
        esc(&target.reference.to_string()),
        access,
        transition,
        esc(&target.facet),
        esc(&target.resolved_selector.description),
        esc(&target.resolved_path),
        target.line_start,
        target.line_end,
        esc(&target.reason),
        esc(&target.reference.to_string())
    )
}

fn render_diagnostic(view: &ValidationDiagnosticView) -> String {
    render_raw_diagnostic(&view.diagnostic, &view.phase)
}

fn render_raw_diagnostic(d: &Diagnostic, phase: &str) -> String {
    let status = match d.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    };
    format!(
        "<article class=diagnostic data-status=\"{}\" data-phase=\"{}\"><header><span>●</span><div><b>{}</b><h2>{}</h2></div></header><code>{}:{}</code><p>{}</p>{}</article>",
        status,
        esc(phase),
        esc(&d.rule_id),
        esc(&d.message),
        esc(&d.primary.path),
        d.primary
            .line
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".into()),
        d.help.as_deref().map(esc).unwrap_or_default(),
        d.fix
            .as_ref()
            .map(|fix| format!(
                "<p class=notice>Safe fix available: {}</p>",
                esc(&fix.description)
            ))
            .unwrap_or_default()
    )
}

fn role_links() -> &'static str {
    "<a class=active data-nav=work href=\"/?page=work\">◉ <span>Work</span></a><a data-nav=scope href=\"/?page=scope\">◇ <span>Scope</span></a><a data-nav=items href=\"/?page=items\">▣ <span>Items</span></a><a data-nav=diagnostics href=\"/?page=diagnostics\">✓ <span>Diagnostics</span></a>"
}
fn work_tabs(slices: usize, diagnostics: usize) -> String {
    format!(
        "<div class=tabs><button class=active data-work-tab=overview>Overview</button><button data-work-tab=slices>Slices <span>{slices}</span></button><button data-work-tab=context>Context</button><button data-work-tab=validation>Validation <span>{diagnostics}</span></button></div>"
    )
}
fn short(value: &str) -> String {
    value.chars().take(9).collect()
}
fn esc(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

const HEAD: &str = r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Syu Workbench</title><style>
:root{font-family:Inter,ui-sans-serif,system-ui;color:#20252c;background:#eef0f2}*{box-sizing:border-box}body{margin:0}.app{display:grid;grid-template-columns:220px 1fr;min-height:100vh}.sidebar{position:sticky;top:0;height:100vh;background:#111418;color:#c9ced5;padding:22px 14px;display:flex;flex-direction:column}.brand{color:white;text-decoration:none;padding:0 12px 26px}.brand b{font-size:26px}.brand span{display:block;font-size:9px;letter-spacing:.2em;color:#858d98}.sidebar nav{display:grid;gap:5px}.sidebar nav a{color:#aab1ba;text-decoration:none;padding:11px 14px;border-radius:8px}.sidebar nav a.active,.sidebar nav a:hover{background:#292e35;color:white}.workspace{margin-top:auto;display:grid;gap:4px;padding:12px;border-top:1px solid #343a42;font-size:11px}.workspace span{color:#7f8792}main{min-width:0}.topbar{height:92px;background:white;border-bottom:1px solid #d6d9dd;padding:14px 28px;display:flex;align-items:flex-start;gap:12px}.palette{width:min(670px,65vw);height:38px;border:1px solid #cfd3d8;border-radius:8px;background:#f7f8f9;text-align:left;color:#69717c}.palette kbd{margin-right:15px;background:white;border:1px solid #d5d8dc;border-radius:5px;padding:3px 7px}.filters{position:absolute;top:57px;display:flex;gap:6px}.filters button,.tabs button{border:0;background:transparent;color:#68717d;padding:5px 9px}.filters button:first-child,.tabs .active{color:#111418;font-weight:700;border-bottom:2px solid #111418}.gear{margin-left:auto;color:#333;text-decoration:none;font-size:20px}.content{padding:28px}.content>section{max-width:1250px;margin:auto}.eyebrow{font-size:10px;letter-spacing:.18em;color:#7d858f}.title{display:flex;align-items:center;gap:14px}h1{font-size:30px;margin:5px 0 16px}h2{margin:5px 0;font-size:20px}h3{font-size:14px}.lead,p{color:#626b76;line-height:1.55}.toolbar{display:flex;gap:9px;margin:12px 0 18px}.toolbar select,.toolbar input,.toolbar code{flex:1}.toolbar select,.toolbar input,.toolbar code,button{border:1px solid #cdd2d7;background:white;border-radius:8px;padding:10px 13px}button{font-weight:650}button:disabled{opacity:.45}.primary{background:#111418;color:white}.tabs{display:flex;gap:10px;border-bottom:1px solid #d4d8dc;margin-bottom:0}.tabs span,.count{border-radius:10px;background:#e5e8eb;padding:2px 7px;font-size:10px}.canvas,.target,.diagnostic,.item-list article,.settings{background:white;border:1px solid #d7dade}.canvas{padding:25px;border-radius:0 0 10px 10px}.chips{display:flex;flex-wrap:wrap;gap:6px;margin:10px 0}.chips span,.status{border:1px solid #d7dade;border-radius:14px;padding:4px 9px;font-size:11px;background:#f7f8f9}.summary{display:grid;grid-template-columns:1fr 1fr;gap:18px;margin:26px 0}.summary section{border-left:2px solid #a9b0b8;padding-left:14px}.summary small{font-size:9px;color:#7b838d;letter-spacing:.15em}.slices{display:grid;gap:8px}.slice{border:1px solid #dce0e3;border-radius:9px;padding:14px}.slice header{display:flex;align-items:flex-start;gap:10px}.slice header div{flex:1}.slice p{margin:3px 0}.ok{color:#1b9b5d}.status[data-status=Blocked],[data-status=error]>header>span{color:#c43c3c}.status[data-status="Needs review"],[data-status=warning]>header>span{color:#d88619}.target-grid,.diagnostics,.item-list{display:grid;gap:10px;margin-top:15px}.target,.diagnostic{border-radius:9px;padding:18px}.target code,.diagnostic code,pre{display:block;background:#f3f5f6;padding:10px;border-radius:6px;font-size:12px;white-space:pre-wrap;overflow:auto}.item-list article{display:flex;align-items:center;gap:18px;padding:14px;border-radius:8px}.item-list article>div:first-child{flex:1}.item-list small{display:block;color:#777}.counts{display:flex;gap:6px}.counts span{font-size:11px;background:#f1f3f4;padding:5px 8px;border-radius:12px}.notice,.empty{margin-top:20px;padding:25px;border:1px dashed #bcc2c8;border-radius:9px;background:#f8f9fa}.settings{display:grid;grid-template-columns:230px 1fr}.settings nav{display:grid;align-content:start;padding:14px;background:#f5f6f7}.settings nav button{text-align:left;border:0;background:transparent}.settings nav .active{background:white}.settings article{padding:24px}.settings label{display:grid;gap:5px;margin:15px 0;font-size:12px}.settings input,.settings textarea{padding:10px;border:1px solid #d4d8dc}.settings textarea{min-height:320px;width:100%;font-family:monospace}.palette-overlay,.editor-overlay{display:none;position:fixed;inset:0;background:#1118;place-items:start center;padding-top:80px;z-index:20}.palette-overlay.open,.editor-overlay.open{display:grid}.palette-dialog,.editor-dialog{width:min(720px,90vw);background:white;border-radius:12px;box-shadow:0 20px 70px #0005;padding:12px}.palette-dialog input{width:100%;border:0;border-bottom:1px solid #ddd;padding:15px;font-size:17px}.editor-dialog header,.editor-dialog footer{display:flex;justify-content:space-between;gap:8px}.editor-dialog label{display:grid;gap:5px;margin:12px 0;font-size:12px}.editor-dialog input,.editor-dialog textarea{width:100%;padding:10px;border:1px solid #ccd1d6;border-radius:6px}.editor-dialog textarea{min-height:360px;font-family:monospace}.editor-dialog footer{justify-content:flex-end}.result{display:block;width:100%;text-align:left;margin-top:8px}.result small{display:block;color:#777}.toast{position:fixed;right:20px;bottom:20px;max-width:420px;background:#111;color:white;padding:12px 16px;border-radius:8px;opacity:0;transform:translateY(8px);transition:.2s;pointer-events:none;z-index:30}.toast.show{opacity:1;transform:none}.focus-ring{outline:3px solid #e54b4b;outline-offset:3px}@media(max-width:760px){.app{grid-template-columns:64px 1fr}.sidebar{padding:16px 7px}.brand span,.sidebar nav span,.workspace{display:none}.sidebar nav a{text-align:center}.topbar{padding:14px}.filters{display:none}.content{padding:16px}.summary{grid-template-columns:1fr}.toolbar{flex-wrap:wrap}.settings{grid-template-columns:1fr}.settings nav{grid-template-columns:1fr 1fr}.counts{display:none}}
.request-overlay{display:none;position:fixed;inset:0;background:#1118;place-items:start center;padding-top:80px;z-index:20}.request-overlay.open{display:grid}.request-dialog{width:min(720px,90vw);background:white;border-radius:8px;padding:18px;border:1px solid #c9ced4}.request-dialog header,.request-dialog footer{display:flex;justify-content:space-between;gap:8px}.request-dialog footer{justify-content:flex-end}.request-dialog label{display:grid;gap:5px;margin:12px 0;font-size:12px}.request-dialog textarea,.request-dialog select{width:100%;padding:10px;border:1px solid #ccd1d6;border-radius:6px}.request-dialog textarea{min-height:110px}
/* Workbench v1 visual system: flat surfaces, compact controls, semantic color only. */
:root{--ink:#171a1f;--muted:#66707c;--line:#d9dde2;--soft:#f5f6f7;--canvas:#eef0f2;--accent:#252a31}body{color:var(--ink);background:var(--canvas);font-size:14px}.sidebar{width:220px;background:#15181d;padding:24px 12px}.sidebar nav a{border-radius:6px;padding:10px 12px;font-weight:590}.sidebar nav a.active{background:#2a2f37}.topbar{height:88px;padding:14px 28px;border-color:var(--line)}.palette{height:38px;border-radius:6px;background:#fafafa}.content{padding:28px 32px 48px}.content>section{max-width:1320px}.eyebrow{font-weight:700;color:#8a929c}.title h1,h1{letter-spacing:-.025em}.toolbar{align-items:center}.toolbar button,.toolbar select,.toolbar input{min-height:38px;border-radius:6px}.primary{background:#1b1e23;border-color:#1b1e23}.primary:hover{background:#30353c}.tabs{gap:18px}.tabs button{padding:10px 2px 9px;border-radius:0}.tabs button:hover{color:var(--ink);background:transparent}.canvas{border-radius:0 0 6px 6px;border-top:0;padding:28px}.slice,.target,.diagnostic,.item-list article{border-radius:6px;background:#fff}.slice:hover,.target:hover,.item-list article:hover{border-color:#aeb5be}.chips span,.status{border-radius:999px;background:#fafafa}.settings{border-radius:6px;overflow:hidden}.settings nav{border-right:1px solid var(--line);gap:2px}.settings nav button{border-radius:4px;padding:10px 12px}.settings nav button.active{background:#fff;border:1px solid var(--line)}.settings article{min-height:500px}.form-grid{display:grid;grid-template-columns:1fr 1fr;gap:14px}.check{display:flex!important;align-items:center;gap:8px}.check input{width:auto}.palette-dialog,.editor-dialog{border:1px solid #c9ced4;border-radius:8px;box-shadow:none;padding:16px}.palette-overlay,.editor-overlay,.request-overlay{backdrop-filter:blur(2px)}button{transition:border-color .12s,background .12s,color .12s}button:hover:not(:disabled){border-color:#929aa5}.toast{border:1px solid #343a42;box-shadow:none}.empty{background:#fafafa;border-style:solid}.notice{border-radius:5px}.focus-ring{outline:2px solid #d04a4a;outline-offset:3px}@media(max-width:760px){.content{padding:18px 14px}.form-grid{grid-template-columns:1fr}.toolbar button{flex:1}.tabs{overflow-x:auto;gap:14px}.canvas{padding:20px}}
.button-link{display:inline-flex;align-items:center;justify-content:center;min-height:38px;padding:9px 13px;border:1px solid #cdd2d7;border-radius:6px;text-decoration:none;font-weight:650}.button-link.primary{color:#fff;background:#1b1e23;border-color:#1b1e23}.target footer{display:flex;gap:8px;margin-top:14px;padding-top:12px;border-top:1px solid var(--line)}
.sidebar{display:flex;flex-direction:column}.sidebar-bottom{margin-top:auto}.sidebar-bottom>a{display:flex;color:#d7dbe0;text-decoration:none;padding:10px 12px;gap:10px;border-radius:6px}.sidebar-bottom>a.active{background:#2a2f37}.layer-tabs{display:flex;gap:4px;border-bottom:1px solid var(--line);margin:18px 0}.layer-tabs button{padding:11px 18px;border:0;background:transparent;border-bottom:2px solid transparent}.layer-tabs button.active{border-bottom-color:var(--ink)}.theme-choices{display:grid;grid-template-columns:repeat(3,1fr);gap:10px}.theme-choices button.active{outline:2px solid #4f76d1}.settings textarea{width:100%;min-height:360px}[hidden]{display:none!important}[data-theme=dark]{color-scheme:dark;--ink:#f2f4f7;--muted:#aab2bd;--line:#3a414c;--soft:#242932;--canvas:#171a1f;--accent:#e7eaf0;background:#171a1f}[data-theme=dark] body,[data-theme=dark] .topbar,[data-theme=dark] .canvas,[data-theme=dark] .slice,[data-theme=dark] .target,[data-theme=dark] .diagnostic,[data-theme=dark] .item-list article,[data-theme=dark] .settings article,[data-theme=dark] .settings nav{background:#1f232a;color:var(--ink)}[data-theme=dark] input,[data-theme=dark] select,[data-theme=dark] textarea,[data-theme=dark] button{background:#282e37;color:var(--ink);border-color:var(--line)}[data-locale=ja]{font-family:"Noto Sans JP","Yu Gothic UI","Yu Gothic",Meiryo,sans-serif}
@media(max-width:760px){.app{display:block;padding-bottom:64px}.sidebar{position:fixed;z-index:20;inset:auto 0 0;width:auto;height:64px;padding:0;background:#15181d}.brand,.workspace{display:none!important}.sidebar nav,.sidebar-bottom{display:contents}.sidebar nav a,.sidebar-bottom>a{position:absolute;bottom:0;width:20%;height:64px;display:grid;place-items:center;font-size:17px;padding:6px}.sidebar nav a span,.sidebar-bottom>a span{display:block;font-size:10px}.sidebar nav a:nth-child(1){left:0}.sidebar nav a:nth-child(2){left:20%}.sidebar nav a:nth-child(3){left:40%}.sidebar nav a:nth-child(4){left:60%}.sidebar-bottom>a{left:80%}.toolbar{display:grid;grid-template-columns:minmax(0,1fr) 1fr}.toolbar select,.toolbar input,.toolbar code{min-width:0;grid-column:1/-1}.toolbar button{min-width:0}.topbar{height:68px}.palette{width:calc(100vw - 40px)}.filters{display:none}.item-list article{display:grid;grid-template-columns:minmax(0,1fr) auto auto;gap:8px;min-width:0;width:100%}.item-list article button{min-width:0;padding-inline:10px}.settings{grid-template-columns:1fr}.settings nav{display:flex;overflow-x:auto}.theme-choices{grid-template-columns:1fr}}
</style></head>"#;
const PALETTE: &str = r#"<div class=palette-overlay role=dialog aria-modal=true aria-label="Command palette"><div class=palette-dialog><input autofocus placeholder="Search commands"><button class=result data-route=work><b>Plan or replan work</b><small>Work › Overview › plan action</small></button><button class=result data-route=scope><b>Open exact target</b><small>Scope › Change › target detail</small></button><button class=result data-route=diagnostics><b>Validate work plan</b><small>Diagnostics › Work Plan</small></button><button class=result data-route=settings><b>Edit syu.yaml</b><small>Settings › structured section</small></button></div></div>"#;
const EDITOR: &str = r#"<div class=editor-overlay role=dialog aria-modal=true aria-label="Source editor"><form class=editor-dialog><header><h2 data-editor-title>Editor</h2><button type=button data-editor-close>×</button></header><label>Source path<input data-editor-path readonly></label><label>Source<textarea data-editor-content></textarea></label><pre data-editor-preview hidden></pre><footer><button type=button data-editor-preview-action>Preview diff & validate</button><button type=submit class=primary disabled>Apply</button></footer></form></div>"#;
const REQUEST_EDITOR: &str = r#"<div class=request-overlay role=dialog aria-modal=true aria-label="Work request editor"><form class=request-dialog><header><h2>Edit WorkRequest</h2><button type=button data-request-close>×</button></header><label>Summary<textarea data-request-summary required></textarea></label><label>Operation<select data-request-operation><option value=add>Add</option><option value=modify>Modify</option><option value=remove>Remove</option><option value=refactor>Refactor</option><option value=document>Document</option><option value=investigate>Investigate</option></select></label><p>Seeds, requested targets, and constraints remain exact canonical values. This editor changes only the human summary and operation.</p><footer><button type=button data-request-close>Cancel</button><button type=submit class=primary>Plan & save</button></footer></form></div>"#;
const SCRIPT: &str = r#"<script>(()=>{
const bootUrl=location.href,q=s=>document.querySelector(s),qa=s=>[...document.querySelectorAll(s)],toast=m=>{const e=q('.toast');e.textContent=m;e.classList.add('show');setTimeout(()=>e.classList.remove('show'),3500)},refresh=p=>setTimeout(()=>location.assign(`/?page=${p}`),250),api=async(url,opt={})=>{const r=await fetch(url,{headers:{'content-type':'application/json'},...opt});const t=await r.text();if(!r.ok)throw new Error((()=>{try{return JSON.parse(t).error}catch{return t}})());try{return JSON.parse(t)}catch{return t}};
const safe=s=>String(s??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c])),renderDiagnostics=v=>{const host=q('[data-page=diagnostics] .diagnostics');host.innerHTML=v.diagnostics.length?v.diagnostics.map(d=>`<article class=diagnostic data-status="${safe(d.severity)}" data-phase="${safe(d.phase)}"><header><span aria-label="${safe(d.severity)}">●</span><div><b>${safe(d.rule_id)}</b><h2>${safe(d.message)}</h2></div></header><code>${safe(d.primary.path)}:${safe(d.primary.line??'-')}</code><p>${safe(d.help??'')}</p></article>`).join(''):`<div class=empty data-diagnostic-result><span class=ok aria-label="success">●</span><h2>${SyuPreferences.t('diagnostics.zero.title')}</h2><p>${SyuPreferences.t('diagnostics.zero.description')}</p><div class=chips><span>${v.applicable_phase_count} ${SyuPreferences.t('diagnostics.applicable')}</span><span>${v.skipped_phase_count} ${SyuPreferences.t('diagnostics.skipped')}</span></div></div>`};
const supported=['en','ja'],osLang=(navigator.language||'en').toLowerCase().startsWith('ja')?'ja':'en';let locale='en',themePreference='system';
const translate=lang=>{locale=supported.includes(lang)?lang:'en';const dict=window.SYU_I18N[locale],base=window.SYU_I18N.en,reverse=Object.fromEntries(Object.keys(base).map(k=>[base[k],k]));document.documentElement.lang=locale;document.documentElement.dataset.locale=locale;document.title=locale==='ja'?'Syu ワークベンチ':'Syu Workbench';document.querySelectorAll('body *').forEach(el=>{if(el.children.length===0){const raw=el.textContent.trim(),key=el.dataset.i18n||reverse[raw];if(key&&dict[key]!==undefined){el.dataset.i18n=key;el.textContent=dict[key]}}for(const attr of ['placeholder','title','aria-label']){const raw=el.getAttribute(attr),key=raw&&reverse[raw];if(key&&dict[key]!==undefined)el.setAttribute(attr,dict[key])}});qa('[data-language-select]').forEach(el=>{el.value=locale;const label=el.closest('label'),text=label?.childNodes[0];if(text)text.nodeValue=`${dict['settings.interface_language']} `});localStorage.setItem('syu.locale',locale)};
const applyTheme=value=>{themePreference=['system','light','dark'].includes(value)?value:'system';const dark=matchMedia('(prefers-color-scheme: dark)').matches;document.documentElement.dataset.theme=themePreference==='system'?(dark?'dark':'light'):themePreference;document.documentElement.dataset.themePreference=themePreference;qa('[data-theme-choice]').forEach(el=>el.classList.toggle('active',el.dataset.themeChoice===themePreference));localStorage.setItem('syu.theme',themePreference)};
window.SyuPreferences={t:key=>window.SYU_I18N[locale][key]||key,translate,theme:applyTheme};const initialParams=new URL(bootUrl).searchParams;translate(initialParams.get('lang')||localStorage.getItem('syu.locale')||osLang);applyTheme(initialParams.get('theme')||localStorage.getItem('syu.theme')||'system');matchMedia('(prefers-color-scheme: dark)').addEventListener('change',()=>{if(themePreference==='system')applyTheme('system')});
const setParam=(key,value)=>{const url=new URL(location);url.searchParams.set(key,value);history.pushState({},'',url)},show=(p,push=true)=>{qa('[data-page]').forEach(e=>e.hidden=e.dataset.page!==p);qa('[data-nav]').forEach(e=>e.classList.toggle('active',e.dataset.nav===p));if(push)history.pushState({page:p},'',`/?page=${p}`)};show(new URLSearchParams(location.search).get('page')||'work',false);onpopstate=()=>location.reload();qa('[data-nav]').forEach(a=>a.onclick=e=>{e.preventDefault();show(a.dataset.nav)});
qa('[data-work-tab]').forEach(b=>b.onclick=()=>{qa('[data-work-tab]').forEach(x=>x.classList.toggle('active',x===b));qa('[data-work-panel]').forEach(x=>x.hidden=x.dataset.workPanel!==b.dataset.workTab);setParam('tab',b.dataset.workTab)});
qa('[data-scope-tab]').forEach(b=>b.onclick=()=>{qa('[data-scope-tab]').forEach(x=>x.classList.toggle('active',x===b));qa('.target').forEach(x=>x.hidden=b.dataset.scopeTab!=='intent'&&x.dataset.access!==b.dataset.scopeTab);setParam('view',b.dataset.scopeTab)});q('[data-scope-tab="editable"]')?.click();
q('[data-item-search]')?.addEventListener('input',e=>qa('[data-item-id]').forEach(x=>x.hidden=!x.dataset.itemId.toLowerCase().includes(e.target.value.toLowerCase())));
qa('[data-item-kind]').forEach(b=>b.onclick=()=>{qa('[data-item-kind]').forEach(x=>x.classList.toggle('active',x===b));qa('[data-item-id]').forEach(x=>x.hidden=x.dataset.kind!==b.dataset.itemKind);setParam('kind',b.dataset.itemKind)});q('[data-item-kind="requirement"]')?.click();
qa('[data-phase]').forEach(b=>{if(b.closest('.tabs'))b.onclick=()=>{qa('[data-page=diagnostics] [data-phase]').forEach(x=>{if(x.tagName==='BUTTON')x.classList.toggle('active',x===b);else x.hidden=b.dataset.phase!=='all'&&x.dataset.phase!==b.dataset.phase});setParam('phase',b.dataset.phase)}});
const overlay=q('.palette-overlay'),paletteInput=q('.palette-dialog input');q('[data-open-palette]').onclick=()=>{overlay.classList.add('open');paletteInput.focus()};paletteInput.oninput=()=>qa('.palette-dialog .result').forEach(x=>x.hidden=!x.textContent.toLowerCase().includes(paletteInput.value.toLowerCase()));overlay.onclick=e=>{if(e.target===overlay)overlay.classList.remove('open')};qa('[data-route]').forEach(b=>b.onclick=()=>{show(b.dataset.route);overlay.classList.remove('open');if(b.dataset.route==='settings'){q('[data-settings-layer=application]')?.click();q('[data-settings-tab=language]')?.click()}const f=b.dataset.route==='settings'?q('[data-language-select]'):q(`[data-page=${b.dataset.route}] button, [data-page=${b.dataset.route}] select`);f?.focus();f?.classList.add('focus-ring');setTimeout(()=>f?.classList.remove('focus-ring'),1800)});
qa('[data-palette-filter]').forEach(b=>b.onclick=()=>{overlay.classList.add('open');paletteInput.value=b.dataset.paletteFilter;paletteInput.dispatchEvent(new Event('input'));paletteInput.focus()});
const ed=q('.editor-overlay'),form=q('.editor-dialog');let sourceHash='';const openEditor=async(path,title,template='')=>{let src=await api(`/api/source?path=${encodeURIComponent(path)}`);sourceHash=src.hash;q('[data-editor-title]').textContent=title;q('[data-editor-path]').value=path;q('[data-editor-content]').value=src.content||template;q('[data-editor-content]').readOnly=false;q('[data-editor-preview-action]').hidden=false;form.querySelector('[type=submit]').hidden=false;q('[data-editor-preview]').hidden=true;form.querySelector('[type=submit]').disabled=true;ed.classList.add('open')},openReadOnly=async(path,title)=>{const src=await api(`/api/source?path=${encodeURIComponent(path)}`);q('[data-editor-title]').textContent=title;q('[data-editor-path]').value=path;q('[data-editor-content]').value=src.content;q('[data-editor-content]').readOnly=true;q('[data-editor-preview-action]').hidden=true;form.querySelector('[type=submit]').hidden=true;q('[data-editor-preview]').hidden=true;ed.classList.add('open')};q('[data-editor-close]').onclick=()=>ed.classList.remove('open');q('[data-editor-preview-action]').onclick=async()=>{try{const body={path:q('[data-editor-path]').value,content:q('[data-editor-content]').value,expected_hash:sourceHash},p=await api('/api/file/preview',{method:'POST',body:JSON.stringify(body)});q('[data-editor-preview]').textContent=`${p.changed_lines} changed line(s)\n${p.validation_errors.length?p.validation_errors.join('\n'):'Validation passed'}`;q('[data-editor-preview]').hidden=false;form.querySelector('[type=submit]').disabled=p.validation_errors.length>0}catch(e){toast(e.message)}};form.onsubmit=async e=>{e.preventDefault();try{const p=await api('/api/file/apply',{method:'PUT',body:JSON.stringify({path:q('[data-editor-path]').value,content:q('[data-editor-content]').value,expected_hash:sourceHash})});sourceHash=p.new_hash;toast('Applied after validation');ed.classList.remove('open');refresh('items')}catch(e){toast(e.message)}};
qa('[data-action=edit-item]').forEach(b=>b.onclick=()=>openEditor(b.closest('[data-item-path]').dataset.itemPath,`Edit ${b.closest('[data-item-id]').dataset.itemId}`));q('[data-action=new-requirement]')?.addEventListener('click',()=>{const id=`REQ-NEW-${Date.now().toString().slice(-6)}`,path=`spec/${id.toLowerCase()}.yaml`,yaml=`schema: syu/spec/v1\nkind: requirements\nnamespace: workbench\ncategory: Workbench\nrequirements:\n  - id: ${id}\n    title: New requirement\n    description: Describe the requirement.\n    priority: medium\n    status: planned\n    criteria:\n      - id: acceptance\n        kind: behavior\n        statement: Describe the acceptance condition.\n        governed_by: []\n    bindings: []\n`;openEditor(path,'New requirement',yaml)});
qa('[data-action=open-target]').forEach(b=>b.onclick=()=>{const target=b.closest('[data-target-path]');openReadOnly(target.dataset.targetPath,target.dataset.targetRef)});qa('[data-action=copy-target]').forEach(b=>b.onclick=async()=>{const value=b.closest('[data-target-ref]').dataset.targetRef;try{await navigator.clipboard.writeText(value);toast('Target locator copied')}catch{toast(value)}});
q('[data-action=new-work]')?.addEventListener('click',()=>{show('items');toast('Select a Requirement with an exact criterion to create Work')});
const requestOverlay=q('.request-overlay'),requestForm=q('.request-dialog');let activeRequest=null;q('[data-action=edit-request]')?.addEventListener('click',()=>{activeRequest=JSON.parse(q('[data-page=work]').dataset.request);q('[data-request-summary]').value=activeRequest.summary;q('[data-request-operation]').value=activeRequest.operation;requestOverlay.classList.add('open')});qa('[data-request-close]').forEach(b=>b.onclick=()=>requestOverlay.classList.remove('open'));requestForm.onsubmit=async e=>{e.preventDefault();activeRequest.summary=q('[data-request-summary]').value;activeRequest.operation=q('[data-request-operation]').value;try{await api('/api/work/request',{method:'PUT',body:JSON.stringify(activeRequest)});toast('Request updated and planned');requestOverlay.classList.remove('open');refresh('work')}catch(e){toast(e.message)}};q('[data-action=replan]')?.addEventListener('click',async()=>{try{await api('/api/work/replan',{method:'POST'});toast('Plan regenerated from current revision');refresh('work')}catch(e){toast(e.message)}});
qa('[data-action=export-context]').forEach(b=>b.onclick=async()=>{try{const yaml=await api(`/api/context/${encodeURIComponent(b.dataset.slice)}`,{method:'POST'}),o=q('[data-context-output]');o.textContent=yaml;o.hidden=false;toast('Context exported')}catch(e){toast(e.message)}});qa('[data-action=validate]').forEach(b=>b.onclick=async()=>{b.disabled=true;const context=q('[data-validation-context]')?.value||'work-plan',slice=context==='slice'?q('[data-action=export-context]')?.dataset.slice:null;try{const v=await api('/api/validate',{method:'POST',body:JSON.stringify({context,slice})});renderDiagnostics(v);toast(`Validation complete: ${v.diagnostics.length} diagnostic(s)`)}catch(e){toast(e.message)}finally{b.disabled=false}});
q('[data-action=create-work]')&&qa('[data-action=create-work]').forEach(b=>b.onclick=async()=>{const id=b.closest('[data-item-id]').dataset.itemId,request={schema:'syu/work-request/v1',id:`WORK-${id}`,summary:`Implement ${id}`,operation:'modify',seeds:[id],constraints:{include_facets:[],exclude_paths:[]},requested_targets:[]};try{await api('/api/work/request',{method:'PUT',body:JSON.stringify(request)});toast(`Created canonical WorkPlan from ${id}`);refresh('work')}catch(e){toast(e.message)}});
let configState=q('[data-page=settings]')?JSON.parse(q('[data-page=settings]').dataset.config):null,configHash='';const splitList=v=>v.split(',').map(x=>x.trim()).filter(Boolean),loadConfig=async()=>{const [source,structured]=await Promise.all([api('/api/source?path=syu.yaml'),api('/api/config')]);configHash=structured.hash;configState=structured.config;const e=q('[data-config-editor]');e.value=source.content;e.hidden=false;return structured},collectConfig=()=>{configState.workspace.spec_roots=splitList(q('[data-config-field=spec-roots]').value);configState.workspace.artifact_roots=splitList(q('[data-config-field=artifact-roots]').value);configState.profiles.active=splitList(q('[data-config-field=profiles]').value);configState.validation.preset=q('[data-config-field=preset]').value;configState.validation.deny_warnings=q('[data-config-field=deny-warnings]').checked;configState.validation.changed.require_owned_changes=q('[data-config-field=require-owned]').checked;configState.work.slicing.max_editable_files=Number(q('[data-config-field=max-editable-files]').value);configState.work.slicing.max_editable_symbols=Number(q('[data-config-field=max-editable-symbols]').value);configState.work.slicing.max_verification_targets=Number(q('[data-config-field=max-verification]').value);configState.work.slicing.max_readonly_targets=Number(q('[data-config-field=max-readonly]').value);configState.work.slicing.max_total_bytes=Number(q('[data-config-field=max-bytes]').value);configState.adapters.enabled=splitList(q('[data-config-field=adapters]').value);return configState};q('[data-action=view-config]')?.addEventListener('click',loadConfig);q('[data-action=preview-config]')?.addEventListener('click',async()=>{try{if(!configHash)await loadConfig();const p=await api('/api/config/preview',{method:'POST',body:JSON.stringify({config:collectConfig(),expected_hash:configHash})}),o=q('[data-preview-output]');o.textContent=`${p.changed_lines} changed line(s)\n${p.validation_errors.length?p.validation_errors.join('\n'):'Validation passed'}`;o.hidden=false}catch(e){toast(e.message)}});q('[data-action=apply-config]')?.addEventListener('click',async()=>{try{if(!configHash)await loadConfig();const p=await api('/api/config/apply',{method:'PUT',body:JSON.stringify({config:collectConfig(),expected_hash:configHash})});configHash=p.new_hash;toast('Configuration validated and applied');refresh('settings')}catch(e){toast(e.message)}});
qa('[data-settings-tab]').forEach(b=>b.onclick=()=>{qa('[data-settings-tab]').forEach(x=>x.classList.toggle('active',x===b));qa('[data-settings-panel]').forEach(x=>x.hidden=x.dataset.settingsPanel!==b.dataset.settingsTab);setParam('section',b.dataset.settingsTab)});
qa('[data-settings-layer]').forEach(b=>b.onclick=()=>{qa('[data-settings-layer]').forEach(x=>x.classList.toggle('active',x===b));qa('[data-settings-layer-panel]').forEach(x=>x.hidden=x.dataset.settingsLayerPanel!==b.dataset.settingsLayer);qa('[data-settings-toolbar]').forEach(x=>x.hidden=x.dataset.settingsToolbar!==b.dataset.settingsLayer);const first=q(`[data-settings-layer-panel=${b.dataset.settingsLayer}] [data-settings-tab]`);first?.click();setParam('settingsLayer',b.dataset.settingsLayer)});qa('[data-language-select]').forEach(el=>el.onchange=()=>translate(el.value));qa('[data-theme-choice]').forEach(el=>el.onclick=()=>applyTheme(el.dataset.themeChoice));q('[data-reset-preferences]')?.addEventListener('click',()=>{translate(osLang);applyTheme('system')});
const params=new URL(bootUrl).searchParams,requestedLayer=params.get('settingsLayer'),requestedSettingsPage=params.get('settingsPage')||params.get('section');if(requestedLayer)q(`[data-settings-layer=${requestedLayer}]`)?.click();if(requestedSettingsPage)q(`[data-settings-layer-panel=${requestedLayer||'application'}] [data-settings-tab=${requestedSettingsPage}]`)?.click();for(const [name,selector] of [['tab','data-work-tab'],['view','data-scope-tab'],['kind','data-item-kind'],['phase','data-phase']]){const value=params.get(name),button=value&&q(`[${selector}="${value}"]`);button?.click()}history.replaceState({},'',bootUrl);
addEventListener('keydown',e=>{if((e.metaKey||e.ctrlKey)&&e.key==='k'){e.preventDefault();overlay.classList.toggle('open')}if(e.key==='Escape'){overlay.classList.remove('open');ed.classList.remove('open');requestOverlay.classList.remove('open')}})
})();</script>"#;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escapes_user_content() {
        assert_eq!(esc("<x & y>"), "&lt;x &amp; y&gt;");
    }

    #[test]
    fn browser_actions_are_api_backed_and_do_not_use_prompt() {
        assert!(SCRIPT.contains("/api/work/request"));
        assert!(SCRIPT.contains("/api/work/replan"));
        assert!(SCRIPT.contains("/api/context/"));
        assert!(SCRIPT.contains("/api/validate"));
        assert!(SCRIPT.contains("/api/file/preview"));
        assert!(SCRIPT.contains("/api/file/apply"));
        assert!(SCRIPT.contains("/api/config/preview"));
        assert!(SCRIPT.contains("openReadOnly"));
        assert!(!SCRIPT.contains("prompt("));
        assert!(REQUEST_EDITOR.contains("Work request editor"));
    }

    #[test]
    fn localized_settings_contract_is_complete() {
        let en: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(include_str!("../assets/locales/en.json")).unwrap();
        let ja: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(include_str!("../assets/locales/ja.json")).unwrap();
        assert_eq!(en.keys().collect::<Vec<_>>(), ja.keys().collect::<Vec<_>>());
        assert!(SCRIPT.contains("navigator.language"));
        assert!(SCRIPT.contains("document.documentElement.lang"));
        assert!(SCRIPT.contains("prefers-color-scheme: dark"));
        assert!(SCRIPT.contains("data-settings-layer-panel"));
        assert!(!SCRIPT.contains("diagPhase"));
    }

    #[test]
    fn settings_is_not_in_the_top_command_bar() {
        let topbar = "<main><header class=topbar>";
        assert!(!topbar.contains("gear"));
        assert!(HEAD.contains("@media(max-width:760px)"));
        assert!(HEAD.contains(".sidebar-bottom>a{left:80%}"));
    }
}
