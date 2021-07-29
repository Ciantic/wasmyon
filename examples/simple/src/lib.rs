use crossbeam_channel::{unbounded, Receiver, Sender};
use once_cell::sync::Lazy;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasmyon::wasmyon_promise;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn logv(x: &JsValue);
}

// If for some reason wasm-pack doesn't create `imports.wbg` in the JS file, it
// fails in that case. I found it it's enough to make a dummy method that forces
// the generation

// #[wasm_bindgen]
// pub fn __dummy() {
//     // failed?
//     log(&"Foo");
// }

// Rayon
// ----------------------------------------------------------------------------

#[wasmyon_promise]
pub fn sum_in_workers() -> i32 {
    (0..100000 as i32).into_par_iter().sum::<i32>()
}

// Shared Channel
// ----------------------------------------------------------------------------

static CHANNEL: Lazy<(Sender<String>, Receiver<String>)> = Lazy::new(|| unbounded());

#[wasm_bindgen]
pub fn send_to_channel(str: &str) {
    let _ = CHANNEL.0.send(str.into());
}

#[wasmyon_promise]
pub fn receive_from_channel() -> String {
    CHANNEL.1.recv().unwrap()
}

// TypeScript with full type
// -----------------------------------------------------------------------------

// unfortunately this is only way to get proper type for your exported function:

#[wasm_bindgen(typescript_custom_section)]
const _TS: &'static str = r#"
export function sum_in_workers_with_ts(): Promise<number>;
"#;

#[wasmyon_promise(skip_typescript)]
pub fn sum_in_workers_with_ts() -> i32 {
    (0..100000 as i32).into_par_iter().sum::<i32>()
}

// Wasm_bindgen struct
// ----------------------------------------------------------------------------

#[wasm_bindgen]
pub struct ExampleObject {
    pub value: i32,
}

#[wasmyon_promise(serde)]
pub fn example_with_object() -> ExampleObject {
    ExampleObject {
        value: (0..100000 as i32).into_par_iter().sum::<i32>(),
    }
}

// Wasm_bindgen with serde
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct ExampleAnonymous {
    pub some_numbers: Vec<i32>,
    pub some_string: String,
}

// #[wasmyon_promise]
// pub fn example_with_serde() -> ExampleAnonymous {
//     ExampleAnonymous {
//         some_string: "Okay".to_owned(),
//         some_numbers: vec![(0..100000 as i32).into_par_iter().sum::<i32>()],
//     }
// }
