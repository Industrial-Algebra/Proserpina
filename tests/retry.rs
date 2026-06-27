// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for retry / timeout / backoff.

#![cfg(feature = "backend-http")]

use std::sync::{Arc, Mutex};

use praxis::backend::http::RetryPolicy;

#[test]
fn default_policy_has_sensible_values() {
    let p = RetryPolicy::DEFAULT;
    assert_eq!(p.max_attempts, 3);
    assert_eq!(p.initial_backoff_ms, 500);
    assert_eq!(p.backoff_factor, 2.0);
    assert_eq!(p.max_backoff_ms, 8000);
    assert_eq!(p.timeout_secs, 60);
}

#[test]
fn none_policy_is_single_attempt_no_retry() {
    let p = RetryPolicy::NONE;
    assert_eq!(p.max_attempts, 1);
}

#[test]
fn policy_builder_overrides_fields() {
    let p = RetryPolicy::DEFAULT
        .with_max_attempts(5)
        .with_timeout_secs(120)
        .with_initial_backoff_ms(250);
    assert_eq!(p.max_attempts, 5);
    assert_eq!(p.timeout_secs, 120);
    assert_eq!(p.initial_backoff_ms, 250);
    // Unchanged fields keep the default.
    assert_eq!(p.backoff_factor, 2.0);
    assert_eq!(p.max_backoff_ms, 8000);
}

// ---- should_retry ----

#[test]
fn should_retry_returns_true_for_transient_statuses() {
    use praxis::backend::http::should_retry_status;
    // Rate-limit, request-timeout, and all server errors retry.
    assert!(should_retry_status(429));
    assert!(should_retry_status(408));
    assert!(should_retry_status(500));
    assert!(should_retry_status(502));
    assert!(should_retry_status(503));
    assert!(should_retry_status(504));
}

#[test]
fn should_retry_returns_false_for_non_transient_statuses() {
    use praxis::backend::http::should_retry_status;
    // Caller bugs and redirects are NOT retried — retrying won't fix them.
    assert!(!should_retry_status(200));
    assert!(!should_retry_status(400));
    assert!(!should_retry_status(401));
    assert!(!should_retry_status(403));
    assert!(!should_retry_status(404));
    assert!(!should_retry_status(422));
}

// ---- backoff_delay ----

#[test]
fn backoff_delay_grows_exponentially_and_caps() {
    use praxis::backend::http::backoff_delay;
    use std::time::Duration;
    let p = RetryPolicy::DEFAULT;
    // attempt 1 -> delay before attempt 2 = initial = 500ms.
    assert_eq!(backoff_delay(&p, 1, 0), Duration::from_millis(500));
    // attempt 2 -> before attempt 3 = 500 * 2 = 1000ms.
    assert_eq!(backoff_delay(&p, 2, 0), Duration::from_millis(1000));
    // attempt 3 -> 2000ms; the policy caps at 8000.
    assert_eq!(backoff_delay(&p, 3, 0), Duration::from_millis(2000));
    // attempt 7 -> 500 * 2^6 = 32000, capped at 8000.
    assert_eq!(backoff_delay(&p, 7, 0), Duration::from_millis(8000));
}

#[test]
fn backoff_delay_adds_jitter_within_bounds() {
    use praxis::backend::http::backoff_delay;
    let p = RetryPolicy::DEFAULT;
    // With jitter 100ms, the delay must be in [base, base + 100].
    for _ in 0..50 {
        let d = backoff_delay(&p, 1, 100);
        let ms = d.as_millis();
        assert!((500..=600).contains(&ms), "delay {ms}ms out of [500, 600]");
    }
}

// ---- send_chat_completion retry loop (local scripted server) ----
//
// A minimal HTTP/1.1 server that returns a scripted sequence of status codes,
// one per inbound request, then closes the connection. Lets us assert the
// retry loop's behavior deterministically without a real provider.

use praxis::backend::http::send_chat_completion;

/// Spawns a server returning `statuses` in order (one per request), then a
/// final 200 if the sequence is exhausted. Returns the base URL and a handle
/// counting how many requests were received.
fn scripted_server(statuses: Vec<u16>) -> (String, Arc<Mutex<u32>>) {
    use tokio::net::TcpListener;
    let count = Arc::new(Mutex::new(0u32));
    let count_clone = count.clone();
    let statuses: Arc<Vec<u16>> = Arc::new(statuses);

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test rt");
    let listener = rt
        .block_on(async { TcpListener::bind("127.0.0.1:0").await })
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let url = format!("http://{addr}/chat/completions");

    std::thread::spawn(move || {
        rt.block_on(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut idx = 0usize;
            loop {
                let (mut sock, _) = match listener.accept().await {
                    Ok(p) => p,
                    Err(_) => break,
                };
                // Read and discard the request (until blank line).
                let mut buf = vec![0u8; 1024];
                let _ = tokio::time::timeout(
                    std::time::Duration::from_millis(200),
                    sock.read(&mut buf),
                )
                .await;
                let status = if idx < statuses.len() {
                    let s = statuses[idx];
                    idx += 1;
                    s
                } else {
                    200
                };
                let body = if status == 200 {
                    r#"{"choices":[{"message":{"role":"assistant","content":"ok"}}]}"#
                } else {
                    "{}"
                };
                let resp = format!(
                    "HTTP/1.1 {status} REASON\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.flush().await;
                let _ = sock.shutdown().await;
                *count_clone.lock().unwrap() += 1;
            }
        });
    });

    (url, count)
}

