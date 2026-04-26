use wasm_bindgen::prelude::*;
use serde::Serialize;
use khmernormalizer::normalize;
use number_verbalize::{number_replacer, number_translate2ascii};

// ── Text normalization ────────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn normalize_khmer(text: &str, remove_zwsp: bool, verbalize_numbers: bool) -> String {
    let mut result = normalize(text, remove_zwsp);
    if verbalize_numbers {
        let ascii = number_translate2ascii(&result);
        result = number_replacer(&ascii);
    }
    result
}

// ── Word-level timestamps ─────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct WordTimestamp {
    pub text: String,
    pub start: f64,
    pub end: f64,
}

/// Linear word-timestamp estimation (equal time per word).
/// This mirrors the logic in `kfa-wasm::generate_linear_srt` but returns a
/// JS array of `{ text, start, end }` objects instead of raw SRT text —
/// directly consumable by the karaoke player without parsing.
///
/// Replace the body of `align_words` below with real CTC inference once an
/// onnxruntime-web glue layer is available (see kfa-core::alignment for the
/// Viterbi backtrack logic that would come after emission extraction).
fn linear_timestamps(text: &str, duration_seconds: f64) -> Vec<WordTimestamp> {
    let words: Vec<&str> = text.split_whitespace().filter(|w| !w.is_empty()).collect();
    if words.is_empty() || duration_seconds <= 0.0 {
        return Vec::new();
    }
    let dur_per_word = duration_seconds / words.len() as f64;
    words
        .iter()
        .enumerate()
        .map(|(i, w)| WordTimestamp {
            text: w.to_string(),
            start: i as f64 * dur_per_word,
            end: (i + 1) as f64 * dur_per_word,
        })
        .collect()
}

/// Returns a JS array of `{ text, start, end }` objects.
/// Currently delegates to linear estimation.
/// Future: call onnxruntime-web via `js_sys` for CTC emission extraction,
/// then run the Viterbi decode (see `kfa-core::alignment::{get_trellis, backtrack, merge_words}`).
#[wasm_bindgen]
pub fn align_words(text: &str, duration_seconds: f64) -> JsValue {
    let ts = linear_timestamps(text, duration_seconds);
    serde_wasm_bindgen::to_value(&ts).unwrap_or(JsValue::NULL)
}

/// Convenience: same as `align_words` but accepts raw PCM (f32 samples) so the
/// signature is ready for the real aligner — the audio data is currently unused.
#[wasm_bindgen]
pub fn align_words_with_audio(
    text: &str,
    duration_seconds: f64,
    _audio_pcm_f32: &[f32],
) -> JsValue {
    align_words(text, duration_seconds)
}

// ── SRT generation (kept for back-compat with existing web/example) ───────────

fn fmt_ts(s: f64) -> String {
    let h = (s / 3600.0) as u32;
    let m = ((s % 3600.0) / 60.0) as u32;
    let sec = (s % 60.0) as u32;
    let ms = (s.fract() * 1000.0) as u32;
    format!("{:02}:{:02}:{:02},{:03}", h, m, sec, ms)
}

#[wasm_bindgen]
pub fn generate_srt(text: &str, duration_seconds: f64) -> String {
    linear_timestamps(text, duration_seconds)
        .iter()
        .enumerate()
        .map(|(i, w)| {
            format!(
                "{}\n{} --> {}\n{}\n\n",
                i + 1,
                fmt_ts(w.start),
                fmt_ts(w.end),
                w.text
            )
        })
        .collect()
}
