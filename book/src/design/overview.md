# Design Overview

Proserpina is layered: a pure **engine** (graph + runner + transcript) that knows
nothing about providers, with **backends** (echo, HTTP) implementing an `Agent`
trait at the edges.

```
                ┌──────────────────────────────────────┐
   subject ───► │  Runner  (walks the InteractionGraph) │
                │   │                                   │
                │   ▼                                   │
                │  Transcript  (ordered Messages)       │
                └──────────────────┬───────────────────┘
                                   │
              ┌────────────────────┴────────────────────┐
              ▼                                         ▼
       Summarizer (HTTP)                         Report
       clusters into Findings                markdown │ JSON
```

## The provider boundary

```rust
pub trait Agent {
    fn id(&self) -> &AgentId;
    fn persona(&self) -> &Persona;
    fn respond(&mut self, msg: &Message) -> Result<Message, ProserpinaError>;
}
```

Every backend implements `Agent`. The engine never imports `reqwest` or touches
the network — that lives entirely behind `HttpAgent` (gated `backend-http`).
The sync/async bridge (a dedicated Tokio runtime per `HttpAgent` calling
`block_on`) keeps `respond` synchronous, so the engine, runner, CLI, and all
tests are sync and free of async plumbing.

## The roster is pure policy

Provider assignment (`random_roster`) is a pure function of
`(personas, authed_configs, rng)` — no env inside. The env-touching layer
(`config_from_env`, `authed_configs_with`) is thin and isolated, so the core is
fully unit-testable. See [Providers and Credentials](../guide/providers.md).

## Where the design lives

The full design history is in [the design documents](./docs.md) — one per
major feature, capturing the decisions and alternatives considered.
