//! Process-wide multi-threaded Tokio runtime.
//!
//! Blocking callers (e.g. the FFI layer) drive async operations on this shared
//! runtime via [`runtime`] instead of building a throwaway runtime per call —
//! necessary for the jj subprocess runner, which may be invoked frequently.
//! (For contrast, `connect.rs` builds a one-shot current-thread runtime per
//! connection probe, which is fine for rare network calls.)

use std::sync::OnceLock;

use tokio::runtime::Runtime;

/// Number of worker threads for the shared runtime.
const WORKER_THREADS: usize = 4;

/// Thread-name prefix for runtime workers (visible in debuggers/profilers).
const THREAD_NAME: &str = "vcs-core";

/// Lazily-initialized shared runtime. Runtime construction only fails on
/// resource exhaustion, so a panic here is acceptable and mirrors `ai_core`'s
/// runtime bootstrap.
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Returns the process-wide Tokio runtime, building it on first use.
pub fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(WORKER_THREADS)
            .thread_name(THREAD_NAME)
            .enable_all()
            .build()
            .expect("failed to build vcs_core tokio runtime")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_is_a_lazily_built_singleton() {
        assert!(std::ptr::eq(runtime(), runtime()));
    }

    #[test]
    fn runtime_runs_a_trivial_async_task() {
        let result = runtime().block_on(async { 40 + 2 });
        assert_eq!(result, 42);
    }
}
