// ============================================================================
// transcribe-audio-to-text
//
// Dual-target crate:
//   • native  → ffmpeg conversion + reqwest blocking HTTP
//   • wasm32  → wasm-bindgen async fn, browser fetch via gloo-net
// ============================================================================

// Native still uses absolute URL
#[cfg(not(target_arch = "wasm32"))]
const API_ENDPOINT: &str = "https://www.google.com/speech-api/v2/recognize";

// WASM uses the Vite proxy to bypass CORS
#[cfg(target_arch = "wasm32")]
const API_ENDPOINT: &str = "/google-speech-api/v2/recognize";

const DEFAULT_KEY: &str = "AIzaSyBOti4mM-6x9WDnZIjIeyEU21OpBXqWBgw";

// ── Shared response parser ────────────────────────────────────────────────────

/// Parse the Google Speech API v2 multi-JSON response and return the first
/// recognised transcript, or an error if nothing was recognised.
pub fn parse_google_speech_response(response_text: &str) -> Result<String, String> {
    // Google Speech API v2 returns multiple JSON objects separated by newlines:
    //   {"result":[]}
    //   {"result":[{"alternative":[{"transcript":"hello","confidence":0.9}],"final":true}],"result_index":0}
    for line in response_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(results) = v.get("result").and_then(|r| r.as_array()) {
                if !results.is_empty() {
                    if let Some(alternatives) =
                        results[0].get("alternative").and_then(|a| a.as_array())
                    {
                        if !alternatives.is_empty() {
                            if let Some(transcript) =
                                alternatives[0].get("transcript").and_then(|t| t.as_str())
                            {
                                return Ok(transcript.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    Err("Speech recognition could not understand the audio".to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// NATIVE target
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(not(target_arch = "wasm32"))]
pub mod native {
    use super::{parse_google_speech_response, API_ENDPOINT, DEFAULT_KEY};
    use anyhow::{anyhow, Context, Result};
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    /// Convert any audio format → 16 kHz mono FLAC using `ffmpeg`.
    pub fn convert_to_flac(input_audio_file: &Path) -> Result<tempfile::NamedTempFile> {
        let output_flac = tempfile::Builder::new()
            .suffix(".flac")
            .tempfile()
            .context("Failed to create temp FLAC file")?;

        let status = Command::new("ffmpeg")
            .arg("-y")
            .arg("-i")
            .arg(input_audio_file.as_os_str())
            .arg("-ac")
            .arg("1")
            .arg("-ar")
            .arg("16000")
            .arg("-f")
            .arg("flac")
            .arg(output_flac.path().as_os_str())
            .output()
            .context("Failed to execute ffmpeg — is it installed?")?;

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr);
            return Err(anyhow!("ffmpeg conversion failed: {}", stderr));
        }
        Ok(output_flac)
    }

    /// Send FLAC bytes to Google Speech API v2 and return the transcript.
    pub fn transcribe_flac(flac_data: Vec<u8>, language: &str) -> Result<String> {
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
        let text = response.text()?;
        parse_google_speech_response(&text).map_err(|e| anyhow!("{}", e))
    }

    /// High-level helper: convert audio file → FLAC → transcribe in Khmer.
    pub fn transcribe_audio_to_text(input_audio_file: &Path) -> Result<String> {
        println!("Converting to .flac with 16000 Hz sample rate…");
        let temp_flac = convert_to_flac(input_audio_file)?;
        println!("Starting transcription…");
        let flac_data = fs::read(temp_flac.path()).context("Failed to read temp FLAC")?;
        transcribe_flac(flac_data, "km-KH")
    }
}

// Re-export the native public API at crate root for the CLI binary.
#[cfg(not(target_arch = "wasm32"))]
pub use native::transcribe_audio_to_text;

// ─────────────────────────────────────────────────────────────────────────────
// WASM target
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::{parse_google_speech_response, API_ENDPOINT, DEFAULT_KEY};
    use gloo_net::http::Request;
    use wasm_bindgen::prelude::*;

    /// Transcribe raw audio bytes (any format the Google Speech API accepts,
    /// e.g. FLAC at 16 kHz) and return the recognised text.
    ///
    /// Called from JavaScript as:
    /// ```js
    /// const text = await transcribe_audio(uint8Array, "km-KH");
    /// ```
    #[wasm_bindgen]
    pub async fn transcribe_audio(
        audio_bytes: &[u8],
        language: &str,
        content_type: &str,
    ) -> Result<String, JsValue> {
        let url = format!(
            "{}?client=chromium&lang={}&key={}",
            API_ENDPOINT, language, DEFAULT_KEY
        );

        // gloo-net body() requires Into<JsValue>; copy the bytes into a Uint8Array.
        let typed_array = js_sys::Uint8Array::from(audio_bytes);

        let response = Request::post(&url)
            .header("Content-Type", content_type)
            .body(typed_array)
            .map_err(|e| JsValue::from_str(&e.to_string()))?
            .send()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        if !response.ok() {
            return Err(JsValue::from_str(&format!(
                "Google Speech API error: {}",
                response.status()
            )));
        }

        let text = response
            .text()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        parse_google_speech_response(&text).map_err(|e| JsValue::from_str(&e))
    }
}

// Re-export WASM public API.
#[cfg(target_arch = "wasm32")]
pub use wasm::transcribe_audio;
