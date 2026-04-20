use serde_json::{json, Value};

pub mod info;
pub mod paths;
pub mod responses;
pub mod schemas;
pub mod servers;
pub mod swagger_ui;

pub const SWAGGER_UI_HTML: &str = swagger_ui::SWAGGER_UI_HTML;

pub fn document() -> Value {
    json!({
        "openapi": "3.1.0",
        "info": info::info(),
        "servers": servers::servers(),
        "paths": paths::paths(),
        "components": {
            "responses": responses::responses(),
            "schemas": schemas::schemas()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::document;

    #[test]
    fn openapi_includes_tts_endpoint() {
        let doc = document();
        assert!(doc["paths"]["/v1/tts"]["post"].is_object());
        assert_eq!(doc["openapi"], "3.1.0");
    }
}
