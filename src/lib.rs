// This file copies a lot from wasm-bindgen example:
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

use futures_channel::oneshot;
use once_cell::sync::OnceCell;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::RwLock;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};
use web_sys::{ErrorEvent, Event, Worker};

pub use wasmyon_macro_support::*;

static WORKER_POOL: OnceCell<WorkerPool> = OnceCell::new();
static THREAD_POOL: OnceCell<rayon::ThreadPool> = OnceCell::new();

// Proper way to get main module url, unfortunately following does not work, the
// WASM module alone cannot get the import.meta.url, which is a bit shame, it's
// simply not defined when called by the WASM.
//
// #[wasm_bindgen]
// pub fn get_main_import_meta_url() -> Result<String, JsValue> {
//     match js_sys::eval("import.meta.url") {
//         Ok(v) => Ok(v.as_string().unwrap_or("".to_owned())),
//         Err(err) => Err(err.clone()),
//     }
// }

#[wasm_bindgen(module = "/thread_worker.js")]
extern "C" {
    fn createThreadWorker(wasm_module: JsValue, wasm_memory: JsValue) -> Worker;
}

fn init_thread_workers(
    initial: usize,
    num_of_threads: usize,
) -> (&'static WorkerPool, &'static rayon::ThreadPool) {
    let worker_pool = WORKER_POOL.get_or_init(|| WorkerPool::new(initial).unwrap());

    let thread_pool = THREAD_POOL.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_of_threads)
            .spawn_handler(|thread| Ok(worker_pool.run(|| thread.run()).unwrap()))
            .build()
            .unwrap()
    });
    (&worker_pool, &thread_pool)
}

pub fn run_in_worker<R, F1>(f: F1) -> js_sys::Promise
where
    F1: (FnOnce() -> R) + Send + 'static,
    R: Into<JsValue> + Send + 'static,
{
    let (tx, rx) = oneshot::channel();

    let hardware_threads = web_sys::window()
        .unwrap()
        .navigator()
        .hardware_concurrency() as usize;

    let (worker_pool, thread_pool) = init_thread_workers(0, hardware_threads);
    worker_pool
        .run(move || {
            thread_pool.install(|| {
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
    pub fn new(initial: usize) -> Result<WorkerPool, JsValue> {
        let pool = WorkerPool {
            state: Arc::new(PoolState {
                workers: RwLock::new(Vec::with_capacity(initial)),
                callback: Closure::wrap(Box::new(|event: Event| {
                    console_log!("unhandled event: {}", event.type_());
                    logv(&event);
                }) as Box<dyn FnMut(Event)>),
            }),
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
        Ok(createThreadWorker(
            wasm_bindgen::module(),
            wasm_bindgen::memory(),
        ))
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

/// Entry point invoked by `thread_worker.js`, a bit of a hack but see the "TODO" above
/// about `thread_worker.js` in general.
#[wasm_bindgen]
pub fn _thread_entry_point(ptr: u32) -> Result<(), JsValue> {
    let ptr = unsafe { Box::from_raw(ptr as *mut Work) };
    let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
    (ptr.func)();
    global.post_message(&JsValue::undefined())?;
    Ok(())
}
