#![cfg(target_arch = "wasm32")]
extern crate wasm_bindgen_test;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn pass() -> Result<(), String> {
    let diagram = std::include_str!("./diagram.bpmn");
    parser::parse(diagram).map_err(|err| {
        format!("{err}")
    })?;
    Ok(())
}
