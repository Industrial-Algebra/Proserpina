# Installing and Configuring

## Install

```bash
cargo install praxis
```

This installs the binary with default features. For the full feature set
(HTTP backend, roster, summarizer, JSON output):

```bash
cargo install praxis --features cli,backend-http,json
```

> **Note:** `cargo install praxis` installs with *default* features only (`std`).
> To actually critique documents with real LLMs, you need `backend-http`. If you
> install with defaults, Praxis falls back to the echo backend with a notice.
> The recommended install is `cargo install praxis --features cli,backend-http,json`.

## Build from source

```bash
git clone https://github.com/Industrial-Algebra/Praxis
cd Praxis
cargo build --features cli,backend-http,json
```

## Verify

```bash
praxis --version
praxis capabilities   # JSON self-description; check the `authed` field
```

## The credentials file

All configuration lives in one TOML file, discovered in order:

1. `--config <path>` CLI flag
2. `$PRAXIS_CONFIG`
3. `$XDG_CONFIG_HOME/praxis/credentials.toml`
4. `~/.config/praxis/credentials.toml`

A missing file is not an error — Praxis proceeds with whatever provider keys
are in the environment. See [Providers and Credentials](./providers.md) for
the format.
