//! Human-readable colored output for diagnostic reports.
//!
//! This is a placeholder implementation for Phase 0/1. Full colored output
//! with symbols and tables will be implemented in Phase 5.

use crate::model::{Report, Status};

/// Print a report in human-readable format to stdout.
pub fn print_report(report: &Report) {
    for result in &report.results {
        let symbol = match result.status {
            Status::Ok => "✔",
            Status::Warn => "⚠",
            Status::Fail => "✘",
            Status::Skipped => "⊘",
        };

        println!("{} {:<12} {}", symbol, result.name, result.status);

        for finding in &result.findings {
            println!("  → {}", finding.message);
            if let Some(ref fix) = finding.fix {
                println!("  💡 Fix: {}", fix);
            }
        }
    }

    println!();
    println!("Verdict: {}", report.verdict.summary);
    if let Some(ref fix) = report.verdict.fix {
        println!("Fix: {}", fix);
    }
}
