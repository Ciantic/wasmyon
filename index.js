import init, {
    sum_in_workers,
    send_to_channel,
    receive_from_channel,
    init_thread_workers,
} from "./pkg/shared_wasm_experiments.js";
await init();
await init_thread_workers("./worker.js", 8, 0);

// TODO: WorkerPool halts until the receive_from_channel finishes.
//
// Most likely because the WorkerPool does not reserve the worker and queues on
// individual workers instead of single queue

// Rayon
// ----------------------------------------------------------------------------

sum_in_workers().then((s) => {
    console.log("Sum numbers with rayon", s);
});

// Shared Channel
// ----------------------------------------------------------------------------

// Wait asynchronously in the worker for a result
receive_from_channel().then((value) => {
    console.log("Got from a channel!", value);
});

// Send a result to channel
setTimeout(() => {
    console.log("Send to channel!");
    send_to_channel("Send me!");
}, 1300);
