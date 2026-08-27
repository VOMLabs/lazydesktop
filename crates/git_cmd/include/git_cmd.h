/* git_cmd.h — LazyDesktop git/jj CLI command C ABI.
 *
 * Provides access to git and jujutsu CLI commands without depending on Qt.
 * All returned strings are JSON and must be freed with `vcs_free_string`.
 *
 * Thread safety: all functions are re-entrant.
 */

#ifndef GIT_CMD_H
#define GIT_CMD_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ─── Detection ────────────────────────────────────────────── */

bool vcs_is_git_available(void);
bool vcs_is_jj_available(void);
bool vcs_is_git_repo(const char *path);
bool vcs_is_jj_repo(const char *path);

/* ─── Git read commands ────────────────────────────────────── */

/*
 * Get `git status --porcelain` as a JSON array of {status, path}.
 * Caller frees with vcs_free_string.
 */
char *vcs_git_status(const char *repo_path);

/*
 * Get `git log --oneline` as a JSON array of {hash, subject}.
 * `limit` is the max number of commits (0 = default 10).
 * Caller frees with vcs_free_string.
 */
char *vcs_git_log(const char *repo_path, uint32_t limit);

/*
 * Get `git branch -a` as a JSON array of {name, current}.
 * Caller frees with vcs_free_string.
 */
char *vcs_git_branches(const char *repo_path);

/*
 * Get the current branch name.
 * Caller frees with vcs_free_string.
 */
char *vcs_git_current_branch(const char *repo_path);

/*
 * Get the diff for a single file.
 * Caller frees with vcs_free_string.
 */
char *vcs_git_diff_file(const char *repo_path, const char *file);

/*
 * Check if the repository has uncommitted changes.
 */
bool vcs_git_is_dirty(const char *repo_path);

/*
 * List files changed in a commit as a JSON array of {status, path}.
 * Caller frees with vcs_free_string.
 */
char *vcs_git_commit_files(const char *repo_path, const char *hash);

/*
 * Read a git config value. Returns the value string or NULL.
 * Caller frees with vcs_free_string.
 */
char *vcs_git_config_get(const char *repo_path, const char *key);

/*
 * Get the staged diff (--cached).
 * Caller frees with vcs_free_string.
 */
char *vcs_git_diff_staged(const char *repo_path);

/*
 * Run an arbitrary git command and return raw stdout.
 * `args_json` is a JSON array of argument strings.
 * Caller frees with vcs_free_string.
 */
char *vcs_git_run_raw(const char *repo_path, const char *args_json);

/* ─── Git mutating commands ────────────────────────────────── */

/*
 * Stage files. `files_json` is a JSON array of path strings.
 * Returns 0 on success, -1 on error.
 */
int32_t vcs_git_add(const char *repo_path, const char *files_json);

/*
 * Commit with a message. Returns 0 on success, -1 on error.
 */
int32_t vcs_git_commit(const char *repo_path, const char *message);

/*
 * Push to remote. Returns 0 on success, -1 on error.
 */
int32_t vcs_git_push(const char *repo_path);

/*
 * Fetch from remote. Returns 0 on success, -1 on error.
 */
int32_t vcs_git_fetch(const char *repo_path);

/*
 * Pull from remote. Returns 0 on success, -1 on error.
 */
int32_t vcs_git_pull(const char *repo_path);

/*
 * Checkout a branch. Returns 0 on success, -1 on error.
 */
int32_t vcs_git_checkout(const char *repo_path, const char *branch);

/*
 * Create a new branch and checkout. Returns 0 on success, -1 on error.
 */
int32_t vcs_git_create_branch(const char *repo_path, const char *name);

/*
 * Delete a branch. Returns 0 on success, -1 on error.
 */
int32_t vcs_git_delete_branch(const char *repo_path, const char *name);

/*
 * Clone a repository. Returns 0 on success, -1 on error.
 */
int32_t vcs_git_clone(const char *url, const char *dest);

/*
 * Initialize a new git repository. Returns 0 on success, -1 on error.
 */
int32_t vcs_git_init(const char *path);

/*
 * Set a git config value. Returns 0 on success, -1 on error.
 */
int32_t vcs_git_config_set(const char *repo_path, const char *key,
                           const char *value);

/*
 * Restore a file to HEAD state (discard changes).
 * Returns 0 on success, -1 on error.
 */
int32_t vcs_git_restore_file(const char *repo_path, const char *file);

/* ─── jj commands ──────────────────────────────────────────── */

/*
 * Get `jj status` as a JSON array of {status, path}.
 * Caller frees with vcs_free_string.
 */
char *vcs_jj_status(const char *repo_path);

/*
 * Get the diff for a single file.
 * Caller frees with vcs_free_string.
 */
char *vcs_jj_diff_file(const char *repo_path, const char *file);

/* ─── Memory ───────────────────────────────────────────────── */

void vcs_free_string(char *s);

#ifdef __cplusplus
}
#endif

#endif /* GIT_CMD_H */
