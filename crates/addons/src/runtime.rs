//! Background execution: a lazily-initialized tokio multi-thread runtime plus
//! flume channels for offloading blocking work off the calling (FFI) thread.
//!
//! FFI entry points must stay synchronous, so nothing here blocks *callers*
//! unless they choose to wait on the returned [`flume::Receiver`]. The blocking
//! work (archive extraction, directory loading) runs on tokio's dedicated
//! blocking pool, letting independent packages prepare concurrently.

use std::sync::OnceLock;

/// The process-wide background runtime, created on first use.
///
/// A small multi-thread worker pool keeps the overhead negligible while still
/// allowing concurrent blocking jobs; heavy syscalls run on tokio's blocking
/// pool (`spawn_blocking`), not on these workers.
pub(crate) fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("lda-addons")
            .build()
            .expect("failed to build addons background runtime")
    })
}

/// Run `f` on the background blocking pool, returning a flume receiver that
/// yields the result once the job finishes.
///
/// Jobs are unbounded and never dropped: even if the receiver is abandoned,
/// the blocking task still completes (the send simply fails silently).
pub(crate) fn spawn_blocking<F, T>(f: F) -> flume::Receiver<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = flume::unbounded();
    runtime().spawn_blocking(move || {
        let _ = tx.send(f());
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_blocking_runs_off_thread_and_delivers() {
        let main_tid = std::thread::current().id();
        let rx = spawn_blocking(move || {
            let tid = std::thread::current().id();
            (tid, 21u32 * 2)
        });
        let (tid, value) = rx.recv().unwrap();
        assert_eq!(value, 42);
        assert_ne!(tid, main_tid, "blocking job ran on the caller thread");
    }

    #[test]
    fn many_jobs_deliver_results_in_order() {
        let rxs: Vec<_> = (0..16).map(|i| spawn_blocking(move || i * i)).collect();
        for (i, rx) in rxs.into_iter().enumerate() {
            assert_eq!(rx.recv().unwrap(), i * i);
        }
    }
}
