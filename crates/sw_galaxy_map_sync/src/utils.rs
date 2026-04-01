use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;

use crate::models::{DbPlanetRow, SyncRow};

/// Normalize whitespace without altering semantic casing.
pub fn normalize_field(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Build a normalized key for planet name matching/storage.
pub fn build_planet_norm(value: &str) -> String {
    normalize_field(value).to_lowercase()
}

/// Normalize a value for case-insensitive comparison.
pub fn cmp_key(value: &str) -> String {
    normalize_field(value).to_lowercase()
}

/// Remove a trailing Roman numeral suffix such as III, IV, V, VI, VII, VIII, IX, X.
pub fn strip_roman_suffix(value: &str) -> String {
    let re = Regex::new(r"\s+(i|ii|iii|iv|v|vi|vii|viii|ix|x)$").expect("valid regex");
    re.replace(value, "").to_string()
}

/// Return true when two names differ only by a trailing Roman numeral suffix.
pub fn names_match_by_roman_suffix(left: &str, right: &str) -> bool {
    let left_key = cmp_key(left);
    let right_key = cmp_key(right);

    if left_key == right_key {
        return true;
    }

    let left_base = strip_roman_suffix(&left_key);
    let right_base = strip_roman_suffix(&right_key);

    left_base == right_base
}

/// Check if DB row is exactly equal to CSV row on the 4 relevant fields.
pub fn is_exact_match(db: &DbPlanetRow, row: &SyncRow) -> bool {
    cmp_key(&db.planet) == cmp_key(&row.system)
        && cmp_key(&db.sector) == cmp_key(&row.sector)
        && cmp_key(&db.region) == cmp_key(&row.region)
        && cmp_key(&db.grid) == cmp_key(&row.grid)
}

/// Check whether a CSV row is invalid for synchronization.
pub fn is_invalid_csv_row(row: &SyncRow) -> bool {
    row.system.is_empty() || row.region.is_empty() || row.grid.is_empty()
}

/// Build a standard progress bar for long-running operations.
pub fn make_progress_bar(len: u64, prefix: &str) -> Result<ProgressBar> {
    let pb = ProgressBar::new(len);

    let style = ProgressStyle::with_template(
        "{prefix:<14}: [{percent:>3}%] {bar:40.cyan/blue} {pos}/{len}",
    )?
    .progress_chars("#.-");

    pb.set_style(style);
    pb.set_prefix(prefix.to_string());

    Ok(pb)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DbPlanetRow, SyncRow};

    /// Test basic whitespace normalization.
    #[test]
    fn normalize_field_collapses_whitespace() {
        assert_eq!(normalize_field("  Yavin   IV "), "Yavin IV");
        assert_eq!(normalize_field("Core\tWorlds"), "Core Worlds");
        assert_eq!(normalize_field(""), "");
    }

    /// Test case-insensitive comparison key generation.
    #[test]
    fn cmp_key_is_lowercase_and_trimmed() {
        assert_eq!(cmp_key("  Coruscant "), "coruscant");
        assert_eq!(cmp_key("Outer   Rim Territories"), "outer rim territories");
    }

    /// Test Roman numeral suffix stripping.
    #[test]
    fn strip_roman_suffix_removes_valid_suffix() {
        assert_eq!(strip_roman_suffix("yavin iv"), "yavin");
        assert_eq!(strip_roman_suffix("endor iii"), "endor");
        assert_eq!(strip_roman_suffix("coruscant"), "coruscant");
    }

    /// Test suffix-based name matching.
    #[test]
    fn names_match_by_roman_suffix_works() {
        assert!(names_match_by_roman_suffix("Yavin", "Yavin IV"));
        assert!(names_match_by_roman_suffix("Yavin IV", "Yavin"));
        assert!(names_match_by_roman_suffix("Tython", "Tython"));
        assert!(!names_match_by_roman_suffix("Yavin", "Endor IV"));
    }

    /// Test strict 4-field exact match.
    #[test]
    fn is_exact_match_requires_all_four_fields() {
        let db = DbPlanetRow {
            fid: 1,
            planet: "Yavin".to_string(),
            sector: "Gordian Reach".to_string(),
            region: "Outer Rim Territories".to_string(),
            grid: "P-17".to_string(),
        };

        let same = SyncRow {
            system: "Yavin".to_string(),
            sector: "Gordian Reach".to_string(),
            region: "Outer Rim Territories".to_string(),
            grid: "P-17".to_string(),
        };

        let diff_name = SyncRow {
            system: "Yavin IV".to_string(),
            sector: "Gordian Reach".to_string(),
            region: "Outer Rim Territories".to_string(),
            grid: "P-17".to_string(),
        };

        let diff_grid = SyncRow {
            system: "Yavin".to_string(),
            sector: "Gordian Reach".to_string(),
            region: "Outer Rim Territories".to_string(),
            grid: "P-18".to_string(),
        };

        assert!(is_exact_match(&db, &same));
        assert!(!is_exact_match(&db, &diff_name));
        assert!(!is_exact_match(&db, &diff_grid));
    }

    /// Test invalid CSV row detection.
    #[test]
    fn is_invalid_csv_row_detects_missing_required_fields() {
        let valid = SyncRow {
            system: "Coruscant".to_string(),
            sector: "Corusca".to_string(),
            region: "Core Worlds".to_string(),
            grid: "L-9".to_string(),
        };

        let missing_system = SyncRow {
            system: "".to_string(),
            sector: "Corusca".to_string(),
            region: "Core Worlds".to_string(),
            grid: "L-9".to_string(),
        };

        let missing_region = SyncRow {
            system: "Coruscant".to_string(),
            sector: "Corusca".to_string(),
            region: "".to_string(),
            grid: "L-9".to_string(),
        };

        let missing_grid = SyncRow {
            system: "Coruscant".to_string(),
            sector: "Corusca".to_string(),
            region: "Core Worlds".to_string(),
            grid: "".to_string(),
        };

        assert!(!is_invalid_csv_row(&valid));
        assert!(is_invalid_csv_row(&missing_system));
        assert!(is_invalid_csv_row(&missing_region));
        assert!(is_invalid_csv_row(&missing_grid));
    }
}
