# Proserpina — Auth Validation + Human CLI

- **Date:** 2026-06-28
- **Branch:** `feature/auth-validation-and-human-cli`
- **Motivation:** A bad key killed the run (bug); the CLI is opaque for humans (UX).
  Both stem from the same root cause: the CLI was built agent-first.

## 1. The Bug: key-present ≠ key-valid

pi's auth store exports `DASHSCOPE_API_KEY` on norma-wall. Proserpina sees the
env var, marks Alibaba as authed, assigns a critic to `qwen-plus`, and the key
is stale → 401 → the whole run dies with a raw HTTP error dump.

**Fix:** two layers:
1. **Auth validation** — `proserpina auth check` makes a minimal API call per
   authed provider (GET /models or a 1-token completion) to verify the key works
   before a run. Reports human-readably which providers pass/fail.
2. **Graceful degradation** — if a provider fails mid-run (after retries), the
   runner skips it and continues with other authed providers. Only fails if ALL
   providers are exhausted. The report notes which providers were skipped.

## 2. The CLI UX: opaque and limited

Right now: two subcommands (`critique`, `capabilities`), both JSON-first.
Humans get silence during runs, raw HTTP dumps on failure, and no way to check
their setup.

**Fix:** human-first defaults, agent output behind `--json`:
1. **`capabilities` defaults to a human-readable table.** Shows providers,
   authed status, model, panels, topologies — formatted, not JSON. `--json`
   keeps the machine form.
2. **Progress during `critique` runs.** Stderr lines: which critic is running,
   which provider/model, timing, ✓/✗ per critic. Stdout stays clean for piping.
3. **Actionable error messages.** A 401 says "Your DASHSCOPE_API_KEY is set but
   rejected. Run `proserpina auth check` to diagnose." Not a raw HTTP dump.
4. **New subcommands:**
   - `proserpina auth check` — validate all authed keys (the bug fix).
   - `proserpina auth list` — show which providers have keys set (env/config).
   - `proserpina panels` — list available panels with descriptions.
5. **`--help` improvements.** `--panel` surfaced prominently; examples in help.

## 3. Implementation Sequencing (TDD)

1. **`validate_provider(config) -> Result<(), PraxisError>`** — minimal API call
   per provider. Tests with the scripted-server pattern + `#[ignore]` live test.
2. **`proserpina auth check` subcommand** — validates each authed config; reports
   human-readably (✓/✗ per provider with model + latency).
3. **Graceful degradation in the runner** — if an agent fails after retries, mark
   the provider as failed, skip remaining critics on it, continue with others.
   Test with echo + a failing agent.
4. **Human-readable `capabilities`** — table output by default; `--json` for
   the existing machine form.
5. **Progress output during `critique`** — stderr progress lines.
6. **Actionable error messages** — map HTTP error types to guidance.
7. **`proserpina panels` + `proserpina auth list` subcommands.**

## 4. Out of Scope

- Interactive setup wizard (`proserpina init`).
- `proserpina auth set <provider>` (keychain write) — deferred to a future PR.
- Streaming/chunked output during long runs.
