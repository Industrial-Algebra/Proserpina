# API Reference

Proserpina's public API, grouped by concern. All items are documented at the
[rustdoc](https://docs.rs/proserpina) level; this section is a navigable map of
the surface.

- [The provider boundary: `Agent`, `AgentId`](./agent.md)
- [The document and the transcript: `Subject`, `Transcript`](./subject.md)
- [Messages: `Message`, `MessageKind`](./message.md)
- [The interaction graph: `InteractionGraph`, `Topology`](./graph.md)
- [Executing a run: `Runner`](./runner.md)
- [Critics and panels: `Persona`, `Panel`](./persona.md)
- [Findings and reports: `Finding`, `Report`, `Severity`](./report.md)
- [Backends: `EchoAgent`, `HttpAgent`, `HttpConfig`](./backends.md)
- [The multi-provider roster: `Provider`, `random_roster`](./roster.md)
- [Credentials config: `Credentials`, `resolve_configs`](./credentials.md)
- [The summarizer: `summarize`, `parse_findings`](./summarizer.md)
- [Agent integration: `Capabilities`, `Plan`, exit codes](./agent-info.md)
- [Errors: `ProserpinaError`](./errors.md)
