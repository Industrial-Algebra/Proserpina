# Proserpina — Design Document

- **Date:** 2026-06-19
- **Status:** Approved (scaffold phase complete; implementation via TDD)
- **Project:** Proserpina — Multi-Agent Critique & Cross-Examination Pipeline
- **Crate:** standalone, `proserpina`, edition 2021, AGPL-3.0-only

## 1. Purpose

Proserpina is a Rust crate and CLI that runs **multi-agent critique and
cross-examination** over Industrial Algebra documents — pre-prints, roadmaps,
plans, and specs — anything that benefits from intellectual rigor before it
ships.

A configurable ensemble of critic **personas** interrogates a document. The
core is a **provider-agnostic interaction-graph engine**: LLM backends are
pluggable, and a deterministic `EchoAgent` backend makes the entire pipeline
testable with zero LLM dependencies.

## 2. Key Design Decisions (brainstorm outcomes)

1. **Provider-agnostic framework, not a hard-coded LLM path.** An `Agent` trait
   is the provider boundary; backends (CLI subprocess, HTTP API, MCP, echo) are
   pluggable behind additive features. Ship the echo backend first so the core
   is fully unit-testable from day one. Keeps critique logic decoupled from any
   single model or transport until we know what works.

2. **General interaction graph, with topologies as templates.** A run is a
   directed graph of agent-to-agent message passing. `parallel` (fan-out),
   `rounds` (adversarial cross-examination), and `moderated` (Socratic
   dialectic) are pre-built topology templates over the same graph core — not
   special cases. Composability and generality over privileged topologies.

3. **First vertical slice is a thin CLI.** `proserpina critique <file> -o <report.md>`
   exercises the graph core, one backend (echo), one topology (parallel
   fan-out), and a markdown report renderer end-to-end. Round-based/moderated
   topologies and real LLM backends arrive in later PRs.

## 3. Core Data Model

### `Subject`
The document under critique. v1: opaque markdown text plus metadata (source
path, title). Later: an optional claim/section extraction stage; the graph
operates on whichever units are present (whole document, claims, or sections).

### `Agent` trait — the provider boundary
```rust
pub trait Agent {
    fn id(&self) -> &AgentId;
    fn persona(&self) -> &Persona;
    fn respond(&mut self, msg: &Message) -> Result<Response, ProserpinaError>;
}
```
Every backend (echo, CLI subprocess, HTTP API, MCP) implements `Agent`.

### `Persona`
The lens a critic applies — **data, not an enum**: name, framing, focus area.
Ships with a built-in registry of archetypes (Devil's Advocate, Methodologist,
Red Teamer, Domain Expert, Editor) and is user-extensible so critics can be
configured without code changes.

### `Message`
A graph edge payload: sender `AgentId`, optional recipient/broadcast, a
`MessageKind`, and text content. `MessageKind` is an **exhaustive enum**:
`Critique`, `Rebuttal`, `Question`, `Concession`, `Verdict`.

### `AgentId`
Newtype token, identity-stable across the graph. v1 keeps the core amari-free
and dependency-light; amari-style phantom typing for `AgentId` is available
later under an optional feature.

## 4. Interaction Graph & Execution

### `InteractionGraph`
Directed graph: nodes are agents, edges are message routes. Built from
`Topology` constructors:
- `Topology::parallel(critics)` — v1
- `Topology::rounds(critics, n)` — next
- `Topology::moderated(moderator, critics)` — later

`parallel` is a degenerate single-round graph, so the engine generalizes from
the start rather than being refactored later.

### `Runner`
`Runner::execute(graph, subject, &mut dyn Agent) -> Result<Transcript>` walks
the graph, routes messages per edges, and collects the ordered `Transcript`.

### `Transcript`, `Report`, `Finding`, `Severity`
- **`Transcript`** — ordered `Message`s produced during the run.
- **`Report`** — aggregates `Finding`s, one per substantive point.
- **`Finding`** — author (`AgentId`), `Severity`, summary, optional location,
  reasoning, and references back into the `Transcript`.
- **`Severity`** — exhaustive enum: `Info`, `Minor`, `Major`, `Blocker`.

The `parallel` topology's synthesis step folds per-critic `Critique` messages
into `Finding`s. `Report::to_markdown()` renders the human-readable critique;
JSON output lands behind the `json` feature.

## 5. Crate Layout & Features

Standalone crate (Schubert pattern), edition 2021, `AGPL-3.0-only`,
`rust-toolchain.toml` nightly + rustfmt/clippy, IA gitflow (`main` + `develop`).

**Features (additive only):**
- `default = ["std"]`, `std`
- `cli` — binary + clap
- `serde` — `Serialize`/`Deserialize`
- `json` — JSON report output (implies `serde`)
- future: `parallel` (rayon), `async` (async backends)

**Modules (land incrementally):**
```
src/
├── lib.rs            # crate docs, feature surface, re-exports
├── main.rs           # `proserpina` binary
├── error.rs          # ProserpinaError (thiserror)
├── subject.rs        # Subject
├── agent.rs          # Agent trait, AgentId
├── persona.rs        # Persona, built-in registry
├── message.rs        # Message, MessageKind
├── graph.rs          # InteractionGraph, Topology constructors
├── runner.rs         # Runner::execute
├── transcript.rs     # Transcript, Report, Finding, Severity
├── backend/
│   ├── mod.rs        # backend module
│   └── echo.rs       # EchoAgent (deterministic test backend)
├── report.rs         # markdown rendering
└── cli/
    ├── mod.rs
    └── critique.rs   # `proserpina critique` subcommand
```

## 6. Implementation Sequencing

Each step is TDD: failing test first, then minimal implementation, then
refactor. Every public item documented with `# Examples` / `# Errors`.

1. **Scaffold + design doc** *(this commit)* — Cargo.toml, lib.rs stub,
   LICENSE/README/toolchain, gitflow branches.
2. **Data model + echo backend** — `error`, `subject`, `agent`, `persona`,
   `message`, `backend/echo`. Deterministic from the start.
3. **Graph + parallel topology + runner** — `graph`, `runner`, `transcript`;
   `Topology::parallel` end-to-end against `EchoAgent`.
4. **Report + CLI vertical slice** — `report` (markdown), `cli`/`main.rs`;
   `proserpina critique <file> -o <report.md>` works with the echo backend.
5. **Round-based topology** — `Topology::rounds`; real cross-examination flows.
6. **Real LLM backend(s)** — pick the first concrete provider (CLI subprocess,
   HTTP API, or MCP) based on what the echo-driven core has taught us.

## 7. Conventions

IA coding standards throughout: TDD, `Result` (never panic in library code),
exhaustive `match`, `thiserror` errors, additive feature gates, doc tests on
all public items, AGPL-3.0-only headers on every file. Gitflow: feature
branches → PR to `develop` → release PR to `main`; human review only, no
auto-merge.

## 8. Open Questions / Future Work

- **Execution model:** synchronous in v1; evaluate `async`-trait backends once
  real providers need concurrency.
- **Extraction stage:** claim/section parsing as an optional `Subject` transform
  before critique.
- **First real backend:** decide between subprocess LLM CLIs (claude/codex/
  gemini), direct HTTP API, or MCP-driven based on integration cost vs.
  determinism.
- **Phantom typing:** optional `amari`-backed `AgentId` under a feature.
- **Report formats:** stabilize the markdown layout; define a JSON schema under
  the `json` feature.
- **Verdict semantics:** how a `moderated` topology's final `Verdict` is
  derived and scored.
