# Praxis — Retry / Timeout / Backoff Design

- **Date:** 2026-06-23
- **Status:** Approved (design phase; implementation via TDD)
- **Branch:** `feature/retry-timeout-backoff`
- **Depends on:** HTTP backend (#6), roster (#7), credentials (#8), panels (#11),
  Z.ai gateway fix (#12)
- **Motivation:** A single transient provider failure (429/5xx/timeout) kills a
  critic call. In a 5-critic `--panel panel` run, one hiccup loses a finding —
  or, if it's the summarizer, the whole report. Multi-provider runs make retry
  essential.

## 1. Purpose

Add three missing capabilities with one shared implementation:
1. **Timeout** — a hung provider no longer hangs Praxis forever.
2. **Retry** — transient failures (429/408/5xx/network) get retried.
3. **Backoff** — exponential + jitter, so retry doesn't hammer a rate-limited
   provider.

## 2. Key Design Decisions

1. **One shared async helper.** Both HTTP call sites (`HttpAgent::fetch_response`
   and `summarize`) do the same POST. Extract `send_chat_completion` (async,
   in `http.rs`) that owns the timeout + retry + backoff loop. Two callers,
   one implementation, one place to unit-test the loop.
2. **`RetryPolicy` is data** (like `Persona`). `DEFAULT` (sensible), `NONE`
   (single attempt, no retry — for tests / dry-run semantics), plus a builder.
3. **Retry only transient failures.** HTTP 429, 408, 5xx, and
   connection/timeout errors retry. Other 4xx (400/401/403/404) do NOT — a
   caller bug isn't fixed by retrying. The Z.ai 1113 (HTTP 429) retries, which
   is correct.
4. **Exponential backoff with jitter.** `min(initial * factor^(n-1), max) +
   random(0..jitter)` between attempts. Jitter avoids thundering-herd on a
   shared rate limit.
5. **Config knobs shipped now** (per Justin's choice): `[retry]` in
   credentials.toml + `--max-attempts` / `--timeout` CLI flags. Sensible
   defaults baked in; overrides available from day one.
6. **Per-retry stderr logging.** `praxis: {label} attempt {n}/{max} failed
   ({status}), retrying in {ms}ms` so a retrying run isn't silent. Final
   failure still goes through the existing error path (`--json`-gated JSON,
   exit codes).

## 3. Data Model

```rust
pub struct RetryPolicy {
    pub max_attempts: u32,       // total tries incl. first; default 3
    pub initial_backoff_ms: u64, // default 500
    pub backoff_factor: f64,     // default 2.0 (exponential)
    pub max_backoff_ms: u64,     // default 8000 (cap)
    pub timeout_secs: u64,       // per-attempt socket+read; default 60
}
```

- `RetryPolicy::DEFAULT` — the values above.
- `RetryPolicy::NONE` — `{max_attempts: 1, ...}` (one try, no retry).
- Builder: `.with_max_attempts(n)` etc.

### Config (`credentials.toml`)

```toml
[retry]
max_attempts = 5
timeout_secs = 120
# initial_backoff_ms / backoff_factor / max_backoff_ms optional
```

A top-level `[retry]` table. `Credentials` gains a `retry: RetryConfig` field
(parsed from the table, with serde defaults so omitting fields keeps the
default). Resolution precedence: **CLI flag > config > default**.

### CLI

- `--max-attempts <N>` — overrides `max_attempts`.
- `--timeout <SECS>` — overrides `timeout_secs`.
- (Backoff shape stays config/default only — too fiddly for flags.)

## 4. The Helper

```rust
/// Sends a chat-completion request with timeout + retry + backoff.
/// Returns the response body text on success.
async fn send_chat_completion(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    body: &serde_json::Value,
    policy: &RetryPolicy,
    label: &str,          // for the retry log line (e.g. "Devil's Advocate (glm-5.2)")
) -> Result<String, PraxisError>;
```

Loop:
1. For `attempt` in `1..=max_attempts`:
   - POST with the client (timeout set on the client builder, not per-request,
     so all requests including retries respect it).
   - On 2xx → return body.
   - On retryable status / network error → if more attempts remain, log + sleep
     `backoff_delay(policy, attempt)` + continue; else return the error.
   - On non-retryable status → return the error immediately.

`fetch_response` and `summarize` both build their `reqwest::Client` with the
policy's timeout, then call `send_chat_completion`.

## 5. Testability

- `should_retry(status)`, `backoff_delay(policy, attempt)` — pure, fully unit-
  tested (the retry rule, exponential-with-cap, jitter within bounds).
- `send_chat_completion` loop — tested via a tiny local HTTP server
  (`httptest` or hand-rolled `tokio::net::TcpListener`) that returns a scripted
  sequence (e.g. 429, 429, 200) so we assert the loop retries-then-succeeds and
  honors `NONE` (no retry). Network round-trip to a real provider is an
  `#[ignore]` live test.

## 6. Crate Shape

- `src/backend/http.rs` — `RetryPolicy`, `send_chat_completion`; `HttpAgent`
  carries a `RetryPolicy` (default `DEFAULT`), uses the helper.
- `src/summary.rs` — `summarize` takes a `&RetryPolicy`, uses the helper.
- `src/backend/credentials.rs` — `RetryConfig` parse; `Credentials::retry()`.
- `src/cli/critique.rs` — resolve policy from (CLI flags + config + default),
  pass into `HttpAgent`/`summarize`.
- `src/main.rs` — `--max-attempts` / `--timeout` flags.

## 7. Implementation Sequencing (TDD)

Each step RED → GREEN → REFACTOR.

1. **`RetryPolicy` + `DEFAULT`/`NONE` + builder** — pure.
2. **`should_retry(status)`** — 429/408/5xx yes; other 4xx no.
3. **`backoff_delay(policy, attempt)`** — exponential + cap; jitter within bounds.
4. **`send_chat_completion` retry loop** — scripted local server: retries to
   success; honors `NONE`; surfaces last error on exhaustion; skips retry on
   non-retryable.
5. **Wire `fetch_response` + `summarize`** — both use the helper + policy.
6. **`[retry]` config + `--max-attempts`/`--timeout` flags** — resolution
   (CLI > config > default).

## 8. Open Questions / Future Work

- **Circuit breaker per provider** — after N failures in a window, stop
  retrying that provider for the rest of the run (vs. the current per-call
  retry). Defer until we see retry storms in practice.
- **`Retry-After` header** — honor the 429's `Retry-After` when present instead
  of computing backoff. Small enhancement; defer.
- **Per-provider policy overrides** — `[providers.zai] retry = {...}`. Defer.
