use crate::g2p::G2pModel;
use crate::lexicon::LEXICON;
use crate::vocabs;
use khmernormalizer::normalize;
use crfs::Model;
use once_cell::sync::Lazy;
use regex::Regex;

static CRF_MODEL_FILE: &[u8] = include_bytes!("../../../data/crf_ner_10000.crfsuite");

pub static CRF_MODEL: Lazy<Model> = Lazy::new(|| {
    Model::new(CRF_MODEL_FILE).expect("Failed to load CRF model")
});

pub fn phonemize(text: &str, g2p: &G2pModel) -> Option<Vec<String>> {
    let lower_text = text.to_lowercase();
    let ascii_num_text = number_verbalize::number_translate2ascii(&lower_text);
    let verbalized_text = number_verbalize::number_replacer(&ascii_num_text);

    if verbalized_text.contains('▁') {
        let mut result = Vec::new();
        for subtoken in verbalized_text.split('▁') {
            if let Some(mut phonemes) = phonemize(subtoken, g2p) {
                result.append(&mut phonemes);
                result.push(".".to_string());
            }
        }
        return Some(result);
    }

    // Keep only Khmer chars and basic latin
    let re_clean = Regex::new(r"[^\u{1780}-\u{17d2}a-z]+").unwrap();
    let cleaned = re_clean.replace_all(&verbalized_text, "").to_string();

    if cleaned.trim().is_empty() {
        return None;
    }

    if let Some(lex) = LEXICON.get(&cleaned) {
        return Some(lex.clone());
    }

    g2p.phoneticize(&cleaned).ok()
}

pub fn tokenize_phonemize(text: &str, g2p: &G2pModel) -> Vec<(String, String, Vec<usize>)> {
    let normalized = normalize(text, true);
    let mut results = Vec::new();

    let tokens = khmercut::tokenize(&CRF_MODEL, &normalized);
    for token in tokens {
        if let Some(phonemic) = phonemize(&token, g2p) {
            let lattices_str = phonemic.join("");
            let re_dots = Regex::new(r"\.+").unwrap();
            let cleaned_lattices = re_dots.replace_all(&lattices_str, ".").to_string();

            let mut token_ids = Vec::new();
            // The python code does: token_ids = [vocabs[lat] for lat in lattices]
            // where `lattices` is a string (characters or graphemes?), wait, in python:
            // phonemic is list of strings (phonemes), joined into `lattices` which is a string.
            // Wait, looking closely at python `token_ids = [vocabs[lat] for lat in lattices]`
            // Here `lattices` is a string, and it's iterating char by char? No, some vocabs are multi-char!
            // Wait! If `lattices` is just a single string, iterating over it in python iterates over chars.
            // But some vocabulary keys are "[UNK]", "[PAD]". However those never appear out of phonemicizer.
            // Let's iterate over `cleaned_lattices` by char.
            for ch in cleaned_lattices.chars() {
                if let Some(id) = vocabs::get_vocab_id(&ch.to_string()) {
                    token_ids.push(id);
                }
            }

            results.push((token.clone(), cleaned_lattices, token_ids));
        } else {
            // (token, None) equivalent in python, just skip or add empty
            // In python it yields (token, phonemic) which has len=2
        }
    }
    results
}
