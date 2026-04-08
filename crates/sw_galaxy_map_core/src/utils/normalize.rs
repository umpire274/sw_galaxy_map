use regex::Regex;
use std::sync::OnceLock;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;
use crate::model::PC_TO_LY;

fn non_alnum_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[^a-z0-9]+").expect("invalid non-alnum regex"))
}

fn spaces_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s+").expect("invalid spaces regex"))
}

pub fn normalize_text(input: &str) -> String {
    let lower = input.trim().to_lowercase();

    // NFKD + rimozione combining marks
    let no_diacritics: String = lower.nfkd().filter(|c| !is_combining_mark(*c)).collect();

    // Solo a-z0-9 -> spazio, collassa spazi.
    let tmp = non_alnum_regex().replace_all(&no_diacritics, " ");

    spaces_regex().replace_all(tmp.trim(), " ").to_string()
}

/// Converts a coordinate pair to the requested unit.
///
/// Semantics:
/// - "ly": input is interpreted as parsecs and converted to light years
/// - "pc": input is interpreted as light years and converted to parsecs
pub fn convert_coordinates_to(x: f64, y: f64, target_unit: &str) -> Option<(f64, f64)> {
    match target_unit {
        "ly" => Some((x * PC_TO_LY, y * PC_TO_LY)),
        "pc" => Some((x / PC_TO_LY, y / PC_TO_LY)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_text;

    #[test]
    fn normalize_text_removes_diacritics_and_extra_separators() {
        assert_eq!(
            normalize_text("  Tàtôôïne -- Outer   Rim "),
            "tatooine outer rim"
        );
    }

    #[test]
    fn normalize_text_returns_empty_string_for_punctuation_only_input() {
        assert_eq!(normalize_text("  --__...  "), "");
    }
}
