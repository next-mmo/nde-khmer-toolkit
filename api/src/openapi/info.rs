use serde_json::json;

pub fn info() -> serde_json::Value {
    json!({
        "title": "KFA Edge TTS API",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "HTTP API for Microsoft Edge text-to-speech synthesis."
    })
}
