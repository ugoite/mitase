use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndicatorStatus {
    Success,
    Warning,
    Error,
    Running,
    Disabled,
    Inferred,
}

impl IndicatorStatus {
    const fn class(self) -> &'static str {
        match self {
            Self::Success => "bg-emerald-500",
            Self::Warning => "bg-amber-500",
            Self::Error => "bg-red-500",
            Self::Running => "bg-blue-500 motion-safe:animate-pulse",
            Self::Disabled => "bg-slate-400",
            Self::Inferred => "bg-orange-400",
        }
    }
}

#[component]
pub fn StatusCircle(status: IndicatorStatus, label: String, count: Option<usize>) -> Element {
    rsx! {
        span { class: "inline-flex items-center gap-1.5 text-xs text-slate-600", title: "{label}", "aria-label": "{label}",
            span { class: "h-2.5 w-2.5 shrink-0 rounded-full {status.class()}", "aria-hidden": "true" }
            if let Some(count) = count { span { "{count}" } }
        }
    }
}

#[component]
pub fn EvidenceStamp(label: String, inferred: bool) -> Element {
    let class = if inferred {
        "border-orange-200 bg-orange-50 text-orange-700"
    } else {
        "border-emerald-200 bg-emerald-50 text-emerald-700"
    };
    rsx! { span { class: "inline-flex rounded-full border px-2.5 py-1 text-xs {class}", "{label}" } }
}
