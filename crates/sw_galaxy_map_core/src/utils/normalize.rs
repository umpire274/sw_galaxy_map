use crate::model::PC_TO_LY;
use regex::Regex;
use std::sync::OnceLock;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

fn non_alnum_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[^a-z0-9]+").expect("invalid non-alnum regex"))
}

fn spaces_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s+").expect("invalid spaces regex"))
}

/// Rounds a floating-point value to 2 decimal places.
pub fn round_2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub fn normalize_text(input: &str) -> String {
    let lower = input.trim().to_lowercase();

    // NFKD + rimozione combining marks
    let no_diacritics: String = lower.nfkd().filter(|c| !is_combining_mark(*c)).collect();

    // Solo a-z0-9 -> spazio, collassa spazi.
    let tmp = non_alnum_regex().replace_all(&no_diacritics, " ");

    spaces_regex().replace_all(tmp.trim(), " ").to_string()
}

/// Converts a pair of coordinates between parsecs and light years.
///
/// This function performs a **raw conversion**, meaning:
/// - No rounding is applied
/// - Full floating-point precision is preserved
///
/// # Semantics
/// - `"ly"`: interprets the input values as **parsecs** and converts them to **light years**
/// - `"pc"`: interprets the input values as **light years** and converts them to **parsecs**
///
/// # Parameters
/// - `x`: X coordinate
/// - `y`: Y coordinate
/// - `target_unit`: Target unit (`"pc"` or `"ly"`)
///
/// # Returns
/// - `Some((x, y))` with converted values if `target_unit` is valid
/// - `None` if `target_unit` is not supported
///
/// # Notes
/// - This function is intended for **data transformations and internal calculations**
/// - Use [`convert_coordinates_display`] for user-facing output with rounding
///
/// # Example
/// ```
/// let (x, y) = sw_galaxy_map_core::utils::normalize::convert_coordinates_raw(100.0, 50.0, "ly").unwrap();
/// ```
pub fn convert_coordinates_raw(x: f64, y: f64, target_unit: &str) -> Option<(f64, f64)> {
    Some((
        convert_coordinate_raw(x, target_unit)?,
        convert_coordinate_raw(y, target_unit)?,
    ))
}

/// Converts a single coordinate value between parsecs and light years.
///
/// This function performs a **raw conversion**, preserving full precision.
///
/// # Semantics
/// - `"ly"`: interprets the input value as **parsecs** and converts it to **light years**
/// - `"pc"`: interprets the input value as **light years** and converts it to **parsecs**
///
/// # Parameters
/// - `value`: Coordinate value
/// - `target_unit`: Target unit (`"pc"` or `"ly"`)
///
/// # Returns
/// - `Some(value)` with the converted result if `target_unit` is valid
/// - `None` if `target_unit` is not supported
///
/// # Notes
/// - This function is used internally by [`convert_coordinates_raw`]
/// - No rounding is applied
///
/// # Example
/// ```
/// let value = sw_galaxy_map_core::utils::normalize::convert_coordinate_raw(100.0, "ly").unwrap();
/// ```
pub fn convert_coordinate_raw(value: f64, target_unit: &str) -> Option<f64> {
    match target_unit {
        "ly" => Some(value * PC_TO_LY),
        "pc" => Some(value / PC_TO_LY),
        _ => None,
    }
}

/// Converts a pair of coordinates for display purposes.
///
/// This function:
/// - Performs unit conversion using [`convert_coordinates_raw`]
/// - Applies rounding to **2 decimal places**
///
/// # Semantics
/// - `"ly"`: interprets the input values as **parsecs** and converts them to **light years**
/// - `"pc"`: interprets the input values as **light years** and converts them to **parsecs**
///
/// # Parameters
/// - `x`: X coordinate
/// - `y`: Y coordinate
/// - `target_unit`: Target unit (`"pc"` or `"ly"`)
///
/// # Returns
/// - `Some((x, y))` with converted and rounded values if `target_unit` is valid
/// - `None` if `target_unit` is not supported
///
/// # Notes
/// - Intended for **CLI, TUI, and GUI output**
/// - Rounds values to 2 decimal places using internal normalization
/// - Does not modify stored data
///
/// # Example
/// ```
/// let (x, y) = sw_galaxy_map_core::utils::normalize::convert_coordinates_display(100.0, 50.0, "ly").unwrap();
/// ```
#[allow(dead_code)]
pub fn convert_coordinates_display(x: f64, y: f64, target_unit: &str) -> Option<(f64, f64)> {
    let (x, y) = convert_coordinates_raw(x, y, target_unit)?;
    Some((round_2(x), round_2(y)))
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
