// REQ-CORE-009
// FEAT-INIT-007

use std::io::{self, IsTerminal, Write};

use anyhow::{Context, Result, bail};

pub(crate) trait PromptIo {
    fn is_terminal(&self) -> bool;
    fn prompt_line(&mut self, label: &str, default: Option<&str>) -> Result<String>;
}

pub(crate) struct StdioPromptIo;

impl PromptIo for StdioPromptIo {
    fn is_terminal(&self) -> bool {
        io::stdin().is_terminal() && io::stdout().is_terminal()
    }

    fn prompt_line(&mut self, label: &str, default: Option<&str>) -> Result<String> {
        match default {
            Some(value) => print!("{label} [{value}]: "),
            None => print!("{label}: "),
        }
        io::stdout()
            .flush()
            .context("failed to flush interactive prompt")?;

        let mut input = String::new();
        let bytes_read = io::stdin()
            .read_line(&mut input)
            .context("failed to read interactive input")?;
        normalize_prompt_response(input, bytes_read)
    }
}

fn normalize_prompt_response(input: String, bytes_read: usize) -> Result<String> {
    if bytes_read == 0 {
        bail!("interactive input closed before a response was entered");
    }

    Ok(input.trim().to_string())
}

pub(crate) fn ensure_prompt_terminal(prompt_io: &impl PromptIo, message: &str) -> Result<()> {
    if prompt_io.is_terminal() {
        return Ok(());
    }

    bail!("{message}");
}

pub(crate) fn prompt_required(prompt_io: &mut impl PromptIo, label: &str) -> Result<String> {
    loop {
        let raw = prompt_io.prompt_line(label, None)?;
        if !raw.is_empty() {
            return Ok(raw);
        }
        eprintln!("{label} is required.");
    }
}

pub(crate) fn prompt_with_default(
    prompt_io: &mut impl PromptIo,
    label: &str,
    default: &str,
) -> Result<String> {
    let raw = prompt_io.prompt_line(label, Some(default))?;
    if raw.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(raw)
    }
}

pub(crate) fn prompt_optional(
    prompt_io: &mut impl PromptIo,
    label: &str,
    default: Option<&str>,
) -> Result<Option<String>> {
    let raw = prompt_io.prompt_line(label, default)?;
    if raw.is_empty() {
        Ok(None)
    } else {
        Ok(Some(raw))
    }
}

pub(crate) fn prompt_optional_with_default(
    prompt_io: &mut impl PromptIo,
    label: &str,
    default: Option<&str>,
) -> Result<Option<String>> {
    let raw = prompt_io.prompt_line(label, default)?;
    if raw.is_empty() {
        Ok(default
            .map(std::string::ToString::to_string)
            .filter(|value| !value.is_empty()))
    } else {
        Ok(Some(raw))
    }
}

pub(crate) fn prompt_bool(
    prompt_io: &mut impl PromptIo,
    label: &str,
    default: bool,
) -> Result<bool> {
    let default_value = if default { "yes" } else { "no" };
    loop {
        let raw = prompt_with_default(prompt_io, label, default_value)?;
        match raw.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" | "true" => return Ok(true),
            "n" | "no" | "false" => return Ok(false),
            _ => eprintln!("Please enter yes or no."),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PromptIo, normalize_prompt_response, prompt_bool, prompt_optional_with_default};
    use anyhow::Result;
    use std::collections::VecDeque;

    #[derive(Default)]
    struct FakePromptIo {
        lines: VecDeque<String>,
    }

    impl PromptIo for FakePromptIo {
        fn is_terminal(&self) -> bool {
            true
        }

        fn prompt_line(&mut self, _label: &str, _default: Option<&str>) -> Result<String> {
            Ok(self.lines.pop_front().unwrap_or_default())
        }
    }

    #[test]
    fn prompt_optional_with_default_uses_default_for_blank_lines() {
        let mut prompt_io = FakePromptIo {
            lines: VecDeque::from([String::new()]),
        };
        assert!(prompt_io.is_terminal());

        let value = prompt_optional_with_default(&mut prompt_io, "Shared ID stem", Some("store"))
            .expect("blank responses should keep the default");

        assert_eq!(value.as_deref(), Some("store"));
    }

    #[test]
    fn prompt_bool_retries_invalid_values_and_accepts_false_aliases() {
        let mut prompt_io = FakePromptIo {
            lines: VecDeque::from(["maybe".to_string(), "no".to_string()]),
        };
        assert!(prompt_io.is_terminal());

        let value = prompt_bool(
            &mut prompt_io,
            "Enable stricter validation defaults now?",
            true,
        )
        .expect("boolean prompts should retry");

        assert!(!value);
    }

    #[test]
    fn normalize_prompt_response_rejects_eof() {
        let error = normalize_prompt_response(String::new(), 0)
            .expect_err("EOF should abort interactive prompting");

        assert!(error.to_string().contains("interactive input closed"));
    }

    #[test]
    fn normalize_prompt_response_preserves_blank_lines_when_input_was_read() {
        let value = normalize_prompt_response("\n".to_string(), 1)
            .expect("blank responses should remain valid prompt answers");

        assert!(value.is_empty());
    }
}
