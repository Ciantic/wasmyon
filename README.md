# Rust WASM experiment with rayon

I'm experimenting with Rayon here. This pretty much copies the `pool.rs` from the official example, adding some stuff.

To test out how to use this library, see the [`examples/simple/src/lib.rs`](./examples/simple/src/lib.rs). In essence all rayon calls must return a JS `Promise` to work correctly, so the API is just single function at the moment: `run_in_workers(|| yourstuff)`.

To test out, go to `examples/simple` directory and do the following:

1. Install [wasm-pack](https://github.com/rustwasm/wasm-pack)
2. Install [deno](https://deno.land/) for static File HTTP server, see [file-server-deno.ts](./file-server-deno.ts) <sup>1</sup>
3. Run `wasm-pack build --target web --out-name index`
4. Run `deno run --allow-run --allow-net --allow-read ../file-server-deno.ts
   simple`
5. Navigate to `http://localhost:8000`
6. Open a DevTools to see the communication in console

## How it works?

It initalizes only _one_ `WebAssembly.Memory` object and shares it between the
workers. It also creates the thread workers within wasm-bindgen JS snippet.

## Footnotes

1: If you don't want Deno, you still need a file server that is capable of setting headers `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`, otherwise SharedArrayBuffer is not defined. [See documentation.](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer)

https://github.com/rustwasm/wasm-bindgen/blob/master/crates/cli-support/src/lib.rs#L632
