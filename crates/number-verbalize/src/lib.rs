//! Khmer number verbalization.
//!
//! Converts numeric values to Khmer text representations.
//! Ported from Python `kfa/number_verbalize.py`.

use once_cell::sync::Lazy;
use regex::Regex;

const DEFAULT_SEPARATOR: &str = "▁";
const DEFAULT_MINUS: &str = "ដក";
const DEFAULT_DELIMITER: &str = "ក្បៀស";

static DIGITS: &[&str] = &[
    "សូន្យ", "មួយ", "ពីរ", "បី", "បួន",
    "ប្រាំ", "ប្រាំមួយ", "ប្រាំពីរ", "ប្រាំបី", "ប្រាំបួន",
];

static PREFIX: &[&str] = &[
    "", "ដប់", "ម្ភៃ", "សាមសិប", "សែសិប",
    "ហាសិប", "ហុកសិប", "ចិតសិប", "ប៉ែតសិប", "កៅសិប",
];

/// (exponent, suffix) pairs for large numbers.
static SUFFIX: &[(u32, &str)] = &[
    (2, "រយ"),
    (3, "ពាន់"),
    (4, "ម៉ឺន"),
    (5, "សែន"),
    (6, "លាន"),
    (9, "ប៊ីលាន"),
    (12, "ទ្រីលាន"),
    (15, "ក្វាទ្រីលាន"),
    (18, "គ្វីនទីលាន"),
    (21, "សិចទីលាន"),
    (24, "សិបទីលាន"),
    (27, "អុកទីលាន"),
    (30, "ណូនីលាន"),
    (33, "ដេស៊ីលាន"),
    (36, "អាន់ដេស៊ីលាន"),
];

fn get_suffix(exp: u32) -> Option<&'static str> {
    SUFFIX.iter().find(|(e, _)| *e == exp).map(|(_, s)| *s)
}

/// Convert an integer to Khmer text.
pub fn integer(num: i64, sep: &str) -> String {
    integer_with_minus(num, sep, DEFAULT_MINUS)
}

fn integer_with_minus(num: i64, sep: &str, minus_sign: &str) -> String {
    if num < 0 {
        return format!("{}{}{}", minus_sign, sep, integer_with_minus(num.abs(), sep, minus_sign));
    }

    let num = num as u64;

    if num < 10 {
        return DIGITS[num as usize].to_string();
    }

    if num < 100 {
        let r = num % 10;
        if r == 0 {
            return PREFIX[(num / 10) as usize].to_string();
        }
        return format!(
            "{}{}",
            PREFIX[(num / 10) as usize],
            integer_with_minus(r as i64, sep, minus_sign)
        );
    }

    // Find the appropriate exponent
    let mut exp = (num as f64).log10().floor() as u32;
    while exp > 0 && get_suffix(exp).is_none() {
        exp -= 1;
    }

    let d = 10u64.pow(exp);
    let pre = integer_with_minus((num / d) as i64, sep, minus_sign);
    let suf = get_suffix(exp).unwrap_or("");
    let r = num % d;

    if r == 0 {
        format!("{}{}", pre, suf)
    } else {
        format!(
            "{}{}{}{}",
            pre,
            suf,
            sep,
            integer_with_minus(r as i64, sep, minus_sign)
        )
    }
}

/// Convert a decimal number to Khmer text.
pub fn decimal(num: f64, sep: &str) -> String {
    decimal_full(num, sep, DEFAULT_DELIMITER, DEFAULT_MINUS)
}

fn decimal_full(num: f64, sep: &str, delimiter: &str, minus_sign: &str) -> String {
    if num.is_nan() {
        return String::new();
    }

    // If it's a whole number
    if num.fract() == 0.0 {
        return integer_with_minus(num as i64, sep, minus_sign);
    }

    let s = format!("{}", num);
    let parts: Vec<&str> = s.split('.').collect();
    let right = parts.get(1).unwrap_or(&"0");

    let word = if right.len() > 3 {
        right
            .chars()
            .map(|c| integer(c.to_digit(10).unwrap_or(0) as i64, sep))
            .collect::<Vec<_>>()
            .join(sep)
    } else {
        integer(right.parse::<i64>().unwrap_or(0), sep)
    };

    let n = num.trunc() as i64;
    let prefix_str = if n == 0 && num < 0.0 {
        minus_sign.to_string()
    } else {
        String::new()
    };

    format!(
        "{}{}{}{}{}",
        prefix_str,
        integer_with_minus(n, sep, minus_sign),
        sep,
        delimiter,
        word
    )
}

/// Khmer digit translation table: ០-៩ → 0-9, swap . and ,
pub fn number_translate2ascii(text: &str) -> String {
    static RE_KM_NUM: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"([\u{17e0}-\u{17e9}]+[,.]?)+").unwrap()
    });

    RE_KM_NUM
        .replace_all(text, |caps: &regex::Captures| {
            let matched = caps.get(0).unwrap().as_str();
            matched
                .chars()
                .map(|c| match c {
                    '\u{17e0}' => '0',
                    '\u{17e1}' => '1',
                    '\u{17e2}' => '2',
                    '\u{17e3}' => '3',
                    '\u{17e4}' => '4',
                    '\u{17e5}' => '5',
                    '\u{17e6}' => '6',
                    '\u{17e7}' => '7',
                    '\u{17e8}' => '8',
                    '\u{17e9}' => '9',
                    '.' => ',',
                    ',' => '.',
                    other => other,
                })
                .collect::<String>()
        })
        .to_string()
}

/// Verbalize a number string to Khmer text.
pub fn number_verbalize(input_str: &str) -> String {
    if let Ok(n) = input_str.parse::<i64>() {
        return integer(n, DEFAULT_SEPARATOR);
    }
    if let Ok(n) = input_str.parse::<f64>() {
        return decimal(n, DEFAULT_SEPARATOR);
    }
    input_str.to_string()
}

/// Regex-based number replacer for use in text processing.
pub fn number_replacer(text: &str) -> String {
    static RE_GENERIC_NUMBER: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"\d+\.?\d*").unwrap()
    });

    RE_GENERIC_NUMBER
        .replace_all(text, |caps: &regex::Captures| {
            let matched = caps.get(0).unwrap().as_str();
            number_verbalize(matched)
        })
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_single_digit() {
        assert_eq!(integer(0, "▁"), "សូន្យ");
        assert_eq!(integer(1, "▁"), "មួយ");
        assert_eq!(integer(5, "▁"), "ប្រាំ");
        assert_eq!(integer(9, "▁"), "ប្រាំបួន");
    }

    #[test]
    fn test_integer_tens() {
        assert_eq!(integer(10, "▁"), "ដប់");
        assert_eq!(integer(20, "▁"), "ម្ភៃ");
        assert_eq!(integer(15, "▁"), "ដប់ប្រាំ");
    }

    #[test]
    fn test_integer_hundreds() {
        assert_eq!(integer(100, "▁"), "មួយរយ");
        assert_eq!(integer(200, "▁"), "ពីររយ");
    }

    #[test]
    fn test_km_number_translate() {
        assert_eq!(number_translate2ascii("០១២៣"), "0123");
        assert_eq!(number_translate2ascii("hello"), "hello");
    }

    #[test]
    fn test_number_verbalize() {
        assert_eq!(number_verbalize("5"), "ប្រាំ");
        assert_eq!(number_verbalize("10"), "ដប់");
    }
}
