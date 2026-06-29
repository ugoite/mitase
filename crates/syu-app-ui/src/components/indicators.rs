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
    const fn circle_class(self) -> &'static str {
        match self {
            Self::Success => {
                "bg-emerald-500 shadow-[inset_0_0_0_1px_rgba(5,150,105,0.18),0_1px_2px_rgba(15,23,42,0.12)]"
            }
            Self::Warning => {
                "bg-amber-400 text-amber-950 shadow-[inset_0_0_0_1px_rgba(180,83,9,0.2),0_1px_2px_rgba(15,23,42,0.12)]"
            }
            Self::Error => {
                "bg-red-500 text-white shadow-[inset_0_0_0_1px_rgba(185,28,28,0.24),0_1px_2px_rgba(15,23,42,0.14)]"
            }
            Self::Disabled => "bg-slate-300 shadow-inner",
            Self::Inferred => "bg-orange-400 shadow-inner",
            Self::Running => "",
        }
    }
}

#[component]
pub fn StatusCircle(status: IndicatorStatus, label: String, count: Option<usize>) -> Element {
    let issue_count = count.unwrap_or(1);
    rsx! {
        span { class: "inline-flex items-center text-xs text-slate-600", title: "{label}", "aria-label": if matches!(status, IndicatorStatus::Warning | IndicatorStatus::Error) { format!("{label}: {issue_count}") } else { label.clone() }, "data-status-circle": format!("{status:?}").to_lowercase(), "data-status-count": if matches!(status, IndicatorStatus::Warning | IndicatorStatus::Error) { Some(issue_count.to_string()) } else { None },
            if status == IndicatorStatus::Running {
                span { class: "relative grid h-5 w-5 shrink-0 place-items-center rounded-full bg-white shadow-[0_1px_3px_rgba(15,23,42,0.14)]", "aria-hidden": "true",
                    span { class: "h-4 w-4 rounded-full border-2 border-blue-100 border-t-blue-600 motion-safe:animate-spin" }
                }
            } else if matches!(status, IndicatorStatus::Warning | IndicatorStatus::Error) {
                span { class: "grid h-5 min-w-5 shrink-0 place-items-center rounded-full px-1 text-[10px] font-bold leading-none tabular-nums {status.circle_class()}", "aria-hidden": "true", "{issue_count}" }
            } else {
                span { class: "h-5 w-5 shrink-0 rounded-full {status.circle_class()}", "aria-hidden": "true" }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn render(status: IndicatorStatus, count: Option<usize>) -> String {
        dioxus_ssr::render_element(rsx! {
            StatusCircle { status, label: "status".to_string(), count }
        })
    }

    #[test]
    fn success_circle_hides_counts() {
        let html = render(IndicatorStatus::Success, Some(12));
        assert!(html.contains("bg-emerald-500"));
        assert!(!html.contains(">12<"));
    }

    #[test]
    fn running_circle_uses_a_reduced_motion_safe_spinner() {
        let html = render(IndicatorStatus::Running, Some(7));
        assert!(html.contains("motion-safe:animate-spin"));
        assert!(html.contains("border-t-blue-600"));
        assert!(!html.contains(">7<"));
    }

    #[test]
    fn warning_and_error_counts_are_centered_inside_the_circle() {
        for status in [IndicatorStatus::Warning, IndicatorStatus::Error] {
            let html = render(status, Some(4));
            assert!(html.contains("place-items-center"));
            assert!(html.contains(">4<"));
        }
    }
}
