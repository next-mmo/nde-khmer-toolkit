use serde_json::json;

pub fn responses() -> serde_json::Value {
    json!({
        "BadRequest": {
            "description": "The request body is invalid.",
            "content": {
                "application/json": {
                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                }
            }
        },
        "UpstreamFailure": {
            "description": "The Edge TTS upstream request failed.",
            "content": {
                "application/json": {
                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                }
            }
        }
    })
}
