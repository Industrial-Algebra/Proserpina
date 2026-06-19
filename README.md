# Praxis — Multi-Agent Critique & Cross-Examination

> A pipeline that puts IA documents — pre-prints, roadmaps, plans, specs — in
> the witness box and cross-examines them for intellectual rigor.

Praxis runs a configurable ensemble of critic **personas** over a document via
a **provider-agnostic interaction-graph engine**. LLM backends are pluggable
behind an `Agent` trait, and a deterministic `EchoAgent` backend makes the whole
pipeline testable with zero LLM dependencies.

## Why a graph?

"Cross-examination" means critics interrogate each other *and* the document —
not just annotate it independently. Praxis models a run as a directed graph of
agent-to-agent message passing, so the topologies compose:

| Topology | Behavior |
|----------|----------|
| `parallel` | Fan-out: each critic annotates independently, then a synthesizer merges findings. |
| `rounds` | Adversarial: critics see each other's critiques and must challenge, defend, or concede over successive rounds. |
| `moderated` | Socratic: a moderator drives the dialectic, calls on critics, and adjudicates a verdict. |

All three are templates over the same `InteractionGraph` core — no topology is
privileged, and new ones are just new graph constructors.

## Status

Scaffold. The crate surface and design are in place; modules arrive
incrementally via test-driven development. See
[`docs/plans/2026-06-19-praxis-design.md`](docs/plans/2026-06-19-praxis-design.md).

Intended first slice:

```text
praxis critique path/to/roadmap.md -o critique.md
```

## License

Dual-licensed under **AGPL-3.0-only** and a separate commercial license. See
[LICENSE](LICENSE) and [LICENSE-COMMERCIAL](LICENSE-COMMERCIAL). Commercial
licensing: <license@industrialalgebra.com>.
