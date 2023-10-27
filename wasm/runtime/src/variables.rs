use wasm_bindgen::prelude::*;

use wfrs_model::json::JsonValue;

pub struct JsRuntimeVariables<'a>(pub &'a JsonValue);

impl<'a> From<JsRuntimeVariables<'a>> for JsValue {
    fn from(val: JsRuntimeVariables<'a>) -> Self {
        match val.0 {
            JsonValue::Null => JsValue::null(),
            JsonValue::Bool(v) => js_sys::Boolean::from(*v).into(),
            JsonValue::Number(v) => js_sys::Number::from(v.as_f64()).into(),
            JsonValue::String(v) => js_sys::JsString::from(v.as_str()).into(),
            JsonValue::Array(v) => {
                let result = js_sys::Array::new_with_length(v.len() as u32);
                for i in 0..v.len() {
                    result.set(i as u32, JsRuntimeVariables(&v[i]).into());
                }
                result.into()
            }
            JsonValue::Object(v) => {
                let object = js_sys::Object::new();
                for (k, v) in v {
                    js_sys::Reflect::set(&object, &k.into(), &JsRuntimeVariables(v).into()).ok();
                }
                object.into()
            }
        }
    }
}
