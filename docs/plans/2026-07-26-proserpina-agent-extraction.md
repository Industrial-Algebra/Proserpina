# proserpina-agent Extraction + Consumer-Driven v0.4.0 Plan

> **REQUIRED SUB-SKILL:** Use the executing-plans skill to implement this plan task-by-task.

**Goal:** Extract Proserpina's thin LLM abstraction layer (`Agent` trait + `Persona` + `HttpAgent` + credential resolution core) into a standalone `proserpina-agent` crate that daemons like Ijima can depend on without the critique pipeline, then re-scope v0.4.0 around actual consumer needs.

**Architecture:** Convert the repo to a Cargo workspace. The root package stays `proserpina` (the critique pipeline); a new member crate `proserpina-agent/` holds the agent layer. `proserpina` depends on `proserpina-agent` and re-exports its full public surface, so all existing import paths (`proserpina::Agent`, `proserpina::backend::http::HttpAgent`, …) keep working — zero breaking changes for Ijima and crates.io users.

**Tech Stack:** Rust 2021, Cargo workspaces, reqwest/tokio (behind `backend-http`), serde/toml (behind `credentials`).

**Context:** PULSE_2026-07-23_Proserpina.md found that Ijima — Proserpina's only production consumer — uses exactly 8 items (`Agent`, `AgentId`, `Message`, `MessageKind`, `Persona`, `ProserpinaError`, `HttpAgent`, `HttpConfig`) and none of the critique pipeline, CLI, or auth subsystem. `resolve_configs` is already public and pure but unreachable without heavyweight feature deps.

