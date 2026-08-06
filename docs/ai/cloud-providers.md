# Cloud Providers

LazyDesktop supports four cloud providers for AI commit messages. All of them
require an API key from the provider's dashboard.

## Supported providers

| Provider | Endpoint | Auth |
|----------|----------|------|
| **OpenRouter** | `openrouter.ai/api/v1/chat/completions` | Bearer token |
| **OpenAI** | `api.openai.com/v1/chat/completions` | Bearer token |
| **Anthropic** | `api.anthropic.com/v1/messages` | `x-api-key` header |
| **Google AI Studio** | `generativelanguage.googleapis.com/v1beta/models/...` | API key query parameter |

## Setup

1. Open **Settings → AI**.
2. Toggle **Enable AI**.
3. Set **Provider** to the service you want to use.
4. Paste your **API key** (stored in `~/.config/lazydesktop/lazydesktop.conf`).
5. Set the **Model** name. The default is `gpt-4o-mini`; the editable combo
   can auto-fetch available models from the API.

### Where to get API keys

- **OpenRouter** — https://openrouter.ai/keys
- **OpenAI** — https://platform.openai.com/api-keys
- **Anthropic** — https://console.anthropic.com/settings/keys
- **Google AI Studio** — https://makersuite.google.com/app/apikey

## Switching providers

- Use the dropdown in Settings → AI.
- **Right-click** the AI button in the commit panel to switch the provider
  on the fly.

## Customizing prompts

The system prompts are editable in Settings → AI:

- **Summary prompt** — used for the commit summary.
- **Description prompt** — used for the commit description.

Both may contain a `<diff>` placeholder, which is replaced with the diff of
your checked files at generation time.

## Privacy notes

- Your diff text is sent to the provider's API as part of the prompt.
- The API key is stored locally in plain text (QSettings INI format); protect
  `~/.config/lazydesktop/lazydesktop.conf` like any credential file.
- For fully offline operation, use [local models](local-models.md) instead.
