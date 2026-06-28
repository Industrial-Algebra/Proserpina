# The Interaction Graph

A Proserpina run is modeled as a **directed graph of agent-to-agent message
passing**: nodes are agents (critics, a synthesizer) and edges are message
routes. Topologies — `parallel`, `rounds` — are templates that produce a graph.

## `parallel`

Fan-out: every critic receives the subject prompt independently and produces
a critique. The simplest cross-examination — a degenerate single-round graph.

```
        ┌──► Critic A ──► Critique
subject ─┼──► Critic B ──► Critique
        └──► Critic C ──► Critique
```

## `rounds`

Adversarial cross-examination over successive rounds:

- **Round 1:** the subject is broadcast to every critic as a `Prompt`; each
  produces a `Critique`.
- **Rounds 2..=max_rounds:** each critic receives the prior round's messages
  from the *other* critics. A `Critique` elicits a `Rebuttal`, addressed to
  that critic. A critic never rebuts itself.
- **Convergence:** a round that produces zero rebuttals stops the run early,
  never exceeding `max_rounds`.

```
subject ──► [Critique, Critique, Critique]   (round 1)
                 │            │
                 ▼            ▼
            [Rebuttal, Rebuttal, Rebuttal]   (round 2, each sees the others)
                 │            │
                 ▼            ▼
            ...converges or hits max_rounds
```

The transcript (ordered messages) is then handed to the
[summarizer](./findings.md).

## Generalizing

`parallel` is a degenerate single-round graph; `rounds` adds inter-critic
edges. Future topologies (`moderated` — a Socratic moderator drives the
dialectic) are new graph constructors, not special cases. See the
[design overview](../design/overview.md).
