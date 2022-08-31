use json_oracle_encoder::messages_to_calldata;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn compile(json: &str, calldata: bool) -> String {
    let json_value: serde_json::Value = serde_json::from_str(json).unwrap();
    messages_to_calldata(json_value).unwrap_or_else(|e| e.to_string())
}
