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
API_JS = (ROOT / "crates/syu-app-ui/assets/js/api.js").read_text()
SPECIFICATIONS_JS = (ROOT / "crates/syu-app-ui/assets/js/pages/specifications.js").read_text()
assert "readInlineProjection" in MAIN_JS
assert "establishSession" in MAIN_JS
assert "}[state.selectedPage]" in MAIN_JS
assert "function disableBusyButtons()" in MAIN_JS
assert "runJourneyAction" in API_JS
assert "journey-advanced" in WORK_JS
assert "journeyQuery" in WORK_JS
assert "if (!state.specificationQuery.trim())" in SPECIFICATIONS_JS
assert "async function runBusy" in SPECIFICATIONS_JS
assert 'data-workbench-status role="status" aria-live="polite"' in HTML
CSS_COMPACT = re.sub(r"\s+", "", CSS)
for token in ("--bg:#f6f7f8", "--paper:#fff", "--ink:#15171a", "--sidebar:246px", "--topbar:98px", "--rail:310px"):
    assert token in CSS_COMPACT, f"missing normative CSS token {token}"
assert "grid-template-columns:repeat(5,1fr)" in CSS_COMPACT
assert "[data-settings-layer-panel][hidden]{display:none!important}" in CSS_COMPACT
assert ".settings-panel[hidden]" in CSS_COMPACT
assert ".settings-toolbar[hidden]" in CSS_COMPACT
print("Workbench DOM, icon, localization, asset, and geometry contract passed")
