# The Multi-Provider Roster: `Provider`, `random_roster`

```rust
pub struct Provider { /* name, base_url, model, key_env_var */ }

impl Provider {
    pub fn new(name: impl Into<String>) -> Self;
    pub fn with_base_url/self.with_model/self.with_key_env_var(...) -> Self;
    pub fn name/base_url/model/key_env_var(&self) -> &str;
    pub fn config_from_env(&self) -> Option<HttpConfig>;   // None if key unset
    pub fn registry() -> &'static [Provider];              // the 6 presets
}

pub fn random_roster(
    personas: &[Persona],
    configs: &[HttpConfig],
    rng: &mut impl rand::Rng,
) -> Vec<(Persona, HttpConfig)>;

pub fn roster_from_env(
    personas: &[Persona],
    providers: &[Provider],
    seed: u64,
) -> Result<Vec<(Persona, HttpConfig)>, ProserpinaError>;
```

`Provider` is a preset (data, like `Persona`) for an OpenAI-compatible
provider. The six built-in presets live in `registry()`. `random_roster` is
**pure** — deterministic given the RNG state, no env inside — so it's fully
unit-testable. `roster_from_env` is the CLI convenience that composes
`config_from_env` + `random_roster`, erroring
[`NoAuthedProviders`](./errors.md) when zero keys are set.

In practice you'll usually go through the [credentials config](./credentials.md)
(`authed_configs_with`) rather than `roster_from_env` directly, so model
overrides and custom providers are picked up. See
[Providers and Credentials](../guide/providers.md).
