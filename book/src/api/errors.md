# Errors: `PraxisError`

```rust
pub enum PraxisError {
    AgentFailure { agent_id: String, detail: String },   // exit 11
    MissingAgent(AgentId),                               // exit 15
    NoAuthedProviders(Vec<String>),                      // exit 10
    SummaryFailed { detail: String },                    // exit 12
    MalformedCredentials { path: String, detail: String }, // exit 13
    IncompleteCustomProvider { name: String, missing: Vec<&'static str> }, // exit 14
    UnknownPanel { name: String, available: Vec<String> }, // exit 16
}
```

Every fallible public operation returns `Result<_, PraxisError>`. Library code
never panics; all failure modes flow through this enum.

Each variant carries enough context to locate the failure, and the
agent-facing methods make failures machine-consumable:

- `error_kind()` — a stable machine string (`"agent_failure"`, etc.) for
  switching without parsing the human message.
- `exit_code()` — the Praxis-specific exit code (above).
- `to_error_json()` (behind `json`) — `{error: {kind, message, details}}` for
  structured stderr output when `--json` is set.

The `AgentFailure` `agent_id` includes the **provider attribution**
(`Devil's Advocate (glm-5.2)`), so a multi-provider run can tell which provider
died — see [Retry, Timeout, Backoff](../guide/reliability.md).
