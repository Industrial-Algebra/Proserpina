# The Summarizer: `summarize`, `parse_findings`

```rust
pub fn summarize(
    subject: &Subject,
    transcript: &Transcript,
    config: &HttpConfig,
    policy: &RetryPolicy,
) -> Result<Vec<Finding>, PraxisError>;

pub fn parse_findings(body: &str) -> Vec<Finding>;
pub fn render_summary_prompt(subject: &Subject, transcript: &Transcript) -> Vec<SummaryMessage>;
```

After a run, `summarize` makes a second LLM call over the whole transcript +
subject, asking the model to group critiques into distinct issues and emit each
as a fenced ` ```praxis-finding ` block. `parse_findings` extracts those blocks
and parses `key: value` lines into [`Finding`](./report.md)s.

**Graceful degradation:** `parse_findings` never fails on a malformed response
— an unrecognized `severity` defaults to `Major`, a block with no `summary` is
skipped, a response with no blocks yields an empty `Vec`. Only a failed
summarizer *call* (network/HTTP) surfaces [`SummaryFailed`](./errors.md)
(exit 12).

See [The Summarizer and Findings](../concepts/findings.md) for the contract.
