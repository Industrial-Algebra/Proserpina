# Providers and Credentials

Proserpina reaches any OpenAI-compatible provider. Six are built-in; custom ones
(Ollama, LM Studio, OpenRouter, a proxy) work too.

## Authentication

Proserpina offers three ways to authenticate, in order of convenience:

### 1. `proserpina auth login` (recommended)

Interactive credential acquisition — prompts for keys (or runs an OAuth browser
flow for OpenAI), validates them, and stores them:

```bash
proserpina auth login deepseek    # API key prompt → validate → store
proserpina auth login openai      # OAuth browser flow → token exchange → store
proserpina auth login             # provider selector
```

Stored in `~/.config/proserpina/credentials.toml` (or the OS keychain with the
`keyring` feature). OAuth tokens are auto-refreshed before each run.

### 2. Environment variables

```bash
export DEEPSEEK_API_KEY=sk-...
```

Any of `DEEPSEEK_API_KEY`, `OPENAI_API_KEY`, `MOONSHOT_API_KEY`,
`DASHSCOPE_API_KEY`, `ZAI_API_KEY`, `GOOGLE_API_KEY`. Easiest for CI/scripts.

### 3. Config file

`~/.config/proserpina/credentials.toml` — directly edit:

```toml
[deepseek]
api_key = "sk-..."

[openai]
oauth_access = "eyJ..."
oauth_refresh = "rt_1..."
oauth_expires = 1781648822320
```

### Resolution precedence

For each provider, a credential is resolved with precedence:
**OAuth access token > keyring > env var > config file > pi discovery > none**

A provider is **authed** iff a credential resolved. The roster only assigns
authed providers to critics.

### Pi discovery (convenience)

If pi is installed, Proserpina auto-discovers pi's provider configs
(`~/.pi/agent/models.json` + `~/.pi/agent/auth.json`), so providers configured
in pi work in Proserpina without separate setup. This is a convenience layer;
`auth login` credentials take precedence.

## The built-in registry

| Provider | Base URL | Default model | Auth method |
|---|---|---|---|
| deepseek | `api.deepseek.com` | `deepseek-chat` | API key |
| openai | `api.openai.com/v1` | `gpt-4o` | OAuth (ChatGPT) |
| zai | `api.z.ai/api/coding/paas/v4` | `glm-5.2` | API key |
| dashscope | `dashscope.aliyuncs.com/compatible-mode/v1` | `qwen-plus` | API key |
| google | `generativelanguage.googleapis.com/v1beta/openai` | `gemini-1.5-pro` | API key |
| moonshot | `api.moonshot.cn/v1` | `moonshot-v1-auto` | API key |

> **Z.ai coding plans:** Praxis uses the `api.z.ai/api/coding/paas/v4` gateway,
> so a Z.ai *coding* plan works without an API plan.

## Config format

```toml
# API keys (or use `proserpina auth login`)
[deepseek]
api_key = "sk-..."

[zai]
api_key = "..."
base_url = "https://api.z.ai/api/coding/paas/v4"
model = "glm-5.2"

# OAuth tokens (stored automatically by `proserpina auth login openai`)
[openai]
oauth_access = "eyJ..."
oauth_refresh = "rt_1..."
oauth_expires = 1781648822320

# Custom provider (name not in the registry): all three required.
[my-ollama]
base_url = "http://localhost:11434/v1"
model = "llama3"
api_key = "ollama"
```

## See what's authed

```bash
proserpina auth check    # validate all keys live
proserpina auth list     # show which keys are set (no network)
proserpina capabilities  # human-readable overview
```
