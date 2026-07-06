#![forbid(unsafe_code)]

use std::fmt::Write;
use syu_diagnostics::{Diagnostic, Severity};
use syu_work_model::{
    ExecutionSlice, PlanStatus, PlannedTarget, TargetAccessMode, TargetTransition,
};
use syu_workbench_server::WorkspaceProjection;

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
        self.projection.validation.diagnostics.iter()
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
        write!(out, "<body><div class=app><aside class=sidebar><a class=brand href=\"/?page=work\"><b>syu</b><span>WORKBENCH</span></a><nav>{}</nav><div class=workspace><small>WORKSPACE</small><b>{}</b><span title=\"{}\">{}</span></div></aside>", role_links(), esc(&self.projection.workspace.root), esc(&self.projection.workspace.revision), short(&self.projection.workspace.revision)).unwrap();
        out.push_str("<main><header class=topbar><button class=palette data-open-palette><kbd>⌘ K</kbd><span>Search work, slices, items, targets, validation…</span></button><div class=filters><button>All</button><button>Navigate</button><button>Plan</button><button>Validate</button><button>Configure</button></div><a class=gear href=\"/?page=settings\" aria-label=Settings>⚙</a></header><div id=page class=content>");
        out.push_str(&self.render_work());
        out.push_str("</div></main></div>");
        out.push_str(PALETTE);
        out.push_str(EDITOR);
        out.push_str(REQUEST_EDITOR);
        out.push_str("<div class=toast role=status aria-live=polite></div>");
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
        write!(out, "<section data-page=work data-request='{}'><div class=eyebrow>WORKBENCH</div><div class=title><h1>Work</h1><span class=status data-status=\"{}\">● {}</span></div><div class=toolbar><select aria-label=\"Work plan\"><option>{} · {}</option></select><button data-action=edit-request>Edit request</button><button class=primary data-action=replan>Replan</button></div>{}<article class=canvas data-work-panel=overview><h2>{}</h2><p>{}</p><div class=chips><span>{:?}</span><span>{} isolated slices</span><span>basis {}</span></div><div class=summary><section><small>INTENT</small><h3>What this work changes</h3><p>{}</p></section><section><small>EXECUTION</small><h3>Independent slices</h3><p>Each slice has exact editable, verification and readonly targets and must run in an isolated branch or worktree.</p></section></div></article><article class=canvas data-work-panel=slices hidden><h2>Execution slices</h2><p>Each slice is independently executable and has explicit acceptance, non-goals, budgets, and completion checks.</p><div class=slices>", esc(&serde_json::to_string(&plan.request).unwrap_or_default()), status, status, esc(&plan.id), esc(&plan.request.summary), work_tabs(plan.slices.len(), plan.diagnostics.len()), esc(&plan.request.summary), esc(&format!("Exact {:?} work projected from canonical specification anchors.", plan.request.operation)), plan.request.operation, plan.slices.len(), short(&plan.basis.revision), esc(&plan.request.summary)).unwrap();
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
        if self.projection.validation.diagnostics.is_empty() {
            out.push_str("<div class=empty><span class=ok>●</span><h2>No diagnostics</h2><p>The current canonical projection is valid.</p></div>");
        }
        for diagnostic in &self.projection.validation.diagnostics {
            out.push_str(&render_diagnostic(diagnostic));
        }
        out.push_str("</div></section>");
        out
    }

    fn render_settings(&self) -> String {
        format!(
            "<section data-page=settings hidden><div class=eyebrow>UTILITY</div><h1>Settings</h1><div class=toolbar><code>{}</code><button data-action=view-config>View YAML</button><button data-action=preview-config>Preview diff</button><button class=primary data-action=apply-config>Validate &amp; Apply</button></div><div class=settings><nav><button data-settings-tab=workspace>Workspace</button><button data-settings-tab=profiles>Profiles &amp; Facets</button><button class=active data-settings-tab=validation>Validation</button><button data-settings-tab=planning>Work Planning</button><button data-settings-tab=adapters>Adapters</button></nav><article><h2>Validation</h2><p>Structured projection of <code>syu/config/v1</code>. Parsing, validation and source-preserving writes remain server responsibilities.</p><label>Schema<input value=\"{}\" readonly></label><label>Workspace fingerprint<input value=\"{}\" readonly></label><textarea data-config-editor hidden></textarea><pre data-preview-output hidden></pre></article></div></section>",
            esc(&self.projection.workspace.root),
            esc(&self.projection.workspace.config_schema),
            esc(&self.projection.workspace.fingerprint)
        )
    }
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
        write!(
            out,
            "<button class=primary data-action=export-context data-slice=\"{}\">Export {}</button>",
            esc(&slice.id),
            esc(&slice.id)
        )
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
        out.push_str(&render_diagnostic(diagnostic));
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
        "<article class=target data-access=\"{}\"><div class=chips><span>{}</span><span>{}</span><span>{}</span></div><h2>{}</h2><code>{} · lines {}-{}</code><p>{}</p><small>{}</small></article>",
        access.to_ascii_lowercase(),
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

