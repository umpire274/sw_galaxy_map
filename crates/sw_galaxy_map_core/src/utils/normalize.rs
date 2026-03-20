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

pub fn normalize_text(input: &str) -> String {
    let lower = input.trim().to_lowercase();

    // NFKD + rimozione combining marks
    let no_diacritics: String = lower.nfkd().filter(|c| !is_combining_mark(*c)).collect();

    // Solo a-z0-9 -> spazio, collassa spazi.
    let tmp = non_alnum_regex().replace_all(&no_diacritics, " ");

    spaces_regex().replace_all(tmp.trim(), " ").to_string()
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
