---
description: Audit Rust backend for Tokio async bottlenecks, Rayon parallelization, and FFI lock contention.
agent: opencoder
---

Audit the Rust backend codebase for concurrency, memory efficiency, and FFI safety:

1. **Async I/O Bottlenecks (`tokio`)**:
   - Locate blocking sync operations (e.g., `std::fs`, `std::process`) executing inside Tokio tasks.
   - Refactor blocking routines to `tokio::task::spawn_blocking` or async alternatives.

2. **Data Parallelism (`rayon`)**:
   - Identify sequential CPU-bound operations over large collections (e.g., diff parsing, tree traversals, status mapping).
   - Convert eligible `.iter()` chains to Rayon `par_iter()`.

3. **Lock Contention & State**:
   - Audit shared state for coarse `std::sync::Mutex` or `tokio::sync::Mutex` usage in hot paths.
   - Replace with lock-free structures (`dashmap`) or FFI-friendly bounded channels (`flume`, `crossbeam`).

4. **Validation & Verification**:
   - Execute `cargo clippy --all-targets -D warnings`.
   - Run `cargo test`.
   - Output a prioritized list of refactor targets with concrete code diffs.
