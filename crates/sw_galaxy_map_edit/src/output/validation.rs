//! Output helpers for validation messages.

use crate::validate::field::{ValidationIssue, ValidationSeverity};

pub fn print_validation_issues(issues: &[ValidationIssue]) {
    if issues.is_empty() {
        return;
    }

    println!("Validation:");
    for issue in issues {
        let label = match issue.severity {
            ValidationSeverity::Error => "ERROR",
            ValidationSeverity::Warning => "WARNING",
        };

        println!("- [{}] {}", label, issue.message);
    }
    println!();
}
