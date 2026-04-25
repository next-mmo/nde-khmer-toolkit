use wasm_bindgen::prelude::*;
use khmernormalizer::normalize;
use number_verbalize::{number_replacer, number_translate2ascii};

#[wasm_bindgen]
pub fn normalize_khmer_text(text: &str, remove_zwsp: bool, verbalize_numbers: bool) -> String {
    // 1. Normalize the text
    let mut result = normalize(text, remove_zwsp);

    // 2. Optionally verbalize numbers
    if verbalize_numbers {
        let ascii_num_text = number_translate2ascii(&result);
        result = number_replacer(&ascii_num_text);
    }

    result
}

fn format_timestamp(seconds: f64) -> String {
    let hours = (seconds / 3600.0) as u32;
    let mins = ((seconds % 3600.0) / 60.0) as u32;
    let secs = (seconds % 60.0) as u32;
    let millis = (seconds.fract() * 1000.0) as u32;
    format!("{:02}:{:02}:{:02},{:03}", hours, mins, secs, millis)
}

#[wasm_bindgen]
pub fn generate_linear_srt(text: &str, duration_seconds: f64) -> String {
    // This is a naive linear estimation for the web, since true acoustic
    // forced alignment requires ONNX which isn't currently loaded in WASM.
    let words: Vec<&str> = text.split_whitespace().filter(|w| !w.is_empty()).collect();
    if words.is_empty() {
        return String::new();
    }

    let duration_per_word = duration_seconds / (words.len() as f64);
    let mut srt_output = String::new();

    for (i, word) in words.iter().enumerate() {
        let start_time = (i as f64) * duration_per_word;
        let end_time = start_time + duration_per_word;

        srt_output.push_str(&format!("{}\n", i + 1));
        srt_output.push_str(&format!("{} --> {}\n", format_timestamp(start_time), format_timestamp(end_time)));
        srt_output.push_str(&format!("{}\n\n", word));
    }

    srt_output
}
