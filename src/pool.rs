// This file is near identical copy from wasm-bindgen:
//
// https://github.com/rustwasm/wasm-bindgen/blob/master/examples/raytrace-parallel/src/pool.rs
//
// Modifications include:
// - Expose WorkerPool as single function init_thread_workers
// - Store WorkerPool and ThreadPool in the OnceCell
// - run_in_worker helper that allows to run in worker and return JS Promise

// Silences warnings from the compiler about Work.func and child_entry_point
// being unused when the target is not wasm.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

//! A small module that's intended to provide an example of creating a pool of
//! web workers which can be used to execute `rayon`-style work.

use futures_channel::oneshot;
use js_sys::Array;
use js_sys::Reflect;
use once_cell::sync::OnceCell;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::RwLock;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::BlobPropertyBag;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};
use web_sys::{ErrorEvent, Event, Worker, WorkerOptions};

static WORKER_POOL: OnceCell<WorkerPool> = OnceCell::new();
static THREAD_POOL: OnceCell<rayon::ThreadPool> = OnceCell::new();

#[wasm_bindgen]
pub fn _init_thread_workers(wasm_jsmodule_url: &str, initial: usize, num_of_threads: usize) {
    let worker_pool = WorkerPool::new(initial, wasm_jsmodule_url).unwrap();
    let _ = WORKER_POOL.set(worker_pool);

    // logv(&js_sys::eval(&"document.currentScript").unwrap());
    // logv(&js_sys::global().unchecked_into::<JsValue>());
    // logv(&url);

    let thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_of_threads)
        .spawn_handler(|thread| Ok(WORKER_POOL.get().unwrap().run(|| thread.run()).unwrap()))
        .build()
        .unwrap();

    THREAD_POOL.set(thread_pool).unwrap();
}

pub fn run_in_worker<R, F1>(f: F1) -> js_sys::Promise
where
    F1: (FnOnce() -> R) + Send + 'static,
    R: Into<JsValue> + Send + 'static,
{
    let (tx, rx) = oneshot::channel();
    WORKER_POOL
        .get()
        .unwrap()
        .run(move || {
            THREAD_POOL.get().unwrap().install(|| {
                let _ = tx.send(f());
            })
        })
        .unwrap();
    let done = async move {
        match rx.await {
            Ok(data) => Ok(data.into()),
            Err(_) => Err(JsValue::undefined()),
        }
    };
    wasm_bindgen_futures::future_to_promise(done)
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn logv(x: &JsValue);
}

macro_rules! console_log {
    ($($t:tt)*) => (crate::log(&format_args!($($t)*).to_string()))
}

struct WorkerPool {
    wasm_jsmodule_url: String,
    state: Arc<PoolState>,
}

struct PoolState {
    workers: RwLock<Vec<Worker>>,
    callback: Closure<dyn FnMut(Event)>,
}

// TODO: I don't know what I'm doing, but this probably required because
// callback is not Send+Sync
unsafe impl Send for PoolState {}
unsafe impl Sync for PoolState {}

struct Work {
    func: Box<dyn FnOnce() + Send + 'static>,
}

// #[wasm_bindgen]
impl WorkerPool {
    /// Creates a new `WorkerPool` which immediately creates `initial` workers.
    ///
    /// The pool created here can be used over a long period of time, and it
    /// will be initially primed with `initial` workers. Currently workers are
    /// never released or gc'd until the whole pool is destroyed.
    ///
    /// # Errors
    ///
    /// Returns any error that may happen while a JS web worker is created and a
    /// message is sent to it.
    // #[wasm_bindgen(constructor)]
    pub fn new(initial: usize, wasm_jsmodule_url: &str) -> Result<WorkerPool, JsValue> {
        let pool = WorkerPool {
            state: Arc::new(PoolState {
                workers: RwLock::new(Vec::with_capacity(initial)),
                callback: Closure::wrap(Box::new(|event: Event| {
                    console_log!("unhandled event: {}", event.type_());
                    logv(&event);
                }) as Box<dyn FnMut(Event)>),
            }),
            wasm_jsmodule_url: wasm_jsmodule_url.into(),
        };
        for _ in 0..initial {
            let worker = pool.spawn()?;
            pool.state.push(worker);
        }

        Ok(pool)
    }

    /// Unconditionally spawns a new worker
    ///
    /// The worker isn't registered with this `WorkerPool` but is capable of
    /// executing work for this wasm module.
    ///
    /// # Errors
    ///
    /// Returns any error that may happen while a JS web worker is created and a
    /// message is sent to it.
    fn spawn(&self) -> Result<Worker, JsValue> {
        console_log!("spawning new worker");

        let mut opts = BlobPropertyBag::new();
        opts.type_("text/javascript");

        let arr = Array::new();
        arr.set(
            0,
            format!(
                r#"
                import {{ _init_worker, _child_entry_point }} from "{}"; 

                // Wait for the main thread to send us the shared module/memory. Once we've got
                // it, initialize it all with the `init`.
                //
                // After our first message all subsequent messages are an entry point to run, so
                // we just do that.
                self.onmessage = (event) => {{
                    const [module, memory] = event.data;
                    const initialised = _init_worker(module, memory).catch((err) => {{
                        // Propagate to main `onerror`:
                        setTimeout(() => {{
                            throw err;
                        }});
                
                        // Rethrow to keep promise rejected and prevent execution of further commands:
                        throw err;
                    }});
                
                    self.onmessage = async (event) => {{
                        // This will queue further commands up until the module is fully initialised:
                        await initialised;
                        _child_entry_point(event.data);
                    }};
                }};"#,
                self.wasm_jsmodule_url
            ).into(),
        );
        let blob = web_sys::Blob::new_with_str_sequence_and_options(&arr, &opts).unwrap();
        let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();

        // Create worker (module)
        let opts = WorkerOptions::new();
        js_sys::Reflect::set(&opts, &"type".into(), &"module".into()).unwrap();
        let worker = Worker::new_with_options(&url, &opts)?;

        // With a worker spun up send it the module/memory so it can start
        // instantiating the wasm module. Later it might receive further
        // messages about code to run on the wasm module.
        let array = js_sys::Array::new();
        array.push(&wasm_bindgen::module());
        array.push(&wasm_bindgen::memory());
        worker.post_message(&array)?;

        Ok(worker)
    }

