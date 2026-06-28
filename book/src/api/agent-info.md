# Agent Integration: `Capabilities`, `Plan`, Exit Codes

```rust
pub struct ProviderInfo { pub name: String, pub model: String, pub authed: bool }
pub struct Capabilities { /* version, subcommands, output_formats, topologies,
                             providers, personas, panels, exit_codes */ }
impl Capabilities {
    pub fn static_info() -> Self;             // registry + fixed metadata
    pub fn with_current_auth() -> Self;       // + dynamic authed state from env/config
}

pub struct PlanSlot { pub persona: String, pub provider: String, pub model: String }
pub struct Plan { /* seed, topology, roster, n_critic_calls, n_summarizer_calls,
                     estimated_total_calls */ }
impl Plan {
    pub fn for_parallel(personas: &[Persona], configs: &[HttpConfig], seed: u64) -> Self;
}
```

These types back the agent-discoverability surface:

- `proserpina capabilities` emits `Capabilities::with_current_auth()` as JSON. The
  `providers[].authed` field is **dynamic** — it reflects the real config +
  env, so an agent learns what's runnable *in this environment*.
- `proserpina critique --dry-run` emits a `Plan` — the resolved roster + call
  counts, with **zero API calls**.

## Exit codes

`ProserpinaError::exit_code()` returns the scheme reported in `capabilities`:
`0` success, `2` usage, `10` no authed providers, `11` agent failure,
`12` summary failed, `13` malformed credentials, `14` incomplete custom
provider, `15` missing agent, `16` unknown panel, `70` other.

See [Agent Integration](../guide/agent-integration.md) for the full loop.
