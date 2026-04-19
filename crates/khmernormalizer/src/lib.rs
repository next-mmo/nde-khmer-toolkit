//! Khmer text normalization library.
//!
//! Ported from Python `khmernormalizer` package.
//! Provides Unicode normalization, character replacements,
//! quote fixing, and Khmer-specific syllable reordering.

pub mod khnormal;
pub mod mappings;

use mappings::*;
use unicode_normalization::UnicodeNormalization;

/// Normalize Khmer text.
///
/// Performs the following steps:
/// 1. Remove zero-width spaces (if `remove_zwsp` is true)
/// 2. Fix quotes (single and double)
/// 3. Remove duplicate punctuation
/// 4. Clean trailing duplicate vowels
/// 5. Clean duplicate coengs
/// 6. Unicode NFKC normalization
/// 7. Apply character replacements (accented Latin → ASCII)
/// 8. Apply Unicode replacements (Khmer-specific fixes)
/// 9. Normalize whitespace
/// 10. Normalize ellipsis
/// 11. Remove space before punctuation
/// 12. Khmer syllable-level normalization
pub fn normalize(text: &str, remove_zwsp: bool) -> String {
    let mut text = text.to_string();

    // 1. Remove ZWSP and related characters
    if remove_zwsp {
        text = ZWSP_RE.replace_all(&text, "").to_string();
    }

    // 2. Fix quotes
    text = SINGLE_QUOTE_REGEX.replace_all(&text, "'").to_string();
    text = DOUBLE_QUOTE_REGEX.replace_all(&text, "\"").to_string();

    // 3. Remove duplicate punctuation
    text = MULTIPLE_PUNCT_REGEX.replace_all(&text, "$1").to_string();

    // 4. Clean trailing duplicate vowels
    text = TRAILING_VOWEL_RE.replace_all(&text, "$1").to_string();

    // 5. Clean duplicate coengs
    text = DUPLICATE_COENG_RE.replace_all(&text, "$1").to_string();

    // 6. Unicode NFKC normalization
    text = text.nfkc().collect::<String>();

    // 7. Apply character replacements
    text = text
        .chars()
        .map(|c| {
            if let Some(replacement) = char_replacement(c) {
                replacement.to_string()
            } else {
                c.to_string()
            }
        })
        .collect();

    // 8. Apply Unicode replacements
    text = UNICODE_REPLACEMENTS_REGEX
        .replace_all(&text, |caps: &fancy_regex::Captures| {
            let matched = caps.get(0).unwrap().as_str();
            UNICODE_REPLACEMENTS_MAP
                .get(matched)
                .copied()
                .unwrap_or(matched)
                .to_string()
        })
        .to_string();

    // 9. Normalize whitespace (but not newlines)
    text = WHITESPACES_HANDLER_REGEX.replace_all(&text, " ").to_string();

    // 10. Ellipsis
    text = ELLIPSIS_RE.replace_all(&text, "\u{2026}").to_string();

    // 11. Remove space before punctuation
    text = SPACE_BEFORE_PUNCT_RE.replace_all(&text, "$1").to_string();

    // 12. Remove space before repeat char
    text = SPACE_BEFORE_REPEAT_RE.replace_all(&text, "ៗ").to_string();

    // 13. Khmer syllable-level normalization
    let text = text.trim().to_string();
    khnormal::khmer_normalize(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_basic() {
        let result = normalize("ការ​ប្រើ​ប្រាស់", true);
        // ZWSP should be removed
        assert!(!result.contains('\u{200b}'));
        assert!(result.contains("ការ"));
    }

    #[test]
    fn test_normalize_quotes() {
        let result = normalize("«hello»", false);
        assert_eq!(result, "\"hello\"");
    }

    #[test]
    fn test_normalize_duplicate_punct() {
        let result = normalize("។។។", false);
        assert_eq!(result, "។");
    }

    #[test]
    fn test_normalize_ellipsis() {
        let result = normalize("test...", false);
        assert_eq!(result, "test\u{2026}");
    }
}
