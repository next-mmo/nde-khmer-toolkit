//! Character replacement mappings for Khmer text normalization.
//!
//! Ported from Python `khmernormalizer/mappings.py`.

use once_cell::sync::Lazy;
use fancy_regex::{Regex, Captures};
use std::collections::HashMap;

/// Build the character replacement table (accented Latin → ASCII).
pub fn char_replacement(c: char) -> Option<&'static str> {
    CHAR_REPLACEMENTS.get(&c).copied()
}

static CHAR_REPLACEMENTS: Lazy<HashMap<char, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    // Superscript digits
    m.insert('¹', "1"); m.insert('²', "2"); m.insert('³', "3");
    // A variants
    for c in "ÀÁÂÃÄÅĀĂĄǍǞǡǺȀȂȦḀẠẢẤẦẨẪẬẮẰẲẴẶ".chars() { m.insert(c, "A"); }
    for c in "àáâãäåªāăąǎǟǡǻȁȃȧḁạảấầẩẫậắằẳẵặ".chars() { m.insert(c, "a"); }
    // B variants
    for c in "ḂḄḆ".chars() { m.insert(c, "B"); }
    for c in "ḃḅḇ".chars() { m.insert(c, "b"); }
    // C variants
    for c in "ÇĆĈĊČḈ".chars() { m.insert(c, "C"); }
    for c in "çćĉċčḉ".chars() { m.insert(c, "c"); }
    // D variants
    for c in "ÐĎĐḊḌḎḐḒ".chars() { m.insert(c, "D"); }
    for c in "ďđḋḍḏḑḓ".chars() { m.insert(c, "d"); }
    // E variants
    for c in "ÈÉÊËĒĔĖĘĚȄȆȨḔḖḘḚḜẸẺẼẾỀỂỄỆ".chars() { m.insert(c, "E"); }
    for c in "èéêëēĕėęěȅȇȩḕḗḙḛḝẹẻẽếềểễệ".chars() { m.insert(c, "e"); }
    // F variants
    m.insert('Ḟ', "F"); m.insert('ḟ', "f");
    // G variants
    for c in "ĜĞĠĢǦǴḠ".chars() { m.insert(c, "G"); }
    for c in "ĝğġģǧǵḡ".chars() { m.insert(c, "g"); }
    // H variants
    for c in "ĤĦȞḢḤḦḨḪ".chars() { m.insert(c, "H"); }
    for c in "ĥħȟḣḥḧḩḫẖ".chars() { m.insert(c, "h"); }
    // I variants
    for c in "ÌÍÎÏĨĪĬĮİǏȈȊḬḮỈỊ".chars() { m.insert(c, "I"); }
    for c in "ìíîïĩīĭįıǐȉȋḭḯỉị".chars() { m.insert(c, "i"); }
    // J variants
    m.insert('Ĵ', "J"); m.insert('ĵ', "j");
    // K variants
    for c in "ĶǨḰḲḴ".chars() { m.insert(c, "K"); }
    for c in "ķǩḱḳḵ".chars() { m.insert(c, "k"); }
    // L variants
    for c in "ĹĻĽĿŁḶḸḺḼ".chars() { m.insert(c, "L"); }
    for c in "ĺļľŀłḷḹḻḽ".chars() { m.insert(c, "l"); }
    // M variants
    for c in "ḾṀṂ".chars() { m.insert(c, "M"); }
    for c in "ḿṁṃ".chars() { m.insert(c, "m"); }
    // N variants
    for c in "ÑŃŅŇǸṄṆṈṊ".chars() { m.insert(c, "N"); }
    for c in "ñńņňǹṅṇṉṋ".chars() { m.insert(c, "n"); }
    // O variants
    for c in "ÒÓÔÕÖŌŎŐƠǑǪǬȌȎȪȬȮȰṌṎṐṒỌỎỐỒỔỖỘỚỜỞỠỢ".chars() { m.insert(c, "O"); }
    for c in "òóôõöōŏőơǒǫǭȍȏȫȭȯȱṍṏṑṓọỏốồổỗộớờởỡợ".chars() { m.insert(c, "o"); }
    // P variants
    for c in "ṔṖ".chars() { m.insert(c, "P"); }
    for c in "ṕṗ".chars() { m.insert(c, "p"); }
    // R variants
    for c in "ŔŖŘȐȒṘṚṜṞ".chars() { m.insert(c, "R"); }
    for c in "ŕŗřȑȓṙṛṝṟ".chars() { m.insert(c, "r"); }
    // S variants
    for c in "ŚŜŞȘṠṢṤṦṨ".chars() { m.insert(c, "S"); }
    m.insert('Š', "s"); // matches Python original (lowercase)
    for c in "śŝşšșṡṣṥṧṩ".chars() { m.insert(c, "s"); }
    // T variants
    for c in "ŢŤŦȚṪṬṮṰ".chars() { m.insert(c, "T"); }
    for c in "ţťŧțṫṭṯṱẗ".chars() { m.insert(c, "t"); }
    // U variants
    for c in "ÙÚÛÜŨŪŬŮŰŲƯǓǕǗǙǛȔȖṲṴṶṸṺỤỦỨỪỬỮỰ".chars() { m.insert(c, "U"); }
    for c in "ùúûüũūŭůűųưǔǖǘǚǜȕȗṳṵṷṹṻụủứừửữự".chars() { m.insert(c, "u"); }
    // V variants
    for c in "ṼṾ".chars() { m.insert(c, "V"); }
    for c in "ṽṿ".chars() { m.insert(c, "v"); }
    // W variants
    for c in "ŴẀẂẄẆẈ".chars() { m.insert(c, "W"); }
    for c in "ŵẁẃẅẇẉẘ".chars() { m.insert(c, "w"); }
    // X variants
    for c in "ẊẌ".chars() { m.insert(c, "X"); }
    for c in "ẋẍ".chars() { m.insert(c, "x"); }
    // Y variants
    m.insert('Ý', "y"); // matches Python original
    for c in "ŶŸȲẎỲỴỶỸ".chars() { m.insert(c, "Y"); }
    for c in "ýÿŷȳẏỳỵỷỹẙ".chars() { m.insert(c, "y"); }
    // Z variants
    for c in "ŹŻŽẐẒẔ".chars() { m.insert(c, "Z"); }
    for c in "źżžẑẓẕ".chars() { m.insert(c, "z"); }
    // Special
    m.insert('Ĳ', "IJ"); m.insert('ĳ', "ij");
    m.insert('ø', "o"); m.insert('Ø', "O");
    m.insert('ɨ', "i"); m.insert('ð', "d");
    m
});

