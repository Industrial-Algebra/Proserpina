# Praxis — Rich Findings & Dual Render Design

- **Date:** 2026-06-21
- **Status:** Approved (design phase; implementation via TDD)
- **Branch:** `feature/rich-findings`
- **Depends on:** `backend-http` (PR #6), roster (PR #7), credentials (PR #8)
- **Unblocks:** session summary UX, agent-discoverability (`--json`), provider
  attribution, recommended-changes output

## 1. Purpose

Today `praxis critique` emits a flat report: one `[major]` finding per critic
message, verbatim text, no structure. That serves neither of Justin's two
needs: (a) **actionable recommended changes** to the document, and (b) a
**human-readable session summary** he can consume at a glance. It also blocks
agent-discoverability, which needs structured (JSON) output.

This design adds a **rich Finding** data model populated by a **dedicated
summarizer LLM call**, and renders it two ways: a human digest (markdown) and
a machine form (JSON). It is the foundation that summary, attribution, and
agent-discoverability compose on top of.

## 2. Key Design Decisions

1. **Freeform critique + separate summarizer call.** Critics stay
   unconstrained (no schema tax on their reasoning → better critique quality).
   A second LLM call reads the whole transcript + subject and structures it
   into findings. Doubles cost/latency per run vs. a single call, but buys
   quality and a clean separation.
2. **Per-issue clustered findings.** The summarizer groups across critics: a
   6-critic run yields ~5–15 findings, not 6×N. This is what a digest wants
   (issues, not per-critic noise) and what an author wants (actionable
   changes).
3. **Full field set.** `severity`, `category`, `summary`, `location?`,
   `quote?`, `suggested_change?`, `supporting_critics: Vec<AgentId>`.
4. **Graceful degradation.** If a finding block is unparseable, the parse
   layer falls back to `{severity: Major, summary: <text>}`. A run never
   fails on a malformed response — only on a failed summarizer *call*
   (network/429 → `SummaryFailed`).
5. **Dual render from one source.** `to_markdown()` (human digest) and
   `to_json()` (machine) both project the same `Vec<Finding>` — single source
   of truth.

## 3. Summarizer Contract

The summarizer is a second HTTP call, reusing `HttpAgent`'s transport and the
block-on-runtime sync/async bridge. Input: the subject text plus the full
transcript serialized as `[critic, kind, text]` turns. Instruction:

> You are summarizing a multi-critic peer review. Group the critiques into
> distinct issues. For each issue, emit a fenced ` ```praxis-finding ` block
> with fields: `severity` (info|minor|major|blocker), `category`, `summary`,
> `location`, `quote`, `suggested_change`, `supporting_critics` (comma-separated
> critic names). Emit one block per issue.

The parser extracts each ` ```praxis-finding ` fence and parses its
field-lines. Unparseable blocks degrade gracefully (§2.4).

### Provider selection

The summarizer uses **one** provider from the authed set — the first resolved
config — to keep the foundation focused. Configurable summarizer-model
selection is a follow-up.

## 4. Data Model

```rust
pub struct Finding {
    severity: Severity,                 // Info | Minor | Major | Blocker
    category: Option<String>,           // e.g. "methodology", "falsifiability"
    summary: String,                    // the issue, one line
    location: Option<String>,           // e.g. "§2", "line 47" (freeform)
    quote: Option<String>,              // the excerpt being critiqued
    suggested_change: Option<String>,   // actionable recommendation
    supporting_critics: Vec<AgentId>,   // who raised/agreed (clustered)
}
```

`Severity` unchanged. `Report::from_transcript` (echo-backend path) stays:
one `Finding` per `Critique`, `Major`, no extra fields — so all existing tests
remain green.

## 5. Dual Render

### `to_markdown()` — human digest
- Header with subject + counts-by-severity (executive summary).
- Findings sorted by severity desc (Blocker first).
- Each finding: `## [severity] category — summary`, then location/quote/
  suggested-change/attribution as applicable.
- This **is** the session summary Justin asked for.

### `to_json()` — machine form (behind existing `json` feature)
- The same `Vec<Finding>` serialized as JSON.
- Foundation for agent-discoverability: an agent parses this instead of
  scraping markdown.

Both consume the same `Vec<Finding>`.

## 6. Crate Shape

- `src/report.rs` — `Finding` extended; markdown renderer rewritten as a
  digest; `to_json()` added.
- `src/summary.rs` (new, behind `backend-http`) — the summarizer: prompt
  rendering, `praxis-finding` block parsing, `summarize(transcript, subject,
  &HttpConfig) -> Result<Vec<Finding>>`. Pure parse logic unit-tested; network
  covered by an `#[ignore]` live test + an example harness
  (`examples/summarize_smoke.rs`).
- `src/cli/critique.rs` — `run_critique` makes the summarizer call after the
  run, builds a rich `Report`, renders markdown (default) or JSON (`--json`).
- New `PraxisError::SummaryFailed { detail }` variant.

## 7. Deliberately Out of Scope (next PRs)

- **Provider attribution in errors** — trivial now that `Finding` carries
  `supporting_critics`, but separate.
- **`--dry-run` / self-describe** — the agent-discoverability cluster.
- **Retry/timeout/backoff** — separate.
- **Multi-model summarizer panel** — v1 uses one provider.

## 8. Implementation Sequencing (TDD)

Each step RED → GREEN → REFACTOR, every public item documented.

1. **`Finding` extension + accessors** — pure; extend existing `report` tests.
2. **`praxis-finding` block parser** — pure; well-formed multi-finding case +
   graceful-degradation cases (missing fields, unparseable block).
3. **Summarizer prompt rendering** — pure; carries subject + transcript turns.
4. **`summarize` orchestration** — live `#[ignore]` test + example harness.
5. **Markdown renderer rewrite** — golden tests for the digest layout.
6. **`to_json` + CLI `--json` flag.**

## 9. Open Questions / Future Work

- **Summarizer model selection** — config-driven (`[summarizer]` section in
  credentials.toml) or `--summarizer` flag.
- **Severity calibration** — different summarizer models may calibrate
  severity differently; may need rubric refinement after seeing real outputs.
- **Multi-model summarizer panel** — vote across summarizers for robustness.
- **Streaming** — emit findings as they parse, for long runs.
