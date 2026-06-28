# Executing a Run: `Runner`

```rust
pub struct Runner { /* graph + HashMap<AgentId, Box<dyn Agent>> */ }

impl Runner {
    pub fn new(graph: InteractionGraph) -> Self;
    #[must_use]
    pub fn with_agent(self, agent: impl Agent + 'static) -> Self;
    pub fn execute(&mut self, subject: &Subject) -> Result<Transcript, ProserpinaError>;
}
```

`Runner` owns the executable state of a run — the graph plus a registry of
agents keyed by `AgentId` — and produces a `Transcript` when executed over a
`Subject`.

`execute` takes `&mut self` because agents are stateful
([`Agent::respond`](./agent.md) takes `&mut self`); this keeps the synchronous
core free of interior mutability. The CLI constructs a runner once and calls
`execute` once per run.

## The `SYSTEM_AGENT` sender

`pub const SYSTEM_AGENT: &str = "system";`

The reserved sender identity for system-originated prompts. In `parallel` and
`rounds`, the subject is broadcast to critics as a `Prompt` from `system`;
critics reply to `Some(system)`.

## Errors

- [`ProserpinaError::MissingAgent`](./errors.md) — a graph node has no registered agent.
- [`ProserpinaError::AgentFailure`](./errors.md) — an agent's `respond` failed.
