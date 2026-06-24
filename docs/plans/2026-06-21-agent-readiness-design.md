# Praxis — Agent-Readiness Cluster Design

- **Date:** 2026-06-21
- **Status:** Approved (design phase; implementation via TDD)
- **Branch:** `feature/agent-readiness`
- **Depends on:** `backend-http` (PR #6), roster (#7), credentials (#8), rich
  findings (#9)
- **Motivation:** Justin intends to deploy Praxis across all his dev
  environments and call it on the fly from AI coding agents. This PR makes the
  CLI thoroughly agent-discoverable.

## 1. Purpose

An agent calling Praxis needs four moves: ask what it can do, ask what a run
would do, do it, and understand failures. This PR adds the missing three
(self-describe, dry-run, structured errors) and a cross-cutting provider-
attribution fix. Praxis stays a **CLI** (the lightest, most universal
agent-callable form) — MCP server mode is explicitly deferred.

## 2. Key Design Decisions

1. **CLI-only, no MCP.** A standalone binary with `--json` + self-describe is
   the most universal agent-callable form; works with any agent (pi, Claude
   Code, Codex, Gemini) with zero integration. MCP couples to MCP-aware
   clients and turns Praxis into a server.
2. **Capabilities carries DYNAMIC auth state.** `praxis capabilities` reports
   not just the registry but which providers are *currently authed* in this
   environment (reads config + env). An agent learns what it can actually do
   right now, not just what Praxis could do in theory.
3. **Praxis-specific exit codes (10–15, 70).** Small, self-documenting, and
   emitted in `capabilities` so an agent can learn the scheme programmatically.
   Not BSD sysexits — clearer for Praxis-specific semantics.
4. **Error JSON is `--json`-gated.** Humans get prose on stderr by default;
   agents opt into structured JSON via `--json`. Avoids noisy stderr for
   interactive use while giving agents a clean machine contract.
5. **`--json` plumbing already exists** (PR #9 report). This PR extends it to
   capabilities, dry-run, and errors.

## 3. Provider Attribution

`HttpAgent`'s error detail currently names only the persona (`agent "Devil's
Advocate" failed`). Prepend provider/model: `agent "Devil's Advocate"
(deepseek-chat) failed`. Small change to `fetch_response`'s error strings; no
type change. Makes every multi-provider failure self-diagnosing (the gap from
the 429 debugging session).

## 4. `praxis capabilities` (the keystone)

New subcommand emitting JSON (default, since it's machine-facing):

```json
{
  "version": "0.1.0",
  "subcommands": ["critique", "capabilities"],
  "output_formats": ["markdown", "json"],
  "topologies": ["parallel", "rounds"],
  "providers": [
    {"name": "deepseek", "model": "deepseek-chat", "authed": true},
    {"name": "openai",   "model": "gpt-4o",        "authed": false}
  ],
  "personas": [{"name": "Devil's Advocate", "framing": "...", "focus": "..."}],
  "exit_codes": {"0": "success", "2": "usage", "10": "no_authed_providers", ...}
}
```

- `providers[].authed` is **dynamic** — derived from `authed_configs_with()`
  against the real config + env.
- `personas` reflects the current default panel (configurable panels are a
  separate follow-up; this exposes what's there now).
- `exit_codes` documents the scheme below, so an agent learns it from Praxis
  itself.

## 5. `praxis critique --dry-run`

Resolves the roster and emits a **Plan** JSON without making any API calls:

```json
{
  "seed": 42,
  "topology": "parallel",
  "roster": [{"persona": "Devil's Advocate", "provider": "deepseek", "model": "deepseek-chat"}],
  "n_critic_calls": 1,
  "n_summarizer_calls": 1,
  "estimated_total_calls": 2
}
```

Lets an agent verify intent and estimate cost before spending tokens.
`n_critic_calls` is deterministic from topology + panel; the summarizer is
always +1.

## 6. Structured Errors + Exit Codes

When `--json` is set and a run fails, emit on **stderr**:

```json
{"error": {"kind": "no_authed_providers", "message": "...", "tried": ["deepseek","openai",...]}}
```

Exit codes (Praxis-specific, documented in `capabilities`):

| Code | Meaning                  | Variant                |
|------|--------------------------|------------------------|
| 0    | success                  | —                      |
| 2    | usage error              | clap default           |
| 10   | no authed providers      | `NoAuthedProviders`    |
| 11   | agent (provider) failure | `AgentFailure`         |
| 12   | summarizer failure       | `SummaryFailed`        |
| 13   | malformed credentials    | `MalformedCredentials` |
| 14   | incomplete custom provider | `IncompleteCustomProvider` |
| 15   | missing agent            | `MissingAgent`         |
| 70   | other / internal         | fallback               |

`PraxisError` gains `exit_code()` and `to_error_json()` methods. The binary
maps errors → code + JSON-on-stderr (when `--json`).

## 7. Crate Shape

- `src/agent_info.rs` (new) — `Capabilities` and `Plan` types + builders,
  pure data with JSON serialization. Gated `#[cfg(feature = "backend-http")]`
  where they reference provider resolution.
- `src/error.rs` — `exit_code()` + `error_kind()` (the machine string) +
  `to_error_json()`.
- `src/backend/http.rs` — provider attribution in error detail.
- `src/cli/` — `capabilities` subcommand; `--dry-run` flag on `critique`;
  `--json`-gated error path in `main.rs` with exit-code mapping.

## 8. Implementation Sequencing (TDD)

Each step RED → GREEN → REFACTOR.

1. **Provider attribution** — `HttpAgent` errors include `(model)`.
2. **`Capabilities` type + builder** — pure; takes the authed-provider snapshot.
3. **`Plan` type + dry-run resolution** — resolves roster without calls.
4. **`PraxisError::exit_code` + `error_kind` + `to_error_json`** — variant →
   code/kind mapping + structured JSON.
5. **CLI wiring** — `capabilities` subcommand, `--dry-run`, `--json` error
   path + exit codes.

## 9. Deliberately Deferred

- **MCP server mode** — Praxis stays CLI; MCP is a heavier, separate path.
- **Configurable persona panels** — capabilities *reports* the current panel;
  making it configurable is separate.
- **Streaming output** — emit findings as they parse, for long runs.
- **Cost estimation in USD** — dry-run shows call counts, not $ (too fuzzy).

## 10. Open Questions / Future Work

- **Capabilities schema versioning** — once agents depend on the shape, a
  `schema_version` field may be needed for evolution.
- **`--markdown` flag on capabilities** — human-readable capabilities (currently
  JSON-default since it's machine-facing).
- **Shell-completion generation** — `praxis completions <shell>` via clap-
  complete, for interactive ergonomics.
