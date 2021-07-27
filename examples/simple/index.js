import init, { sum_in_workers, send_to_channel, receive_from_channel } from "./pkg/index.js";

await init();

// Rayon
// ----------------------------------------------------------------------------

sum_in_workers().then((s) => {
    console.log("Sum numbers with rayon", s);
});

// Shared Channel
// ----------------------------------------------------------------------------

// Wait asynchronously in the worker for a result
receive_from_channel().then((value) => {
    console.log("Got from a channel! 1", value);
});
receive_from_channel().then((value) => {
    console.log("Got from a channel! 2", value);
});

// Send a result to channel
setTimeout(() => {
    console.log("Send to channel 1");
    send_to_channel("First");
}, 500);

// Send a result to channel
setTimeout(() => {
    console.log("Send to channel 2");
    send_to_channel("Second");
}, 800);
