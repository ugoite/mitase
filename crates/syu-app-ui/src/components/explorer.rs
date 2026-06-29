use crate::i18n::Locale;
use crate::model::{FocusIntent, PageSection, WorkbenchPage};
use dioxus::prelude::*;

pub fn page_href(
    page: WorkbenchPage,
    locale: Locale,
    section: Option<PageSection>,
    entity: Option<&str>,
    focus: Option<FocusIntent>,
) -> String {
    let mut values = vec![
        format!("page={}", page.slug()),
        format!("lang={}", locale.slug()),
    ];
    if let Some(section) = section {
        values.push(format!("section={}", section.slug()));
    }
    if let Some(entity) = entity {
        values.push(format!("entity={}", urlencoding::encode(entity)));
    }
    if let Some(focus) = focus {
        values.push(format!("focus={}", focus.slug()));
    }
    format!("?{}", values.join("&"))
}

#[component]
pub fn PageHeader(kicker: String, title: String, description: String, actions: Element) -> Element {
    rsx! {
        div { class: "mb-4 flex flex-wrap items-start justify-between gap-4",
            div { class: "min-w-0",
                p { class: "text-[10px] uppercase tracking-[0.22em] text-slate-400", "{kicker}" }
                h1 { class: "mt-1 text-xl font-semibold text-slate-950", "{title}" }
                p { class: "mt-1 text-sm text-slate-600", "{description}" }
            }
            div { class: "flex items-center gap-2", {actions} }
        }
    }
}

#[component]
pub fn EmptyDetail(title: String, body: String) -> Element {
    rsx! { div { class: "grid min-h-64 place-items-center rounded-lg border border-dashed border-slate-300 bg-slate-50 p-8 text-center", div { h2 { class: "font-semibold", "{title}" } p { class: "mt-2 max-w-lg text-sm text-slate-600", "{body}" } } } }
}
