use crate::model::{Report, Status};

pub fn print_report(report: &Report) {
    for result in &report.results {
        let symbol = match result.status {
            Status::Ok => "✔",
            Status::Warn => "⚠",
            Status::Fail => "✘",
            Status::Skipped => "⊘",
        };

        println!("{symbol} {:<12} {}", result.name, result.status);

        for finding in &result.findings {
            println!("  → {}", finding.message);
            if let Some(ref fix) = finding.fix {
                println!("  Fix: {fix}");
            }
        }
    }

    println!();
    println!("Verdict: {}", report.verdict.summary);
    if let Some(ref fix) = report.verdict.fix {
        println!("Fix: {fix}");
    }
}
