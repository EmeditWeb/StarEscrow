//! Issue #139 — Progress bar support for batch CLI operations via `indicatif`.
//!
//! Progress bars are automatically disabled when:
//!   - `--json` flag is set, or
//!   - stdout is not a TTY (CI / piped output).

use indicatif::{ProgressBar, ProgressStyle};

/// Create a progress bar for a batch operation of `total` items.
///
/// Returns `None` when `json_mode` is true or stdout is not a TTY,
/// so callers can skip rendering entirely.
pub fn batch_bar(total: u64, json_mode: bool) -> Option<ProgressBar> {
    if json_mode || !console::Term::stdout().is_term() {
        return None;
    }
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)",
        )
        .unwrap()
        .progress_chars("=>-"),
    );
    Some(pb)
}

/// Advance the progress bar by one step (no-op if `None`).
pub fn tick(pb: &Option<ProgressBar>) {
    if let Some(bar) = pb {
        bar.inc(1);
    }
}

/// Finish and clear the progress bar (no-op if `None`).
pub fn finish(pb: Option<ProgressBar>) {
    if let Some(bar) = pb {
        bar.finish_and_clear();
    }
}
