use crossbeam_channel::{unbounded, Receiver, Sender};
use js_sys::Promise;
use once_cell::sync::Lazy;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use wasm_bindgen::prelude::*;
use wasmyon::run_in_worker;

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

#[wasm_bindgen(typescript_custom_section)]
const _T1: &'static str = r#"
export function sum_in_workers(): Promise<number>;
"#;

#[wasm_bindgen(skip_typescript)]
pub fn sum_in_workers() -> Promise {
    run_in_worker(|| (0..100000 as i32).into_par_iter().sum::<i32>())
}

// Shared Channel
// ----------------------------------------------------------------------------

static CHANNEL: Lazy<(Sender<String>, Receiver<String>)> = Lazy::new(|| unbounded());

#[wasm_bindgen]
pub fn send_to_channel(str: &str) {
    let _ = CHANNEL.0.send(str.into());
}

#[wasm_bindgen(typescript_custom_section)]
const _T1: &'static str = r#"
export function receive_from_channel(): Promise<string>;
"#;

#[wasm_bindgen(skip_typescript)]
pub fn receive_from_channel() -> Promise {
    run_in_worker(|| CHANNEL.1.recv().unwrap())
}
