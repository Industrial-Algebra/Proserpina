# Introduction

**Praxis** is a pipeline for multi-agent critique and cross-examination of
documents that require intellectual rigor — pre-prints, roadmaps, plans, and
specs.

It runs a configurable ensemble of critic **personas** over a document using a
**provider-agnostic interaction-graph engine**. Each critic is backed by a
frontier LLM, drawn (seeded, reproducibly) from your authed providers so a run
naturally spans DeepSeek, Z.ai GLM, OpenAI, Moonshot, Alibaba, and Google.
A dedicated **summarizer** LLM pass then clusters the panel's critiques into
actionable, per-issue findings.

> Praxis puts your document in the witness box and cross-examines it.

## Why multi-agent critique?

Different frontier models have different blind spots, biases, and strengths.
A panel that mixes them catches what a single reviewer — or a homogeneous
panel — misses. The summarizer tells you where the panel *agreed* (many
critics converging on one finding) versus where it *contested*.

## Design principles

- **Provider-agnostic.** An `Agent` trait is the provider boundary; every
  backend (echo, HTTP, future) implements it. The deterministic `EchoAgent`
  makes the entire engine testable with zero LLM dependencies.
- **Composable topologies.** A run is a directed graph of agent-to-agent
  message passing. `parallel` and `rounds` are templates over one core engine.
- **Spec-shaped.** Clean separation between policy (panels, roster, retry) and
  execution (the runner, the graph walk).
- **Agent-discoverable.** `praxis capabilities`, `--dry-run`, structured error
  JSON, and documented exit codes make Praxis callable on the fly by AI agents.

## Status

v0.1.0. See the [CHANGELOG](https://github.com/Industrial-Algebra/Praxis/blob/main/CHANGELOG.md)
for the full feature set.
