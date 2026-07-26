# Proserpina — Directions

> **v0.1.0 Snapshot** — The core pipeline is complete and usable: parallel +
> rounds topologies, HTTP backend, multi-provider roster, credentials config,
> rich summarized findings, dual markdown/JSON, agent-discoverability,
> configurable panels, retry/timeout/backoff. The directions below are
> explorations and known gaps, **not commitments**. See
> [CHANGELOG.md](../CHANGELOG.md) for the full v0.1.0 feature list.

**Version:** 0.3.0 — Auth subsystem complete. 152 tests, clippy clean.
**Gitflow:** `main` (releases) ← `develop` (integration) ← `feature/*` (work)
**Consumers:** Ijima (production — mining-LLM tier, uses the Agent abstraction
layer directly). See PULSE_2026-07-23 for the consumer analysis that drives
the version sequencing below.

---

## Version Sequencing

Decided 2026-07-26. The spine: **serve the consumer that exists, then
validate the premise, then grow the pipeline.**

### v0.4.0 — Extraction (consumer-driven)

Theme: serve the consumers that actually exist. Ijima uses the Agent
abstraction layer, not the critique pipeline — so the layer becomes its own
crate.

Plan: `docs/plans/2026-07-26-proserpina-agent-extraction.md`

- proserpina-agent crate: `Agent` trait + `Persona` + `Message` types +
  `EchoAgent` + `HttpAgent`, extracted into a standalone crate.
  Zero breaking changes — `proserpina` re-exports the full surface.
- `credentials` feature: daemon-friendly pure config resolution
  (serde+toml only — no filesystem/env/keyring/HTTP-client deps).
- ADR-001: sync-trait-in-async-daemon pattern (`spawn_blocking` + internal
  tokio runtime), validated in production by Ijima.
- Stability commitment: no breaking changes to the `Agent` trait through 0.4.x.
- Post-publish: Ijima migration PR (`proserpina` → `proserpina-agent`).

### v0.5.0 — Validation (does the premise hold?)

Theme: answer the Rabbit Hole's unanswered question — *does multi-agent
adversarial review produce meaningfully better critique than single-pass,
or just more words?* The architecture enables the experiment; v0.5.0 runs it.

- **Evaluation harness** (`proserpina eval`, feature-gated): seeded-flaw
  corpus — documents with known injected defects (logical contradictions,
  unsupported claims, methodology errors) — run across configurations:
  single-pass vs. parallel panel vs. rounds, cheap-model panel vs. one
  expensive model. Metrics: flaw-detection precision/recall, corroboration
  rates, severity calibration accuracy, **cost per configuration** (the
  "3 cheap vs. 1 expensive" comparison needs token accounting).
- **Severity rubric**: a documented rubric in the summarizer prompt so
  severity calibration is consistent across models (research question #1).
- **Translation-panel validation domain** — a built-in `translation` panel
  preset (fidelity keeper, native-fluency editor, scholarly editor,
  poet/author, terminology consistency) critiquing machine translations
  *against their source texts*. The adversarial structure is intrinsic:
  fidelity vs. fluency is translation's classic dialectic, and `rounds`
  maps onto it without contrivance. Ground truth is cheap (native-speaker
  judgment), making this the cleanest quantitative answer to the Rabbit
  Hole question available today. Concrete first experiment: the Karpal
  mdbook Japanese translation (glm-5.2 single-pass; native review: "accurate
  but 1:1 literal, no native flavor") — run the panel, apply revisions,
  blind re-rate by the same reviewer. The control group already exists.
  (Panel preset is config-level — personas are data. Subject pairing works
  today by concatenating source+translation into the subject body; a
  `--source` convenience flag is v0.6.0 pipeline work if the domain
  validates.)
- **Diversity measurement**: the harness's corroboration data answers whether
  panel diversity improves critique vs. a homogeneous panel (question #2).
- **Convergence semantics**: rounds-topology runs in the harness produce the
  data to evaluate the zero-rebuttals stopping rule (question #3).
- Companion items (same release, motivated by long eval runs):
  per-provider **circuit breaker**, **`Retry-After` header honoring**,
  **Ollama as a first-class preset** (local models keep eval costs near zero).

If the harness validates the premise, its output doubles as the marketing
dataset. If it falsifies single-config assumptions, we learn that cheaply.

### v0.6.0+ — Pipeline evolution (usage-driven)

Theme: features for the critique use case, built when our own document
workflow (or a second consumer) demands them.

- **Batch critique** — many documents, one run (deferred from v0.3.0 planning).
- **Full-repo critique** — a repository as `Subject` (deferred from v0.3.0).
- **Streaming** — emit findings as the summarizer parses them.
- **Per-persona provider pinning** and **`[personas.NAME]` reusable personas**.
- **Report rendering**: consensus-vs-contested highlighting, per-critic
  attribution view.
- **Translation workflow support** (`--source` flag, paired-subject handling)
  — if the v0.5.0 translation-panel experiment validates the domain.
- **`capabilities` schema versioning** — when downstream agents depend on the
  shape.

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

## Backlog (unscheduled)

Items not yet pulled into a version. Ordered loosely by value.

### Reliability
- **Per-provider retry policy overrides** — `[providers.zai] retry = {...}`.

### Expressiveness
- **`moderated` topology** — a Socratic moderator drives the dialectic, calls
  on specific critics, and adjudicates a final `Verdict`. The last topology
  from the original design. **Deferred until a panel-critique consumer
  emerges** — Ijima explicitly does not need it (Ijima ADR M5).

### Trust & ergonomics
- **Keyring on Linux** — the `keyring` crate's gnome-keyring backend has a
  write-Ok/read-NoEntry quirk; macOS/Windows work. Stabilize the Linux path
  (pin `secret-service` with an explicit collection, or document the
  workaround more loudly).
- **Credentials file permissions check** — warn if world-readable.
- **Full ratatui TUI** — the auth login UI is currently stdin-based; a proper
  ratatui list widget is planned (behind the AuthUi trait seam for Knopper
  migration).
- **Device-code flow** — for headless machines without a browser.

---

## Explicitly Deferred / Out of Scope

- **MCP server mode.** Proserpina is a CLI (the lightest, most universal
  agent-callable form). An MCP server would couple it to MCP-aware clients and
  turn it into a long-running process; revisit if there's demand.
- **Async engine.** The sync/async bridge (block-on-runtime per `HttpAgent`)
  works and keeps the engine simple — now validated in production by Ijima
  (ADR-001). A full async engine is a large rewrite with unclear benefit
  until concurrency within a run is needed.
- **Claim/section extraction.** v1 critiques whole documents. An optional
  `Subject` transform that extracts claims/sections first is a natural
  enhancement, deferred until we see what's useful from real outputs.

---

## Research Questions

All three original questions are now **scheduled for answers via the v0.5.0
evaluation harness** rather than open-ended musing:

- **Severity calibration** → rubric in the summarizer prompt (v0.5.0).
- **Diversity measurement** → corroboration data from harness runs (v0.5.0).
- **Convergence semantics for `rounds`** → stopping-rule data from harness
  runs (v0.5.0).

New questions the harness itself will raise (cost-normalized quality curves,
corpus contamination, rubric drift across models) get tracked here as they
emerge.

If you work on any of these, open an issue or PR — contributions follow the
[CONTRIBUTING](../CONTRIBUTING.md) flow (CLA required).
