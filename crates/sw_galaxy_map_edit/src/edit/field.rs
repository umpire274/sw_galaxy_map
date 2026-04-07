//! Editable field definitions.

use std::fmt;

/// Supported editable planet fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditableField {
    Planet,
    Region,
    Sector,
    System,
    Grid,
    X,
    Y,
    Lat,
    Long,
    Status,
    Reference,
}

impl EditableField {
    /// Returns all fields available in the interactive editor.
    pub fn all() -> &'static [EditableField] {
        &[
            EditableField::Planet,
            EditableField::Region,
            EditableField::Sector,
            EditableField::System,
            EditableField::Grid,
            EditableField::X,
            EditableField::Y,
            EditableField::Lat,
            EditableField::Long,
            EditableField::Status,
            EditableField::Reference,
        ]
    }

    /// Returns true if the field supports NULL.
    pub fn nullable(self) -> bool {
        match self {
            EditableField::Planet => false,
            EditableField::X => false,
            EditableField::Y => false,
            EditableField::Region
            | EditableField::Sector
            | EditableField::System
            | EditableField::Grid
            | EditableField::Lat
            | EditableField::Long
            | EditableField::Status
            | EditableField::Reference => true,
        }
    }

    /// Returns the SQL column name.
    pub fn column_name(self) -> &'static str {
        match self {
            EditableField::Planet => "Planet",
            EditableField::Region => "Region",
            EditableField::Sector => "Sector",
            EditableField::System => "System",
            EditableField::Grid => "Grid",
            EditableField::X => "X",
            EditableField::Y => "Y",
            EditableField::Lat => "lat",
            EditableField::Long => "long",
            EditableField::Status => "status",
            EditableField::Reference => "ref",
        }
    }

    /// Parses a field name from CLI input.
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "planet" => Some(EditableField::Planet),
            "region" => Some(EditableField::Region),
            "sector" => Some(EditableField::Sector),
            "system" => Some(EditableField::System),
            "grid" => Some(EditableField::Grid),
            "x" => Some(EditableField::X),
            "y" => Some(EditableField::Y),
            "lat" => Some(EditableField::Lat),
            "long" => Some(EditableField::Long),
            "status" => Some(EditableField::Status),
            "reference" | "ref" => Some(EditableField::Reference),
            _ => None,
        }
    }

    /// Returns the accepted CLI field names.
    pub fn accepted_names() -> &'static [&'static str] {
        &[
            "planet",
            "region",
            "sector",
            "system",
            "grid",
            "x",
            "y",
            "lat",
            "long",
            "status",
            "reference",
        ]
    }
}

impl fmt::Display for EditableField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EditableField::Planet => "planet",
            EditableField::Region => "region",
            EditableField::Sector => "sector",
            EditableField::System => "system",
            EditableField::Grid => "grid",
            EditableField::X => "x",
            EditableField::Y => "y",
            EditableField::Lat => "lat",
            EditableField::Long => "long",
            EditableField::Status => "status",
            EditableField::Reference => "reference",
        };
        f.write_str(s)
    }
}

/// Typed value ready to be written to the database.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Text(String),
    Real {
        value: f64,
        raw: String, // Preserve original input for better error messages
    },
    Null,
}
