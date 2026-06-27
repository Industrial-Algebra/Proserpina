# Panels

A **panel** is a named set of critic personas. Pick one with `--panel <name>`.

## Built-in panels

| Panel | Critics | Use |
|---|---|---|
| `default` | Devil's Advocate | a quick single-critic pass (the default) |
| `duo` | Devil's Advocate + Methodologist | a second lens |
| `panel` | + Red Team, Domain Expert, Editor | full cross-examination |

## Custom panels

Define your own under `[panels.NAME]` in the credentials file:

```toml
[panels.red-team]
personas = [
  { name = "Skeptic", framing = "Doubt everything.", focus = "assumptions" },
  { name = "Nitpicker", framing = "Find the small flaws.", focus = "details" },
  { name = "Architect", framing = "Question the structure.", focus = "design" },
]
```

Each persona is `{ name, framing?, focus? }`. Then:

```bash
praxis critique plan.md --panel red-team
```

A config-defined panel overrides a same-named built-in (so you can redefine
`default`). Names must be unique within a panel — agents are keyed by persona
name.

## How providers get assigned

Each critic in the panel is assigned a provider drawn (seeded, reproducibly)
from your authed providers. So a `--panel panel` run with DeepSeek + Z.ai
authed will fan the five critics across both. The seed (printed in every
report) makes the assignment reproducible.

## Discover panels

```bash
praxis capabilities | jq '.panels'
```

Lists built-in + config-defined panel names.
