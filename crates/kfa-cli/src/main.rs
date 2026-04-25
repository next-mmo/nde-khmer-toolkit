use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use kfa_core::{AlignmentSession, G2pModel}; // Ensure G2pModel is used if needed outside
use serde::Serialize;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "Khmer Forced Aligner (KFA) written in Rust")]
struct Args {
    /// Audio input file (16kHz mono WAV)
    #[arg(short, long)]
    audio: PathBuf,

    /// Text transcript input file
    #[arg(short, long)]
    text: PathBuf,

    /// Output file
    #[arg(short, long)]
    output: PathBuf,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Jsonl)]
    format: OutputFormat,

    /// Target device for inference
    #[arg(short, long, value_enum, default_value_t = Device::Cpu)]
    device: Device,

    /// Suppress progress bar
    #[arg(short, long, default_value_t = false)]
    quiet: bool,
}

#[derive(ValueEnum, Clone, Debug)]
enum OutputFormat {
    Jsonl,
    Whisper,
    Srt,
}

#[derive(ValueEnum, Clone, Debug)]
enum Device {
    Cpu,
    Cuda,
}

#[derive(Serialize)]
struct JsonlLine {
    text: String,
    start: f64,
    end: f64,
    actual_start: f64,
    actual_end: f64,
    score: f32,
}

#[derive(Serialize)]
struct WhisperOutput {
    text: String,
    segments: Vec<WhisperSegment>,
    language: String,
}

#[derive(Serialize)]
struct WhisperSegment {
    id: usize,
    text: String,
    start: f64,
    end: f64,
}

fn read_text_file(path: &PathBuf) -> Result<String> {
    std::fs::read_to_string(path).map_err(|e| anyhow::anyhow!("Failed to read text file: {:?}", e))
}

fn write_jsonl(
    output_path: &PathBuf,
    results: &[(String, f64, f64, f64, f64, f32)],
) -> Result<()> {
    let mut dest = File::create(output_path)?;
    for r in results {
        let line = JsonlLine {
            text: r.0.clone(),
            start: r.1,
            end: r.2,
            actual_start: r.3,
            actual_end: r.4,
            score: r.5,
        };
        let js = serde_json::to_string(&line)?;
        writeln!(dest, "{}", js)?;
    }
    Ok(())
}

fn write_whisper(
    output_path: &PathBuf,
    results: &[(String, f64, f64, f64, f64, f32)],
) -> Result<()> {
    let mut dest = File::create(output_path)?;
    
    let mut segments = Vec::new();
    let mut full_text = String::new();

    for (i, r) in results.iter().enumerate() {
        let trimmed_text = r.0.trim();
        if trimmed_text.is_empty() {
            continue;
        }
        full_text.push_str(trimmed_text);
        full_text.push(' ');
        
        segments.push(WhisperSegment {
            id: i,
            text: trimmed_text.to_string(),
            start: r.1,
            end: r.2,
        });
    }

    let out = WhisperOutput {
        text: full_text.trim().to_string(),
        segments,
        language: "km".to_string(),
    };

    let js = serde_json::to_string(&out)?;
    writeln!(dest, "{}", js)?;

    Ok(())
}

fn format_timestamp(seconds: f64) -> String {
    let hours = (seconds / 3600.0) as u32;
    let mins = ((seconds % 3600.0) / 60.0) as u32;
    let secs = (seconds % 60.0) as u32;
    let millis = (seconds.fract() * 1000.0) as u32;
    format!("{:02}:{:02}:{:02},{:03}", hours, mins, secs, millis)
}

fn write_srt(
    output_path: &PathBuf,
    results: &[(String, f64, f64, f64, f64, f32)],
) -> Result<()> {
    let mut dest = File::create(output_path)?;
    let mut srt_index = 1;
    
    for r in results {
        let trimmed = r.0.trim();
        if trimmed.is_empty() {
            continue;
        }
        let start_time = format_timestamp(r.1);
        let end_time = format_timestamp(r.2);
        
        writeln!(dest, "{}", srt_index)?;
        writeln!(dest, "{} --> {}", start_time, end_time)?;
        writeln!(dest, "{}", trimmed)?;
        writeln!(dest)?;
        
        srt_index += 1;
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("Loading audio...");
    let mut reader = hound::WavReader::open(&args.audio).context("Failed to open audio file")?;
    let spec = reader.spec();
    
    if spec.sample_rate != 16000 {
        eprintln!("Warning: Expected 16kHz audio, got {}Hz. Results may be poor.", spec.sample_rate);
    }
    if spec.channels != 1 {
        eprintln!("Warning: Expected mono audio, got {} channels.", spec.channels);
    }

    let samples: Vec<f32> = if spec.sample_format == hound::SampleFormat::Int {
        reader.samples::<i16>().map(|s: std::result::Result<i16, hound::Error>| s.unwrap_or(0) as f32 / 32768.0).collect()
    } else {
        reader.samples::<f32>().map(|s: std::result::Result<f32, hound::Error>| s.unwrap_or(0.0)).collect()
    };

    println!("Loading text...");
    let text = read_text_file(&args.text)?;

    println!("Initializing model sessions (this may take a moment)...");
    let use_cuda = matches!(args.device, Device::Cuda);
    let mut session = AlignmentSession::new(use_cuda).context("Failed to initialize ONNX/G2P session")?;

    println!("Aligning...");
    let results = session.align(&samples, spec.sample_rate as usize, &text, args.quiet)?;

    println!("Writing results to {:?}...", args.output);
    match args.format {
        OutputFormat::Jsonl => write_jsonl(&args.output, &results)?,
        OutputFormat::Whisper => write_whisper(&args.output, &results)?,
        OutputFormat::Srt => write_srt(&args.output, &results)?,
    }

    println!("Done!");
    Ok(())
}
