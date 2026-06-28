# Feature Flags

Proserpina uses additive feature gates — features add capability; they never
remove existing API.

| Feature | What it adds |
|---|---|
| `std` (default) | standard library support |
| `cli` | the `proserpina` binary |
| `serde` | `Serialize`/`Deserialize` impls for core types |
| `json` | machine-readable JSON report output (implies `serde`) |
| `backend-http` | the OpenAI-compatible HTTP agent, multi-provider roster, credentials config, summarizer (implies `serde`) |
| `keyring` | OS keychain credential tier (implies `backend-http`); macOS Keychain + Windows Credential Manager, Linux gnome-keyring has a known limitation |

## Recommended install

For real use with LLM providers:

```bash
cargo install proserpina --features cli,backend-http,json
```

## Default build

The default build (`std` only) is **synchronous, key-free, and network-free** —
it pulls in zero HTTP/async/rand/toml deps. The echo backend still works, so
the core engine is fully exercisable without any LLM dependency.
