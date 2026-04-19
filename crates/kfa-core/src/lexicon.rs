use once_cell::sync::Lazy;
use std::collections::HashMap;

static LEXICON_DATA: &str = include_str!("../../../data/lexicon.json");

pub static LEXICON: Lazy<HashMap<String, Vec<String>>> = Lazy::new(|| {
    serde_json::from_str(LEXICON_DATA).expect("Failed to parse lexicon.json")
});
