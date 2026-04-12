use crate::model::Issue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TextIssueFormat {
    Validate,
    BrowseNonInteractive,
}

pub(super) fn format_text_issue(issue: &Issue, format: TextIssueFormat) -> Vec<String> {
    match format {
        TextIssueFormat::Validate => {
            let mut lines = vec![format!(
                "- [{:?}] {}{} {}: {}",
                issue.severity,
                issue.code,
                rule_title_suffix(&issue.code),
                issue_subject_with_location(issue),
                issue.message
            )];

            if let Some(rule) = crate::rules::rule_by_code(&issue.code) {
                lines.push(format!(
                    "  rule: {} / {} / {}",
                    rule.genre, rule.code, rule.title
                ));
            }
            if let Some(suggestion) = issue_suggestion(issue, false) {
                lines.push(format!("  suggestion: {suggestion}"));
            }

            lines
        }
        TextIssueFormat::BrowseNonInteractive => {
            let separator = if crate::rules::rule_by_code(&issue.code).is_some() {
                " — "
            } else {
                ": "
            };
            let mut lines = vec![format!(
                "  [{}] {}{}{}",
                issue.code,
                issue_subject_with_location(issue),
                separator,
                issue_message_with_rule_title(issue)
            )];

            if let Some(suggestion) = issue_suggestion(issue, true) {
                lines.push(format!("    suggestion: {suggestion}"));
            }

            lines
        }
    }
}

fn issue_subject_with_location(issue: &Issue) -> String {
    let mut subject = issue.subject.clone();
    if let Some(location) = issue.location.as_deref() {
        subject.push_str(&format!(" ({location})"));
    }
    subject
}

fn issue_message_with_rule_title(issue: &Issue) -> String {
    let message = collapse_whitespace(&issue.message);
    crate::rules::rule_by_code(&issue.code)
        .map(|rule| format!("{}: {message}", rule.title))
        .unwrap_or(message)
}

fn issue_suggestion(issue: &Issue, collapse: bool) -> Option<String> {
    issue.suggestion.as_ref().map(|suggestion| {
        if collapse {
            collapse_whitespace(suggestion)
        } else {
            suggestion.clone()
        }
    })
}

fn rule_title_suffix(code: &str) -> String {
    crate::rules::rule_by_code(code)
        .map(|rule| format!(" ({})", rule.title))
        .unwrap_or_default()
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
