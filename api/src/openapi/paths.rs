use serde_json::json;

pub fn paths() -> serde_json::Value {
    json!({
        "/health": {
            "get": {
                "operationId": "health",
                "summary": "Health check",
                "responses": {
                    "200": {
                        "description": "The API process is running.",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/HealthResponse" }
                            }
                        }
                    }
                }
            }
        },
        "/v1/tts": {
            "post": {
                "operationId": "synthesizeSpeech",
                "summary": "Synthesize speech",
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/TtsRequest" },
                            "examples": {
                                "khmer": {
                                    "summary": "Khmer speech",
                                    "value": {
                                        "text": "Hello from KFA",
                                        "voice_name": "km-KH-SreymomNeural",
                                        "rate": 0,
                                        "pitch": 0
                                    }
                                }
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "MP3 audio bytes.",
                        "content": {
                            "audio/mpeg": {
                                "schema": {
                                    "type": "string",
                                    "format": "binary"
                                }
                            }
                        }
                    },
                    "400": { "$ref": "#/components/responses/BadRequest" },
                    "502": { "$ref": "#/components/responses/UpstreamFailure" }
                }
            }
        },
        "/v1/voices": {
            "get": {
                "operationId": "listVoices",
                "summary": "List Edge TTS voices",
                "responses": {
                    "200": {
                        "description": "Available Edge TTS voices.",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "array",
                                    "items": { "$ref": "#/components/schemas/EdgeVoice" }
                                }
                            }
                        }
                    },
                    "502": { "$ref": "#/components/responses/UpstreamFailure" }
                }
            }
        }
    })
}
