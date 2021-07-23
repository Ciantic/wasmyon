import init, { get_from_map, add_to_map } from "./pkg/shared_wasm_experiments.js";

let id = -1;

onmessage = async function (m) {
    if (m.data.task === "init") {
        await init(undefined, m.data.mem);
        id = m.data.id;
        console.log("Worker", id, "init done");
        postMessage("init done");
    }

    if (m.data.task === "get_from_map") {
        console.log("Worker", id, ": get_from_map", m.data.key, get_from_map(m.data.key));
    }

    if (m.data.task === "add_to_map") {
        console.log("Worker", id, ": add_to_map", m.data.key, m.data.value);
        add_to_map(m.data.key, m.data.value);
    }
};
