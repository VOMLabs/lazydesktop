/* watcher.h — LazyDesktop file watcher C ABI.
 *
 * Provides a lightweight, typed file-watching API over VCS state files
 * (.git/HEAD, .git/index, .jj/working_copy, .jj/repo) with built-in
 * debouncing.  The Rust side owns the watcher; the C/Qt side polls via
 * `watcher_poll`.
 *
 * Memory contract:
 *   - Returned strings must be freed with `watcher_free_string`.
 *   - Returned path strings are heap-allocated UTF-8 NUL-terminated C strings.
 *   - The watcher handle must be destroyed exactly once via `watcher_destroy`.
 *
 * Thread safety:
 *   - All functions are re-entrant and may be called from any thread.
 *   - `watcher_poll` blocks until an event is available or the timeout (ms)
 *     expires.
 */

#ifndef WATCHER_H
#define WATCHER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handle to a running watcher. */
typedef struct watcher_handle watcher_handle;

/* Event kind codes — mirrors the Rust `FileEvent` enum. */
typedef enum watcher_event_kind {
    WATCHER_EVENT_CREATED  = 0,
    WATCHER_EVENT_MODIFIED = 1,
    WATCHER_EVENT_REMOVED  = 2,
    WATCHER_EVENT_RENAMED  = 3,
    WATCHER_EVENT_RESCAN   = 4,
} watcher_event_kind;

/* A single debounced filesystem event. */
typedef struct watcher_event {
    watcher_event_kind kind;
    /* For CREATED / MODIFIED / REMOVED: the affected path.
     * For RENAMED: `path` is the destination; the source is in `rename_from`.
     * For RESCAN: both paths are NULL. */
    char *path;
    /* Only populated for RENAMED events; NULL otherwise. */
    char *rename_from;
} watcher_event;

/*
 * Create a watcher for the given repository path.
 *
 * `repo_path`  — absolute path to the repository root.
 * `vcs_kind`   — "git" or "jujutsu".
 * `debounce_ms`— debounce interval in milliseconds (0 = default 2000ms).
 * `out_handle` — on success, receives the watcher handle.
 *
 * Returns 0 on success, non-zero on error.
 */
int32_t watcher_create(const char *repo_path,
                       const char *vcs_kind,
                       uint32_t debounce_ms,
                       watcher_handle **out_handle);

/*
 * Destroy a watcher and join its background threads.
 * After this call `*handle` is set to NULL.
 */
int32_t watcher_destroy(watcher_handle **handle);

/*
 * Poll for the next filesystem event.
 *
 * `handle`     — a live watcher handle.
 * `timeout_ms` — maximum time to block (0 = infinite until event).
 * `out_event`  — on success, receives a heap-allocated `watcher_event`.
 *                The caller MUST call `watcher_free_event` when done.
 *
 * Returns:
 *   0  — event received (out_event is populated).
 *   1  — timeout (no event).
 *  -1  — error (handle is invalid or null).
 */
int32_t watcher_poll(watcher_handle *handle,
                     uint32_t timeout_ms,
                     watcher_event **out_event);

/*
 * Free a `watcher_event` and its contained strings.
 */
void watcher_free_event(watcher_event *event);

/*
 * Free a heap-allocated string returned by this API.
 */
void watcher_free_string(char *s);

#ifdef __cplusplus
}
#endif

#endif /* WATCHER_H */
