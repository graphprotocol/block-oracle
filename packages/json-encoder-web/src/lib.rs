use json_oracle_encoder::messages_to_calldata;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn compile(json: &str, calldata: bool) -> Result<Vec<u8>, String> {
    let json_value: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let calldata = messages_to_calldata(json_value).map_err(|e| e.to_string())?;
    Ok(calldata)
}
