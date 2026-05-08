use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Normalize whitespace and trim surrounding spaces.
pub fn normalize_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Build a normalized planet key.
pub fn build_planet_norm(value: &str) -> String {
    normalize_text(value).to_lowercase()
}

/// Build a deterministic-ish hash for source comparison.
///
/// This is intentionally lightweight for now. A future version can switch to SHA-256
/// once the source model is stable.
pub fn hash_source(value: &serde_json::Value) -> String {
    let mut hasher = DefaultHasher::new();
    value.to_string().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Build a comparison key for CSV overlay matching.
pub fn cmp_key(value: &str) -> String {
    normalize_text(value)
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

/// Remove a trailing Roman numeral suffix from a normalized comparison key.
///
/// This helps matching rows such as `Yavin` against DB records like `Yavin IV`.
pub fn strip_roman_suffix(value: &str) -> String {
    const ROMAN_SUFFIXES: &[&str] = &[
        "i", "ii", "iii", "iv", "v", "vi", "vii", "viii", "ix", "x", "xi", "xii", "xiii", "xiv",
        "xv", "xvi", "xvii", "xviii", "xix", "xx",
    ];

    for suffix in ROMAN_SUFFIXES.iter().rev() {
        if value.len() > suffix.len() && value.ends_with(suffix) {
            let prefix = &value[..value.len() - suffix.len()];
            if !prefix.is_empty() {
                return prefix.to_string();
            }
        }
    }

    value.to_string()
}

/// Returns true when a database row and a CSV overlay row contain identical
/// curated metadata.
pub fn same_overlay_fields(
    db_sector: &str,
    db_region: &str,
    db_grid: &str,
    csv_sector: &str,
    csv_region: &str,
    csv_grid: &str,
) -> bool {
    normalize_text(db_sector) == normalize_text(csv_sector)
        && normalize_text(db_region) == normalize_text(csv_region)
        && normalize_text(db_grid) == normalize_text(csv_grid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_text_collapses_whitespace() {
        assert_eq!(normalize_text("  Core   Worlds "), "Core Worlds");
        assert_eq!(normalize_text("Yavin\tIV"), "Yavin IV");
    }

    #[test]
    fn build_planet_norm_lowercases() {
        assert_eq!(build_planet_norm(" Coruscant "), "coruscant");
    }
}
