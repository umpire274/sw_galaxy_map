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
    /// All fields available in the interactive editor.
    pub const ALL: [EditableField; 11] = [
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
    ];

    /// Returns all fields available in the interactive editor.
    pub fn all() -> &'static [EditableField] {
        &Self::ALL
    }

    /// Returns true if the field supports NULL.
    pub fn nullable(self) -> bool {
        !matches!(self, EditableField::Planet | EditableField::X | EditableField::Y)
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

    /// Returns the preferred CLI field name.
    pub fn cli_name(self) -> &'static str {
        match self {
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
            "ref",
        ]
    }

    /// Returns the logical value type accepted by the field.
    pub fn value_kind(self) -> FieldKind {
        match self {
            EditableField::X | EditableField::Y | EditableField::Lat | EditableField::Long => {
                FieldKind::Real
            }
            EditableField::Planet
            | EditableField::Region
            | EditableField::Sector
            | EditableField::System
            | EditableField::Grid
            | EditableField::Status
            | EditableField::Reference => FieldKind::Text,
        }
    }

    /// Returns a short human-readable description.
    pub fn description(self) -> &'static str {
        match self {
            EditableField::Planet => "Primary planet display name",
            EditableField::Region => "Galaxy region name",
            EditableField::Sector => "Sector name",
            EditableField::System => "System name",
            EditableField::Grid => "Grid reference such as L-9",
            EditableField::X => "Galactic X coordinate",
            EditableField::Y => "Galactic Y coordinate",
            EditableField::Lat => "Optional latitude value",
            EditableField::Long => "Optional longitude value",
            EditableField::Status => "Status or editorial note",
            EditableField::Reference => "Reference/source field",
        }
    }
}

impl fmt::Display for EditableField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.cli_name())
    }
}

/// Typed value ready to be written to the database.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Text(String),
    Real {
        value: f64,
        raw: String,
    },
    Null,
}

/// Logical value type accepted by an editable field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    Real,
}

impl FieldKind {
    /// Returns the lowercase textual representation of the field kind.
    pub fn as_str(self) -> &'static str {
        match self {
            FieldKind::Text => "text",
            FieldKind::Real => "real",
        }
    }
}