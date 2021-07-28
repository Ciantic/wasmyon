Deno.run({
    cmd: ["cargo", "publish", "--no-verify"],
    cwd: "macro-support",
});

Deno.run({
    cmd: ["cargo", "publish", "--no-verify"],
});
