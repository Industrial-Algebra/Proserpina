# Proserpina — Directions

> **v0.1.0 Snapshot** — The core pipeline is complete and usable: parallel +
> rounds topologies, HTTP backend, multi-provider roster, credentials config,
> rich summarized findings, dual markdown/JSON, agent-discoverability,
> configurable panels, retry/timeout/backoff. The directions below are
> explorations and known gaps, **not commitments**. See
> [CHANGELOG.md](../CHANGELOG.md) for the full v0.1.0 feature list.

**Version:** 0.3.0 — Auth subsystem complete. 152 tests, clippy clean.
**Gitflow:** `main` (releases) ← `develop` (integration) ← `feature/*` (work)

---

## Current State

Proserpina is a provider-agnostic multi-agent critique pipeline. It is synchronous,
testable end-to-end via the echo backend (zero LLM deps), and reaches six
frontier providers (DeepSeek, Z.ai GLM, OpenAI, Moonshot, Alibaba, Google)
plus any custom OpenAI-compatible endpoint. 152 tests, zero warnings across all
feature combinations, `cargo publish --dry-run` clean.

**Completed for v0.3.0:**
- ✅ **Auth subsystem**: `proserpina auth login <provider>` (interactive API key + OAuth PKCE), `auth check`, `auth list`, `auth logout`. Token storage + auto-refresh lifecycle.
- ✅ **AuthUi trait seam**: ratatui now, Knopper later (zero logic changes).
- ✅ **Model override at login**: `auth login --model gpt-5.5` pins the version.
- ✅ **Provider exclusion**: `exclude = [...]` in config + `--exclude` CLI flag.
  Disable a model from the roster (billing lapsed, model retired).
- ✅ **Host-based provider dedup**: pi configs replace registry entries for same
  host; prevents double-weighting a provider (e.g. deepseek-chat + deepseek-v4-pro).
- ✅ **OpenAI default bumped** to gpt-5.4 (registry).
- ✅ **Pi provider discovery**: auto-reads pi's models.json + auth.json (convenience layer).
- ✅ **`--language` flag**: output in any supported language.
- ✅ **Human-readable CLI**: capabilities table, progress output, actionable errors.
- ✅ **Expanded HTTP test suite**: 148 tests covering retry, degradation, timeout,
  backoff, and graceful-failure edge cases.

**Completed for v0.1.0–v0.2.1:**
- ✅ Interaction-graph engine (`parallel`, `rounds`) with convergence early-stop
- ✅ Provider-agnostic `Agent` trait; echo + HTTP backends
- ✅ Multi-provider roster (seeded, reproducible) + standalone credentials config
- ✅ Custom-provider support (Ollama, LM Studio, OpenRouter, proxies)
- ✅ Rich per-issue findings via a dedicated summarizer LLM pass
- ✅ Dual markdown/JSON render from one `Vec<Finding>`
- ✅ Agent-discoverability (`capabilities`, `--dry-run`, structured errors, exit codes)
- ✅ Configurable persona panels (built-in + `[panels.NAME]`)
- ✅ Retry / timeout / backoff (config + CLI knobs)
- ✅ OS keychain credential tier (`keyring` feature; macOS/Windows; Linux
  gnome-keyring has a known limitation — see near-term directions)
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
- **Keyring on Linux** — the `keyring` crate's gnome-keyring backend has a
  write-Ok/read-NoEntry quirk; macOS/Windows work. Stabilize the Linux path
  (pin `secret-service` with an explicit collection, or document the
  workaround more loudly).
- **Credentials file permissions check** — warn if world-readable.
- **Ollama as a first-class preset** — common local-LLM case.
- **Full ratatui TUI** — the auth login UI is currently stdin-based; a proper
  ratatui list widget is planned (behind the AuthUi trait seam for Knopper
  migration).
- **Device-code flow** — for headless machines without a browser.

---

## Explicitly Deferred / Out of Scope for v0.1.x

- **MCP server mode.** Proserpina is a CLI (the lightest, most universal
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
  differently. Does Proserpina need a rubric, or per-model calibration?
- **Diversity measurement.** How do we *measure* whether a panel's diversity
  actually improved the critique, vs. a homogeneous panel? Open empirical
  question.
- **Convergence semantics for `rounds`.** Today: a round with zero rebuttals
  stops the run. Is that the right convergence signal, or should concessions
  count differently?

If you work on any of these, open an issue or PR — contributions follow the
[CONTRIBUTING](../CONTRIBUTING.md) flow (CLA required).
