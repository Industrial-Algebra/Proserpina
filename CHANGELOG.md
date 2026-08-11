# Changelog

All notable changes to Proserpina are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — proserpina-agent extraction

The reusable agent abstraction layer is now its own crate, so daemon
consumers (Ijima) can depend on it without the critique pipeline, CLI, or
auth subsystem. Ijima's entire integration surface — `Agent`, `AgentId`,
`Message`, `MessageKind`, `Persona`, `ProserpinaError`, `HttpAgent`,
`HttpConfig` — resolves against `proserpina-agent` standalone (proven by
`agent/tests/ijima_surface.rs`).

- **`proserpina-agent` v0.1.0**: new workspace member crate with `Agent`
  trait + `Persona` + `Message` types + `EchoAgent` (deterministic test
  oracle) + `HttpAgent` (OpenAI-compatible, behind `backend-http`).
- **`credentials` feature** (in `proserpina-agent`): daemon-friendly pure
  credential resolution — `Provider` registry, `Credentials`, `HttpConfig`,
  and `resolve_configs` / `resolve_configs_with_keyring` take explicit
  snapshots (no filesystem, environment, keychain, or HTTP-client deps).
- **`json` / `keyring` features** (in `proserpina-agent`): carry the
  error-variant cfg gates so `ProserpinaError` is identical across crates.
- **ADR-001**: the sync-trait-in-async-daemon pattern (`spawn_blocking` +
  internal tokio runtime), validated in production by Ijima. `HttpAgent`'s
  docstring links to it.

### Changed

- **Zero breaking changes.** `proserpina` re-exports the full moved surface
  from `proserpina-agent`: every existing path (`proserpina::Agent`,
  `proserpina::backend::http::HttpAgent`,
  `proserpina::backend::credentials::RetryConfig`,
  `proserpina::persona::{Persona, Panel, resolve_panel}`, …) resolves
  unchanged. 152 tests preserved; 5 new (3 daemon-surface purity + 2
  Ijima-surface replication).
- `Panel` + `resolve_panel` stay in `proserpina` (new `src/panels.rs`) —
  panel presets are a critique-pipeline concern; `proserpina::persona` is a
  shim re-exporting the agent crate's items plus the panel layer.


## [0.3.0] — 2026-07-07

### Added — Auth Subsystem

