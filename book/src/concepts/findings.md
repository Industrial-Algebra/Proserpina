# The Summarizer and Findings

Critics stay **freeform** — no schema tax on their reasoning, which produces
better critiques. After the run, a **dedicated summarizer LLM call** reads the
whole transcript plus the subject and structures it into per-issue findings.

## Per-issue clustering

The summarizer groups critiques *across critics*: a 5-critic run might yield
~5–15 findings, not 5×N. When several critics raise the same issue, the
summarizer merges them and records all the critics in `supporting_critics` —
so you can see where the panel *agreed*.

## The Finding model

Each finding carries the full rich field set:

| Field | Meaning |
|---|---|
| `severity` | Info / Minor / Major / Blocker |
| `category` | a short label, e.g. "methodology" |
| `summary` | the issue, one line |
| `location` | where in the document (e.g. `§2`, `line 47`) |
| `quote` | the excerpt being critiqued |
| `suggested_change` | an actionable recommended change |
| `supporting_critics` | the critics that raised or agreed |

## Graceful degradation

The summarizer-parse layer never fails a run on a malformed response: an
unrecognized severity defaults to `Major`, an unparseable block is skipped, a
response with no blocks yields an empty finding list. Only a failed summarizer
*call* (network/HTTP) surfaces an error (`SummaryFailed`, exit code 12).

## Dual render

Both renders project the same `Vec<Finding>`:

- **`to_markdown()`** — the human digest: executive summary (counts by
  severity), findings sorted by severity desc, each with its fields.
- **`to_json()`** — machine-readable JSON (behind the `json` feature), for
  piping into other tools or agent consumption.

See [Agent Integration](../guide/agent-integration.md) for the `--json` flow.
