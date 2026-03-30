//! Issue #140 — Interactive CLI prompts via `dialoguer`.
//!
//! All prompts are disabled when:
//!   - `--no-confirm` flag is set (auto-confirms everything), or
//!   - stdout is not a TTY (CI / piped output).

use anyhow::Result;
use dialoguer::{Confirm, Select};

/// Ask the user to confirm an action.
///
/// Returns `true` immediately (without prompting) when `no_confirm` is set
/// or when stdout is not a TTY.
pub fn confirm(message: &str, no_confirm: bool) -> Result<bool> {
    if no_confirm || !console::Term::stdout().is_term() {
        return Ok(true);
    }
    Confirm::new()
        .with_prompt(message)
        .default(false)
        .interact()
        .map_err(Into::into)
}

/// Present a selection menu and return the chosen index.
///
/// Falls back to `default_index` without prompting when `no_confirm` is set
/// or when stdout is not a TTY.
pub fn select(message: &str, items: &[&str], default_index: usize, no_confirm: bool) -> Result<usize> {
    if no_confirm || !console::Term::stdout().is_term() {
        return Ok(default_index);
    }
    Select::new()
        .with_prompt(message)
        .items(items)
        .default(default_index)
        .interact()
        .map_err(Into::into)
}
