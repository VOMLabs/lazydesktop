#ifndef AI_CORE_H
#define AI_CORE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handle to the model manager */
typedef struct ModelManager ModelManager;

/* Callbacks */

/* Download progress callback. id is the download ID returned by mm_download_model.
   received and total are in bytes. user_data is the opaque pointer passed to mm_download_model. */
typedef void (*MM_ProgressCallback)(int32_t id, int64_t received, int64_t total, void *user_data);

/* Streaming token callback. token is a null-terminated UTF-8 string. */
typedef void (*MM_TokenCallback)(const char *token, void *user_data);

/* Error callback. message is a null-terminated UTF-8 string. */
typedef void (*MM_ErrorCallback)(const char *message, void *user_data);

/* Finish callback. message is the final (normalized) generated text, or an
   empty string when the run was cancelled. */
typedef void (*MM_FinishCallback)(const char *message, void *user_data);

/* Cancellation poll callback. Returns true if the operation should be cancelled. */
typedef bool (*MM_CancelCallback)(void *user_data);

/* ─── Lifecycle ─────────────────────────────────────── */

/* Initialize the model manager. models_dir is where GGUF files are stored.
   config_path is where model metadata is persisted. Both are UTF-8 paths. */
ModelManager *mm_init(const char *models_dir, const char *config_path);

/* Destroy the model manager and free all resources. */
void mm_destroy(ModelManager *mm);

/* ─── Download ───────────────────────────────────────── */

/* Start downloading a model. Returns a positive download ID on success,
   or -1 on failure. progress_cb is optional (may be NULL).
   The caller must ensure user_data remains valid until the download completes
   or is cancelled. */
int32_t mm_download_model(ModelManager *mm, const char *url, const char *dest_path,
                          const char *expected_sha256,
                          MM_ProgressCallback progress_cb, void *user_data);

/* Cancel an active download. */
void mm_cancel_download(ModelManager *mm, int32_t download_id);

/* ─── Model Listing ──────────────────────────────────── */

/* Returns a JSON array of all known models (curated + discovered).
   Each entry has: name, url, path, size_bytes, size_label, sha256, downloaded (bool).
   The caller must free the string with mm_free_string. */
char *mm_list_local_models(ModelManager *mm);

/* Returns a JSON array of all discoverable models from HuggingFace API + curated list.
   The caller must free the string with mm_free_string. */
char *mm_discover_models(ModelManager *mm);

/* ─── Delete ─────────────────────────────────────────── */

/* Delete a model file from disk. Returns true on success. */
bool mm_delete_model(ModelManager *mm, const char *path);

/* ─── Inference ──────────────────────────────────────── */

/* Run streaming inference on a loaded model.
   Spawns an internal thread and returns immediately. Callbacks are invoked from
   the inference thread. on_cancelled is polled periodically; return true to cancel.
   on_finish delivers the full generated text when the run terminates.
   Returns true if the inference was started successfully, false if the manager
   is null, the arguments are invalid, or another inference is already running. */
bool mm_stream_inference(ModelManager *mm, const char *model_path, const char *prompt,
                         int32_t n_gpu_layers,
                         MM_TokenCallback on_token, MM_ErrorCallback on_error,
                         MM_CancelCallback on_cancelled, MM_FinishCallback on_finish,
                         void *user_data);

/* Generate a Conventional Commits message for the changes described in
   context_json. The context is a UTF-8 JSON object with the following fields
   (all optional unless noted):

   {
     "vcs":            "git" | "jujutsu",
     "repo_path":      string,   // repository root
     "diff":           string,   // diff text for the changes
     "files":          [ { "path": string, "status": "M" } ],
     "staged":         [ string, ... ],   // git staged paths
     "branch":         string,   // branch name / working-copy change id
     "recent_messages":[ string, ... ],   // recent commit subjects
     "mode":           "message" | "description",
     "system_prompt":  string    // optional override; may contain "<diff>"
   }

   Raw tokens are streamed through on_token (for live display); the normalized
   Conventional Commits message is delivered through on_finish. Cancellation and
   concurrency behavior match mm_stream_inference. */
bool mm_generate_commit_message(ModelManager *mm, const char *model_path,
                                const char *context_json, int32_t n_gpu_layers,
                                MM_TokenCallback on_token, MM_ErrorCallback on_error,
                                MM_CancelCallback on_cancelled,
                                MM_FinishCallback on_finish, void *user_data);

/* ─── Memory ─────────────────────────────────────────── */

/* Free a string returned by mm_list_local_models or mm_discover_models. */
void mm_free_string(char *s);

#ifdef __cplusplus
}
#endif

#endif /* AI_CORE_H */
