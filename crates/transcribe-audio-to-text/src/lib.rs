use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

const API_ENDPOINT: &str = "https://www.google.com/speech-api/v2/recognize";
const DEFAULT_KEY: &str = "AIzaSyBOti4mM-6x9WDnZIjIeyEU21OpBXqWBgw";

/// Convert input audio to 16kHz mono FLAC using ffmpeg
pub fn convert_to_flac(input_audio_file: &Path) -> Result<tempfile::NamedTempFile> {
    let output_flac = tempfile::Builder::new()
        .suffix(".flac")
        .tempfile()
        .context("Failed to create temp flac file")?;

    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(input_audio_file.as_os_str())
        .arg("-ac")
        .arg("1")
        .arg("-ar")
        .arg("16000")
        // Required so it formats it correctly into the temp file path
        .arg("-f")
        .arg("flac")
        .arg(output_flac.path().as_os_str())
        .output()
        .context("Failed to execute ffmpeg. Make sure ffmpeg is installed.")?;

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr);
        return Err(anyhow!("ffmpeg conversion failed: {}", stderr));
    }

    Ok(output_flac)
}

fn parse_google_speech_response(response_text: &str) -> Result<String> {
    // Google speech api returns multiple JSON objects separated by newlines.
    // Example:
    // {"result":[]}
    // {"result":[{"alternative":[{"transcript":"hello testing","confidence":0.9}],"final":true}],"result_index":0}
    for line in response_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(results) = v.get("result").and_then(|r| r.as_array()) {
                if !results.is_empty() {
                    let first_result = &results[0];
                    if let Some(alternatives) = first_result.get("alternative").and_then(|a| a.as_array()) {
                        if !alternatives.is_empty() {
                            let best_alt = &alternatives[0];
                            if let Some(transcript) = best_alt.get("transcript").and_then(|t| t.as_str()) {
                                return Ok(transcript.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    
    Err(anyhow!("Speech recognition could not understand audio"))
}

pub fn transcribe_audio(flac_path: &Path, language: &str) -> Result<String> {
    let flac_data = fs::read(flac_path).context("Failed to read FLAC audio data")?;

    let url = format!(
        "{}?client=chromium&lang={}&key={}",
        API_ENDPOINT, language, DEFAULT_KEY
    );

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(url)
        .header("Content-Type", "audio/x-flac; rate=16000")
        .body(flac_data)
        .send()
        .context("Failed to send request to Google Speech API")?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Google Speech API returned an error: {}",
            response.status()
        ));
    }

    let response_text = response.text()?;
    parse_google_speech_response(&response_text)
}

pub fn transcribe_audio_to_text(input_audio_file: &Path) -> Result<String> {
    println!("Converting to .flac with 16000 Hz sample rate...");
    let temp_flac = convert_to_flac(input_audio_file)?;
    
    println!("Starting transcription process...");
    // Language set to km-KH per the user specification.
    transcribe_audio(temp_flac.path(), "km-KH")
}
