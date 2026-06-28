# Changelog

All notable changes to Proserpina are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


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
