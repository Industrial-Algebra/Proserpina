# Getting Started

## Install

```bash
cargo install proserpina
```

Or build from source:

```bash
git clone https://github.com/Industrial-Algebra/Proserpina
cd Proserpina
cargo install --path . --features cli,backend-http,json
```

## Authenticate one provider

The zero-config path: log in interactively.

```bash
proserpina auth login deepseek
```

This prompts for your API key, validates it, and stores it in
`~/.config/proserpina/credentials.toml` (or the OS keychain if the `keyring`
feature is on).

For OpenAI (ChatGPT), an OAuth browser flow is used:

```bash
proserpina auth login openai
```

Or set an env var directly (any of `DEEPSEEK_API_KEY`, `OPENAI_API_KEY`,
`MOONSHOT_API_KEY`, `DASHSCOPE_API_KEY`, `ZAI_API_KEY`, `GOOGLE_API_KEY`
works). If pi is installed, Proserpina auto-discovers pi's provider configs.

## Critique a document

```bash
proserpina critique roadmap.md
```

You'll get a markdown digest: an executive summary of findings by severity,
then each finding with its category, location, quote, suggested change, and
the critics that raised it.

## Try a multi-critic panel

```bash
proserpina critique roadmap.md --panel panel
```

`--panel panel` runs all five built-in critics (Devil's Advocate,
Methodologist, Red Team, Domain Expert, Editor), fanned across your authed
providers. The summarizer clusters their critiques — you'll see findings
"raised by" multiple critics where the panel converged.

## Reproduce or automate

```bash
# Reproduce a run exactly (the seed is printed in every report):
proserpina critique roadmap.md --seed 3

# Machine-readable output for piping into another tool:
proserpina critique roadmap.md --json

# See what a run would do without spending tokens:
proserpina critique roadmap.md --dry-run --seed 3
```

Next: [Providers and Credentials](./guide/providers.md) to set up multiple
providers and a credentials file.
