use super::*;

#[component]
pub fn CommandPalette(ui: WorkbenchUiState, active_pane: WorkbenchPane) -> Element {
    let entries = ui.visible_actions();
    let cli_entries = ui.visible_cli_commands();
    let has_entries = !entries.is_empty() || !cli_entries.is_empty();
    let copy = ui.copy();
    rsx! {
        form { class: "group relative", action: "/", method: "get", "data-command-palette": "true",
            input { type: "hidden", name: "pane", value: active_pane.slug() }
            input { type: "hidden", name: "sidebar", value: "0" }
            input { type: "hidden", name: "lang", value: "{ui.locale.slug()}" }
            div { class: "flex items-center gap-2",
                div { class: "relative min-w-0 flex-1",
                    span { class: "pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-sm text-foreground/50", "⌘" }
                    input {
                        class: "w-full rounded-lg border border-border bg-background py-2.5 pl-10 pr-12 text-sm shadow-sm outline-none transition focus:border-foreground/20 focus:shadow-[0_0_0_4px_rgba(15,23,42,0.04)]",
                        value: "{ui.command_query}",
                        name: "query",
                        placeholder: copy.palette_placeholder(),
                        autocomplete: "off",
                        spellcheck: "false",
                        "data-command-input": "true",
                    }
                }
            }
            div { class: "command-palette-results absolute left-0 right-0 top-[calc(100%+0.5rem)] z-20 hidden max-h-[26rem] grid-cols-1 gap-1 overflow-auto rounded-lg border border-border bg-panel p-1.5 shadow-lg", "data-command-results": "true",
                for entry in entries {
                    CommandItem { entry: entry, selected: false, locale: ui.locale }
                }
                for entry in cli_entries {
                    CliCommandItem { entry, locale: ui.locale }
                }
                if !has_entries {
                    EmptyState { title: "No matches".to_string(), body: copy.palette_hint().to_string() }
                }
            }
        }
    }
}

#[component]
pub(super) fn CliCommandItem(entry: CliCommandEntry, locale: Locale) -> Element {
    let state = if entry.mutates_files {
        "confirm"
    } else if entry.requires_input {
        "input"
    } else {
        "ready"
    };
    let href = format!(
        "?pane=commands&sidebar=0&lang={}&cli={}",
        locale.slug(),
        entry.id
    );
    rsx! {
        a {
            class: classes::COMMAND_ITEM,
            href,
            title: "{entry.description}",
            "data-command-item": "true",
            "data-command-text": format!("{} {} {} {}", entry.id, entry.title, entry.description, entry.invocation),
            "data-command-id": entry.id,
            "data-command-title": entry.title,
            div { class: "flex items-start gap-3 text-left",
                span { class: "grid h-8 w-8 shrink-0 place-items-center rounded-full border border-border bg-panel-muted text-xs text-foreground/70", "›" }
                div { class: "flex min-w-0 flex-col gap-1",
                    span { class: "text-sm font-medium text-foreground", "{entry.title}" }
                    span { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{entry.id}" }
                }
            }
            div { class: "flex flex-col items-end gap-1 text-xs uppercase tracking-[0.18em]",
                span { class: "normal-case tracking-normal text-foreground/50", "{state}" }
            }
        }
    }
}
