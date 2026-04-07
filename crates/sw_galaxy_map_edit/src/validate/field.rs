//! Smart validation helpers for editable field updates.

use crate::edit::field::{EditableField, FieldValue};

/// Severity level for a validation issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSeverity {
    Error,
    Warning,
}

/// One validation issue produced for a field update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub message: String,
}

impl ValidationIssue {
    /// Creates a blocking validation error.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Error,
            message: message.into(),
        }
    }

    /// Creates a non-blocking validation warning.
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Warning,
            message: message.into(),
        }
    }
}

/// Validates a typed field value and returns all detected issues.
pub fn validate_field_value(field: EditableField, value: &FieldValue) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    match (field, value) {
        (EditableField::Planet, FieldValue::Text(text)) => {
            if text.trim().is_empty() {
                issues.push(ValidationIssue::error(
                    "Planet name cannot be empty or whitespace only.",
                ));
            }

            if text != text.trim() {
                issues.push(ValidationIssue::warning(
                    "Planet name contains leading or trailing whitespace.",
                ));
            }
        }

        (
            EditableField::Region
            | EditableField::Sector
            | EditableField::System
            | EditableField::Status
            | EditableField::Reference,
            FieldValue::Text(text),
        ) => {
            if text != text.trim() {
                issues.push(ValidationIssue::warning(
                    "Text value contains leading or trailing whitespace.",
                ));
            }
        }

        (EditableField::Grid, FieldValue::Text(text)) => {
            if text != text.trim() {
                issues.push(ValidationIssue::warning(
                    "Grid value contains leading or trailing whitespace.",
                ));
            }

            if !is_valid_grid(text.trim()) {
                issues.push(ValidationIssue::error(
                    "Grid must follow the expected format, for example 'L-9' or 'AA-12'.",
                ));
            }
        }

        (EditableField::Lat, FieldValue::Real { value, .. }) => {
            if !(-90.0..=90.0).contains(value) {
                issues.push(ValidationIssue::error(
                    "Latitude must be within the range [-90, 90].",
                ));
            }
        }

        (EditableField::Long, FieldValue::Real { value, .. }) => {
            if !(-180.0..=180.0).contains(value) {
                issues.push(ValidationIssue::error(
                    "Longitude must be within the range [-180, 180].",
                ));
            }
        }

        (EditableField::X | EditableField::Y, FieldValue::Real { value, .. }) => {
            if !value.is_finite() {
                issues.push(ValidationIssue::error(
                    "Coordinate value must be a finite number.",
                ));
            }
        }

        (_, FieldValue::Null) => {
            // Nullability is already handled by parse_input / apply logic.
            // No extra issues here for now.
        }

        (_, FieldValue::Real { value, .. }) => {
            if !value.is_finite() {
                issues.push(ValidationIssue::error(
                    "Numeric value must be a finite number.",
                ));
            }
        }

        _ => {}
    }

    issues
}

fn is_valid_grid(input: &str) -> bool {
    let Some((left, right)) = input.split_once('-') else {
        return false;
    };

    if left.is_empty() || right.is_empty() {
        return false;
    }

    if !left.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }

    if !right.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    true
}

/// Returns true if the issue list contains at least one blocking error.
pub fn has_errors(issues: &[ValidationIssue]) -> bool {
    issues
        .iter()
        .any(|issue| issue.severity == ValidationSeverity::Error)
}
