import init, {
    sum_in_workers,
    sum_in_workers2,
    init_thread_workers,
} from "./pkg/shared_wasm_experiments.js";
const mod = await init();
const threads = await init_thread_workers("./worker.js", 0, 3);
console.log("threads", threads);
console.log("SUM", await sum_in_workers());
console.log(
    "SUM",
    await sum_in_workers2({
        from: 0,
        to: 50,
    })
);
