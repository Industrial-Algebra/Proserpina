# Getting Started

## Install

```bash
cargo install praxis
```

Or build from source:

```bash
git clone https://github.com/Industrial-Algebra/Praxis
cd Praxis
cargo install --path . --features cli,backend-http,json
```

## Authenticate one provider

The zero-config path: export a DeepSeek key.

```bash
export DEEPSEEK_API_KEY=sk-...
```

(Any of `DEEPSEEK_API_KEY`, `OPENAI_API_KEY`, `MOONSHOT_API_KEY`,
`DASHSCOPE_API_KEY`, `ZAI_API_KEY`, `GOOGLE_API_KEY` works. For providers pi
mediates via OAuth/extensions, put the key in
[~/.config/praxis/credentials.toml](./providers.md) instead.)

## Critique a document

```bash
praxis critique roadmap.md
```

You'll get a markdown digest: an executive summary of findings by severity,
then each finding with its category, location, quote, suggested change, and
the critics that raised it.

## Try a multi-critic panel

```bash
praxis critique roadmap.md --panel panel
```

`--panel panel` runs all five built-in critics (Devil's Advocate,
Methodologist, Red Team, Domain Expert, Editor), fanned across your authed
providers. The summarizer clusters their critiques — you'll see findings
"raised by" multiple critics where the panel converged.

## Reproduce or automate

```bash
# Reproduce a run exactly (the seed is printed in every report):
praxis critique roadmap.md --seed 3

# Machine-readable output for piping into another tool:
praxis critique roadmap.md --json

# See what a run would do without spending tokens:
praxis critique roadmap.md --dry-run --seed 3
```

Next: [Providers and Credentials](./guide/providers.md) to set up multiple
providers and a credentials file.
