#![forbid(unsafe_code)]

mod components;
mod document;
mod pages;
mod shell;

use syu_workbench_server::WorkspaceProjection;

/// Browser document renderer. Page markup, styling, localization and behavior
/// live in named modules and external assets; this type only composes them.
pub struct WorkbenchView<'a> {
    projection: &'a WorkspaceProjection,
}

impl<'a> WorkbenchView<'a> {
    pub fn new(projection: &'a WorkspaceProjection) -> Self {
        Self { projection }
    }

    pub fn render_html(&self) -> String {
        document::render(self.projection)
    }
}

pub const WORKBENCH_CSS: &str = include_str!("../assets/workbench.css");
pub const WORKBENCH_I18N_JS: &str = include_str!("../assets/i18n.js");
pub const WORKBENCH_MAIN_JS: &str = include_str!("../assets/js/main.js");

pub fn locale_catalog_script() -> String {
    format!(
        "window.SYU_I18N={{en:{},ja:{}}};",
        include_str!("../assets/locales/en.json"),
        include_str!("../assets/locales/ja.json")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn locale_catalogs_have_identical_semantic_keys() {
        let en_catalog: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(include_str!("../assets/locales/en.json")).unwrap();
        let ja: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(include_str!("../assets/locales/ja.json")).unwrap();
        let en = en_catalog.keys().cloned().collect::<BTreeSet<_>>();
        let ja = ja.keys().cloned().collect::<BTreeSet<_>>();
        assert_eq!(
            en, ja,
            "English and Japanese catalogs must stay in lockstep"
        );
        let html = include_str!("../assets/workbench.html");
        for attribute in [
            "data-i18n=\"",
            "data-i18n-placeholder=\"",
            "data-i18n-title=\"",
            "data-i18n-aria=\"",
        ] {
            for tail in html.split(attribute).skip(1) {
                let key = tail.split('"').next().unwrap();
                assert!(
                    en_catalog.contains_key(key),
                    "missing English catalog key: {key}"
                );
            }
        }
        for tag in html.split('<').filter(|tag| tag.contains("aria-label=\"")) {
            assert!(
                tag.contains("data-i18n-aria=\"") || tag.contains("aria-hidden=\"true\""),
                "accessible name is not localized: {tag}"
            );
        }
        assert!(WORKBENCH_MAIN_JS.contains("renderWork"));
        assert!(WORKBENCH_MAIN_JS.contains("renderSpecifications"));
    }

    #[test]
    fn document_uses_external_assets_and_required_landmarks() {
        let html = include_str!("../assets/workbench.html");
        for landmark in [
            "app-shell",
            "class=\"role-sidebar\"",
            "command-bar",
            "content-viewport",
            "class=\"surface\"",
            "class=\"rail\"",
            "class=\"canvas",
            "data-route=\"settings\"",
        ] {
            assert!(html.contains(landmark), "missing landmark: {landmark}");
        }
        assert!(!html.contains("<style"));
        assert!(!html.contains("class=\"gear\""));
        assert!(html.contains("/assets/workbench.css"));
        assert!(html.contains("type=\"module\" src=\"/assets/js/main.js\""));
        for banned in [
            "REQ-WORKBENCH",
            "SLICE-01",
            "PLAN-WORKBENCH",
            "UI-VISUAL-CONTRACT",
            "No issues found",
            "just now",
        ] {
            assert!(
                !html.contains(banned),
                "static demo content leaked: {banned}"
            );
        }
        assert!(
            WORKBENCH_CSS.contains("[data-settings-layer-panel][hidden]{display:none!important}")
        );
    }

    #[test]
    fn browser_does_not_infer_specification_semantics() {
        assert!(!WORKBENCH_MAIN_JS.contains("selector.names"));
        assert!(!WORKBENCH_MAIN_JS.contains("rawProjection"));
        assert!(!WORKBENCH_MAIN_JS.contains("syu/config"));
    }
}
