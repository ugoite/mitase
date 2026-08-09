#!/usr/bin/env python3
"""Static acceptance gate for the Workbench Visual Contract DOM and catalogs."""

import json
import re
from collections import defaultdict
from html.parser import HTMLParser
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
HTML = (ROOT / "crates/syu-app-ui/assets/workbench.html").read_text()
CSS = (ROOT / "crates/syu-app-ui/assets/workbench.css").read_text()
EN = json.loads((ROOT / "crates/syu-app-ui/assets/locales/en.json").read_text())
JA = json.loads((ROOT / "crates/syu-app-ui/assets/locales/ja.json").read_text())


class ContractParser(HTMLParser):
    def __init__(self):
        super().__init__()
        self.routes = []
        self.keys = set()
        self.unlocalized_names = []
        self.tab = None
        self.tab_icons = defaultdict(list)

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        for attribute in ("data-i18n", "data-i18n-placeholder", "data-i18n-title", "data-i18n-aria"):
            if attribute in attrs:
                self.keys.add(attrs[attribute])
        if attrs.get("data-route"):
            self.routes.append(attrs["data-route"])
        if "aria-label" in attrs and "data-i18n-aria" not in attrs and attrs.get("aria-hidden") != "true":
            self.unlocalized_names.append(attrs["aria-label"])
        if tag == "button" and attrs.get("data-tab-group"):
            self.tab = (attrs["data-tab-group"], [])
        elif self.tab and tag in ("path", "circle", "rect"):
            self.tab[1].append((tag, tuple(sorted(attrs.items()))))

    def handle_endtag(self, tag):
        if tag == "button" and self.tab:
            group, icon = self.tab
            self.tab_icons[group].append(tuple(icon))
            self.tab = None


parser = ContractParser()
parser.feed(HTML)
assert parser.routes[:6] == ["work", "readiness", "scope", "specifications", "diagnostics", "settings"], parser.routes[:6]
assert set(EN) == set(JA), "English and Japanese catalogs differ"
assert not (parser.keys - set(EN)), f"missing catalog keys: {sorted(parser.keys - set(EN))}"
assert not parser.unlocalized_names, f"unlocalized accessible names: {parser.unlocalized_names}"
for group, icons in parser.tab_icons.items():
    assert all(icons), f"{group} has a tab without an icon"
    assert len(icons) == len(set(icons)), f"{group} repeats a section-tab icon"

assert re.search(r'class="topbar command-bar"', HTML)
assert 'data-workspace-branch' not in HTML
assert re.search(r'class="workspace-name".*class="workspace-icon"', HTML, re.S)
assert 'class="gear"' not in HTML
assert '<style' not in HTML
assert 'HEAD...HEAD' not in HTML and 'HEAD…HEAD' not in HTML
assert set(re.findall(r'data-settings-layer="([^"]+)"', HTML)) == {"application", "workspace"}
assert 'data-specifications-search=""' in HTML
assert 'data-i18n-placeholder="items.search.placeholder"' in HTML
assert 'data-specifications-new=""' in HTML
assert 'data-route="items"' not in HTML
assert 'data-page="items"' not in HTML
assert re.search(r'data-settings-toolbar="workspace" hidden=""', HTML)
assert re.search(r'data-settings-layer-panel="workspace" hidden=""', HTML)
assert set(re.findall(r'data-settings-page="([^"]+)"', HTML)) == {
    "language", "appearance", "accessibility", "general", "profiles", "validation", "planning", "adapters"
}
for banned in ("REQ-WORKBENCH", "SLICE-01", "PLAN-WORKBENCH", "UI-VISUAL-CONTRACT", "just now", "No issues found"):
    assert banned not in HTML, f"static demo content leaked into HTML: {banned}"
for banned in ("issue-762", "8954b70"):
    assert banned not in HTML, f"static repository identity leaked into HTML: {banned}"
for asset in ("workbench.css", "catalog.js", "i18n.js", "js/main.js"):
    assert f'/assets/{asset}' in HTML
assert '/assets/projection.js' not in HTML
assert 'type="module" src="/assets/js/main.js"' in HTML
for module in (
    "./api.js", "./state.js", "./router.js", "./pages/work.js",
    "./pages/readiness.js", "./pages/scope.js", "./pages/specifications.js",
    "./pages/diagnostics.js", "./pages/settings.js",
):
    assert module in (ROOT / "crates/syu-app-ui/assets/js/main.js").read_text(), module
