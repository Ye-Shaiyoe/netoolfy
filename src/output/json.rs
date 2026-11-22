//! JSON output formatter for diagnostic reports.
//!
//! Outputs the full [`Report`] as a stable JSON structure suitable for
//! piping into other tools.

use crate::model::Report;

/// Print a report as JSON to stdout.
///
/// # Errors
///
/// Returns an error if serialization fails (should not happen with our types).
pub fn print_report(report: &Report) -> serde_json::Result<()> {
    let json = serde_json::to_string_pretty(report)?;
    println!("{json}");
    Ok(())
}
