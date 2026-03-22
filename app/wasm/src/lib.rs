// FEAT-APP-001

use syu_core::{AppPayload, build_browser_workspace};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn build_browser_workspace_from_js(payload: JsValue) -> Result<JsValue, JsValue> {
    let payload: AppPayload = serde_wasm_bindgen::from_value(payload)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    serde_wasm_bindgen::to_value(&build_browser_workspace(payload))
        .map_err(|error| JsValue::from_str(&error.to_string()))
}