MAIN_JS = (ROOT / "crates/syu-app-ui/assets/js/main.js").read_text()
WORK_JS = (ROOT / "crates/syu-app-ui/assets/js/pages/work.js").read_text()
SCOPE_JS = (ROOT / "crates/syu-app-ui/assets/js/pages/scope.js").read_text()
API_JS = (ROOT / "crates/syu-app-ui/assets/js/api.js").read_text()
SPECIFICATIONS_JS = (ROOT / "crates/syu-app-ui/assets/js/pages/specifications.js").read_text()
I18N_MODULE = (ROOT / "crates/syu-app-ui/assets/js/i18n.js").read_text()
SERVER_RS = (ROOT / "crates/syu-workbench-server/src/lib.rs").read_text()
assert "readInlineProjection" in MAIN_JS
assert "data-workspace-branch" not in MAIN_JS
assert "translate('workspace.revision')" in MAIN_JS
assert "establishSession" in MAIN_JS
assert "}[state.selectedPage]" in MAIN_JS
assert "function disableBusyButtons()" in MAIN_JS
assert "runJourneyAction" in API_JS
assert "readScopeDiff" in API_JS
assert "journey-advanced" in WORK_JS
assert "work-start" in WORK_JS
assert "select_specification" in WORK_JS
assert "journey.start." not in WORK_JS
assert "current_step === 'describe'" not in WORK_JS
assert "journey-intake" not in WORK_JS
assert "journey-card" not in WORK_JS
assert "data-scope-create-work" not in HTML
assert "scope-create-work" not in SCOPE_JS
assert "action: 'create'" not in SCOPE_JS
assert "data-create-work" in SPECIFICATIONS_JS
assert "data-target-suggestion-approved" in SPECIFICATIONS_JS
assert "approved_ids" in SPECIFICATIONS_JS
assert "data-review-target-suggestions" in SPECIFICATIONS_JS
assert "data-approve-target-suggestions" in SPECIFICATIONS_JS
assert "result.request" not in SPECIFICATIONS_JS
assert "schema: 'syu/work-origin-capability/v1'" in SPECIFICATIONS_JS
assert "origin_capabilities" in SPECIFICATIONS_JS
assert "work.request.summary_from_anchor" not in SPECIFICATIONS_JS
assert SPECIFICATIONS_JS.count("state.go('work')") >= 2
assert SPECIFICATIONS_JS.count("state.api.runJourneyAction(state.projection") >= 2
assert "data-create-work-from-suggestions" in SPECIFICATIONS_JS
assert "WORK-SUGGESTION-" not in SERVER_RS
assert "approved_target_suggestions" in SERVER_RS
assert "validate_requirement_origin" in SERVER_RS
assert '"/api/work/request"' not in SERVER_RS
assert "WorkRequestCommand" not in SERVER_RS
assert "draft_request = Some(request.clone())" not in SERVER_RS
assert "Describe the change you want to make" not in SERVER_RS
assert "renderDiff" in WORK_JS
assert "initScope" in MAIN_JS
assert "if (!state.specificationQuery.trim())" in SPECIFICATIONS_JS
assert "async function runBusy" in SPECIFICATIONS_JS
assert "localizeEnum" in I18N_MODULE
assert "SyuPreferences?.lookup" in I18N_MODULE
assert "localizeSpecificationTitle" in I18N_MODULE
assert "presentation_title_key" in I18N_MODULE or "presentation_title_key" in SERVER_RS
assert "localizedOptions('criterion.kind'" in SPECIFICATIONS_JS
assert "localizeEnum('operation'" in WORK_JS
assert "localizeEnum('target.access'" in WORK_JS
assert "localizeSpecificationTitle" in SPECIFICATIONS_JS
assert "item.textContent = option" not in SPECIFICATIONS_JS
assert "pub access: TargetAccessMode" in SERVER_RS
assert "pub transition: TargetTransition" in SERVER_RS
assert 'access: format!("{:?}"' not in SERVER_RS
assert 'transition: format!("{:?}"' not in SERVER_RS
assert "presentation_title_key: builtin_presentation_title_key" in SERVER_RS
assert "Box::leak" not in SERVER_RS
assert "kind: String" in SERVER_RS and "lane: String" in SERVER_RS
assert "semantic_candidate_count" in SERVER_RS and "hidden_closure_target_count" in SERVER_RS
for entity in ("binding", "ownership", "target", "claim", "contract"):
    assert f"entity: '{entity}'" in SPECIFICATIONS_JS, f"missing typed {entity} edit"
assert "selector_kind" in SPECIFICATIONS_JS
assert "claim_kind" in SPECIFICATIONS_JS
assert "runtime_timestamp" in SPECIFICATIONS_JS and "runtime_receipt" in SPECIFICATIONS_JS
assert "ArrowRight" in SPECIFICATIONS_JS and "event.key === 'Home'" in SPECIFICATIONS_JS
assert "specificationSourceFocusKey" in SPECIFICATIONS_JS
assert "kind === 'module'" in SPECIFICATIONS_JS and "kind === 'path-prefix'" in SPECIFICATIONS_JS
assert "Runner arguments must be valid JSON" in SPECIFICATIONS_JS
assert "input.readOnly = true" in SPECIFICATIONS_JS and "currentId" in SPECIFICATIONS_JS
assert "current_id" in SERVER_RS and "ids are immutable" in SERVER_RS
assert "onLocationChange" in SPECIFICATIONS_JS and "syncWorkSpecificationLocation" in WORK_JS
assert "[1, 2, 3, 4, 5, 6, 7, 8]" in SPECIFICATIONS_JS
for key in (
    "criterion.kind.behavior",
    "criterion.kind.quality",
    "criterion.kind.security",
    "operation.add",
    "operation.modify",
    "operation.remove",
    "target.access.editable",
    "target.transition.modify",
    "target.access.generated",
    "specification.title.REQ-CAPABILITY-001",
):
    assert key in EN and key in JA, f"missing semantic localization key: {key}"
    assert EN[key] != JA[key], f"semantic localization is unchanged: {key}"
assert 'data-workbench-status="busy" role="progressbar" aria-live="polite"' in HTML
assert "workbench-progress-track" in HTML
CSS_COMPACT = re.sub(r"\s+", "", CSS)
assert ".branch" not in CSS and ".utility" not in CSS
for token in ("--bg:#f6f7f8", "--paper:#fff", "--ink:#15171a", "--sidebar:246px", "--topbar:98px", "--rail:310px"):
    assert token in CSS_COMPACT, f"missing normative CSS token {token}"
assert "grid-template-columns:repeat(6,1fr)" in CSS_COMPACT
assert "[data-settings-layer-panel][hidden]{display:none!important}" in CSS_COMPACT
assert ".settings-panel[hidden]" in CSS_COMPACT
assert ".settings-toolbar[hidden]" in CSS_COMPACT
print("Workbench DOM, icon, localization, asset, and geometry contract passed")
