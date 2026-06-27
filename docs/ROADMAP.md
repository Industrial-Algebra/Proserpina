# Praxis — Directions

> **v0.1.0 Snapshot** — The core pipeline is complete and usable: parallel +
> rounds topologies, HTTP backend, multi-provider roster, credentials config,
> rich summarized findings, dual markdown/JSON, agent-discoverability,
> configurable panels, retry/timeout/backoff. The directions below are
> explorations and known gaps, **not commitments**. See
> [CHANGELOG.md](../CHANGELOG.md) for the full v0.1.0 feature list.

**Version:** 0.1.0 — Foundation complete. IA-conformant. Dual-licensed.
**Gitflow:** `main` (releases) ← `develop` (integration) ← `feature/*` (work)

---

## Current State

Praxis is a provider-agnostic multi-agent critique pipeline. It is synchronous,
testable end-to-end via the echo backend (zero LLM deps), and reaches six
frontier providers (DeepSeek, Z.ai GLM, OpenAI, Moonshot, Alibaba, Google) plus
any custom OpenAI-compatible endpoint. 136 tests, zero warnings across all
feature combinations, `cargo publish --dry-run` clean.

**Completed for v0.1.0:**
- ✅ Interaction-graph engine (`parallel`, `rounds`) with convergence early-stop
- ✅ Provider-agnostic `Agent` trait; echo + HTTP backends
- ✅ Multi-provider roster (seeded, reproducible) + standalone credentials config
- ✅ Custom-provider support (Ollama, LM Studio, OpenRouter, proxies)
- ✅ Rich per-issue findings via a dedicated summarizer LLM pass
- ✅ Dual markdown/JSON render from one `Vec<Finding>`
- ✅ Agent-discoverability (`capabilities`, `--dry-run`, structured errors, exit codes)
- ✅ Configurable persona panels (built-in + `[panels.NAME]`)
- ✅ Retry / timeout / backoff (config + CLI knobs)
- ✅ Z.ai coding-plan gateway support
- ✅ mdbook documentation, CI, CHANGELOG, CONTRIBUTING

---

## Near-term Directions

These are the most-requested / highest-value follow-ups, roughly ordered. None
is committed to a version.

### Reliability
- **Per-provider circuit breaker** — after N failures in a window, stop
  retrying that provider for the rest of the run (vs. the current per-call
  retry). Matters more as panels grow.
- **`Retry-After` header honoring** — when a 429 carries one, respect it
  instead of computing backoff.
- **Per-provider retry policy overrides** — `[providers.zai] retry = {...}`.

### Expressiveness
- **`moderated` topology** — a Socratic moderator drives the dialectic, calls
  on specific critics, and adjudicates a final `Verdict`. The last topology
  from the original design.
- **Per-persona provider pinning** — let a persona name a preferred provider
  (Methodologist → most rigorous model), falling back to random.
- **`[personas.NAME]` reusable personas** — define a persona once, reference it
  from multiple panels.

### Output
- **Streaming** — emit findings as the summarizer parses them, for long runs.
- **Report rendering richer** — severity/location extraction already exists;
  add consensus-vs-contested highlighting, per-critic attribution view.
- **`capabilities` schema versioning** — once agents depend on the shape, add
  a `schema_version` field for evolution.

### Trust & ergonomics
- **Keychain integration** — `keyring`-backed credential tier (more secure
  than plaintext).
- **Credentials file permissions check** — warn if world-readable.
- **Ollama as a first-class preset** — common local-LLM case.

---

## Explicitly Deferred / Out of Scope for v0.1.x

- **MCP server mode.** Praxis is a CLI (the lightest, most universal
  agent-callable form). An MCP server would couple it to MCP-aware clients and
  turn it into a long-running process; revisit if there's demand.
- **Async engine.** The sync/async bridge (block-on-runtime per `HttpAgent`)
  works and keeps the engine simple. A full async engine is a large rewrite
  with unclear benefit until concurrency within a run is needed.
- **Claim/section extraction.** v1 critiques whole documents. An optional
  `Subject` transform that extracts claims/sections first is a natural
  enhancement, deferred until we see what's useful from real outputs.

---

## Research Questions

- **Severity calibration.** Different summarizer models calibrate severity
  differently. Does Praxis need a rubric, or per-model calibration?
- **Diversity measurement.** How do we *measure* whether a panel's diversity
  actually improved the critique, vs. a homogeneous panel? Open empirical
  question.
- **Convergence semantics for `rounds`.** Today: a round with zero rebuttals
  stops the run. Is that the right convergence signal, or should concessions
  count differently?

If you work on any of these, open an issue or PR — contributions follow the
[CONTRIBUTING](../CONTRIBUTING.md) flow (CLA required).
