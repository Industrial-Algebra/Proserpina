# Proserpina — Multi-Agent Critique & Cross-Examination

> A pipeline that puts your documents — pre-prints, roadmaps, plans, specs — in
> the witness box and cross-examines them for intellectual rigor, using a panel
> of frontier-model critics.

[![CI](https://github.com/Industrial-Algebra/Proserpina/actions/workflows/ci.yml/badge.svg)](https://github.com/Industrial-Algebra/Proserpina/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/proserpina.svg)](https://crates.io/crates/proserpina)
[![docs.rs](https://docs.rs/proserpina/badge.svg)](https://docs.rs/proserpina)

Proserpina runs a configurable ensemble of critic **personas** over a document via
a **provider-agnostic interaction-graph engine**. A dedicated **summarizer**
LLM pass clusters the panel's critiques into actionable, per-issue findings.
LLM backends are pluggable; a deterministic `EchoAgent` makes the whole
pipeline testable with zero LLM dependencies.

> ⚠️ **Privacy: Proserpina sends your document to a frontier-model provider.**
> A critique ships the full document text to the provider(s) backing the
> critics and summarizer. For confidential or regulated content, route Proserpina
> to a **local** model (Ollama, LM Studio) via a custom config section — no
> data leaves your machine. See
> [Security & Privacy](https://industrial-algebra-proserpina.netlify.app/security/considerations.html)
> for the full picture.

## Why multi-agent critique?

Different frontier models have different blind spots, biases, and strengths. A
panel that mixes them catches what a single reviewer — or a homogeneous panel —
misses. Proserpina assigns each critic a provider drawn (seeded, reproducibly) from
your authed set, so a 5-critic run naturally spans DeepSeek, Z.ai GLM, OpenAI,
Moonshot, Alibaba, and Google. The summarizer then tells you where the panel
*agreed* (many critics, one finding) vs. where it *contested*.

## How it works

1. **The roster** resolves which of your providers are authed (env vars or a
   credentials file) and assigns each critic persona one, seeded for
   reproducibility.
2. **The interaction graph** routes messages between critics. Today: `parallel`
   (fan-out) and `rounds` (adversarial cross-examination).
3. **The summarizer** — a second LLM call over the transcript — clusters
   critiques into per-issue `Finding`s: severity, category, location, quote,
   suggested change, and the supporting critics.
4. **The report** renders two ways from the same findings: a human-readable
   markdown digest and machine-readable JSON.

## Quick start

```bash
cargo install proserpina

# Set one provider key (DeepSeek is the zero-config default)...
export DEEPSEEK_API_KEY=sk-...

# ...and cross-examine a document.
proserpina critique roadmap.md

# Or use a 5-critic panel fanned across all your authed providers:
proserpina critique roadmap.md --panel panel
```

A run prints a markdown digest:

```
# Critique Report

**Subject:** `roadmap.md`
**Findings:** 5 (3 blocker, 1 major, 1 minor, 0 info)

## 1. [blocker] The proposal conflates consensus with eventual consistency.
- **Category:** logical contradiction
- **Quote:** > "consensus algorithm ... using eventual consistency"
- **Suggested change:** Choose a strong-consistency model (Raft/Paxos) or
  rename to a pattern that doesn't claim consensus.
- **Raised by:** Devil's Advocate, Methodologist, Red Team, Domain Expert

_Reproducibility: seed `3`_
```

Add `--json` for machine-readable output; `--seed N` to reproduce a run exactly.

## Configuration

Providers, persona panels, and retry policy live in a single TOML file at
`~/.config/proserpina/credentials.toml` (overridable via `PROSERPINA_CONFIG` or
`--config`):

```toml
# Auth: a section per provider. Env vars also work (DEEPSEEK_API_KEY etc).
[deepseek]
api_key = "sk-..."

[zai]
api_key = "..."
base_url = "https://api.z.ai/api/coding/paas/v4"   # coding-plan gateway
model = "glm-5.2"

# Panels: built-in (default/duo/panel) or custom.
[panels.red-team]
personas = [
  { name = "Skeptic", framing = "Doubt everything.", focus = "assumptions" },
  { name = "Nitpicker", framing = "Find the small flaws.", focus = "details" },
]

# Retry policy for all HTTP calls (CLI --max-attempts/--timeout override).
[retry]
max_attempts = 4
timeout_secs = 45
```

Any custom provider (Ollama, LM Studio, OpenRouter, a proxy) works too — just
supply `base_url` + `model` + `api_key` in a section whose name isn't a built-in.

## Panels

| Built-in | Critics | When |
|---|---|---|
| `default` | Devil's Advocate | quick single-critic pass (the default) |
| `duo` | + Methodologist | a second lens |
| `panel` | + Red Team, Domain Expert, Editor | full cross-examination |

Define your own under `[panels.NAME]` (above). `--panel <name>` selects one.

## Agent integration

Proserpina is designed to be called on the fly by AI agents across your dev
environments. The full loop:

| Move | Command |
|---|---|
| What can you do, right now? | `proserpina capabilities` |
| What would this run do / cost? | `proserpina critique doc.md --dry-run --seed N` |
| Do it (structured) | `proserpina critique doc.md --json` |
| What went wrong? | structured JSON on stderr + exit code |

`capabilities` reports dynamic auth state (which providers are authed *in this
environment*) and the exit-code scheme. Exit codes: `0` success, `2` usage,
`10` no authed providers, `11` agent failure, `12` summary failed, `13–16`
config errors, `70` other.

## Reliability

Every HTTP call (each critic and the summarizer) has a per-attempt timeout and
retries transient failures (408/429/5xx, network errors) with exponential +
jittered backoff. Non-transient 4xx fails fast. Defaults are sensible; override
via `[retry]` or `--max-attempts`/`--timeout`.

## Features

| Feature | What it adds |
|---|---|
| `std` (default) | standard library support |
| `cli` | the `proserpina` binary |
| `serde` | Serialize/Deserialize impls for core types |
| `json` | machine-readable JSON report output |
| `backend-http` | the OpenAI-compatible HTTP agent, multi-provider roster, credentials config, summarizer |
| `keyring` | OS keychain credential tier (macOS/Windows; Linux limited) — implies `backend-http` |

## License

Licensed under **Apache-2.0**. See [LICENSE](LICENSE). Contributors must sign
the [CLA](https://github.com/Industrial-Algebra/.github/blob/main/CLA.md).
