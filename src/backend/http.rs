// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! The OpenAI-compatible HTTP backend.
//!
//! [`HttpAgent`] implements [`crate::agent::Agent`] by calling an
//! OpenAI-compatible chat-completions endpoint (DeepSeek, OpenAI, OpenRouter,
//! any compatible provider). It is the first real LLM backend; the engine and
//! the echo backend stay fully functional and testable without it.
//!
//! ## Sync/async bridge
//!
//! The engine is synchronous ([`crate::agent::Agent::respond`] takes `&mut
//! self` and returns synchronously), but HTTP is inherently async. `HttpAgent`
//! holds its own Tokio runtime and calls [`tokio::runtime::Runtime::block_on`]
//! internally, so `respond` stays synchronous — zero churn to the engine,
//! runner, CLI, or the 45+ tests that depend on the sync trait. The async
//! burden is contained entirely inside this feature-gated module.
//!
//! ## Testability
//!
//! The network round-trip is the only thing that can't be unit-tested. The
//! pure logic that determines `respond`'s behavior — [`render_prompt`] (turn
//! the persona + incoming message into a chat conversation), request building,
//! and [`parse_completion_response`] (turn the API reply into a
//! [`crate::message::Message`]) — is exposed and unit-tested directly.

use crate::agent::{Agent, AgentId};
use crate::message::{Message, MessageKind};
use crate::persona::Persona;

/// Retry / timeout / backoff policy for HTTP calls.
///
/// Carried by [`HttpAgent`] and passed to the summarizer. [`RetryPolicy::DEFAULT`]
/// is sensible for production; [`RetryPolicy::NONE`] disables retry (one attempt).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetryPolicy {
    /// Total tries including the first. `1` = no retry.
    pub max_attempts: u32,
    /// Backoff before the second attempt, in milliseconds.
    pub initial_backoff_ms: u64,
    /// Exponential growth factor between backoffs.
    pub backoff_factor: f64,
    /// Cap on any single backoff, in milliseconds.
    pub max_backoff_ms: u64,
    /// Per-attempt socket+read timeout, in seconds.
    pub timeout_secs: u64,
}

impl RetryPolicy {
    /// Sensible production defaults: 3 attempts, 500ms initial backoff, 2x
    /// exponential capped at 8s, 60s per-attempt timeout.
    pub const DEFAULT: Self = Self {
        max_attempts: 3,
        initial_backoff_ms: 500,
        backoff_factor: 2.0,
        max_backoff_ms: 8000,
        timeout_secs: 60,
    };

    /// No retry: a single attempt. For tests and dry-run semantics.
    pub const NONE: Self = Self {
        max_attempts: 1,
        ..Self::DEFAULT
    };

    /// Sets `max_attempts`.
    #[must_use]
    pub const fn with_max_attempts(mut self, n: u32) -> Self {
        self.max_attempts = n;
        self
    }