- **`proserpina auth login <provider>`**: interactive credential acquisition.
  API-key providers (DeepSeek, Z.ai, DashScope, Google, Moonshot) prompt for
  a key, validate it via GET /models, and store it. OpenAI uses an OAuth PKCE
  browser flow (same client_id as OpenAI's Codex CLI).
- **`proserpina auth login [--model <name>]`**: model override at login.
  `proserpina auth login openai --model gpt-5.5` pins the model version
  alongside the credential in one step.
- **`proserpina auth login`** (no provider): provider selector with auth
  methods shown.
- **`proserpina auth logout <provider>`**: removes stored credentials.
- **OAuth token lifecycle**: tokens stored with structured fields
  (access/refresh/expires); expired tokens auto-refreshed before each run.
- **AuthUi trait seam**: the auth UI is trait-isolated — ratatui now, Knopper
  later (zero logic changes to swap).

### Added — Provider Exclusion

- **`exclude = ["qwen3.7-max"]`** in credentials.toml disables a model from the
  roster pool. CLI: `--exclude qwen3.7-max,mercury-2`. Both are applied (union).
  Matched by model name (what's visible in capabilities/dry-run).

### Added — Pi Provider Discovery

- Auto-discovers pi's `models.json` + `auth.json` for correct per-user provider
  configs (URLs, models, keys). Solves the "key-exists ≠ key-valid" problem for
  pi-managed providers. Convenience layer; `auth login` credentials take
  precedence.

### Added — Language Flag

- **`--language <lang>`**: critics and summarizer respond in the specified
  language (e.g. `--language Japanese`, `--language français`). Default: model
  chooses based on the document.

### Added — Human-Readable CLI

- `capabilities` defaults to human-readable table (not JSON); `--json` for agents.
- Progress output during `critique` runs (stderr).
- Actionable error messages.
- New subcommands: `auth check/list`, `panels`.

### Added — Graceful Provider Degradation

- If a provider fails mid-run, Proserpina tries reassigning the critic to
  another authed provider, or skips it and continues. A single bad key no longer
  kills the whole run.

### Changed

- **OpenAI default model bumped** from `gpt-4o` to `gpt-5.4` in the registry.
- **Provider dedup now host-based**: when a pi-discovered config shares a host
  with a registry entry (e.g. `api.deepseek.com`), the pi config *replaces* the
  registry one. Prevents double-weighting the same provider with different models.
- **Credential resolution precedence**: OAuth access > keyring > env > config > pi discovery > none.

[0.3.0]: https://github.com/Industrial-Algebra/Proserpina/releases/tag/v0.3.0

## [0.2.1] — 2026-06-28

### Fixed

- **Graceful provider degradation**: when a critic's provider fails mid-run
  (bad key, rate limit, timeout), Proserpina now tries reassigning to another
  authed provider, and if all fail for that persona, skips the critic and
  continues with the rest. Previously, a single provider failure killed the
  entire run. Skipped critics are noted in the report.

[0.2.1]: https://github.com/Industrial-Algebra/Proserpina/releases/tag/v0.2.1

## [0.2.0] — 2026-06-28

### Changed — Licensing

- **License changed from AGPL-3.0-only to Apache-2.0.** The AGPL network-use
  clause created adoption barriers at enterprise customers. Apache-2.0
  maximizes adoption while preserving attribution and patent grants.
  Published 0.1.0 on crates.io remains AGPL; 0.2.0 onward is Apache-2.0.
- **CLA** grants Industrial Algebra the right to relicense contributions.
- LICENSE-COMMERCIAL removed.

### Added — CLI

- **Auth validation**: `proserpina auth check` validates keys before a run.
- **Human-readable CLI**: capabilities table, progress output, actionable
  errors, new subcommands (auth, panels).

[0.2.0]: https://github.com/Industrial-Algebra/Proserpina/releases/tag/v0.2.0

## [0.1.0] — 2026-06-23

The initial release: a provider-agnostic multi-agent critique pipeline with a
deterministic echo backend, an OpenAI-compatible HTTP backend, a multi-provider
roster, rich summarized findings, dual markdown/JSON output, full
agent-discoverability, configurable persona panels, and retry/timeout/backoff.

### Added — Core engine

- **Interaction-graph engine** with two topologies: `parallel` (fan-out) and
  `rounds` (adversarial cross-examination with convergence early-stop).
- **`Agent` trait** as the provider boundary; **`AgentId`**, **`Persona`**,
  **`Message`**/**`MessageKind`** (Critique/Rebuttal/Question/Concession/Verdict/Prompt).
- **`Subject`** (the document under critique), **`Transcript`** (ordered messages),
  **`Runner`** owning a `HashMap<AgentId, Box<dyn Agent>>` registry.
- **`Severity`** (Info/Minor/Major/Blocker), exhaustive thiserror-based `ProserpinaError`.

### Added — Backends

- **`EchoAgent`**: deterministic reference backend (prompt→critique,
  critique→rebuttal) — drives the whole engine in tests with zero LLM deps.
- **`HttpAgent`**: OpenAI-compatible chat-completions backend (DeepSeek, Z.ai
  GLM, OpenAI, Moonshot, Alibaba, Google, Ollama, any compatible endpoint).
  Sync/async bridge via a dedicated Tokio runtime; `respond` stays synchronous.

### Added — Multi-provider roster

- **`Provider` registry** of six frontier presets + **`random_roster`** (pure,
  seeded) assigning authed providers to critic personas for diverse-model
  cross-examination. **`roster_from_env`** + `NoAuthedProviders` error.
- **Standalone credentials config** (`~/.config/proserpina/credentials.toml`):
  provider keys, model/base_url overrides, **custom providers** (Ollama/proxies),
  and `[panels.NAME]` sections. Resolution precedence env > config > registry.
- **Z.ai coding-plan gateway** (`api.z.ai/api/coding/paas/v4`) so Z.ai works
  with a coding plan, not just an API plan.

### Added — Reports

- **Rich `Finding`** model: severity, category, summary, location, quote,
  suggested_change, supporting_critics — produced by a **dedicated summarizer
  LLM pass** that clusters critiques across critics.
- **Dual render** from one `Vec<Finding>`: a human-readable markdown digest
  (executive summary, findings sorted by severity, actionable suggested changes)
  and machine-readable JSON (behind `json`).

### Added — Agent integration

- **`proserpina capabilities`**: JSON self-description with **dynamic auth state**
  (which providers are authed right now), available panels, and the exit-code
  scheme.
- **`proserpina critique --dry-run`**: emits a run plan (roster, call counts) with
  zero API calls.
- **Structured error JSON** on stderr (when `--json`) + **Proserpina-specific exit
  codes** (10–16, 70).
- **Provider attribution** in errors (`agent "Devil's Advocate" (glm-5.2)
  failed`), so multi-provider runs can tell which provider died.

### Added — Panels

- **Configurable persona panels**: built-in `default`/`duo`/`panel` (1/2/5
  archetypes — Devil's Advocate, Methodologist, Red Team, Domain Expert, Editor)
  plus user-defined `[panels.NAME]` sections. `--panel <name>` flag;
  `proserpina capabilities` lists available panels.

### Added — Reliability

- **Retry / timeout / backoff** on every HTTP call: transient-only retry
  (408/429/5xx + network), exponential+jittered backoff, per-attempt timeout.
  `RetryPolicy::DEFAULT`/`NONE`; `[retry]` config + `--max-attempts`/`--timeout`
  CLI flags (precedence CLI > config > default).

### Added — Security

- **OS keychain credential tier** (`keyring` feature): highest-precedence
  key source (keyring > env > config > registry), looked up as
  `proserpina:<KEY_ENV_VAR>`. Works on macOS Keychain and Windows Credential
  Manager; Linux gnome-keyring has a known limitation (use env/config).
  `ProserpinaError::KeyringAccess` (exit 17).

### Tooling

- `proserpina` binary behind the `cli` feature; examples (`deepseek_smoke.rs`);
- 136 tests across 13 integration-test files; fmt + clippy `-D warnings` clean
  on default and all-features.
- CI workflow, this changelog, contributing guide, and mdbook documentation.

[0.1.0]: https://github.com/Industrial-Algebra/Proserpina/releases/tag/v0.1.0
