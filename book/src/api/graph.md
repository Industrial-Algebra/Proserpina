# The Interaction Graph: `InteractionGraph`, `Topology`

```rust
pub enum Topology {
    Parallel { critics: Vec<AgentId> },
    Rounds   { critics: Vec<AgentId>, max_rounds: usize },
}

pub enum InteractionGraph {
    Parallel { critics: Vec<AgentId> },
    Rounds   { critics: Vec<AgentId>, max_rounds: usize },
}

impl Topology {
    pub fn parallel(critics: Vec<AgentId>) -> Self;
    pub fn rounds(critics: Vec<AgentId>, max_rounds: usize) -> Self;
}

impl From<Topology> for InteractionGraph { /* ... */ }
```

A `Topology` is a declarative template; lowerer it into an
`InteractionGraph` with `.into()`. `parallel` is a degenerate single-round
graph; `rounds` adds inter-critic edges with convergence early-stop (a round
producing zero rebuttals stops the run). See [The Interaction Graph](../concepts/graph.md)
for the routing rules.

Future topologies (`moderated`) are new constructors over the same core, not
special cases.
