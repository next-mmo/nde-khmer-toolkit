use serde_json::json;

pub fn servers() -> serde_json::Value {
    json!([
        { "url": "/" }
    ])
}