    /// Sets `timeout_secs`.
    #[must_use]
    pub const fn with_timeout_secs(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Sets `initial_backoff_ms`.
    #[must_use]
    pub const fn with_initial_backoff_ms(mut self, ms: u64) -> Self {
        self.initial_backoff_ms = ms;
        self
    }

    /// Sets `backoff_factor`.
    #[must_use]
    pub const fn with_backoff_factor(mut self, f: f64) -> Self {
        self.backoff_factor = f;
        self
    }

    /// Sets `max_backoff_ms`.
    #[must_use]
    pub const fn with_max_backoff_ms(mut self, ms: u64) -> Self {
        self.max_backoff_ms = ms;
        self
    }

    /// Resolves the effective policy from precedence: CLI flags > `[retry]`
    /// config > [`RetryPolicy::DEFAULT`].
    ///
    /// `cli_max_attempts` / `cli_timeout_secs` are `Some` only when the user
    /// passed `--max-attempts` / `--timeout`. `retry_config` is the parsed
    /// `[retry]` section (all-`None` if absent).
    pub fn resolve(
        retry_config: &crate::backend::credentials::RetryConfig,
        cli_max_attempts: Option<u32>,
        cli_timeout_secs: Option<u64>,
    ) -> Self {
        let mut p = Self::DEFAULT;
        if let Some(n) = retry_config.max_attempts {
            p = p.with_max_attempts(n);
        }
        if let Some(secs) = retry_config.timeout_secs {
            p = p.with_timeout_secs(secs);
        }
        if let Some(ms) = retry_config.initial_backoff_ms {
            p = p.with_initial_backoff_ms(ms);
        }
        if let Some(f) = retry_config.backoff_factor {
            p = p.with_backoff_factor(f);
        }
        if let Some(ms) = retry_config.max_backoff_ms {
            p = p.with_max_backoff_ms(ms);
        }
        // CLI overrides win.
        if let Some(n) = cli_max_attempts {
            p = p.with_max_attempts(n);
        }
        if let Some(secs) = cli_timeout_secs {
            p = p.with_timeout_secs(secs);
        }
        p
    }
}

/// Whether an HTTP status code is worth retrying.
///
/// Retries: 408 (request timeout), 429 (rate limit), and all 5xx (server
/// errors). Does NOT retry 2xx success or other 4xx (caller bugs — retrying
/// won't fix a 400/401/403/404).
pub fn should_retry_status(status: u16) -> bool {
    status == 408 || status == 429 || (500..=599).contains(&status)
}

/// Computes the backoff delay before the next attempt.
///
/// Exponential: `initial * factor^(attempt-1)`, capped at `max_backoff_ms`.
/// Plus up to `jitter_ms` of random jitter (0 for a deterministic, jitter-free
/// delay — useful in tests). `attempt` is 1-based: the delay before attempt 2
/// uses `attempt == 1`.
pub fn backoff_delay(policy: &RetryPolicy, attempt: u32, jitter_ms: u64) -> std::time::Duration {
    let exp = (attempt as u64).saturating_sub(1);
    let raw = (policy.initial_backoff_ms as f64) * policy.backoff_factor.powi(exp as i32);
    let capped = raw.min(policy.max_backoff_ms as f64) as u64;
    let jitter = if jitter_ms == 0 {
        0
    } else {
        use rand::Rng;
        rand::rng().random_range(0..jitter_ms)
    };
    std::time::Duration::from_millis(capped + jitter)
}

/// Jitter envelope added on top of each computed backoff (up to this many ms).
const BACKOFF_JITTER_MS: u64 = 250;

/// Sends a chat-completion POST with timeout + retry + backoff.
///
/// Retries transient failures ([`should_retry_status`]: 408/429/5xx) and
/// network/timeout errors, sleeping [`backoff_delay`] between attempts.
/// Non-retryable statuses surface immediately. On exhaustion the last error
/// is returned. `label` is used in the per-retry stderr log line.
///
/// The caller is responsible for setting the per-attempt timeout on `client`
/// (via `reqwest::ClientBuilder::timeout`); this function does not override it.
pub async fn send_chat_completion(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    body: &serde_json::Value,
    policy: &RetryPolicy,
    label: &str,
) -> Result<String, crate::PraxisError> {
    let max = policy.max_attempts;
    let mut last_err: Option<crate::PraxisError> = None;

    for attempt in 1..=max {
        let result = client
            .post(url)
            .bearer_auth(api_key)
            .json(body)
            .send()
            .await;

        match result {
            Ok(resp) => {
                let status = resp.status();
                let status_code = status.as_u16();
                let text = resp.text().await.map_err(|e| {
                    crate::PraxisError::agent_failure(label, format!("HTTP body: {e}"))
                })?;
                if status.is_success() {
                    return Ok(text);
                }
                let err =
                    crate::PraxisError::agent_failure(label, format!("HTTP {status_code}: {text}"));
                if !should_retry_status(status_code) || attempt == max {
                    return Err(err);
                }
                last_err = Some(err);
            }
            Err(e) => {
                let err = crate::PraxisError::agent_failure(label, format!("HTTP send: {e}"));
                if attempt == max {
                    return Err(err);
                }
                // Network errors are always retryable (no status to check).
                last_err = Some(err);
            }
        }

        // Sleep + retry. (Only reached when we will retry.)
        let delay = backoff_delay(policy, attempt, BACKOFF_JITTER_MS);
        eprintln!(
            "praxis: {label} attempt {attempt}/{max} failed, retrying in {}ms",
            delay.as_millis()
        );
        tokio::time::sleep(delay).await;
    }

    Err(last_err.unwrap_or_else(|| {
        crate::PraxisError::agent_failure(label, "retry loop exited without an error")
    }))
}

/// A single chat message in the OpenAI-compatible conversation format.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    /// `system`, `user`, or `assistant`.
    pub role: String,
    /// The message content.
    pub content: String,
}

/// The kind label expected in the model's reply, given the incoming message.
///
/// A `Prompt` should be answered with a `Critique`; a `Critique` with a
/// `Rebuttal` (matching the echo backend's adversarial contract); anything
/// else is mirrored. This keeps HTTP-driven cross-examination consistent with
/// the echo backend and the rounds topology.
fn expected_reply_kind(incoming: MessageKind) -> MessageKind {
    match incoming {
        MessageKind::Prompt => MessageKind::Critique,
        MessageKind::Critique => MessageKind::Rebuttal,
        other => other,
    }
}

/// Renders the chat conversation for a single `respond` call.
///
/// Emits a system message describing the persona (name, framing, focus),
/// followed by a user turn carrying the incoming message — its kind, sender,
/// and text — and an instruction to reply with the expected kind so the
/// backend can map the response back to a [`MessageKind`].
pub fn render_prompt(persona: &Persona, incoming: &Message) -> Vec<ChatMessage> {
    let mut system = format!(
        "You are {}, a critic on a peer-review panel.",
        persona.name()
    );
    if let Some(framing) = persona.framing() {
        system.push_str(&format!(" Framing: {framing}."));
    }
    if let Some(focus) = persona.focus() {
        system.push_str(&format!(" Focus: {focus}."));
    }

    let reply_kind = expected_reply_kind(incoming.kind());
    let user = format!(
        "From {}: [{}] {}\n\nRespond with kind: {}.",
        incoming.sender(),
        incoming.kind().label(),
        incoming.text(),
        reply_kind.label(),
    );

    vec![
        ChatMessage {
            role: "system".to_owned(),
            content: system,
        },
        ChatMessage {
            role: "user".to_owned(),
            content: user,
        },
    ]
}

