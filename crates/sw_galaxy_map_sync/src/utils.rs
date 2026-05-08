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
