# Backends: `EchoAgent`, `HttpAgent`, `HttpConfig`

## `EchoAgent` — the deterministic reference backend

```rust
pub struct EchoAgent { /* id, persona */ }
impl EchoAgent {
    pub fn new(id: AgentId, persona: Persona) -> Self;
}
```

`EchoAgent` models a deterministic adversarial critic: a `Prompt` elicits a
`Critique`; a `Critique` (another critic's finding) elicits a `Rebuttal`; any
other kind is mirrored. Output is fully determined by `(persona, input)` — no
external state — which makes it the reference backend for testing the engine
end-to-end with zero LLM dependencies.

## `HttpAgent` — the OpenAI-compatible HTTP backend

```rust
pub struct HttpConfig {
    pub base_url: String,   // without /chat/completions
    pub model: String,
    pub api_key: String,
}

pub struct HttpAgent { /* ... */ }
impl HttpAgent {
    pub fn new(id: AgentId, persona: Persona, config: HttpConfig) -> Self;
    pub fn new_with_policy(id, persona, config, policy: RetryPolicy) -> Self;
}
```

`HttpAgent` calls an OpenAI-compatible chat-completions endpoint (DeepSeek,
Z.ai GLM, OpenAI, Moonshot, Alibaba, Google, Ollama, any compatible). Reply
kind follows the adversarial contract (`Critique` for `Prompt`, `Rebuttal` for
`Critique`, else mirrored), addressed to the sender.

The sync/async bridge: `HttpAgent` holds a dedicated Tokio runtime and calls
`block_on` internally; `respond` stays synchronous.

## `RetryPolicy`

```rust
pub struct RetryPolicy {
    pub max_attempts: u32, pub initial_backoff_ms: u64, pub backoff_factor: f64,
    pub max_backoff_ms: u64, pub timeout_secs: u64,
}
impl RetryPolicy {
    pub const DEFAULT: Self;   // 3 / 500ms / 2x / 8s cap / 60s
    pub const NONE: Self;      // single attempt
    pub fn resolve(cfg: &RetryConfig, cli_max_attempts: Option<u32>,
                   cli_timeout_secs: Option<u64>) -> Self;
}
```

See [Retry, Timeout, Backoff](../guide/reliability.md).
