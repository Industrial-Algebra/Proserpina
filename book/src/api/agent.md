# The Provider Boundary: `Agent`, `AgentId`

```rust
pub trait Agent {
    fn id(&self) -> &AgentId;
    fn persona(&self) -> &Persona;
    fn respond(&mut self, msg: &Message) -> Result<Message, ProserpinaError>;
}
```

`Agent` is the provider boundary — every backend (echo, HTTP) implements it.
The engine knows nothing about providers; it only calls `respond`. The
sync/async bridge (a dedicated Tokio runtime per `HttpAgent` calling
`block_on`) keeps `respond` synchronous, so the engine, runner, CLI, and tests
are sync and free of async plumbing.

## `AgentId`

```rust
pub struct AgentId(/* String */);
```

A stable identifier for an agent within a run — a newtype over `String`,
compared by name, `Display`-formatted as the name. `Runner` keys its agent
registry by `AgentId`.

## Built-in implementations

- [`EchoAgent`](./backends.md) — the deterministic reference backend.
- [`HttpAgent`](./backends.md) — the OpenAI-compatible HTTP backend.

To add a provider Proserpina doesn't ship, implement `Agent` (or add a custom
provider section to the [credentials config](./credentials.md), which uses the
HTTP backend under the hood).
