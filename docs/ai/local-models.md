# Local Models

LazyDesktop can run AI commit message generation entirely on your machine
using GGUF models, powered by the bundled Rust crate `ai_core`
(`llama-cpp-2`). No API key, no internet at inference time.

## Model catalog

The built-in catalog offers these models, downloaded from HuggingFace:

| Model | Quantization | Size |
|-------|-------------|------|
| Qwen3-1.7B | Q8_0 | ~1.8 GB |
| Qwen2.5-0.5B | Q5_0 | ~500 MB |
| TinyLlama-1.1B-Chat | Q4_K_M | ~669 MB |
| Llama-3.2-1B-Instruct | Q4_K_M | ~808 MB |
| SmolLM2-1.7B-Instruct | Q4_K_M | ~1.06 GB |
| Gemma-2-2B-it | Q4_K_M | ~1.71 GB |
| Phi-3.5-mini-instruct | Q4_K_M | ~2.39 GB |

Smaller models download fast and run on CPU; larger models produce better
messages and benefit from a GPU.

## Downloading a model

1. Open **Settings → AI**.
2. Toggle **Enable AI**.
3. In the local model section, click **Download** next to a model.
4. Track progress in the UI. Downloads run as a **background worker**: the
   app relaunches itself in a detached "model installer" mode, so you can
   close and reopen LazyDesktop without losing the download.
5. When finished, click **Set as active** to use the model.

Models are stored in `~/.config/lazydesktop/models/`.

### Managing downloads

- **Cancel** — request cancellation of an in-progress download.
- **Delete** — remove a downloaded model (shows disk-space info first).

## GPU acceleration

Enable **GPU acceleration** in Settings → AI to offload inference layers to
the GPU (`n_gpu_layers = 99`). The crate discovers available `ggml` backends
(CPU/GPU) at runtime. Disable it for CPU-only inference.

## Choosing a model

- **Quick, CPU-friendly**: Qwen2.5-0.5B (~500 MB) — usable but terse.
- **Balanced**: TinyLlama-1.1B-Chat or Llama-3.2-1B-Instruct (~0.7–0.8 GB).
- **Quality**: Qwen3-1.7B or Gemma-2-2B-it (~1.7+ GB) — best results,
  GPU recommended.

## How local generation works

1. The UI gathers VCS context — repo path, diff, changed files, staged files,
   current branch, recent commit messages — into a JSON `CommitContext`.
2. The `ai_core` crate builds a prompt (honoring the `<diff>` placeholder)
   and runs streaming inference on a dedicated thread.
3. Tokens stream to the UI in real time; the full text is normalized into a
   Conventional Commits message (`type(scope): subject`) and returned.

Only one inference runs at a time; a second request is rejected while the
first is in flight.
