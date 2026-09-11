//! Presentation capabilities shared by the CLI renderers.
//!
//! This module deliberately contains environment and presentation policy only.
//! Validation and specification semantics must not depend on terminal state.

use clap::ValueEnum;
use std::{fmt::Display, io::IsTerminal};

pub const DEFAULT_TERMINAL_WIDTH: u16 = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorPolicy {
    Enabled,
    Disabled,
}

impl ColorPolicy {
    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorStyle {
    Success,
    Warning,
    Error,
    Link,
    Metadata,
}

impl ColorStyle {
    fn ansi_code(self) -> &'static str {
        match self {
            Self::Success => "32",
            Self::Warning => "33",
            Self::Error => "31",
            Self::Link => "34",
            Self::Metadata => "90",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalCapabilities {
    tty: bool,
    width: u16,
    color: ColorPolicy,
}

impl TerminalCapabilities {
    /// Detect capabilities for the process's standard output.
    pub fn detect() -> Self {
        Self::from_environment(
            std::io::stdout().is_terminal(),
            std::env::var("COLUMNS").ok().as_deref(),
            std::env::var_os("NO_COLOR").is_some(),
        )
    }

    /// Build capabilities from explicit inputs so renderers can be tested
    /// without mutating the process environment.
    pub fn from_environment(tty: bool, columns: Option<&str>, no_color: bool) -> Self {
        let width = columns
            .and_then(|value| value.parse::<u16>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_TERMINAL_WIDTH);
        let color = if tty && !no_color {
            ColorPolicy::Enabled
        } else {
            ColorPolicy::Disabled
        };
        Self { tty, width, color }
    }

    pub fn is_tty(self) -> bool {
        self.tty
    }

    pub fn width(self) -> u16 {
        self.width
    }

    pub fn color_policy(self) -> ColorPolicy {
        self.color
    }

    /// Apply a style when color is permitted, otherwise return plain text.
    pub fn paint(self, style: ColorStyle, value: impl Display) -> String {
        let value = value.to_string();
        if self.color.is_enabled() {
            format!("\x1b[{}m{value}\x1b[0m", style.ansi_code())
        } else {
            value
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tty_without_no_color_enables_color() {
        let capabilities = TerminalCapabilities::from_environment(true, Some("120"), false);

        assert!(capabilities.is_tty());
        assert_eq!(capabilities.width(), 120);
        assert_eq!(capabilities.color_policy(), ColorPolicy::Enabled);
        assert_eq!(
            capabilities.paint(ColorStyle::Success, "passed"),
            "\x1b[32mpassed\x1b[0m"
        );
    }

    #[test]
    fn non_tty_disables_color_even_without_no_color() {
        let capabilities = TerminalCapabilities::from_environment(false, Some("120"), false);

        assert!(!capabilities.is_tty());
        assert_eq!(capabilities.color_policy(), ColorPolicy::Disabled);
        assert_eq!(capabilities.paint(ColorStyle::Error, "failed"), "failed");
    }

    #[test]
    fn no_color_disables_color_for_tty_and_empty_values() {
        let capabilities = TerminalCapabilities::from_environment(true, Some("80"), true);

        assert_eq!(capabilities.color_policy(), ColorPolicy::Disabled);
        assert_eq!(
            capabilities.paint(ColorStyle::Warning, "warning"),
            "warning"
        );
    }

    #[test]
    fn invalid_or_missing_width_uses_the_eighty_column_baseline() {
        for columns in [None, Some(""), Some("0"), Some("not-a-width")] {
            assert_eq!(
                TerminalCapabilities::from_environment(false, columns, false).width(),
                DEFAULT_TERMINAL_WIDTH
            );
        }
    }

    #[test]
    fn output_formats_are_machine_stable_names() {
        assert_eq!(
            OutputFormat::Text.to_possible_value().unwrap().get_name(),
            "text"
        );
        assert_eq!(
            OutputFormat::Json.to_possible_value().unwrap().get_name(),
            "json"
        );
    }
}
