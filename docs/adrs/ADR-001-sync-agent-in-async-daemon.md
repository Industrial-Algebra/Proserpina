# ADR 001: Synchronous Agent trait inside async daemons

**Status:** Accepted — validated in production by Ijima (2026-07)
**Date:** 2026-07-26

## Context

Proserpina's `Agent::respond` is synchronous by design: the interaction-graph
runner is a state machine, and sync composition keeps the engine, the CLI, and
the entire test suite free of async machinery. `HttpAgent` owns an internal
tokio runtime and calls `block_on` inside `respond`, containing all async
complexity inside the backend module.

From the first design discussions, critics predicted this would force
`block_on` inside async contexts and cause runtime-in-runtime panics or
deadlocks in daemon consumers.

## Decision

Keep the trait synchronous. Contain async inside backends. Document the one
rule consumers must follow instead of rewriting the engine to be async.

## The pattern (for daemon consumers)

```rust
let extractions = tokio::task::spawn_blocking(move || {
    let mut agent = HttpAgent::new(id, persona, config)?;
    agent.respond(&prompt) // sync; HttpAgent's internal runtime handles HTTP
}).await?;
```

Rules:

1. **Never call `respond` directly inside an async task** — always go through
   `spawn_blocking` (or a dedicated thread). `block_on` panics only when it
   runs on a thread that already hosts an active tokio executor;
   `spawn_blocking` threads never do.
2. **The blocking thread owns the agent.** Construct the `HttpAgent` inside
   the `spawn_blocking` closure (or move it in); its internal runtime is
   created and dropped on that thread.
3. **Return values cross the boundary as plain data** (`Message`, `Report`,
   extraction structs). Nothing async escapes the closure.

## Evidence

Ijima's mining daemon (`ijima-server/src/api.rs`) has run this pattern in
production since 2026-07: an axum handler wraps `ijima_miner::mine_all` in
`spawn_blocking`, which calls `respond` once per extraction persona on a
Proserpina `HttpAgent`. Zero panics, zero deadlocks. The skepticism was
reasonable; the production data settled it.

## Consequences

- The engine, runner, CLI, and test suite stay synchronous — the `EchoAgent`
  oracle needs no runtime, and the whole graph engine is testable with zero
  async machinery.
- New backends (MCP, subprocess, WebSocket) follow the same rule: own your
  runtime, keep `respond` sync.
- Consumers in async contexts must know the `spawn_blocking` rule. This ADR
  is that documentation; the docstring on `HttpAgent` links here.
- A full-async engine remains explicitly out of scope (see ROADMAP): large
  rewrite, unclear benefit until intra-run concurrency is needed.

## Applies to

Any async service embedding `proserpina-agent` backends: Ijima (mining tier),
and future Anima daemons (Sakamoto, Wallace) that need LLM access.
