use crate::alignment::*;
use crate::g2p::G2pModel;
use crate::text_normalize::tokenize_phonemize;
use crate::vocabs;
use anyhow::{Context, Result};
use ndarray::{Array2, ArrayView2};
use ort::{inputs, session::Session, value::TensorRef, ep::CUDA};
use std::path::{Path, PathBuf};

const EMISSION_INTERVAL: f64 = 30.0;
const MODEL_URL: &str = "https://huggingface.co/seanghay/wav2vec2-base-khmer-phonetisaurus/resolve/main/wav2vec2-km-base-1500.onnx";

fn get_model_path() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("./.cache"));
    let kfa_dir = cache_dir.join("kfa");
    std::fs::create_dir_all(&kfa_dir)?;
    
    let model_path = kfa_dir.join("wav2vec2-km-base-1500.onnx");
    let tmp_path = kfa_dir.join("wav2vec2-km-base-1500.onnx.tmp");

    if !model_path.exists() {
        println!("Downloading ONNX model...");
        let response = reqwest::blocking::get(MODEL_URL)?;
        let mut dest = std::fs::File::create(&tmp_path)?;
        let content = response.bytes()?;
        std::io::copy(&mut content.as_ref(), &mut dest)?;
        std::fs::rename(&tmp_path, &model_path)?;
    }

    Ok(model_path)
}

pub struct AlignmentSession {
    session: Session,
    g2p: G2pModel,
}

impl AlignmentSession {
    pub fn new(use_cuda: bool) -> Result<Self> {
        let _ = ort::init()
            .with_name("KFA")
            .commit(); // Ignore Error if already initialized

        let model_path = get_model_path()?;
        let mut builder = Session::builder()?;
        
        if use_cuda {
            builder = builder
                .with_execution_providers([CUDA::default().build()])
                .map_err(|e| anyhow::anyhow!("Failed to add CUDA execution provider: {:?}", e))?;
        }

        let session = builder.commit_from_file(model_path)?;
        let g2p = G2pModel::new()?;

        Ok(Self { session, g2p })
    }

    pub fn align(
        &mut self,
        audio: &[f32],
        sr: usize,
        text: &str,
        silent: bool,
    ) -> Result<Vec<(String, f64, f64, f64, f64, f32)>> {
        let total_duration = audio.len() as f64 / sr as f64;
        let mut i = 0.0;
        let mut emissions_arr = Vec::new();

        let pb = if !silent {
            let total_steps = (total_duration / EMISSION_INTERVAL).ceil() as u64;
            Some(indicatif::ProgressBar::new(total_steps))
        } else {
            None
        };

        while i < total_duration {
            let segment_start_time = i;
            let segment_end_time = i + EMISSION_INTERVAL;
            let context = EMISSION_INTERVAL * 0.1;

            let input_start_time = (segment_start_time - context).max(0.0);
            let input_end_time = (segment_end_time + context).min(total_duration);

            let chunk_start_idx = (input_start_time * sr as f64) as usize;
            let chunk_end_idx = (input_end_time * sr as f64) as usize;
            let chunk = &audio[chunk_start_idx..chunk_end_idx];

            let input = TensorRef::from_array_view(([1usize, chunk.len()], chunk))?;
            let outputs = self.session.run(inputs![input])?;
            let (shape, data) = outputs[0].try_extract_tensor::<f32>()?;
            
            // shape will be [1, T, classes]. We want a 2D array [T, classes]
            let t_len = shape[1] as usize;
            let classes_len = shape[2] as usize;
            
            let emission_2d = ArrayView2::from_shape((t_len, classes_len), data)?.to_owned();

            let emission_start_frame = vocabs::time_to_frame(segment_start_time);
            let emission_end_frame = vocabs::time_to_frame(segment_end_time);
            let offset = vocabs::time_to_frame(input_start_time);

            let start_idx = emission_start_frame - offset;
            let mut end_idx = emission_end_frame - offset;
            
            if end_idx > emission_2d.shape()[0] {
                end_idx = emission_2d.shape()[0];
            }

            let sliced = emission_2d.slice(ndarray::s![start_idx..end_idx, ..]).to_owned();
            emissions_arr.push(sliced);

            i += EMISSION_INTERVAL;
            if let Some(ref p) = pb {
                p.inc(1);
            }
        }

        if let Some(p) = pb {
            p.finish();
        }

        let mut all_emissions: Vec<f32> = Vec::new();
        for e in &emissions_arr {
            all_emissions.extend(e.iter().copied());
        }
        
        let frames_count: usize = emissions_arr.iter().map(|a: &Array2<f32>| a.shape()[0]).sum();
        let classes_count = emissions_arr[0].shape()[1];
        let total_emissions = Array2::from_shape_vec((frames_count, classes_count), all_emissions)?;

        let emission = log_softmax(&total_emissions);

        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        let mut text_sequences = Vec::new();
        for line in lines {
            let mut tokens = tokenize_phonemize(line.trim(), &self.g2p);
            text_sequences.append(&mut tokens);
        }

        let mut tokens_flat = Vec::new();
        let mut texts_flat = Vec::new();
        let mut original_tokens = Vec::new();
        let mut spans = Vec::new();

        for seq in &text_sequences {
            if seq.1.is_empty() {
                if let Some(last) = spans.last_mut() {
                    *last += 1;
                }
                continue;
            }
            spans.push(0);
            original_tokens.push(seq.0.clone());
            texts_flat.push(seq.1.clone());
            tokens_flat.push(seq.2.clone());
        }

        let blank_id = vocabs::get_vocab_id("[PAD]").unwrap();
        
        let text_concat = vocabs::intersperse(&texts_flat, "|".to_string()).join("");
        
        let mut iter_tokens = Vec::new();
        let pipe_id = vocabs::get_vocab_id("|").unwrap();
        let interspersed_tokens = vocabs::intersperse(&tokens_flat, vec![pipe_id]);
        
        for t_list in interspersed_tokens {
            iter_tokens.extend(t_list);
        }

        let trellis = get_trellis(&emission, &iter_tokens, blank_id);
        let path = backtrack(&trellis, &emission, &iter_tokens, blank_id);
        let segments = merge_repeats(&path, &text_concat);
        let word_segments = merge_words(&segments, "|");

        let mut results = Vec::new();
        let mut second_start = 0.0;

        let ratio = (audio.len() as f64) / (trellis.shape()[0] as f64);

        for (i, word) in word_segments.iter().enumerate() {
            let actual_second_start = ratio * word.start as f64 / sr as f64;
            let mut second_end = ratio * word.end as f64 / sr as f64;
            let actual_second_end = second_end;

            if i < word_segments.len() - 1 {
                second_end = second_end.max(ratio * word_segments[i + 1].start as f64 / sr as f64);
            }

            let seq_idx = spans[0..i].iter().sum::<usize>() + i;
            let span_size = spans[i];

            let mut text_segment = String::new();
            for item in &text_sequences[seq_idx..=seq_idx + span_size] {
                text_segment.push_str(&item.0);
            }

            results.push((
                text_segment,
                second_start,
                second_end,
                actual_second_start,
                actual_second_end,
                word.score,
            ));

            second_start = second_end;
        }

        Ok(results)
    }
}
