# Credentials Config: `Credentials`, `resolve_configs`

```rust
pub struct ProviderOverride { pub api_key: Option<String>, pub model: Option<String>, pub base_url: Option<String> }
pub struct PanelConfig { pub personas: Vec<PersonaSpec> }
pub struct RetryConfig { /* Option fields for each RetryPolicy knob */ }

pub struct Credentials { /* providers, panels, retry */ }

impl Credentials {
    pub fn from_toml(toml: &str) -> Result<Self, ProserpinaError>;
    pub fn from_path(path: &Path) -> Result<Self, ProserpinaError>;
    pub fn discover() -> Result<Self, ProserpinaError>;          // PROSERPINA_CONFIG > XDG > ~/.config; missing = empty
    pub fn discover_or(path: Option<&Path>) -> Result<Self, ProserpinaError>;
    pub fn override_for(&self, name: &str) -> Option<&ProviderOverride>;
    pub fn panels(&self) -> &HashMap<String, PanelConfig>;
    pub fn retry(&self) -> &RetryConfig;
}

pub fn resolve_configs(
    registry: &[Provider],
    credentials: &Credentials,
    env_keys: &HashMap<String, String>,
) -> Result<Vec<HttpConfig>, ProserpinaError>;

pub fn authed_configs() -> Result<Vec<HttpConfig>, ProserpinaError>;
pub fn authed_configs_with(path: Option<&Path>) -> Result<Vec<HttpConfig>, ProserpinaError>;
```

The standalone credentials config (`~/.config/proserpina/credentials.toml`) maps
provider names to keys + optional model/base_url overrides, plus `[panels.NAME]`
sections and a `[retry]` section. `resolve_configs` is the **pure** resolution
core — precedence env > config > registry-default for `api_key`, config >
registry for `model`/`base_url`; custom providers (not in the registry) must
supply all three or error [`IncompleteCustomProvider`](./errors.md). The env
is passed as an explicit snapshot so resolution is fully unit-testable.

See [Providers and Credentials](../guide/providers.md) for the file format.
