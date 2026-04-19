//! CTC vocabulary table for the wav2vec2 Khmer model.
//!
//! Ported from Python `kfa/utils.py` / `kfa/text_normalize.py`.

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Vocabulary: character → token id.
pub static VOCABS: Lazy<HashMap<&'static str, usize>> = Lazy::new(|| {
    let entries: &[(&str, usize)] = &[
        (".", 0),
        ("a", 1),
        ("c", 2),
        ("e", 3),
        ("f", 4),
        ("g", 5),
        ("h", 6),
        ("i", 7),
        ("j", 8),
        ("k", 9),
        ("l", 10),
        ("m", 11),
        ("n", 12),
        ("o", 13),
        ("p", 14),
        ("r", 15),
        ("s", 16),
        ("t", 17),
        ("u", 18),
        ("w", 19),
        ("z", 20),
        ("\u{014b}", 21),
        ("\u{0251}", 22),
        ("\u{0253}", 23),
        ("\u{0254}", 24),
        ("\u{0257}", 25),
        ("\u{0259}", 26),
        ("\u{025b}", 27),
        ("\u{0268}", 28),
        ("\u{0272}", 29),
        ("\u{0294}", 30),
        ("|", 31),
        ("[UNK]", 32),
        ("[PAD]", 33),
    ];
    entries.iter().copied().collect()
});

pub const BLANK_ID: usize = 33; // [PAD]
pub const SEPARATOR_ID: usize = 31; // |

/// Look up a single character's token id. Falls back to `[UNK]`.
pub fn lookup(ch: &str) -> usize {
    *VOCABS.get(ch).unwrap_or(&32)
}
