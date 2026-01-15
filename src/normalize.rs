use regex::Regex;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

pub fn normalize_text(input: &str) -> String {
    let lower = input.trim().to_lowercase();

    // NFKD + rimozione combining marks
    let no_diacritics: String = lower.nfkd().filter(|c| !is_combining_mark(*c)).collect();

    // Solo a-z0-9 -> spazio, collassa spazi
    // Nota: regex compilata a ogni chiamata; per performance si può usare lazy_static/once_cell.
    let re_non_alnum = Regex::new(r"[^a-z0-9]+").unwrap();
    let tmp = re_non_alnum.replace_all(&no_diacritics, " ");

    let re_spaces = Regex::new(r"\s+").unwrap();
    re_spaces.replace_all(tmp.trim(), " ").to_string()
}