#[test]
fn send_retries_transient_then_succeeds() {
    // Server returns 429, then 429, then 200. With DEFAULT (3 attempts) the
    // call should succeed on the 3rd attempt and have made 3 requests.
    let (url, count) = scripted_server(vec![429, 429]);

    // Use a policy with tiny backoffs so the test is fast.
    let policy = RetryPolicy::DEFAULT
        .with_initial_backoff_ms(1)
        .with_max_backoff_ms(2);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("rt");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("client");
    let body = serde_json::json!({"model":"x","messages":[]});
    let result = rt.block_on(send_chat_completion(
        &client,
        &url,
        "dummy",
        &body,
        &policy,
        "test-agent",
    ));
    assert!(
        result.is_ok(),
        "should succeed after retries: {:?}",
        result.err()
    );
    // 2 scripted failures + 1 success = 3 requests.
    assert_eq!(*count.lock().unwrap(), 3);
}

#[test]
fn send_with_none_policy_does_not_retry() {
    // Server would succeed on the 2nd request, but NONE means one attempt:
    // the first (429) is the final result, and only 1 request is made.
    let (url, count) = scripted_server(vec![429, 200]);
    let policy = RetryPolicy::NONE;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("rt");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("client");
    let body = serde_json::json!({"model":"x","messages":[]});
    let result = rt.block_on(send_chat_completion(
        &client,
        &url,
        "dummy",
        &body,
        &policy,
        "test-agent",
    ));
    assert!(result.is_err(), "NONE should surface the 429, not retry");
    assert_eq!(*count.lock().unwrap(), 1);
}

#[test]
fn send_does_not_retry_non_transient_status() {
    // A 401 is not retryable: the call fails immediately with one request.
    let (url, count) = scripted_server(vec![401]);
    let policy = RetryPolicy::DEFAULT.with_initial_backoff_ms(1);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("rt");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("client");
    let body = serde_json::json!({"model":"x","messages":[]});
    let result = rt.block_on(send_chat_completion(
        &client,
        &url,
        "dummy",
        &body,
        &policy,
        "test-agent",
    ));
    assert!(result.is_err());
    assert_eq!(*count.lock().unwrap(), 1, "401 should not retry");
}

// ---- RetryPolicy::resolve (precedence: CLI > config > default) ----

use praxis::backend::credentials::RetryConfig;

#[test]
fn resolve_uses_defaults_when_nothing_specified() {
    let cfg = RetryConfig::default();
    let p = RetryPolicy::resolve(&cfg, None, None);
    assert_eq!(p, RetryPolicy::DEFAULT);
}

#[test]
fn resolve_applies_config_overrides() {
    let cfg = RetryConfig {
        max_attempts: Some(7),
        timeout_secs: Some(99),
        ..Default::default()
    };
    let p = RetryPolicy::resolve(&cfg, None, None);
    assert_eq!(p.max_attempts, 7);
    assert_eq!(p.timeout_secs, 99);
    // Unspecified fields keep the default.
    assert_eq!(
        p.initial_backoff_ms,
        RetryPolicy::DEFAULT.initial_backoff_ms
    );
}

#[test]
fn resolve_cli_overrides_config() {
    // Config says 5 attempts; CLI says 2 — CLI wins.
    let cfg = RetryConfig {
        max_attempts: Some(5),
        ..Default::default()
    };
    let p = RetryPolicy::resolve(&cfg, Some(2), None);
    assert_eq!(p.max_attempts, 2);
}

#[test]
fn resolve_reads_retry_from_credentials_toml() {
    use praxis::backend::credentials::Credentials;
    let toml = r#"
[retry]
max_attempts = 4
timeout_secs = 30
"#;
    let creds = Credentials::from_toml(toml).expect("valid toml");
    let p = RetryPolicy::resolve(creds.retry(), None, None);
    assert_eq!(p.max_attempts, 4);
    assert_eq!(p.timeout_secs, 30);
}
