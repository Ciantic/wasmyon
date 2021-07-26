use std::path::Path;

fn main() {
    // println!("cargo:rerun-if-changed=pkg");

    // https://github.com/rustwasm/wasm-bindgen/search?q=CARGO_MANIFEST_DIR
    // https://github.com/rustwasm/wasm-bindgen/blob/master/crates/backend/src/encode.rs

    let wasm_js = env!("CARGO_PKG_NAME").to_owned().replace("-", "_") + ".js";
    let new_file = "threaded.js";
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("pkg")
        .join(new_file);

    std::fs::write(
        root,
        format!(
            r#"
            export * from "./{}";
            export {{ default as _init_worker }} from "./{}";
            import init, {{ _init_thread_workers }} from "./{}";

            /**
             * Initialize module with threads
             * 
             * @param {{number}} initial_threads
             * @param {{number}} num_of_threads
             * */
            export async function init_with_threads(initial_threads, num_of_threads) {{
                await init();
                _init_thread_workers(import.meta.url, initial_threads, num_of_threads);
            }}
            "#,
            wasm_js, wasm_js, wasm_js
        )
        .replace("            ", ""),
    )
    .unwrap();
}