    /// Fetches a worker from this pool, spawning one if necessary.
    ///
    /// This will attempt to pull an already-spawned web worker from our cache
    /// if one is available, otherwise it will spawn a new worker and return the
    /// newly spawned worker.
    ///
    /// # Errors
    ///
    /// Returns any error that may happen while a JS web worker is created and a
    /// message is sent to it.
    fn worker(&self) -> Result<Worker, JsValue> {
        match self.state.workers.write().unwrap().pop() {
            Some(worker) => Ok(worker),
            None => self.spawn(),
        }
    }

    /// Executes the work `f` in a web worker, spawning a web worker if
    /// necessary.
    ///
    /// This will acquire a web worker and then send the closure `f` to the
    /// worker to execute. The worker won't be usable for anything else while
    /// `f` is executing, and no callbacks are registered for when the worker
    /// finishes.
    ///
    /// # Errors
    ///
    /// Returns any error that may happen while a JS web worker is created and a
    /// message is sent to it.
    fn execute(&self, f: impl FnOnce() + Send + 'static) -> Result<Worker, JsValue> {
        let worker = self.worker()?;
        let work = Box::new(Work { func: Box::new(f) });
        let ptr = Box::into_raw(work);
        match worker.post_message(&JsValue::from(ptr as u32)) {
            Ok(()) => Ok(worker),
            Err(e) => {
                unsafe {
                    drop(Box::from_raw(ptr));
                }
                Err(e)
            }
        }
    }

    /// Configures an `onmessage` callback for the `worker` specified for the
    /// web worker to be reclaimed and re-inserted into this pool when a message
    /// is received.
    ///
    /// Currently this `WorkerPool` abstraction is intended to execute one-off
    /// style work where the work itself doesn't send any notifications and
    /// whatn it's done the worker is ready to execute more work. This method is
    /// used for all spawned workers to ensure that when the work is finished
    /// the worker is reclaimed back into this pool.
    fn reclaim_on_message(&self, worker: Worker) {
        let state = Arc::downgrade(&self.state);
        let worker2 = worker.clone();
        let reclaim_slot = Rc::new(RefCell::new(None));
        let slot2 = reclaim_slot.clone();
        let reclaim = Closure::wrap(Box::new(move |event: Event| {
            if let Some(error) = event.dyn_ref::<ErrorEvent>() {
                console_log!("error in worker: {}", error.message());
                // TODO: this probably leaks memory somehow? It's sort of
                // unclear what to do about errors in workers right now.
                return;
            }

            // If this is a completion event then can deallocate our own
            // callback by clearing out `slot2` which contains our own closure.
            if let Some(_msg) = event.dyn_ref::<MessageEvent>() {
                if let Some(state) = state.upgrade() {
                    state.push(worker2.clone());
                }
                *slot2.borrow_mut() = None;
                return;
            }

            console_log!("unhandled event: {}", event.type_());
            logv(&event);
            // TODO: like above, maybe a memory leak here?
        }) as Box<dyn FnMut(Event)>);
        worker.set_onmessage(Some(reclaim.as_ref().unchecked_ref()));
        *reclaim_slot.borrow_mut() = Some(reclaim);
    }
}

impl WorkerPool {
    /// Executes `f` in a web worker.
    ///
    /// This pool manages a set of web workers to draw from, and `f` will be
    /// spawned quickly into one if the worker is idle. If no idle workers are
    /// available then a new web worker will be spawned.
    ///
    /// Once `f` returns the worker assigned to `f` is automatically reclaimed
    /// by this `WorkerPool`. This method provides no method of learning when
    /// `f` completes, and for that you'll need to use `run_notify`.
    ///
    /// # Errors
    ///
    /// If an error happens while spawning a web worker or sending a message to
    /// a web worker, that error is returned.
    pub fn run(&self, f: impl FnOnce() + Send + 'static) -> Result<(), JsValue> {
        let worker = self.execute(f)?;
        self.reclaim_on_message(worker);
        Ok(())
    }
}

impl PoolState {
    fn push(&self, worker: Worker) {
        worker.set_onmessage(Some(self.callback.as_ref().unchecked_ref()));
        worker.set_onerror(Some(self.callback.as_ref().unchecked_ref()));
        let workers = self.workers.read().unwrap();
        for prev in workers.iter() {
            let prev: &JsValue = prev;
            let worker: &JsValue = &worker;
            assert!(prev != worker);
        }
        drop(workers);
        self.workers.write().unwrap().push(worker);
    }
}

/// Entry point invoked by `worker.js`, a bit of a hack but see the "TODO" above
/// about `worker.js` in general.
#[wasm_bindgen]
pub fn _child_entry_point(ptr: u32) -> Result<(), JsValue> {
    let ptr = unsafe { Box::from_raw(ptr as *mut Work) };
    let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
    (ptr.func)();
    global.post_message(&JsValue::undefined())?;
    Ok(())
}
