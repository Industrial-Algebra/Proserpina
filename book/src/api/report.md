# Findings and Reports: `Finding`, `Report`, `Severity`

```rust
pub enum Severity { Info, Minor, Major, Blocker }   // Ord: Blocker > ... > Info

pub struct Finding {
    pub severity: Severity,
    pub category: Option<String>,
    pub summary: String,
    pub location: Option<String>,
    pub quote: Option<String>,
    pub suggested_change: Option<String>,
    /* supporting_critics: Vec<AgentId> */
}

pub struct Report { /* Vec<Finding> */ }

impl Report {
    pub fn new() -> Self;
    pub fn push_finding(&mut self, finding: Finding);
    pub fn from_transcript(t: &Transcript) -> Self;   // echo path: 1 Finding/Critique
    pub fn findings(&self) -> &[Finding];
    pub fn to_markdown(&self) -> String;
    pub fn to_markdown_with_source(&self, source: Option<&str>) -> String;
    #[cfg(feature = "json")]
    pub fn to_json(&self) -> String;
}
```

A `Finding` is one distinct issue, produced either by the simple
`from_transcript` fold (echo path) or by the [summarizer](./summarizer.md)
(clustered across critics — hence `supporting_critics`).

`Report` renders two ways from the same `Vec<Finding>`: a human-readable
markdown digest (executive summary + findings sorted by severity desc, each
with its fields) and machine-readable JSON (behind `json`). See
[The Summarizer and Findings](../concepts/findings.md).
