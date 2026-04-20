mod openapi;

use axum::body::Body;
use axum::extract::Json;
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::Router;
use edge_tts_rs::edge_api::{EdgeTTS, EdgeTTSConfig, TTS};
use serde::{Deserialize, Serialize};

const DEFAULT_VOICE: &str = "km-KH-SreymomNeural";
const DEFAULT_OUTPUT_FORMAT: &str = "audio-24khz-96kbitrate-mono-mp3";
const EDGE_VOICES_URL: &str = "https://speech.platform.bing.com/consumer/speech/synthesize/readaloud/voices/list?trustedclienttoken=6A5AA1D4EAFF4E9FB37E23D68491D6F4";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::env::var("KFA_API_ADDR").unwrap_or_else(|_| "127.0.0.1:8787".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let local_addr = listener.local_addr()?;

    println!("KFA API listening on http://{local_addr}");
    println!("Swagger UI available at http://{local_addr}/swagger-ui");

    axum::serve(listener, app()).await?;
    Ok(())
}

fn app() -> Router {
    Router::new()
        .route("/", get(|| async { Redirect::temporary("/swagger-ui") }))
        .route("/health", get(health))
        .route("/openapi.json", get(openapi_json))
        .route("/api-docs/openapi.json", get(openapi_json))
        .route("/swagger-ui", get(swagger_ui))
        .route("/swagger-ui/", get(swagger_ui))
        .route("/v1/tts", post(synthesize_speech))
        .route("/v1/voices", get(list_voices))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

async fn openapi_json() -> Json<serde_json::Value> {
    Json(openapi::document())
}

async fn swagger_ui() -> Html<&'static str> {
    Html(openapi::SWAGGER_UI_HTML)
}

async fn list_voices() -> Result<Json<Vec<EdgeVoice>>, ApiError> {
    let voices = reqwest::get(EDGE_VOICES_URL)
        .await
        .map_err(ApiError::bad_gateway)?
        .error_for_status()
        .map_err(ApiError::bad_gateway)?
        .json::<Vec<EdgeVoice>>()
        .await
        .map_err(ApiError::bad_gateway)?;

    Ok(Json(voices))
}

async fn synthesize_speech(Json(request): Json<TtsRequest>) -> Result<Response, ApiError> {
    let request = request.validate()?;
    let audio = synthesize_with_edge(request).await?;

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "audio/mpeg")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"speech.mp3\"",
        )
        .body(Body::from(audio))
        .map_err(ApiError::internal)
}

async fn synthesize_with_edge(request: ValidatedTtsRequest) -> Result<Vec<u8>, ApiError> {
    let task = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let mut config = EdgeTTSConfig::default();
        config.voice_name = request.voice_name;
        config.output_format = request.output_format;
        config.rate = request.rate;
        config.pitch = request.pitch;

        let tts = EdgeTTS::new(config);
        let mut client = tts.connect().map_err(|error| error.to_string())?;

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| error.to_string())?;

        runtime
            .block_on(tts.send_content(&mut client, request.text))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(ApiError::internal)?;

    task.map_err(ApiError::bad_gateway)
}

#[derive(Debug, Deserialize)]
struct TtsRequest {
    text: String,
    voice_name: Option<String>,
    output_format: Option<String>,
    rate: Option<i16>,
    pitch: Option<i16>,
}

impl TtsRequest {
    fn validate(self) -> Result<ValidatedTtsRequest, ApiError> {
        let text = self.text.trim();
        if text.is_empty() {
            return Err(ApiError::bad_request("text must not be empty"));
        }

        Ok(ValidatedTtsRequest {
            text: escape_ssml_text(text),
            voice_name: non_empty_or_default(self.voice_name, DEFAULT_VOICE),
            output_format: non_empty_or_default(self.output_format, DEFAULT_OUTPUT_FORMAT),
            rate: self.rate.unwrap_or(0),
            pitch: self.pitch.unwrap_or(0),
        })
    }
}

struct ValidatedTtsRequest {
    text: String,
    voice_name: String,
    output_format: String,
    rate: i16,
    pitch: i16,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct EdgeVoice {
    name: String,
    short_name: String,
    gender: String,
    locale: String,
    friendly_name: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl ToString) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.to_string(),
        }
    }

    fn bad_gateway(error: impl ToString) -> Self {
        Self {
            status: StatusCode::BAD_GATEWAY,
            message: format!("Edge TTS upstream failed: {}", error.to_string()),
        }
    }

    fn internal(error: impl ToString) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: error.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}

fn non_empty_or_default(value: Option<String>, default: &str) -> String {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn escape_ssml_text(input: &str) -> String {
    let mut output = String::with_capacity(input.len());

    for ch in input.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            _ => output.push(ch),
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::{escape_ssml_text, non_empty_or_default, DEFAULT_VOICE};

    #[test]
    fn escapes_text_before_wrapping_in_ssml() {
        assert_eq!(escape_ssml_text("A < B & B > C"), "A &lt; B &amp; B &gt; C");
    }

    #[test]
    fn blank_voice_uses_default() {
        assert_eq!(
            non_empty_or_default(Some("  ".to_string()), DEFAULT_VOICE),
            DEFAULT_VOICE
        );
    }
}
