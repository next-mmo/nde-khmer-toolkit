//! Forced alignment over audio using a wav2vec2 CTC ONNX model.
//!
//! Ported from Python `kfa/forced_alignment.py`.

use anyhow::{anyhow, Result};
use ndarray::{concatenate, Array2, Axis};
use ort::session::builder::GraphOptimizationLevel;
use ort::session::{Session, SessionInputs};
use ort::value::{Tensor, Value};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::text_normalize::{PhonemizedToken, TextPipeline};
use crate::utils::{
    backtrack, get_trellis, intersperse, log_softmax_last_axis, merge_repeats, merge_words,
    time_to_frame,
};
use crate::vocab::{BLANK_ID, SEPARATOR_ID};

const EMISSION_INTERVAL_SECS: f64 = 30.0;
const CONTEXT_RATIO: f64 = 0.1;
pub const SAMPLE_RATE: u32 = 16_000;

/// A single aligned segment covering one (possibly multi-word) span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alignment {
    pub text: String,
    pub start: f64,
    pub end: f64,
    pub actual_start: f64,
    pub actual_end: f64,
    pub score: f64,
}

/// Helper: stringify any `ort::Error` into an `anyhow::Error`.
///
/// `ort::Error<R>` embeds the failed builder/session pointer which isn't
/// `Send + Sync`, so `anyhow::Error::from` doesn't apply. We drop that state
/// and just keep the display message.
fn ort_err<T: std::fmt::Display>(e: T) -> anyhow::Error {
    anyhow!("{e}")
}

/// Create an ONNX Runtime session for a wav2vec2 CTC model.
pub fn create_session(model_path: impl AsRef<Path>) -> Result<Session> {
    let session = Session::builder()
        .map_err(ort_err)?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(ort_err)?
        .commit_from_file(model_path.as_ref())
        .map_err(ort_err)?;
    Ok(session)
}

