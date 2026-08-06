# AI Overview

LazyDesktop can write commit messages for you. It supports two categories of
AI:

| Category | How it runs | Needs |
|----------|-------------|-------|
| **Cloud providers** | HTTP requests to a provider API | API key, internet |
| **Local inference** | GGUF models via the bundled Rust `ai_core` crate | Model download, CPU/GPU |

Both produce a **summary** and an optional **description** from the diffs of
your checked files.

## Enabling AI

1. Open **Settings → AI**.
2. Toggle **Enable AI**.
3. Choose a route:
   - **Cloud** — select a provider and paste an API key
     ([cloud providers](cloud-providers.md)).
   - **Local** — pick a GGUF model and let LazyDesktop download it
     ([local models](local-models.md)).
4. Close settings.

## Using it

- Check the files you want to commit.
- Click the **AI** button next to the commit summary to generate a message,
  or the AI button next to the description for just the description.
- Right-click the AI button to switch providers quickly.
- While generating, the input fields are disabled and an "AI is thinking…"
  overlay appears; use **Show more** to expand the raw generated text.

## How it works

1. The UI collects the diff of your checked files.
2. It builds a prompt from the configured system prompt; the special
   `<diff>` placeholder is replaced with the actual diff text.
3. The provider (cloud or local) generates the text.
4. The response is normalized into a clean commit summary/description.

There are two independent prompts: one for the summary
(`ai/system_prompt`) and one for the description
(`ai/description_system_prompt`) — both editable in Settings → AI.

> **Tip:** smaller local models (0.5B–1B) produce usable but terse messages;
> larger models and cloud providers generally produce richer summaries.