/// Unicode replacement pairs (multi-char aware).
pub static UNICODE_REPLACEMENTS: Lazy<Vec<(&'static str, &'static str)>> = Lazy::new(|| {
    vec![
        ("\u{00ad}", ""),
        ("\u{00a0}", " "),
        ("\u{200b}", ""),
        ("\u{2060}", ""),
        ("\u{201e}", "\""),
        ("\u{201c}", "\""),
        ("\u{201d}", "\""),
        ("\u{2013}", "-"),
        ("\u{2014}", " - "),
        ("\u{00b4}", "'"),
        ("\u{2018}", "'"),
        ("\u{201a}", "'"),
        ("\u{2019}", "'"),
        ("\u{00b4}\u{00b4}", "\""),
        ("\u{00a0}\u{00ab}\u{00a0}", "\""),
        ("\u{00ab}\u{00a0}", "\""),
        ("\u{00ab}", "\""),
        ("\u{00a0}\u{00bb}\u{00a0}", "\""),
        ("\u{00a0}\u{00bb}", "\""),
        ("\u{00bb}", "\""),
        ("\u{09f7}", "\u{0964}"),
        ("\u{ff0c}", ","),
        ("\u{3001}", ","),
        ("\u{2236}", ":"),
        ("\u{ff1a}", ":"),
        ("\u{ff1f}", "?"),
        ("\u{300a}", "\""),
        ("\u{300b}", "\""),
        ("\u{ff09}", ")"),
        ("\u{ff01}", "!"),
        ("\u{ff08}", "("),
        ("\u{ff1b}", ";"),
        ("\u{300d}", "\""),
        ("\u{300c}", "\""),
        ("\u{ff10}", "0"),
        ("\u{ff11}", "1"),
        ("\u{ff12}", "2"),
        ("\u{ff13}", "3"),
        ("\u{ff14}", "4"),
        ("\u{ff15}", "5"),
        ("\u{ff16}", "6"),
        ("\u{ff17}", "7"),
        ("\u{ff18}", "8"),
        ("\u{ff19}", "9"),
        ("\u{ff5e}", "~"),
        ("\u{2501}", "-"),
        ("\u{3008}", "<"),
        ("\u{3009}", ">"),
        ("\u{3010}", "["),
        ("\u{3011}", "]"),
        ("\u{ff05}", "%"),
        // Khmer chars
        ("។ល។", "\u{17d8}"),
        ("ឨញ្ញា", "ឧក"),
        ("ឣ", "អ"),
        ("ឤ", "អា"),
        ("ឲ", "ឱ"),
        ("\u{17DD}", "\u{17D1}"),
        ("\u{17D3}", "\u{17C6}"),
        ("\u{17C1}\u{17B8}", "\u{17BE}"),
        ("\u{17B8}\u{17C1}", "\u{17BE}"),
        ("\u{17C1}\u{17B6}", "\u{17C4}"),
        ("\u{17BB}\u{17D0}", "\u{17C9}\u{17D0}"),
        ("\u{17C9}\u{17C6}", "\u{17BB}\u{17C6}"),
        ("\u{17C6}\u{17BB}", "\u{17BB}\u{17C6}"),
        ("\u{17C7}\u{17B7}", "\u{17B7}\u{17C7}"),
        ("\u{17C7}\u{17B9}", "\u{17B9}\u{17C7}"),
        ("\u{17C7}\u{17BA}", "\u{17BA}\u{17C7}"),
        ("\u{17C7}\u{17C2}", "\u{17C2}\u{17C7}"),
        ("\u{17C6}\u{17B6}", "\u{17B6}\u{17C6}"),
        ("\u{17C7}\u{17BB}", "\u{17BB}\u{17C7}"),
        ("\u{17C7}\u{17C1}", "\u{17C1}\u{17C7}"),
        ("\u{17C7}\u{17C4}", "\u{17C4}\u{17C7}"),
        ("\u{17C6}\u{17BB}", "\u{17BB}\u{17C6}"),
        // Common misspelled words
        ("រយះពេល", "រយៈពេល"),
        ("រយ:", "រយៈ"),
        ("រយះកាល", "រយៈកាល"),
        ("រយ:កាល", "រយៈកាល"),
        ("មួយរយះ", "មួយរយៈ"),
        ("មួយរយ:", "មួយរយៈ"),
    ]
});

