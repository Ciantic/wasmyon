# wasmyon

Experimental "turn key" solution for wasm and rayon. This pretty much copies the `pool.rs` from the official example, adding some stuff.

To see how to use this library, see the [`examples/simple/src/lib.rs`](./examples/simple/src/lib.rs). In essence all rayon calls must return a JS `Promise` to work correctly, so the API is: 

```rust
#[wasmyon::promise]
pub fn sum_in_workers() -> i32 {
    (0..100000 as i32).into_par_iter().sum::<i32>()
}
```

This creates a JS wrapper function:

```typescript
function sum_in_workers() -> Promise<any>
```

Additionally, if you want to run something in a worker by yourself, you can do it like this:

```rust
run_in_worker(|| yourstuff)
```

**Note** Currently the library assumes your application is generated with
`wasm-pack` using `--out-name index`. This is because there is no way to get
`import.meta.url` in the WASM, this is a limitation in wasm-bindgen.

## Try out the example

To test out, go to `examples/simple` directory and do the following:

1. Install [wasm-pack](https://github.com/rustwasm/wasm-pack)
2. Install [deno](https://deno.land/) for static File HTTP server, see [file-server-deno.ts](./examples/file-server-deno.ts) <sup>1</sup>
3. Run `wasm-pack build --target web --out-name index`
4. Run `deno run --allow-run --allow-net --allow-read ../file-server-deno.ts
   simple`
5. Navigate to `http://localhost:8000`
6. Open a DevTools to see the communication in console

## How it works?

It initalizes only _one_ `WebAssembly.Memory` object and shares it between the
workers. It also creates the thread workers within wasm-bindgen JS snippet.

## TODO


- [ ] Auto scalable worker pool, so that it terminates workers when they are not
      being utilized for a while...
- [ ] TypeScript requires work. Because `js_sys::Promise` isn't giving a way to
      type the output type. For now it's just `Promise<any>`. To fix this, it
      probably requires a patch to `wasm_bindgen`.


## Footnotes

1: If you don't want Deno, you still need a file server that is capable of setting headers `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`, otherwise SharedArrayBuffer is not defined. [See documentation.](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer)