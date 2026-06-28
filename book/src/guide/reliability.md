# Retry, Timeout, Backoff

Every HTTP call — each critic *and* the summarizer — runs with a per-attempt
timeout and retries transient failures with exponential + jittered backoff.

## What gets retried

- **Retried:** HTTP 408 (request timeout), 429 (rate limit), all 5xx (server
  errors), and network/timeout errors.
- **Not retried:** other 4xx (400/401/403/404 — caller bugs; retrying won't
  fix them).

So a rate-limited 429 retries (correct), while a 401 fails fast.

## Defaults

| Knob | Default |
|---|---|
| `max_attempts` | 3 (total tries including the first) |
| `initial_backoff_ms` | 500 |
| `backoff_factor` | 2.0 (exponential) |
| `max_backoff_ms` | 8000 (cap) |
| `timeout_secs` | 60 (per-attempt socket+read) |

Backoff is `min(initial * factor^(n-1), max) + jitter`, so a retrying run
doesn't hammer a shared rate limit (thundering-herd avoidance).

## Configuring

**Config file** (`[retry]` in credentials.toml):

```toml
[retry]
max_attempts = 5
timeout_secs = 120
```

**CLI** (overrides config; config overrides default):

```bash
proserpina critique doc.md --max-attempts 5 --timeout 120
```

## Observability

Each retry logs to stderr so a retrying run isn't silent:

```
proserpina: Devil's Advocate (glm-5.2) attempt 1/3 failed, retrying in 532ms
```

The final failure (if all attempts exhaust) goes through the normal error path
— `AgentFailure` for critics (exit 11) or `SummaryFailed` for the summarizer
(exit 12), with `--json`-gated structured JSON when applicable.