fn render_diagnostic(d: &Diagnostic) -> String {
    let status = match d.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    };
    format!(
        "<article class=diagnostic data-status=\"{}\" data-phase=\"{}\"><header><span>●</span><div><b>{}</b><h2>{}</h2></div></header><code>{}:{}</code><p>{}</p>{}</article>",
        status,
        diagnostic_phase(&d.rule_id),
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
                "<button>Preview safe fix: {}</button>",
                esc(&fix.description)
            ))
            .unwrap_or_default()
    )
}

fn diagnostic_phase(rule: &str) -> &'static str {
    let upper = rule.to_ascii_uppercase();
    if upper.contains("WORK") || upper.contains("PLAN") || upper.contains("SLICE") {
        "work"
    } else if upper.contains("TARGET") || upper.contains("BIND") || upper.contains("CONTRACT") {
        "target"
    } else if upper.contains("CHANGE") || upper.contains("RANGE") || upper.contains("OWN") {
        "change"
    } else if upper.contains("SPEC") || upper.contains("ANCHOR") || upper.contains("GRAPH") {
        "spec"
    } else {
        "config"
    }
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
.request-overlay{display:none;position:fixed;inset:0;background:#1118;place-items:start center;padding-top:80px;z-index:20}.request-overlay.open{display:grid}.request-dialog{width:min(720px,90vw);background:white;border-radius:12px;box-shadow:0 20px 70px #0005;padding:12px}.request-dialog header,.request-dialog footer{display:flex;justify-content:space-between;gap:8px}.request-dialog footer{justify-content:flex-end}.request-dialog label{display:grid;gap:5px;margin:12px 0;font-size:12px}.request-dialog textarea,.request-dialog select{width:100%;padding:10px;border:1px solid #ccd1d6;border-radius:6px}.request-dialog textarea{min-height:110px}
</style></head>"#;
const PALETTE: &str = r#"<div class=palette-overlay role=dialog aria-modal=true aria-label="Command palette"><div class=palette-dialog><input autofocus placeholder="Search commands"><button class=result data-route=work><b>Plan or replan work</b><small>Work › Overview › plan action</small></button><button class=result data-route=scope><b>Open exact target</b><small>Scope › Change › target detail</small></button><button class=result data-route=diagnostics><b>Validate work plan</b><small>Diagnostics › Work Plan</small></button><button class=result data-route=settings><b>Edit syu.yaml</b><small>Settings › structured section</small></button></div></div>"#;
const EDITOR: &str = r#"<div class=editor-overlay role=dialog aria-modal=true aria-label="Source editor"><form class=editor-dialog><header><h2 data-editor-title>Editor</h2><button type=button data-editor-close>×</button></header><label>Source path<input data-editor-path readonly></label><label>Source<textarea data-editor-content></textarea></label><pre data-editor-preview hidden></pre><footer><button type=button data-editor-preview-action>Preview diff & validate</button><button type=submit class=primary disabled>Apply</button></footer></form></div>"#;
const REQUEST_EDITOR: &str = r#"<div class=request-overlay role=dialog aria-modal=true aria-label="Work request editor"><form class=request-dialog><header><h2>Edit WorkRequest</h2><button type=button data-request-close>×</button></header><label>Summary<textarea data-request-summary required></textarea></label><label>Operation<select data-request-operation><option value=add>Add</option><option value=modify>Modify</option><option value=remove>Remove</option><option value=refactor>Refactor</option><option value=document>Document</option><option value=investigate>Investigate</option></select></label><p>Seeds, requested targets, and constraints remain exact canonical values. This editor changes only the human summary and operation.</p><footer><button type=button data-request-close>Cancel</button><button type=submit class=primary>Plan & save</button></footer></form></div>"#;
const SCRIPT: &str = r#"<script>(()=>{
const q=s=>document.querySelector(s),qa=s=>[...document.querySelectorAll(s)],toast=m=>{const e=q('.toast');e.textContent=m;e.classList.add('show');setTimeout(()=>e.classList.remove('show'),3500)},api=async(url,opt={})=>{const r=await fetch(url,{headers:{'content-type':'application/json'},...opt});const t=await r.text();if(!r.ok)throw new Error((()=>{try{return JSON.parse(t).error}catch{return t}})());try{return JSON.parse(t)}catch{return t}};
const show=(p,push=true)=>{qa('[data-page]').forEach(e=>e.hidden=e.dataset.page!==p);qa('[data-nav]').forEach(e=>e.classList.toggle('active',e.dataset.nav===p));if(push)history.pushState({page:p},'',`/?page=${p}`)};show(new URLSearchParams(location.search).get('page')||'work',false);onpopstate=()=>show(new URLSearchParams(location.search).get('page')||'work',false);qa('[data-nav]').forEach(a=>a.onclick=e=>{e.preventDefault();show(a.dataset.nav)});
qa('[data-work-tab]').forEach(b=>b.onclick=()=>{qa('[data-work-tab]').forEach(x=>x.classList.toggle('active',x===b));qa('[data-work-panel]').forEach(x=>x.hidden=x.dataset.workPanel!==b.dataset.workTab)});
qa('[data-scope-tab]').forEach(b=>b.onclick=()=>{qa('[data-scope-tab]').forEach(x=>x.classList.toggle('active',x===b));qa('.target').forEach(x=>x.hidden=b.dataset.scopeTab!=='intent'&&x.dataset.access!==b.dataset.scopeTab)});q('[data-scope-tab="editable"]')?.click();
q('[data-item-search]')?.addEventListener('input',e=>qa('[data-item-id]').forEach(x=>x.hidden=!x.dataset.itemId.toLowerCase().includes(e.target.value.toLowerCase())));
qa('[data-item-kind]').forEach(b=>b.onclick=()=>{qa('[data-item-kind]').forEach(x=>x.classList.toggle('active',x===b));qa('[data-item-id]').forEach(x=>x.hidden=x.dataset.kind!==b.dataset.itemKind)});q('[data-item-kind="requirement"]')?.click();
qa('[data-phase]').forEach(b=>{if(b.closest('.tabs'))b.onclick=()=>{qa('[data-page=diagnostics] [data-phase]').forEach(x=>{if(x.tagName==='BUTTON')x.classList.toggle('active',x===b);else x.hidden=b.dataset.phase!=='all'&&x.dataset.phase!==b.dataset.phase})}});
const overlay=q('.palette-overlay'),paletteInput=q('.palette-dialog input');q('[data-open-palette]').onclick=()=>{overlay.classList.add('open');paletteInput.focus()};paletteInput.oninput=()=>qa('.palette-dialog .result').forEach(x=>x.hidden=!x.textContent.toLowerCase().includes(paletteInput.value.toLowerCase()));overlay.onclick=e=>{if(e.target===overlay)overlay.classList.remove('open')};qa('[data-route]').forEach(b=>b.onclick=()=>{show(b.dataset.route);overlay.classList.remove('open');const f=q(`[data-page=${b.dataset.route}] button, [data-page=${b.dataset.route}] select`);f?.focus();f?.classList.add('focus-ring');setTimeout(()=>f?.classList.remove('focus-ring'),1800)});
const ed=q('.editor-overlay'),form=q('.editor-dialog');let sourceHash='';const openEditor=async(path,title,template='')=>{let src=await api(`/api/source?path=${encodeURIComponent(path)}`);sourceHash=src.hash;q('[data-editor-title]').textContent=title;q('[data-editor-path]').value=path;q('[data-editor-content]').value=src.content||template;q('[data-editor-preview]').hidden=true;form.querySelector('[type=submit]').disabled=true;ed.classList.add('open')};q('[data-editor-close]').onclick=()=>ed.classList.remove('open');q('[data-editor-preview-action]').onclick=async()=>{try{const body={path:q('[data-editor-path]').value,content:q('[data-editor-content]').value,expected_hash:sourceHash},p=await api('/api/file/preview',{method:'POST',body:JSON.stringify(body)});q('[data-editor-preview]').textContent=`${p.changed_lines} changed line(s)\n${p.validation_errors.length?p.validation_errors.join('\n'):'Validation passed'}`;q('[data-editor-preview]').hidden=false;form.querySelector('[type=submit]').disabled=p.validation_errors.length>0}catch(e){toast(e.message)}};form.onsubmit=async e=>{e.preventDefault();try{const p=await api('/api/file/apply',{method:'PUT',body:JSON.stringify({path:q('[data-editor-path]').value,content:q('[data-editor-content]').value,expected_hash:sourceHash})});sourceHash=p.new_hash;toast('Applied after validation');ed.classList.remove('open')}catch(e){toast(e.message)}};
qa('[data-action=edit-item]').forEach(b=>b.onclick=()=>openEditor(b.closest('[data-item-path]').dataset.itemPath,`Edit ${b.closest('[data-item-id]').dataset.itemId}`));q('[data-action=new-requirement]')?.addEventListener('click',()=>{const id=`REQ-NEW-${Date.now().toString().slice(-6)}`,path=`spec/${id.toLowerCase()}.yaml`,yaml=`schema: syu/spec/v1\nkind: requirements\nnamespace: workbench\ncategory: Workbench\nrequirements:\n  - id: ${id}\n    title: New requirement\n    description: Describe the requirement.\n    priority: medium\n    status: planned\n    criteria:\n      - id: acceptance\n        kind: behavior\n        statement: Describe the acceptance condition.\n        governed_by: []\n    bindings: []\n`;openEditor(path,'New requirement',yaml)});
q('[data-action=new-work]')?.addEventListener('click',()=>{show('items');toast('Select a Requirement with an exact criterion to create Work')});
const requestOverlay=q('.request-overlay'),requestForm=q('.request-dialog');let activeRequest=null;q('[data-action=edit-request]')?.addEventListener('click',()=>{activeRequest=JSON.parse(q('[data-page=work]').dataset.request);q('[data-request-summary]').value=activeRequest.summary;q('[data-request-operation]').value=activeRequest.operation;requestOverlay.classList.add('open')});qa('[data-request-close]').forEach(b=>b.onclick=()=>requestOverlay.classList.remove('open'));requestForm.onsubmit=async e=>{e.preventDefault();activeRequest.summary=q('[data-request-summary]').value;activeRequest.operation=q('[data-request-operation]').value;try{await api('/api/work/request',{method:'PUT',body:JSON.stringify(activeRequest)});q('[data-page=work]').dataset.request=JSON.stringify(activeRequest);q('[data-work-panel=overview] h2').textContent=activeRequest.summary;toast('Request updated and planned');requestOverlay.classList.remove('open')}catch(e){toast(e.message)}};q('[data-action=replan]')?.addEventListener('click',async()=>{try{await api('/api/work/replan',{method:'POST'});toast('Plan regenerated from current revision')}catch(e){toast(e.message)}});
qa('[data-action=export-context]').forEach(b=>b.onclick=async()=>{try{const yaml=await api(`/api/context/${encodeURIComponent(b.dataset.slice)}`,{method:'POST'}),o=q('[data-context-output]');o.textContent=yaml;o.hidden=false;toast('Context exported')}catch(e){toast(e.message)}});qa('[data-action=validate]').forEach(b=>b.onclick=async()=>{b.disabled=true;const context=q('[data-validation-context]')?.value||'work-plan',slice=context==='slice'?q('[data-action=export-context]')?.dataset.slice:null;try{const v=await api('/api/validate',{method:'POST',body:JSON.stringify({context,slice})});toast(`Validation complete: ${v.diagnostics.length} diagnostic(s)`)}catch(e){toast(e.message)}finally{b.disabled=false}});
q('[data-action=create-work]')&&qa('[data-action=create-work]').forEach(b=>b.onclick=async()=>{const id=b.closest('[data-item-id]').dataset.itemId,request={schema:'syu/work-request/v1',id:`WORK-${id}`,summary:`Implement ${id}`,operation:'modify',seeds:[id],constraints:{include_facets:[],exclude_paths:[]},requested_targets:[]};try{await api('/api/work/request',{method:'PUT',body:JSON.stringify(request)});show('work');toast(`Created canonical WorkPlan from ${id}`)}catch(e){toast(e.message)}});
let configHash='';const loadConfig=async()=>{const s=await api('/api/source?path=syu.yaml');configHash=s.hash;const e=q('[data-config-editor]');e.value=s.content;e.hidden=false;return s};q('[data-action=view-config]')?.addEventListener('click',loadConfig);q('[data-action=preview-config]')?.addEventListener('click',async()=>{try{if(!configHash)await loadConfig();const p=await api('/api/file/preview',{method:'POST',body:JSON.stringify({path:'syu.yaml',content:q('[data-config-editor]').value,expected_hash:configHash})}),o=q('[data-preview-output]');o.textContent=`${p.changed_lines} changed line(s)\n${p.validation_errors.length?p.validation_errors.join('\n'):'Validation passed'}`;o.hidden=false}catch(e){toast(e.message)}});q('[data-action=apply-config]')?.addEventListener('click',async()=>{try{if(!configHash)await loadConfig();const p=await api('/api/file/apply',{method:'PUT',body:JSON.stringify({path:'syu.yaml',content:q('[data-config-editor]').value,expected_hash:configHash})});configHash=p.new_hash;toast('Configuration validated and applied')}catch(e){toast(e.message)}});
qa('[data-settings-tab]').forEach(b=>b.onclick=()=>{qa('[data-settings-tab]').forEach(x=>x.classList.toggle('active',x===b));q('[data-page=settings] article h2').textContent=b.textContent});
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
        assert!(!SCRIPT.contains("prompt("));
        assert!(REQUEST_EDITOR.contains("Work request editor"));
    }
}
