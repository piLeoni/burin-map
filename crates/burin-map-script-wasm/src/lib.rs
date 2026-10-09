//! The interpreter for the page, loaded only when a script has to run: a
//! document script in, the atlas it builds out, as JSON for the engine.

use wasm_bindgen::prelude::*;

/// Run `source`; `{ "atlas": …, "area": … }` as JSON (same shape as the Node
/// binding), or an error carrying `line` and `column` when the position is known.
#[wasm_bindgen]
pub fn evaluate(source: &str) -> Result<String, JsValue> {
    match burin_map_script::evaluate_document(source) {
        Ok(doc) => {
            let out = serde_json::json!({
                "atlas": doc.atlas,
                "area": doc.area,
            });
            Ok(out.to_string())
        }
        Err(e) => {
            let error = js_sys::Error::new(&e.to_string());
            if let Some(at) = e.at {
                let _ = js_sys::Reflect::set(&error, &"line".into(), &at.0.into());
                let _ = js_sys::Reflect::set(&error, &"column".into(), &at.1.into());
            }
            Err(error.into())
        }
    }
}

/// The library's source, so the editor can offer it beside `burin.d.ts`.
#[wasm_bindgen(js_name = librarySource)]
pub fn library_source() -> String {
    include_str!("../../burin-map-script/src/library.js").to_string()
}
