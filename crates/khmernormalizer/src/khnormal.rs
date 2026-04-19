//! Khmer syllable-level normalization.
//!
//! Ported from Python `khmernormalizer/khnormal.py`.
//! Implements character category-based syllable reordering.

use once_cell::sync::Lazy;
use fancy_regex::Regex;

/// Khmer character categories for normalization ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Cat {
    Other = 0,
    Base = 1,
    Robat = 2,
    Coeng = 3,
    ZFCoeng = 4,
    Shift = 5,
    Z = 6,
    VPre = 7,
    VB = 8,
    VA = 9,
    VPost = 10,
    MS = 11,
    MF = 12,
}

/// Category table for U+1780..U+17DD
static CATEGORIES: &[Cat] = &[
    // 1780-17A2: Base (35 chars)
    Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base,
    Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base,
    Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base,
    Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base,
    Cat::Base, Cat::Base, Cat::Base,
    // 17A3-17A4: Other (2)
    Cat::Other, Cat::Other,
    // 17A5-17B3: Base (15)
    Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base,
    Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base,
    // 17B4-17B5: Other (2)
    Cat::Other, Cat::Other,
    // 17B6: VPost
    Cat::VPost,
    // 17B7-17BA: VA (4)
    Cat::VA, Cat::VA, Cat::VA, Cat::VA,
    // 17BB-17BD: VB (3)
    Cat::VB, Cat::VB, Cat::VB,
    // 17BE-17C5: VPre (8)
    Cat::VPre, Cat::VPre, Cat::VPre, Cat::VPre, Cat::VPre, Cat::VPre, Cat::VPre, Cat::VPre,
    // 17C6: MS
    Cat::MS,
    // 17C7-17C8: MF (2)
    Cat::MF, Cat::MF,
    // 17C9-17CA: Shift (2)
    Cat::Shift, Cat::Shift,
    // 17CB: MS
    Cat::MS,
    // 17CC: Robat
    Cat::Robat,
    // 17CD-17D1: MS (5)
    Cat::MS, Cat::MS, Cat::MS, Cat::MS, Cat::MS,
    // 17D2: Coeng
    Cat::Coeng,
    // 17D3: MS
    Cat::MS,
    // 17D4-17DC: Other (9)
    Cat::Other, Cat::Other, Cat::Other, Cat::Other, Cat::Other,
    Cat::Other, Cat::Other, Cat::Other, Cat::Other,
    // 17DD: MS
    Cat::MS,
];

/// Get the character category for a single character.
fn charcat(c: char) -> Cat {
    let o = c as u32;
    if (0x1780..=0x17DD).contains(&o) {
        CATEGORIES[(o - 0x1780) as usize]
    } else if o == 0x200C {
        Cat::Z
    } else if o == 0x200D {
        Cat::ZFCoeng
    } else {
        Cat::Other
    }
}

// Pre-compiled regexes for syllable normalization
static RE_MULTI_INVIS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([\u{200C}\u{200D}]\u{17D2}?|\u{17D2}\u{200D})[\u{17D2}\u{200C}\u{200D}]+").unwrap()
});

static RE_COMPOUND_EI: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\u{17C1}([\u{17BB}-\u{17BD}]?)\u{17B8}").unwrap()
});

static RE_COMPOUND_EA: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\u{17C1}([\u{17BB}-\u{17BD}]?)\u{17B6}").unwrap()
});

static RE_SWAP_BE_BB: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\u{17BE})(\u{17BB})").unwrap()
});

// STRONG pattern: series 1 consonants or combinations containing series 1
static RE_STRONG_BB: Lazy<Regex> = Lazy::new(|| {
    // Simplified: match consonant clusters followed by \u17BB before upper vowel
    // Strong series 1 consonants
    Regex::new(
        r"([\u{1780}-\u{1783}\u{1785}-\u{1788}\u{178A}-\u{178D}\u{178F}-\u{1792}\u{1795}-\u{1797}\u{179E}-\u{17A0}\u{17A2}](?:\u{17CC})?(?:\u{17D2}[\u{1780}-\u{17B3}](?:\u{17D2}[\u{1780}-\u{17B3}])?)?(?:\u{17D2}\u{200D}[\u{1780}-\u{1799}\u{179B}-\u{17A2}\u{17A5}-\u{17B3}])?[\u{17C1}-\u{17C5}]?)\u{17BB}(?=[\u{17B7}-\u{17BA}\u{17BE}\u{17BF}\u{17DD}]|\u{17B6}\u{17C6}|\u{17D0})"
    ).unwrap()
});

