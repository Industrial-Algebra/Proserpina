# Agent Integration

Proserpina is designed to be called on the fly by AI agents across your
environments. The full loop:

| Move | Command |
|---|---|
| What can you do, right now? | `proserpina capabilities` |
| What would this run do / cost? | `proserpina critique doc.md --dry-run --seed N` |
| Do it (structured) | `proserpina critique doc.md --json` |
| What went wrong? | structured JSON on stderr + exit code |

## `proserpina capabilities`

Emits JSON describing Proserpina's surface: version, subcommands, output formats,
topologies, providers (with **dynamic `authed` state**), available panels, and
the exit-code scheme. An agent learns what's runnable *in this environment*
without reading docs.

```bash
proserpina capabilities | jq '.providers[] | select(.authed) | .name'
```

## `--dry-run`

Resolves the roster and emits a plan JSON **without making any API calls**:
seed, topology, the per-slot roster (persona + provider + model), and call
counts. Lets an agent verify intent and estimate cost before spending tokens.

```bash
proserpina critique doc.md --dry-run --seed 3 --panel panel
```

## `--json` and structured errors

`--json` emits the report as structured JSON on stdout. When a run fails *and*
`--json` is set, Proserpina emits structured error JSON on **stderr** and exits
with a Proserpina-specific code:

```json
{"error": {"kind": "agent_failure", "message": "...", "details": {"agent_id": "Devil's Advocate (glm-5.2)", "detail": "HTTP 429 ..."}}}
```

## Exit codes

| Code | Meaning |
|---|---|
| 0 | success |
| 2 | usage error |
| 10 | no authed providers |
| 11 | agent (provider) failure |
| 12 | summarizer failure |
| 13 | malformed credentials |
| 14 | incomplete custom provider |
| 15 | missing agent |
| 16 | unknown panel |
| 70 | other / internal |

The scheme is also reported in `capabilities` (`.exit_codes`), so an agent can
learn it programmatically.
