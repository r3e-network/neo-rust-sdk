use wasm_bindgen::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    
    web_sys::console::log_1(&"Neo3 WASM Demo - Check browser console".into());
}