**Key decisions (pre-made, do not re-litigate):**
1. `ProserpinaError` moves **wholesale** to proserpina-agent. The `Agent::respond` signature returns it, so it must live with the trait. Panel/keyring/summarizer variants ride along — conceptually impure, but zero breakage. Proserpina re-exports it.
2. `RetryConfig` moves to proserpina-agent (http.rs's `RetryPolicy::resolve` depends on it). It's a plain Option-fields data struct.
3. Feature split in proserpina-agent: `credentials` (Provider registry + Credentials + resolve_configs; serde+toml only) is separate from `backend-http` (HttpAgent + retry machinery; reqwest+tokio). Daemons can resolve configs as data without an HTTP client.
4. pi-discovery (`discover_pi_configs`) and auth interop (`src/auth/`) **stay in proserpina** — they are CLI/convenience-layer concerns.
5. Versioning: proserpina-agent starts at 0.1.0. proserpina goes 0.3.0 → 0.4.0 (semver-minor: additive re-exports, no breakage). Publish order: proserpina-agent first, then proserpina.

---

## Phase 1: proserpina-agent extraction

### Task 1: Workspace conversion + crate skeleton

**TDD scenario:** Structural change — verify with `cargo check` and existing test suite staying green.

**Files:**
- Modify: `Cargo.toml` (root — add `[workspace]`, keep `[package]`)
- Create: `agent/Cargo.toml`
- Create: `agent/src/lib.rs`

**Step 1: Add workspace table to root Cargo.toml**

Insert after `[package]` block:

```toml
[workspace]
members = ["agent"]
```

**Step 2: Create agent/Cargo.toml**

```toml
# Copyright (C) 2026 Industrial Algebra
# SPDX-License-Identifier: Apache-2.0

[package]
name = "proserpina-agent"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
description = "Provider-agnostic LLM agent abstraction — Agent trait, Persona, Message types, and HTTP backend. The reusable core of the Proserpina critique pipeline."
license = "Apache-2.0"
repository = "https://github.com/industrial-algebra/Proserpina"
homepage = "https://github.com/industrial-algebra/Proserpina"
documentation = "https://docs.rs/proserpina-agent"
keywords = ["llm", "agent", "provider-agnostic", "http", "abstraction"]
categories = ["api-bindings", "asynchronous"]
readme = "../README.md"

[features]
default = ["std"]
std = []
serde = ["dep:serde"]
credentials = ["serde", "dep:toml", "dep:serde_json"]
backend-http = ["credentials", "dep:reqwest", "dep:tokio", "dep:rand"]

[dependencies]
thiserror = "2"
serde = { version = "1", features = ["derive"], optional = true }
serde_json = { version = "1", optional = true }
toml = { version = "0.8", optional = true }
reqwest = { version = "0.12", features = ["json"], optional = true }
tokio = { version = "1", features = ["rt-multi-thread", "macros"], optional = true }
rand = { version = "0.9", optional = true }

[dev-dependencies]
tempfile = "3"
tokio = { version = "1", features = ["net", "rt-multi-thread", "macros", "time", "io-util"] }
```

**Step 3: Create agent/src/lib.rs**

```rust
// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! Provider-agnostic LLM agent abstraction.
//!
//! This crate is the reusable core of the Proserpina critique pipeline:
//! the [`Agent`] trait, [`Persona`], [`Message`] types, a deterministic
//! [`EchoAgent`] test oracle, and (behind `backend-http`) an
//! OpenAI-compatible [`HttpAgent`]. Behind `credentials`, the pure
//! credential-resolution core ([`resolve_configs`]) for daemon consumers.
//!
//! Extracted from `proserpina` v0.3.0 so daemon consumers (e.g. Ijima's
//! mining tier) can depend on the agent layer without the critique
//! pipeline, CLI, or auth subsystem.

pub mod agent;
pub mod message;
pub mod persona;
pub mod error;

pub mod echo;

#[cfg(feature = "credentials")]
pub mod credentials;

#[cfg(feature = "backend-http")]
pub mod http;

pub use agent::{Agent, AgentId};
pub use echo::EchoAgent;
pub use error::ProserpinaError;
pub use message::{Message, MessageKind};
pub use persona::Persona;

#[cfg(feature = "credentials")]
pub use credentials::{Credentials, Provider, resolve_configs};

#[cfg(feature = "backend-http")]
pub use http::{HttpAgent, HttpConfig, RetryConfig, RetryPolicy};
```

**Step 4: Verify workspace resolves**

Run: `cargo check --workspace`
Expected: FAIL — agent/src modules don't exist yet (next tasks create them), but root `proserpina` must still compile. Confirm the error is only about missing agent modules.

**Step 5: Commit**

```bash
git add Cargo.toml agent/
git commit -m "chore: convert to workspace, add proserpina-agent skeleton"
```

---

### Task 2: Move error type into proserpina-agent

**TDD scenario:** Modifying tested code — run existing tests first to establish green baseline.

**Files:**
- Move: `src/error.rs` → `agent/src/error.rs`
- Modify: `src/lib.rs` (re-export)

**Step 1: Baseline**

Run: `cargo test --all-features`
Expected: 152 passing.

**Step 2: Move the file**

```bash
mkdir -p agent/src
git mv src/error.rs agent/src/error.rs
```

In `agent/src/error.rs`, fix internal imports: `use crate::agent::AgentId;` (path stays valid once agent.rs lands in Task 3 — for now it will not compile; that is expected, do not patch around it).

**Step 3: Re-export from proserpina**

In `src/lib.rs`, replace `pub mod error;` with:

```rust
pub use proserpina_agent::error;
pub use proserpina_agent::ProserpinaError;
```

Remove the old `pub use error::ProserpinaError;` line if present (now redundant). Add to root `[dependencies]`:

```toml
proserpina-agent = { version = "0.1.0", path = "agent" }
```

**Step 4: Verify**

Run: `cargo check -p proserpina`
Expected: compile errors in proserpina only where `crate::agent` / `crate::message` / `crate::persona` paths were used by error.rs — those land in Task 3. If errors are ONLY about agent/message/persona modules, proceed.

**Step 5: Commit**

```bash
git add -A
git commit -m "refactor: move ProserpinaError into proserpina-agent"
```

---

### Task 3: Move core types (agent, message, persona)

**TDD scenario:** Modifying tested code — existing tests are the safety net.

**Files:**
- Move: `src/agent.rs` → `agent/src/agent.rs`
- Move: `src/message.rs` → `agent/src/message.rs`
- Move: `src/persona.rs` → `agent/src/persona.rs`
- Modify: `src/lib.rs` (re-exports)
- Modify: `src/graph.rs`, `src/runner.rs`, `src/report.rs`, `src/summary.rs`, `src/subject.rs`, `src/transcript.rs`, `src/agent_info.rs` (import paths, if needed — re-exports should make this unnecessary)

**Step 1: Move the files**

```bash
git mv src/agent.rs agent/src/agent.rs
git mv src/message.rs agent/src/message.rs
git mv src/persona.rs agent/src/persona.rs
```

**Step 2: Fix feature gates in moved files**

`persona.rs` and `message.rs` have `#[cfg(feature = "serde")]` derives — these now resolve against proserpina-agent's `serde` feature (defined in Task 1). No code change needed; the feature names match.

**Step 3: Re-export from proserpina**

In `src/lib.rs`, replace `pub mod agent; pub mod message; pub mod persona;` with:

```rust
pub use proserpina_agent::{agent, message, persona};
pub use proserpina_agent::{Agent, AgentId, Message, MessageKind, Persona};
```

**Step 4: Verify whole workspace compiles and all tests pass**

Run: `cargo check --workspace --all-features && cargo test --all-features`
Expected: 152 passing. Every existing import path (`proserpina::Agent`, `crate::persona::Persona`, …) resolves through the re-exports.

**Step 5: Commit**

```bash
git add -A
git commit -m "refactor: move Agent/Message/Persona core types into proserpina-agent"
```

---

### Task 4: Move EchoAgent

**TDD scenario:** Modifying tested code — EchoAgent is the test oracle for the whole suite; existing tests are the verification.

**Files:**
- Move: `src/backend/echo.rs` → `agent/src/echo.rs`
- Modify: `src/backend/mod.rs` (re-export)

**Step 1: Move**

```bash
git mv src/backend/echo.rs agent/src/echo.rs
```

Fix imports in moved file: `use crate::agent::…` paths remain valid (same layout inside proserpina-agent).

**Step 2: Re-export**

In `src/backend/mod.rs`, replace `pub mod echo;` with:

```rust
pub use proserpina_agent::echo;
```

Check `src/lib.rs` for `EchoAgent` re-exports and point them at `proserpina_agent::EchoAgent`.

**Step 3: Verify**

Run: `cargo test --all-features`
Expected: 152 passing — the EchoAgent oracle drives most of the suite; any import break shows up immediately.

**Step 4: Commit**

```bash
git add -A
git commit -m "refactor: move EchoAgent into proserpina-agent"
```

---

### Task 5: Move HttpAgent + HttpConfig + RetryPolicy + RetryConfig

**TDD scenario:** Modifying tested code — HTTP backend has its own integration tests (`tests/http_backend.rs`, `tests/retry.rs`); they are the verification.

**Files:**
- Move: `src/backend/http.rs` → `agent/src/http.rs`
- Modify: `agent/src/http.rs` — `RetryConfig` definition moves here (cut from credentials.rs); fix `use crate::backend::credentials::RetryConfig` → local
- Modify: `src/backend/mod.rs` (re-export)
- Modify: `src/backend/credentials.rs` — remove RetryConfig definition, re-export from proserpina-agent
- Modify: root `Cargo.toml` — `backend-http` feature forwards to proserpina-agent

**Step 1: Move and rewire features**

```bash
git mv src/backend/http.rs agent/src/http.rs
```

In `agent/src/http.rs`:
- Change `use crate::backend::credentials::RetryConfig` — cut the `RetryConfig` struct definition (from `src/backend/credentials.rs:58-69`) and paste it into `agent/src/http.rs` above `RetryPolicy`. It needs `#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]` and `Default`.
- All other `use crate::{agent, message, persona}` paths stay valid.

**Step 2: Rewire proserpina's backend-http feature**

Root `Cargo.toml`:

```toml
backend-http = ["serde", "proserpina-agent/backend-http", "dep:toml", "dep:base64", "dep:sha2", "dep:hex", "dep:webbrowser"]
```

(reqwest/tokio/serde_json/rand now come transitively via proserpina-agent; toml stays for credentials.rs; base64/sha2/hex/webbrowser stay for `src/auth/`. Remove now-unused optional deps from root `[dependencies]` if nothing else references them — check with `cargo check`.)

In `src/backend/mod.rs`:

```rust
pub use proserpina_agent::http;
```

In `src/backend/credentials.rs`: delete the `RetryConfig` struct, add `pub use proserpina_agent::http::RetryConfig;` so `crate::backend::credentials::RetryConfig` still resolves for existing code.

**Step 3: Verify**

Run: `cargo test --all-features`
Expected: 152 passing, including `tests/http_backend.rs` (mock TCP server tests) and `tests/retry.rs`.

Run feature matrix: `cargo check -p proserpina-agent --no-default-features`, `--features serde`, `--features backend-http`
Expected: all compile.

**Step 4: Commit**

```bash
git add -A
git commit -m "refactor: move HttpAgent/HttpConfig/retry machinery into proserpina-agent"
```

---

### Task 6: Move credential-resolution core under `credentials` feature

**TDD scenario:** Modifying tested code — `tests/credentials.rs` and `tests/cli_roster.rs` are the verification. New test: proserpina-agent's credentials feature works standalone.

**Files:**
- Create: `agent/src/credentials.rs` — `Provider` (registry type), `Credentials`, `ProviderOverride`, `RetryConfig` re-export, `resolve_configs`, `resolve_configs_with_keyring`, `default_registry()`
- Modify: `src/backend/credentials.rs` — keep `Credentials::from_path`/`discover`/`to_toml_string` (filesystem), `PanelConfig`, `authed_configs_with*`, `discover_pi_configs`, `exclude`; re-export moved items
- Modify: `src/backend/roster.rs` — imports may shift
- Test: `agent/tests/credentials_standalone.rs` (new)

**Step 1: Survey what moves vs stays**

Move to `agent/src/credentials.rs` (pure data + pure functions, no filesystem, no pi):
- `Provider` struct + `default_registry()` (the built-in six-provider registry — check `src/backend/roster.rs` for its current home)
- `Credentials` struct + `ProviderOverride` + `from_toml` (TOML parsing is pure)
- `resolve_configs`, `resolve_configs_with_keyring`
- `extract_host` (host-dedup helper)

Stay in `src/backend/credentials.rs` (filesystem / pi / CLI concerns):
- `Credentials::from_path`, `discover`, `discover_or`, `to_toml_string` (if it does file writes — check)
- `discover_pi_configs` (reads `~/.pi/agent/models.json`)
- `authed_configs_with`, `authed_configs_with_excludes` (env-reading wrappers — these read `std::env`, so they stay; they become thin wrappers calling proserpina-agent's pure core)
- `PanelConfig`, `read_keyring`, `store_model`/oauth accessors if they live here

`Credentials` splitting note: if filesystem methods are `impl Credentials` blocks, they can live in proserpina as an extension `impl` only if Credentials stays in the same crate — **it cannot**. So: all inherent `impl Credentials` methods must move with the struct. Filesystem methods become free functions in proserpina (`load_credentials_from_path(path) -> Result<Credentials>`) that call `Credentials::from_toml`. Update callers in `src/cli/`.

**Step 2: Write the failing standalone test**

Create `agent/tests/credentials_standalone.rs`:

```rust
// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! Daemon-style credential resolution: pure, no filesystem, no env reads,
//! no keyring, no CLI. This is the surface Ijima-style consumers use.

#![cfg(feature = "credentials")]

use proserpina_agent::credentials::{Credentials, default_registry, resolve_configs};
use std::collections::HashMap;

#[test]
fn resolves_authed_config_from_explicit_env_keys() {
    let registry = default_registry();
    let creds = Credentials::from_toml("").unwrap();
    let mut env = HashMap::new();
    env.insert("DEEPSEEK_API_KEY".to_string(), "sk-test".to_string());

    let configs = resolve_configs(&registry, &creds, &env).unwrap();
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].api_key, "sk-test");
    assert!(!configs[0].model.is_empty());
    assert!(!configs[0].base_url.is_empty());
}

#[test]
fn no_keys_yields_no_configs() {
    let registry = default_registry();
    let creds = Credentials::from_toml("").unwrap();
    let configs = resolve_configs(&registry, &creds, &HashMap::new()).unwrap();
    assert!(configs.is_empty());
}
```

Run: `cargo test -p proserpina-agent --features credentials`
Expected: FAIL (module doesn't exist).

**Step 3: Implement the move**

Move the items per Step 1. proserpina's `src/backend/credentials.rs` re-exports:

```rust
pub use proserpina_agent::credentials::{
    Credentials, Provider, ProviderOverride, extract_host, resolve_configs,
    resolve_configs_with_keyring,
};
```

**Step 4: Verify**

Run: `cargo test -p proserpina-agent --features credentials` — new tests PASS.
Run: `cargo test --all-features` (workspace) — 152 + new tests passing.
Run: `cargo check -p proserpina-agent --no-default-features` and `--features serde` — compile.

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: move credential-resolution core into proserpina-agent behind 'credentials' feature

Daemon consumers (Ijima) can now resolve provider configs as pure data —
no filesystem, env, keyring, CLI, or HTTP client dependencies required."
```

---

## Phase 2: Documentation + roadmap

### Task 7: ADR — sync-trait-in-async-daemon pattern

**TDD scenario:** Documentation only — no tests.

**Files:**
- Create: `docs/adrs/ADR-001-sync-agent-in-async-daemon.md`

**Step 1: Write the ADR**

Structure:

```markdown
# ADR 001: Synchronous Agent trait inside async daemons

**Status:** Accepted (validated in production by Ijima, 2026-07)
**Date:** 2026-07-26

## Context
Proserpina's `Agent::respond` is synchronous by design — the interaction-graph
runner is a state machine, and sync composition keeps the engine, CLI, and
test suite free of async machinery. `HttpAgent` owns an internal tokio
runtime and calls `block_on` inside `respond`.

Critics predicted runtime-in-runtime panics when called from async contexts.

## Decision
Keep the trait synchronous. Contain async inside backends.

## The pattern (for daemon consumers)
```rust
tokio::task::spawn_blocking(move || {
    let mut agent = HttpAgent::new(id, persona, config)?;
    agent.respond(&prompt)  // sync; internal runtime handles HTTP
}).await?
```

Rules:
1. NEVER call `respond` directly inside an async task — always `spawn_blocking`.
2. The blocking thread owns the agent; `HttpAgent`'s internal runtime never
   coexists with the caller's executor on the same thread.
3. Return values cross the boundary as plain data (`Message`, `Report`).

## Evidence
Ijima's mining daemon (ijima-server/src/api.rs) has run this pattern in
production since 2026-07: `spawn_blocking` wraps `mine_all`, which calls
`respond` per persona. Zero panics, zero deadlocks.

## Consequences
+ Engine + tests stay sync (EchoAgent oracle needs no runtime)
+ New backends (MCP, subprocess) handle their own async internally
- Consumers in async contexts must know the `spawn_blocking` rule
  (this ADR is the documentation)

## Applies to
Sakamoto, Wallace, any Anima daemon needing LLM access via proserpina-agent.
```

**Step 2: Commit**

```bash
git add docs/adrs/
git commit -m "docs: ADR-001 sync Agent trait in async daemons (Ijima-validated pattern)"
```

---

### Task 8: Re-scope ROADMAP.md for v0.4.0

**TDD scenario:** Documentation only.

**Files:**
- Modify: `docs/ROADMAP.md`

**Step 1: Rewrite the v0.4.0 section**

Replace the v0.4.0 wishlist (Moderated topology, parallel execution, Knopper migration as headline items) with consumer-driven scope:

```markdown
## v0.4.0 — Consumer-Driven (proserpina-agent extraction)

Theme: serve the consumers that actually exist. Ijima (production, mining
tier) uses the Agent abstraction layer, not the critique pipeline.

- [x] proserpina-agent crate: Agent trait + Persona + Message + EchoAgent +
  HttpAgent + credential-resolution core, extracted into a standalone crate
- [x] `credentials` feature: daemon-friendly pure config resolution
  (no filesystem/env/keyring/HTTP-client deps)
- [x] ADR-001: sync-trait-in-async-daemon pattern (Ijima-validated)
- [ ] Stability commitment: no breaking changes to `Agent` trait through 0.4.x
- [ ] Ijima migration guide: depend on proserpina-agent directly
  (slimmer dep tree, faster compiles)

## v0.5.0+ — Pipeline evolution (deferred, consumer-triggered)

- Moderated topology (Socratic dialectic) — build when a panel-critique
  consumer emerges; Ijima explicitly does not need it (ADR M5)
- Parallel execution / batch critique
- Knopper TUI migration (AuthUi seam exists; waiting on Knopper)
- Per-persona provider pinning
- Streaming output
```

**Step 2: Commit**

```bash
git add docs/ROADMAP.md
git commit -m "docs: re-scope v0.4.0 around consumer needs (Ijima), defer panel features"
```

---

## Post-plan: publish sequence (when executed)

1. `cargo publish -p proserpina-agent` (0.1.0) — must land first
2. Switch root dep from `path = "agent"` to `version = "0.1.0"` (path stays for local dev via `patch` or dual spec: `proserpina-agent = { version = "0.1.0", path = "agent" }` already handles both)
3. Bump root to 0.4.0, `cargo publish -p proserpina`
4. Tag v0.4.0 per ia-gitflow (feature branch → develop → release PR → main → tag)
5. Open Ijima PR migrating `proserpina = "0.3.0"` → `proserpina-agent = "0.1"` in ijima-miner (core types) and ijima-server (`backend-http` feature)
6. Blog post per ia-website skill — **new post, new slug** (`proserpina-agent-extracted` or similar); never update an existing release post

## Verification checklist (end of execution)

- [ ] `cargo test --workspace --all-features` — 152 + new tests passing
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] Feature matrix compiles: `--no-default-features`, `serde`, `credentials`, `backend-http`, all
- [ ] Ijima's exact import surface compiles against proserpina-agent standalone (the Task 6 test file proves it)
- [ ] All existing `proserpina::` import paths unchanged (re-export test)
- [ ] docs.rs build succeeds for both crates