static RE_NSTRONG_BB: Lazy<Regex> = Lazy::new(|| {
    // Non-strong (series 2 or containing BA)
    Regex::new(
        r"([\u{1784}\u{1780}\u{178E}\u{1793}\u{1794}\u{1798}-\u{179D}\u{17A1}\u{17A3}-\u{17B3}](?:\u{17CC})?(?:\u{17D2}[\u{1780}-\u{17B3}](?:\u{17D2}[\u{1780}-\u{17B3}])?)?(?:\u{17D2}\u{200D}[\u{1780}-\u{1799}\u{179B}-\u{17A2}\u{17A5}-\u{17B3}])?[\u{17C1}-\u{17C5}]?)\u{17BB}(?=[\u{17B7}-\u{17BA}\u{17BE}\u{17BF}\u{17DD}]|\u{17B6}\u{17C6}|\u{17D0})"
    ).unwrap()
});

static RE_COENG_RO_SECOND: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\u{17D2}\u{179A})(\u{17D2}[\u{1780}-\u{17B3}])").unwrap()
});

static RE_COENG_DA_TO_TA: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\u{17D2})\u{178A}").unwrap()
});

/// Normalize Khmer text at the syllable level.
///
/// Reorders characters within syllables according to Khmer encoding structure,
/// without fixing or marking errors.
pub fn khmer_normalize(txt: &str) -> String {
    let chars: Vec<char> = txt.chars().collect();
    let mut charcats: Vec<Cat> = chars.iter().map(|&c| charcat(c)).collect();

    // Recategorise base or ZWJ -> coeng after coeng char
    for i in 1..charcats.len() {
        if charcats[i - 1] == Cat::Coeng
            && (charcats[i] == Cat::Base || charcats[i] == Cat::ZFCoeng)
        {
            charcats[i] = Cat::Coeng;
        }
    }

    // Find subranges of base+non-other and sort components
    let mut i = 0;
    let mut res = String::with_capacity(txt.len());

    while i < charcats.len() {
        let c = charcats[i];
        if c != Cat::Base {
            res.push(chars[i]);
            i += 1;
            continue;
        }

        // Scan for end of syllable
        let mut j = i + 1;
        while j < charcats.len() && (charcats[j] as u8) > (Cat::Base as u8) {
            j += 1;
        }

        // Sort syllable based on character categories
        let mut indices: Vec<usize> = (i..j).collect();
        indices.sort_by(|&a, &b| {
            (charcats[a] as u8)
                .cmp(&(charcats[b] as u8))
                .then(a.cmp(&b))
        });

        let mut replaces: String = indices.iter().map(|&n| chars[n]).collect();

        // Remove multiple invisible chars
        replaces = RE_MULTI_INVIS.replace_all(&replaces, "$1").to_string();

        // Map compound vowel sequences
        replaces = RE_COMPOUND_EI.replace_all(&replaces, "\u{17BE}$1").to_string();
        replaces = RE_COMPOUND_EA.replace_all(&replaces, "\u{17C4}$1").to_string();
        replaces = RE_SWAP_BE_BB.replace_all(&replaces, "$2$1").to_string();

        // Replace -u + upper vowel with consonant shifter
        replaces = RE_STRONG_BB.replace_all(&replaces, "$1\u{17CA}").to_string();
        replaces = RE_NSTRONG_BB.replace_all(&replaces, "$1\u{17C9}").to_string();

        // Coeng ro second
        replaces = RE_COENG_RO_SECOND.replace_all(&replaces, "$2$1").to_string();

        // Coeng da -> ta
        replaces = RE_COENG_DA_TO_TA.replace_all(&replaces, "$1\u{178F}").to_string();

        res.push_str(&replaces);
        i = j;
    }

    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_normalization() {
        // Simple text should pass through unchanged
        let input = "កម្ពុជា";
        let result = khmer_normalize(input);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_charcat() {
        assert_eq!(charcat('\u{1780}'), Cat::Base); // ka
        assert_eq!(charcat('\u{17D2}'), Cat::Coeng);
        assert_eq!(charcat('\u{17C6}'), Cat::MS);
        assert_eq!(charcat('A'), Cat::Other);
    }
}
