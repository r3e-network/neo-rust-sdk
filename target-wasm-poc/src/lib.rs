use wasm_bindgen::prelude::*;
use std::panic;

#[wasm_bindgen(start)]
pub fn init() {
    // Set up panic hook for better error messages in browser console
    console_error_panic_hook::set_once();
    
    web_sys::console::log_1(&"Neo3 WASM POC initialized".into());
}

#[wasm_bindgen]
pub struct WalletPoc {
    address: String,
}

impl WalletPoc {
    pub fn new() -> Self {
        Self {
            address: "WASM_ADDRESS_PLACEHOLDER".to_string(),
        }
    }
    
    #[wasm_bindgen]
    pub fn get_address(&self) -> String {
        self.address.clone()
    }
    
    #[wasm_bindgen]
    pub async fn test_http_request(&self) -> Result<String, JsValue> {
        // Test HTTP request with reqwest wasm backend
        match reqwest::get("https://api.n3rpc.org").await {
            Ok(response) => {
                let status = response.status();
                Ok(format!("HTTP status: {}", status))
            }
            Err(e) => Err(JsValue::from_str(&format!("Request failed: {}", e))),
        }
    }
    
    #[wasm_bindgen]
    pub fn test_crypto(&self) -> String {
        // Test basic crypto operations
        format!("Crypto library loaded successfully")
    }
}

impl Default for WalletPoc {
    fn default() -> Self {
        Self::new()
    }
}
