use crate::model::Report;

pub fn print_report(report: &Report) -> serde_json::Result<()> {
    let json = serde_json::to_string_pretty(report)?;
    println!("{json}");
    Ok(())
}
