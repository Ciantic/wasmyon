// deno-lint-ignore-file

import init, { _thread_entry_point } from "../../index.js";

/**
 *
 * @param {WebAssembly.Memory} memory
 * @param {WebAssembly.Module} module
 * @returns {Worker}
 */
export function createThreadWorker(module, memory) {
    const w = new Worker(import.meta.url, {
        type: "module",
    });
    w.postMessage([module, memory]);
    return w;
}

self.onmessage = (event) => {
    const [module, memory] = event.data;

    const initialised = init(module, memory).catch((err) => {
        // Propagate to main `onerror`:
        setTimeout(() => {
            {
                throw err;
            }
        });

        // Rethrow to keep promise rejected and prevent execution of further commands:
        throw err;
    });

    self.onmessage = async (event) => {
        // This will queue further commands up until the module is fully initialised:
        const mod = await initialised;
        _thread_entry_point(event.data);
    };
};
