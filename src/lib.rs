mod pool;
mod utils;

use crossbeam_channel::{unbounded, Receiver, Sender};
use js_sys::Promise;
use once_cell::sync::Lazy;
use pool::run_in_worker;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub fn __dummy() {
    // TODO: Without this export, the wasm-pack didn't create imports.wbg and
    // failed?
    log(&"Foo");
}

// Rayon
// ----------------------------------------------------------------------------

#[wasm_bindgen]
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

#[wasm_bindgen]
pub fn receive_from_channel() -> Promise {
    run_in_worker(|| {
        let value = CHANNEL.1.recv().unwrap();
        value
    })
}
