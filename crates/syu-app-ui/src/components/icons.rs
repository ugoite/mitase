use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconName {
    Work,
    Scope,
    Items,
    Diagnostics,
    Settings,
    CodeTests,
    Philosophy,
    Policy,
    Requirement,
    Feature,
    Workspace,
    WorkPlan,
    Trace,
    Repository,
    General,
    App,
    Integrations,
    Goal,
    Repair,
    Deliver,
    Specify,
    Govern,
    Restructure,
    Verify,
    Maintain,
    Retire,
    Review,
    Adopt,
}

impl IconName {
    pub const fn mdi_name(self) -> &'static str {
        match self {
            Self::Work => "mdi-clipboard-edit-outline",
            Self::Scope => "mdi-selection-search",
            Self::Items => "mdi-format-list-bulleted-square",
            Self::Diagnostics => "mdi-stethoscope",
            Self::Settings => "mdi-cog-outline",
            Self::CodeTests => "mdi-code-braces-box",
            Self::Philosophy => "mdi-lightbulb-on-outline",
            Self::Policy => "mdi-shield-check-outline",
            Self::Requirement => "mdi-clipboard-text-outline",
            Self::Feature => "mdi-puzzle-outline",
            Self::Workspace => "mdi-folder-cog-outline",
            Self::WorkPlan => "mdi-target-variant",
            Self::Trace => "mdi-source-branch-check",
            Self::Repository => "mdi-source-repository",
            Self::General => "mdi-tune-variant",
            Self::App => "mdi-monitor-dashboard",
            Self::Integrations => "mdi-connection",
            Self::Goal => "mdi-bullseye-arrow",
            Self::Repair => "mdi-wrench-outline",
            Self::Deliver => "mdi-hammer-wrench",
            Self::Specify => "mdi-file-document-edit-outline",
            Self::Govern => "mdi-shield-edit-outline",
            Self::Restructure => "mdi-graph-outline",
            Self::Verify => "mdi-test-tube",
            Self::Maintain => "mdi-cog-sync-outline",
            Self::Retire => "mdi-archive-arrow-down-outline",
            Self::Review => "mdi-file-search-outline",
            Self::Adopt => "mdi-folder-plus-outline",
        }
    }

    const fn path(self) -> &'static str {
        match self {
            Self::Settings | Self::General | Self::App | Self::Maintain => {
                "M12 15.5A3.5 3.5 0 1 1 12 8a3.5 3.5 0 0 1 0 7.5M19.43 12.98c.04-.32.07-.65.07-.98s-.03-.66-.08-.98l2.11-1.65-2-3.46-2.49 1a7.3 7.3 0 0 0-1.69-.98L15 3.27h-4l-.4 2.66c-.61.25-1.17.58-1.69.98l-2.49-1-2 3.46 2.11 1.65c-.04.32-.07.65-.07.98s.03.66.07.98l-2.11 1.65 2 3.46 2.49-1c.52.4 1.08.73 1.69.98l.4 2.66h4l.4-2.66c.61-.25 1.17-.58 1.69-.98l2.49 1 2-3.46z"
            }
            Self::Diagnostics | Self::Repair | Self::Deliver => {
                "M22.7 19l-9.1-9.1c.9-2.3.4-5-1.5-6.9C10.1 1 7.1.6 4.7 1.7l4.1 4.1-3 3-4.2-4.1C.4 7.1.9 10.1 2.9 12.1c1.9 1.9 4.6 2.4 6.9 1.5l9.1 9.1c.4.4 1 .4 1.4 0l2.3-2.3c.5-.4.5-1 .1-1.4z"
            }
            Self::Goal | Self::WorkPlan => {
                "M12 2a10 10 0 1 0 10 10h-2a8 8 0 1 1-8-8zm0 4a6 6 0 1 0 6 6h-2a4 4 0 1 1-4-4zm0 4a2 2 0 1 0 2 2h8v-2z"
            }
            Self::Scope | Self::Trace | Self::Repository | Self::Integrations => {
                "M9.5 3A6.5 6.5 0 1 0 16 9.5c0-1.3-.38-2.51-1.04-3.53L22 13l-2 2-7.04-7.03A6.47 6.47 0 0 0 9.5 7a2.5 2.5 0 1 1-2.5 2.5H5A4.5 4.5 0 1 0 9.5 5z"
            }
            Self::Work | Self::Items | Self::Requirement | Self::Specify => {
                "M4 3h16a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2m3 4v2h10V7zm0 4v2h10v-2zm0 4v2h7v-2z"
            }
            Self::Policy | Self::Govern => {
                "M12 1 3 5v6c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V5zm-1 16-4-4 1.41-1.41L11 14.17l5.59-5.58L18 10z"
            }
            Self::Feature | Self::Restructure => {
                "M13.5 2v4.5H18V11h4.5v5H18v4.5h-5V16H9v4.5H4V16H1.5v-5H4V6.5h5V2z"
            }
            Self::Philosophy => "M9 21h6v-1H9zm3-20a7 7 0 0 0-4 12.74V17h8v-3.26A7 7 0 0 0 12 1z",
            Self::CodeTests | Self::Verify => {
                "M8 3v2h1v5.27L5.27 18A2 2 0 0 0 7.07 21h9.86a2 2 0 0 0 1.8-3L15 10.27V5h1V3zm3 2h2v5.73L16.5 18h-9L11 10.73z"
            }
            Self::Workspace | Self::Adopt => "M10 4H2v16h20V6H12zm2 8h3V9h2v3h3v2h-3v3h-2v-3h-3z",
            Self::Retire => "M5 4h14v3H5zm1 4h12v12H6zm3 3v2h6v-2z",
            Self::Review => "M6 2h9l5 5v15H6zm8 1.5V8h4.5zM9 12v2h8v-2zm0 4v2h6v-2z",
        }
    }
}

#[component]
pub fn SyuIcon(name: IconName, #[props(default = 20)] size: u16) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 24 24",
            fill: "currentColor", "aria-hidden": "true", "focusable": "false",
            "data-mdi": name.mdi_name(), path { d: name.path() }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_exposes_mdi_names_and_svg_paths() {
        for icon in [
            IconName::Work,
            IconName::Scope,
            IconName::Settings,
            IconName::Deliver,
        ] {
            assert!(icon.mdi_name().starts_with("mdi-"));
            assert!(icon.path().starts_with('M'));
        }
    }
}
