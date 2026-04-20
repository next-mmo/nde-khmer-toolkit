use serde_json::json;

pub fn schemas() -> serde_json::Value {
    json!({
        "HealthResponse": {
            "type": "object",
            "required": ["status"],
            "properties": {
                "status": { "type": "string", "example": "ok" }
            }
        },
        "TtsRequest": {
            "type": "object",
            "required": ["text"],
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Plain text to synthesize."
                },
                "voice_name": {
                    "type": "string",
                    "default": "km-KH-SreymomNeural",
                    "description": "Microsoft Edge voice short name."
                },
                "output_format": {
                    "type": "string",
                    "default": "audio-24khz-96kbitrate-mono-mp3"
                },
                "rate": {
                    "type": "integer",
                    "format": "int32",
                    "default": 0,
                    "description": "Speaking rate percentage adjustment."
                },
                "pitch": {
                    "type": "integer",
                    "format": "int32",
                    "default": 0,
                    "description": "Pitch percentage adjustment."
                }
            }
        },
        "EdgeVoice": {
            "type": "object",
            "required": ["Name", "ShortName", "Gender", "Locale", "FriendlyName"],
            "properties": {
                "Name": { "type": "string" },
                "ShortName": { "type": "string" },
                "Gender": { "type": "string" },
                "Locale": { "type": "string" },
                "FriendlyName": { "type": "string" }
            }
        },
        "ErrorResponse": {
            "type": "object",
            "required": ["error"],
            "properties": {
                "error": { "type": "string" }
            }
        }
    })
}
