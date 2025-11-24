//! Formatting and reporting for health check results

use colored::Colorize;
use tabled::{
    builder::Builder,
    settings::{Alignment, Modify, Style, object::Rows},
};

use super::runner::HealthCheckReport;

/// Formats a health check report as a pretty table
pub fn format_report(report: &HealthCheckReport) -> String {
    let mut builder = Builder::default();

    // Add header
    builder.push_record(["System", "Status", "Duration", "Message"]);

    // Add rows for each check
    for (name, result) in &report.results {
        let duration_str = format!("{:.2?}", result.duration);
        builder.push_record([
            name.as_str(),
            &result.status.as_colored_str(),
            &duration_str,
            &result.message,
        ]);
    }

    let mut table = builder.build();
    table
        .with(Style::rounded())
        .with(Modify::new(Rows::first()).with(Alignment::center()));

    let mut output = String::new();
    output.push_str(&table.to_string());
    output.push('\n');

    // Add summary
    output.push_str(&format_summary(report));

    output
}

/// Formats the summary section of the report
fn format_summary(report: &HealthCheckReport) -> String {
    let mut summary = String::new();

    summary.push_str(&format!("\n{}\n", "Summary".bold().underline()));
    summary.push_str(&format!("  Total checks: {}\n", report.total));
    summary.push_str(&format!("  {} Passed: {}\n", "✓".green(), report.passed));

    if report.warned > 0 {
        summary.push_str(&format!("  {} Warned: {}\n", "⚠".yellow(), report.warned));
    }

    if report.failed > 0 {
        summary.push_str(&format!("  {} Failed: {}\n", "✗".red(), report.failed));
    }

    // Overall status
    summary.push('\n');
    if report.is_healthy() {
        if report.has_warnings() {
            summary.push_str(&format!(
                "  {}\n",
                "Overall: HEALTHY (with warnings)".yellow().bold()
            ));
        } else {
            summary.push_str(&format!("  {}\n", "Overall: HEALTHY".green().bold()));
        }
    } else {
        summary.push_str(&format!("  {}\n", "Overall: UNHEALTHY".red().bold()));
    }

    summary
}

/// Prints a health check report to stdout
pub fn print_report(report: &HealthCheckReport) {
    println!("{}", format_report(report));

    // Print details for any checks that have them
    for (name, result) in &report.results {
        if let Some(details) = &result.details {
            println!("\n{} Details:", name.bold());
            println!("{}", details);
        }
    }
}
