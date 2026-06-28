# Providers and Credentials

Proserpina reaches any OpenAI-compatible provider. Six are built-in; custom ones
(Ollama, LM Studio, OpenRouter, a proxy) work too.

## Authentication

For each provider, a key is resolved with precedence **env var > config file
> none**:

- **Env var** — the registry declares each provider's var (`DEEPSEEK_API_KEY`,
  `OPENAI_API_KEY`, `MOONSHOT_API_KEY`, `DASHSCOPE_API_KEY`, `ZAI_API_KEY`,
  `GOOGLE_API_KEY`). Easiest for DeepSeek (zero-config).
- **Config file** — `~/.config/proserpina/credentials.toml`. Required for providers
  whose keys aren't in the env (e.g. ones pi mediates via OAuth/extensions).

A provider is **authed** iff a key resolved. The roster only assigns authed
providers to critics.

## The built-in registry

| Provider | Base URL | Default model | Env var |
|---|---|---|---|
| deepseek | `api.deepseek.com/v1` | `deepseek-chat` | `DEEPSEEK_API_KEY` |
| openai | `api.openai.com/v1` | `gpt-4o` | `OPENAI_API_KEY` |
| moonshot | `api.moonshot.cn/v1` | `moonshot-v1-auto` | `MOONSHOT_API_KEY` |
| alibaba | `dashscope.aliyuncs.com/compatible-mode/v1` | `qwen-plus` | `DASHSCOPE_API_KEY` |
| zai | `api.z.ai/api/coding/paas/v4` | `glm-5.2` | `ZAI_API_KEY` |
| google | `generativelanguage.googleapis.com/v1beta/openai` | `gemini-1.5-pro` | `GOOGLE_API_KEY` |

> **Z.ai coding plans:** Proserpina uses the `api.z.ai/api/coding/paas/v4` gateway,
> not the public `open.bigmodel.cn` endpoint — so a Z.ai *coding* plan works
> without an API plan.

## Config format

```toml
# A section per provider. api_key makes it authed; model/base_url override defaults.
[deepseek]
api_key = "sk-..."

[zai]
api_key = "..."
base_url = "https://api.z.ai/api/coding/paas/v4"
model = "glm-5.2"

# Custom provider (name not in the registry): all three required.
[my-ollama]
base_url = "http://localhost:11434/v1"
model = "llama3"
api_key = "ollama"
```

## See what's authed

```bash
proserpina capabilities | jq '.providers[] | select(.authed)'
```

The `authed` field is **dynamic** — it reflects the real config + env right
now, so you (or an agent) learn what's actually runnable in this environment.