/// Compiled regex for unicode replacements. Sorted by key length descending
/// so longer patterns match first.
pub static UNICODE_REPLACEMENTS_REGEX: Lazy<Regex> = Lazy::new(|| {
    let mut keys: Vec<&str> = UNICODE_REPLACEMENTS.iter().map(|(k, _)| *k).collect();
    keys.sort_by(|a, b| b.len().cmp(&a.len()));
    let pattern = keys.iter().map(|k| fancy_regex::escape(k)).collect::<Vec<_>>().join("|");
    Regex::new(&pattern).unwrap()
});

/// Lookup for unicode replacements.
pub static UNICODE_REPLACEMENTS_MAP: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    UNICODE_REPLACEMENTS.iter().copied().collect()
});

pub static DOUBLE_QUOTE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        "[\u{00ab}\u{2039}\u{00bb}\u{203a}\u{201e}\u{201c}\u{201f}\u{201d}\u{275d}\u{275e}\u{276e}\u{276f}\u{301d}\u{301e}\u{301f}\u{ff02}]"
    )
    .unwrap()
});

pub static SINGLE_QUOTE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        "[\u{2018}\u{201b}\u{2019}\u{275b}\u{275c}\u{0060}\u{00b4}\u{2032}\u{2035}]"
    )
    .unwrap()
});

pub static WHITESPACES_HANDLER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[^\S\r\n]+").unwrap()
});

pub static MULTIPLE_PUNCT_REGEX: Lazy<fancy_regex::Regex> = Lazy::new(|| {
    fancy_regex::Regex::new(r"([៙៚៖!?។៕\u{17d8}])\1+").unwrap()
});

pub static DUPLICATE_COENG_RE: Lazy<fancy_regex::Regex> = Lazy::new(|| {
    fancy_regex::Regex::new(r"(\u{17d2}[\u{1780}-\u{17b3}])\1+").unwrap()
});

pub static TRAILING_VOWEL_RE: Lazy<fancy_regex::Regex> = Lazy::new(|| {
    fancy_regex::Regex::new(r"([\u{17b6}-\u{17dd}])\1+").unwrap()
});

pub static ELLIPSIS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\.{3,}").unwrap()
});

pub static SPACE_BEFORE_PUNCT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[ ]+([៙៚៖!?។៕\u{17d8}])").unwrap()
});

pub static SPACE_BEFORE_REPEAT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[^\S\r\n]+ៗ").unwrap()
});

pub static ZWSP_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[\u{200a}\u{200b}\u{200c}\u{200d}\u{2028}\u{feff}]").unwrap()
});