/// Run forced alignment. Returns one `Alignment` per word-like segment.
pub fn align(
    samples: &[f32],
    sample_rate: u32,
    text: &str,
    pipeline: &TextPipeline,
    session: &mut Session,
    progress: Option<&dyn Fn(u64, u64)>,
) -> Result<Vec<Alignment>> {
    let total_duration = samples.len() as f64 / sample_rate as f64;
    let total_chunks =
        (total_duration / EMISSION_INTERVAL_SECS).ceil().max(1.0) as u64;

    let mut emissions_arr: Vec<Array2<f32>> = Vec::new();
    let mut i = 0.0_f64;
    let mut processed = 0_u64;

    while i < total_duration {
        let segment_start_time = i;
        let segment_end_time = i + EMISSION_INTERVAL_SECS;
        let context = EMISSION_INTERVAL_SECS * CONTEXT_RATIO;
        let input_start_time = (segment_start_time - context).max(0.0);
        let input_end_time = (segment_end_time + context).min(total_duration);

        let start_sample = (sample_rate as f64 * input_start_time) as usize;
        let end_sample = (sample_rate as f64 * input_end_time) as usize;
        let y_chunk: Vec<f32> = samples[start_sample..end_sample].to_vec();
        let chunk_len = y_chunk.len();

        // Shape [1, chunk_len]
        let shape = [1_i64, chunk_len as i64];
        let tensor: Tensor<f32> = Tensor::from_array((shape, y_chunk)).map_err(ort_err)?;
        let input_value: Value = tensor.into_dyn();
        let inputs: SessionInputs<1> =
            SessionInputs::ValueMap(vec![("input".into(), input_value.into())]);
        let outputs = session.run(inputs).map_err(ort_err)?;

        let emissions_view = outputs[0]
            .try_extract_array::<f32>()
            .map_err(ort_err)?;
        let shape = emissions_view.shape().to_vec();
        if shape.len() != 3 || shape[0] != 1 {
            return Err(anyhow!("unexpected emissions shape: {:?}", shape));
        }
        let frames = shape[1];
        let vocab = shape[2];
        let mut emissions = Array2::<f32>::zeros((frames, vocab));
        for (dst, src) in emissions.iter_mut().zip(emissions_view.iter().copied()) {
            *dst = src;
        }

        let emission_start_frame = time_to_frame(segment_start_time);
        let emission_end_frame = time_to_frame(segment_end_time);
        let offset = time_to_frame(input_start_time);

        let slice_start = emission_start_frame.saturating_sub(offset);
        let slice_end = (emission_end_frame.saturating_sub(offset)).min(frames);
        if slice_end > slice_start {
            let sliced = emissions
                .slice(ndarray::s![slice_start..slice_end, ..])
                .to_owned();
            emissions_arr.push(sliced);
        }

        i += EMISSION_INTERVAL_SECS;
        processed += 1;
        if let Some(cb) = progress {
            cb(processed, total_chunks);
        }
    }

    if emissions_arr.is_empty() {
        return Err(anyhow!("no emissions produced from audio"));
    }

    let views: Vec<_> = emissions_arr.iter().map(|a| a.view()).collect();
    let emissions: Array2<f32> = concatenate(Axis(0), &views)?;

    // Convert to f64 and apply log-softmax along last axis
    let (frames, vocab) = emissions.dim();
    let mut emission = Array2::<f64>::zeros((frames, vocab));
    for ((r, c), v) in emissions.indexed_iter() {
        emission[[r, c]] = *v as f64;
    }
    log_softmax_last_axis(&mut emission);

    // Tokenize and phonemize text, per line
    let mut text_sequences: Vec<PhonemizedToken> = Vec::new();
    for line in text.split('\n') {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        let segs = pipeline.tokenize_phonemize(l)?;
        text_sequences.extend(segs);
    }

    // Walk the phonemized items, collapsing unknowns into spans attached to the preceding known item.
    let mut tokens: Vec<Vec<usize>> = Vec::new();
    let mut texts: Vec<String> = Vec::new();
    let mut spans: Vec<usize> = Vec::new();

    for item in &text_sequences {
        match item {
            PhonemizedToken::Unknown { .. } => {
                if let Some(last) = spans.last_mut() {
                    *last += 1;
                }
            }
            PhonemizedToken::Known {
                lattice,
                token_ids,
                ..
            } => {
                spans.push(0);
                tokens.push(token_ids.clone());
                texts.push(lattice.clone());
            }
        }
    }

    if tokens.is_empty() {
        return Ok(Vec::new());
    }

    // Join into final token sequence with `|` separators
    let joined_text: String = intersperse(&texts, "|".to_string()).join("");
    let joined_tokens: Vec<usize> = {
        let with_sep = intersperse(&tokens, vec![SEPARATOR_ID]);
        with_sep.into_iter().flatten().collect()
    };

    let trellis = get_trellis(emission.view(), &joined_tokens, BLANK_ID);
    let path = backtrack(&trellis, emission.view(), &joined_tokens, BLANK_ID);
    let transcript_chars: Vec<char> = joined_text.chars().collect();
    let segments = merge_repeats(&path, &transcript_chars);
    let word_segments = merge_words(&segments, "|");

    let ratio = samples.len() as f64 / trellis.shape()[0] as f64;
    let mut second_start = 0.0_f64;
    let mut results: Vec<Alignment> = Vec::with_capacity(word_segments.len());

    for (i, word) in word_segments.iter().enumerate() {
        let actual_second_start = ratio * word.start as f64 / sample_rate as f64;
        let mut second_end = ratio * word.end as f64 / sample_rate as f64;
        let actual_second_end = second_end;
        if i + 1 < word_segments.len() {
            let next_start = ratio * word_segments[i + 1].start as f64 / sample_rate as f64;
            if next_start > second_end {
                second_end = next_start;
            }
        }

        let seq_idx: usize = spans.iter().take(i).sum::<usize>() + i;
        let span_size = spans.get(i).copied().unwrap_or(0);
        let mut text_segment = String::new();
        let end = (seq_idx + span_size + 1).min(text_sequences.len());
        for t in &text_sequences[seq_idx..end] {
            match t {
                PhonemizedToken::Known { token, .. } => text_segment.push_str(token),
                PhonemizedToken::Unknown { token } => text_segment.push_str(token),
            }
        }

        results.push(Alignment {
            text: text_segment,
            start: second_start,
            end: second_end,
            actual_start: actual_second_start,
            actual_end: actual_second_end,
            score: word.score,
        });
        second_start = second_end;
    }

    Ok(results)
}