/// Parses an OpenAI-compatible chat-completions response body into a
/// [`Message`] authored by `author`, with the given `kind` (the kind the
/// backend decided the reply should have — see [`expected_reply_kind`]).
///
/// # Errors
///
/// Returns [`crate::error::PraxisError::AgentFailure`] if the body is
/// malformed or has no choices.
pub fn parse_completion_response(
    body: &str,
    author: AgentId,
    kind: MessageKind,
) -> Result<Message, crate::error::PraxisError> {
    let parsed: serde_json::Value = serde_json::from_str(body).map_err(|e| {
        crate::PraxisError::agent_failure(author.as_str(), format!("invalid JSON response: {e}"))
    })?;

    let content = parsed
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or_else(|| {
            crate::PraxisError::agent_failure(
                author.as_str(),
                "response had no choices[0].message.content",
            )
        })?;

    Ok(Message::new(author, None, kind, content.to_owned()))
}

/// Configuration for an [`HttpAgent`]: where to call and how to authenticate.
///
/// Works with any OpenAI-compatible chat-completions endpoint.
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// The base URL of the API (without `/chat/completions`). For DeepSeek:
    /// `https://api.deepseek.com/v1`.
    pub base_url: String,
    /// The model to request, e.g. `deepseek-chat`, `gpt-4o-mini`.
    pub model: String,
    /// The API key. Read from the environment (e.g. `DEEPSEEK_API_KEY`) at
    /// call sites, not hard-coded.
    pub api_key: String,
}

/// Builds the JSON body for a chat-completions request.
fn build_request_body(model: &str, messages: &[ChatMessage]) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "messages": messages,
    })
}

/// An [`Agent`] backed by an OpenAI-compatible HTTP endpoint.
///
/// Construct with [`HttpAgent::new`] from an [`AgentId`], a [`Persona`], and
/// an [`HttpConfig`]. Each [`Agent::respond`] renders the persona + incoming
/// message into a chat conversation, POSTs it to the completions endpoint,
/// and parses the reply into a [`Message`] whose kind follows the adversarial
/// contract (Critique for a Prompt, Rebuttal for a Critique, else mirrored).
pub struct HttpAgent {
    id: AgentId,
    persona: Persona,
    config: HttpConfig,
    runtime: tokio::runtime::Runtime,
    client: reqwest::Client,
    policy: RetryPolicy,
}

impl HttpAgent {
    /// Creates a new HTTP agent with the default retry policy.
    ///
    /// Spawns a dedicated Tokio runtime (used to bridge the synchronous
    /// [`Agent::respond`] to async HTTP) and a reusable reqwest client whose
    /// timeout matches [`RetryPolicy::DEFAULT`].
    pub fn new(id: AgentId, persona: Persona, config: HttpConfig) -> Self {
        Self::new_with_policy(id, persona, config, RetryPolicy::DEFAULT)
    }

    /// Creates a new HTTP agent with an explicit retry policy. The per-attempt
    /// timeout is set on the reqwest client from the policy.
    pub fn new_with_policy(
        id: AgentId,
        persona: Persona,
        config: HttpConfig,
        policy: RetryPolicy,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(policy.timeout_secs))
            .build()
            .expect("praxis: failed to build reqwest client for HttpAgent");
        Self {
            id,
            persona,
            config,
            runtime: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("praxis: failed to build tokio runtime for HttpAgent"),
            client,
            policy,
        }
    }

    /// The label used in failure messages: persona (model), so a multi-provider
    /// run can tell which provider died.
    fn failure_label(&self) -> String {
        format!("{} ({})", self.id.as_str(), self.config.model)
    }

    /// The async inner: render, POST (with retry/backoff), return the raw
    /// response body. Delegates to [`send_chat_completion`].
    async fn fetch_response(&self, incoming: &Message) -> Result<String, crate::PraxisError> {
        let messages = render_prompt(&self.persona, incoming);
        let body = build_request_body(&self.config.model, &messages);
        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );
        let label = self.failure_label();
        send_chat_completion(
            &self.client,
            &url,
            &self.config.api_key,
            &body,
            &self.policy,
            &label,
        )
        .await
    }
}

impl Agent for HttpAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn persona(&self) -> &Persona {
        &self.persona
    }

    fn respond(&mut self, incoming: &Message) -> Result<Message, crate::PraxisError> {
        // Bridge sync -> async: block on this agent's runtime.
        let body = self.runtime.block_on(self.fetch_response(incoming))?;
        let kind = expected_reply_kind(incoming.kind());
        let author = self.id.clone();
        let mut reply = parse_completion_response(&body, author, kind)?;
        // Address the reply to whoever prompted this agent, matching the echo
        // contract (None for broadcasts stays None).
        reply = Message::new(
            self.id.clone(),
            Some(incoming.sender().clone()),
            reply.kind(),
            reply.text().to_owned(),
        );
        Ok(reply)
    }
}
