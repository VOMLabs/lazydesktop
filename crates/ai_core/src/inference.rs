use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, Ordering};

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::AddBos;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::sampling::LlamaSampler;
use tracing::info;

/// Runs inference synchronously on the calling thread, streaming generated
/// tokens through `on_token`. Returns the full generated text.
///
/// Thread management is the caller's responsibility (see the FFI layer).
/// `cancel` is polled between tokens; when set, generation stops early and the
/// text produced so far is returned.
pub fn run_inference_blocking(
    model_path: &str,
    prompt: &str,
    n_gpu_layers: i32,
    cancel: &AtomicBool,
    on_token: &dyn Fn(&str),
) -> Result<String, String> {
    info!("Loading model from {}", model_path);

    let backend = LlamaBackend::init().map_err(|e| format!("Backend init error: {e}"))?;

    let model_params = if n_gpu_layers > 0 {
        LlamaModelParams::default().with_n_gpu_layers(n_gpu_layers as u32)
    } else {
        LlamaModelParams::default()
    };

    let model = LlamaModel::load_from_file(&backend, model_path, &model_params)
        .map_err(|e| format!("Model load error: {e}"))?;

    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(8192))
        .with_n_batch(512);

    let mut ctx = model
        .new_context(&backend, ctx_params)
        .map_err(|e| format!("Context error: {e}"))?;

    let n_ctx = ctx.n_ctx() as usize;
    let n_batch = ctx.n_batch() as usize;

    let tokens = model
        .str_to_token(prompt, AddBos::Always)
        .map_err(|e| format!("Tokenization error: {e}"))?;

    let n_tokens = tokens.len();
    if n_tokens > n_ctx {
        return Err(format!(
            "Prompt too long: {n_tokens} tokens, context is {n_ctx}"
        ));
    }

    // Evaluate prompt in batches
    for i in (0..n_tokens).step_by(n_batch) {
        if cancel.load(Ordering::Relaxed) {
            return Ok(String::new());
        }
        let end = std::cmp::min(i + n_batch, n_tokens);
        let batch_tokens = &tokens[i..end];
        let mut batch = LlamaBatch::new(batch_tokens.len(), 1);
        for (j, &tok) in batch_tokens.iter().enumerate() {
            batch
                .add(tok, j as i32, &[0], j == batch_tokens.len() - 1)
                .map_err(|e| format!("Batch add error: {e}"))?;
        }
        ctx.decode(&mut batch)
            .map_err(|e| format!("Decode error: {e}"))?;
    }

    // Sampler chain
    let mut smpl = LlamaSampler::chain_simple([LlamaSampler::temp(0.1), LlamaSampler::dist(67)]);

    let eos = model.token_eos();
    let max_tokens = 1024;
    let mut generated = 0;
    let mut text = String::new();

    while generated < max_tokens {
        if cancel.load(Ordering::Relaxed) {
            break;
        }

        let token = smpl.sample(&ctx, -1);

        if token == eos {
            break;
        }

        let bytes = model
            .token_to_piece_bytes(token, 8, false, None)
            .map_err(|e| format!("Token decode error: {e}"))?;

        let piece = String::from_utf8_lossy(&bytes).to_string();
        text.push_str(&piece);
        on_token(&piece);

        // Feed the new token back
        let mut batch = LlamaBatch::new(1, 1);
        batch
            .add(token, 0, &[0], true)
            .map_err(|e| format!("Batch add error: {e}"))?;
        ctx.decode(&mut batch)
            .map_err(|e| format!("Decode error: {e}"))?;

        generated += 1;
    }

    Ok(text)
}
