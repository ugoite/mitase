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
        out.push_str(SCRIPT);
        out.push_str("</body></html>");
        out
    }

    fn render_work(&self) -> String {
        let Some(plan) = &self.projection.plan else {
            return "<section data-page=work><div class=eyebrow>WORKBENCH</div><h1>Work</h1><div class=empty><h2>No work request selected</h2><p>Start the server with <code>--request request.yaml</code> to project a deterministic WorkPlan.</p><button class=primary>New work</button></div></section>".into();
        };
        let status = match plan.status {
            PlanStatus::Ready => "Ready",
            PlanStatus::NeedsReview => "Needs review",
            PlanStatus::Blocked => "Blocked",
        };
        let mut out = String::new();
        write!(out, "<section data-page=work><div class=eyebrow>WORKBENCH</div><div class=title><h1>Work</h1><span class=status data-status=\"{}\">● {}</span></div><div class=toolbar><select aria-label=\"Work plan\"><option>{} · {}</option></select><button>Edit request</button><button class=primary>Replan</button></div>{}<article class=canvas><h2>{}</h2><p>{}</p><div class=chips><span>{:?}</span><span>{} isolated slices</span><span>basis {}</span></div><div class=summary><section><small>INTENT</small><h3>What this work changes</h3><p>{}</p></section><section><small>EXECUTION</small><h3>Independent slices</h3><p>Each slice has exact editable, verification and readonly targets and must run in an isolated branch or worktree.</p></section></div><h3>Execution slices</h3><div class=slices>", status, status, esc(&plan.id), esc(&plan.request.summary), work_tabs(plan.slices.len(), plan.diagnostics.len()), esc(&plan.request.summary), esc(&format!("Exact {:?} work projected from canonical specification anchors.", plan.request.operation)), plan.request.operation, plan.slices.len(), short(&plan.basis.revision), esc(&plan.request.summary)).unwrap();
        for slice in &plan.slices {
            out.push_str(&render_slice(slice));
        }
        out.push_str("</div></article></section>");
        out.push_str(&self.render_scope());
        out.push_str(&self.render_items());
        out.push_str(&self.render_diagnostics());
        out.push_str(&self.render_settings());
        out
    }

    fn render_scope(&self) -> String {
        let mut out = String::from(
            "<section data-page=scope hidden><div class=eyebrow>WORKBENCH</div><h1>Scope</h1><p class=lead>Exact PlannedTargets from the selected WorkPlan and ExecutionSlice.</p><div class=tabs><button class=active>Change</button><button>Verify</button><button>Reference</button><button>Intent</button></div><div class=target-grid>",
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
            "<section data-page=items hidden><div class=eyebrow>WORKBENCH</div><h1>Items</h1><div class=toolbar><input placeholder=\"Search specification items, anchors, bindings, or targets\"><button class=primary>+ New requirement</button></div><div class=tabs><button>Philosophy</button><button>Policy</button><button class=active>Requirement</button><button>Feature</button></div><div class=item-list>",
        );
        for item in &self.projection.items {
            write!(out, "<article><div><b>{}</b><small>{}</small></div><div class=counts><span>{} criteria</span><span>{} bindings</span><span>{} contracts</span></div><button>Create work</button></article>", esc(&item.id), esc(&item.path), item.criteria, item.bindings, item.contracts).unwrap();
        }
        out.push_str("</div><p class=notice>Edits use structured fields and require server-side diff preview, validation, and source-hash confirmation before apply.</p></section>");
        out
    }

    fn render_diagnostics(&self) -> String {
        let mut out = String::from(
            "<section data-page=diagnostics hidden><div class=eyebrow>WORKBENCH</div><h1>Diagnostics</h1><div class=toolbar><select><option>Workspace</option><option>Git range</option><option>Work plan</option><option>Slice</option></select><button class=primary>↻ Validate</button></div><div class=tabs><button class=active>Config &amp; Schema</button><button>Spec Graph</button><button>Artifact Targets</button><button>Change Scope</button><button>Work Plan</button></div><div class=diagnostics>",
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
            "<section data-page=settings hidden><div class=eyebrow>UTILITY</div><h1>Settings</h1><div class=toolbar><code>{}</code><button>View YAML</button><button>Preview diff</button><button class=primary>Validate &amp; Apply</button></div><div class=settings><nav><button>Workspace</button><button>Profiles &amp; Facets</button><button class=active>Validation</button><button>Work Planning</button><button>Adapters</button></nav><article><h2>Validation</h2><p>Structured projection of <code>syu/config/v1</code>. Parsing, validation and source-preserving writes remain server responsibilities.</p><label>Schema<input value=\"{}\" readonly></label><label>Workspace fingerprint<input value=\"{}\" readonly></label></article></div></section>",
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
        "<article class=target><div class=chips><span>{}</span><span>{}</span><span>{}</span></div><h2>{}</h2><code>{} · lines {}-{}</code><p>{}</p><small>{}</small></article>",
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
        "<article class=diagnostic data-status=\"{}\"><header><span>●</span><div><b>{}</b><h2>{}</h2></div></header><code>{}:{}</code><p>{}</p>{}</article>",
        status,
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

fn role_links() -> &'static str {
    "<a class=active data-nav=work href=\"/?page=work\">◉ <span>Work</span></a><a data-nav=scope href=\"/?page=scope\">◇ <span>Scope</span></a><a data-nav=items href=\"/?page=items\">▣ <span>Items</span></a><a data-nav=diagnostics href=\"/?page=diagnostics\">✓ <span>Diagnostics</span></a>"
}
fn work_tabs(slices: usize, diagnostics: usize) -> String {
    format!(
        "<div class=tabs><button class=active>Overview</button><button>Slices <span>{slices}</span></button><button>Context</button><button>Validation <span>{diagnostics}</span></button></div>"
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
}

const HEAD: &str = r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Syu Workbench</title><style>
:root{font-family:Inter,ui-sans-serif,system-ui;color:#20252c;background:#eef0f2}*{box-sizing:border-box}body{margin:0}.app{display:grid;grid-template-columns:220px 1fr;min-height:100vh}.sidebar{position:sticky;top:0;height:100vh;background:#111418;color:#c9ced5;padding:22px 14px;display:flex;flex-direction:column}.brand{color:white;text-decoration:none;padding:0 12px 26px}.brand b{font-size:26px}.brand span{display:block;font-size:9px;letter-spacing:.2em;color:#858d98}.sidebar nav{display:grid;gap:5px}.sidebar nav a{color:#aab1ba;text-decoration:none;padding:11px 14px;border-radius:8px}.sidebar nav a.active,.sidebar nav a:hover{background:#292e35;color:white}.workspace{margin-top:auto;display:grid;gap:4px;padding:12px;border-top:1px solid #343a42;font-size:11px}.workspace span{color:#7f8792}main{min-width:0}.topbar{height:92px;background:white;border-bottom:1px solid #d6d9dd;padding:14px 28px;display:flex;align-items:flex-start;gap:12px}.palette{width:min(670px,65vw);height:38px;border:1px solid #cfd3d8;border-radius:8px;background:#f7f8f9;text-align:left;color:#69717c}.palette kbd{margin-right:15px;background:white;border:1px solid #d5d8dc;border-radius:5px;padding:3px 7px}.filters{position:absolute;top:57px;display:flex;gap:6px}.filters button,.tabs button{border:0;background:transparent;color:#68717d;padding:5px 9px}.filters button:first-child,.tabs .active{color:#111418;font-weight:700;border-bottom:2px solid #111418}.gear{margin-left:auto;color:#333;text-decoration:none;font-size:20px}.content{padding:28px}.content>section{max-width:1250px;margin:auto}.eyebrow{font-size:10px;letter-spacing:.18em;color:#7d858f}.title{display:flex;align-items:center;gap:14px}h1{font-size:30px;margin:5px 0 16px}h2{margin:5px 0;font-size:20px}h3{font-size:14px}.lead,p{color:#626b76;line-height:1.55}.toolbar{display:flex;gap:9px;margin:12px 0 18px}.toolbar select,.toolbar input,.toolbar code{flex:1}.toolbar select,.toolbar input,.toolbar code,button{border:1px solid #cdd2d7;background:white;border-radius:8px;padding:10px 13px}button{font-weight:650}.primary{background:#111418;color:white}.tabs{display:flex;gap:10px;border-bottom:1px solid #d4d8dc;margin-bottom:0}.tabs span,.count{border-radius:10px;background:#e5e8eb;padding:2px 7px;font-size:10px}.canvas,.target,.diagnostic,.item-list article,.settings{background:white;border:1px solid #d7dade}.canvas{padding:25px;border-radius:0 0 10px 10px}.chips{display:flex;flex-wrap:wrap;gap:6px;margin:10px 0}.chips span,.status{border:1px solid #d7dade;border-radius:14px;padding:4px 9px;font-size:11px;background:#f7f8f9}.summary{display:grid;grid-template-columns:1fr 1fr;gap:18px;margin:26px 0}.summary section{border-left:2px solid #a9b0b8;padding-left:14px}.summary small{font-size:9px;color:#7b838d;letter-spacing:.15em}.slices{display:grid;gap:8px}.slice{border:1px solid #dce0e3;border-radius:9px;padding:14px}.slice header{display:flex;align-items:flex-start;gap:10px}.slice header div{flex:1}.slice p{margin:3px 0}.ok{color:#1b9b5d}.status[data-status=Blocked],[data-status=error]>header>span{color:#c43c3c}.status[data-status="Needs review"],[data-status=warning]>header>span{color:#d88619}.target-grid,.diagnostics,.item-list{display:grid;gap:10px;margin-top:15px}.target,.diagnostic{border-radius:9px;padding:18px}.target code,.diagnostic code{display:block;background:#f3f5f6;padding:10px;border-radius:6px;font-size:12px}.item-list article{display:flex;align-items:center;gap:18px;padding:14px;border-radius:8px}.item-list article>div:first-child{flex:1}.item-list small{display:block;color:#777}.counts{display:flex;gap:6px}.counts span{font-size:11px;background:#f1f3f4;padding:5px 8px;border-radius:12px}.notice,.empty{margin-top:20px;padding:25px;border:1px dashed #bcc2c8;border-radius:9px;background:#f8f9fa}.settings{display:grid;grid-template-columns:230px 1fr}.settings nav{display:grid;align-content:start;padding:14px;background:#f5f6f7}.settings nav button{text-align:left;border:0;background:transparent}.settings nav .active{background:white}.settings article{padding:24px}.settings label{display:grid;gap:5px;margin:15px 0;font-size:12px}.settings input{padding:10px;border:1px solid #d4d8dc}.palette-overlay{display:none;position:fixed;inset:0;background:#1118;place-items:start center;padding-top:120px}.palette-overlay.open{display:grid}.palette-dialog{width:min(720px,90vw);background:white;border-radius:12px;box-shadow:0 20px 70px #0005;padding:12px}.palette-dialog input{width:100%;border:0;border-bottom:1px solid #ddd;padding:15px;font-size:17px}.result{display:block;width:100%;text-align:left;margin-top:8px}.result small{display:block;color:#777}@media(max-width:760px){.app{grid-template-columns:64px 1fr}.sidebar{padding:16px 7px}.brand span,.sidebar nav span,.workspace{display:none}.sidebar nav a{text-align:center}.topbar{padding:14px}.filters{display:none}.content{padding:16px}.summary{grid-template-columns:1fr}.toolbar{flex-wrap:wrap}.settings{grid-template-columns:1fr}.settings nav{grid-template-columns:1fr 1fr}.counts{display:none}}
</style></head>"#;
const PALETTE: &str = r#"<div class=palette-overlay role=dialog aria-modal=true aria-label="Command palette"><div class=palette-dialog><input autofocus placeholder="Search commands"><button class=result data-route=work><b>Plan or replan work</b><small>Work › Overview › plan action</small></button><button class=result data-route=scope><b>Open exact target</b><small>Scope › Change › target detail</small></button><button class=result data-route=diagnostics><b>Validate work plan</b><small>Diagnostics › Work Plan</small></button><button class=result data-route=settings><b>Edit syu.yaml</b><small>Settings › structured section</small></button></div></div>"#;
const SCRIPT: &str = r#"<script>(()=>{const q=s=>document.querySelector(s),qa=s=>[...document.querySelectorAll(s)],show=p=>{qa('[data-page]').forEach(e=>e.hidden=e.dataset.page!==p);qa('[data-nav]').forEach(e=>e.classList.toggle('active',e.dataset.nav===p));history.replaceState({},'',`/?page=${p}`)};show(new URLSearchParams(location.search).get('page')||'work');qa('[data-nav]').forEach(a=>a.onclick=e=>{e.preventDefault();show(a.dataset.nav)});const overlay=q('.palette-overlay');q('[data-open-palette]').onclick=()=>overlay.classList.add('open');overlay.onclick=e=>{if(e.target===overlay)overlay.classList.remove('open')};qa('[data-route]').forEach(b=>b.onclick=()=>{show(b.dataset.route);overlay.classList.remove('open');q(`[data-page=${b.dataset.route}] button, [data-page=${b.dataset.route}] select`)?.focus()});addEventListener('keydown',e=>{if((e.metaKey||e.ctrlKey)&&e.key==='k'){e.preventDefault();overlay.classList.toggle('open')}if(e.key==='Escape')overlay.classList.remove('open')})})();</script>"#;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escapes_user_content() {
        assert_eq!(esc("<x & y>"), "&lt;x &amp; y&gt;");
    }
}
