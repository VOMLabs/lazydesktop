#ifndef VCS_CORE_H
#define VCS_CORE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * vcs_core — native VCS/SSH operations for LazyDesktop.
 *
 * Conventions:
 * - All functions are synchronous and non-blocking in the sense that network
 *   operations have a bounded timeout (connection tests time out after 10s).
 * - Functions that return `char *` return a heap-allocated UTF-8 string that
 *   the CALLER must free with vcs_free_string.
 * - Functions that can fail return a JSON object `{"error": "message"}` or a
 *   plain error string; successful list operations return a JSON array.
 * - SECURITY: private key material never crosses this boundary. The app only
 *   receives public keys, fingerprints, paths, and status/error strings.
 */

/* ─── Memory ─────────────────────────────────────────── */

/* Free a string returned by any vcs_* function. */
void vcs_free_string(char *s);

/* ─── SSH keys ───────────────────────────────────────── */

/* Returns a JSON array of absolute paths of all `*.pub` keys in ~/.ssh
   (sorted by name), or a JSON object `{"error": "..."}`. Caller frees. */
char *vcs_ssh_list_public_keys(void);

/* Generate an SSH keypair. key_type is "ed25519" or "rsa". Writes the
   private key at path (mode 0600) and the public key at path + ".pub".
   comment is optional (may be ""); passphrase may be NULL for no
   passphrase, or a non-empty string to encrypt the private key
   (aes256-ctr + bcrypt-pbkdf). Returns "ok" on success, otherwise an
   error message. Caller frees the result. */
char *vcs_ssh_generate_key(const char *key_type, const char *path,
                           const char *comment, const char *passphrase);

/* Returns the SHA256 fingerprint of a public key file (formatted like
   `ssh-keygen -lf`, e.g. "SHA256:..."), or an error message.
   Caller frees the result. */
char *vcs_ssh_fingerprint(const char *pub_path);

/* Returns the trimmed OpenSSH text of a public key file, or an error
   message. Only public key material is returned; private keys are never
   read. Caller frees the result. */
char *vcs_ssh_read_public_key(const char *pub_path);

/* Test an SSH connection to host (e.g. "git@github.com") on port using the
   first available default key in ~/.ssh (id_ed25519, then id_rsa).
   BatchMode semantics: no interactive prompts, bounded 10s timeout.
   Returns a human-readable success/error message. Caller frees. */
char *vcs_ssh_test_connection(const char *host, uint16_t port);

/* ─── Git remotes ────────────────────────────────────── */

/* Returns a JSON array of `{"name": "...", "url": "..."}` objects for all
   remotes configured in the repository at repo_path, or a JSON object
   `{"error": "..."}`. Caller frees the result. */
char *vcs_remote_list(const char *repo_path);

/* Add a remote named name pointing at url in the repository at repo_path.
   Also installs the default fetch refspec. Returns "ok" on success,
   otherwise an error message. Caller frees the result. */
char *vcs_remote_add(const char *repo_path, const char *name, const char *url);

/* Remove the remote named name. Returns "ok" on success, otherwise an
   error message. Caller frees the result. */
char *vcs_remote_remove(const char *repo_path, const char *name);

/* Update the URL of the remote named name. Returns "ok" on success,
   otherwise an error message. Caller frees the result. */
char *vcs_remote_set_url(const char *repo_path, const char *name,
                         const char *url);

/* Rename the remote old_name to new_name, migrating all keys (url, fetch,
   push) and re-keying the fetch refspec to the new name. Returns "ok" on
   success, otherwise an error message. Caller frees the result. */
char *vcs_remote_rename(const char *repo_path, const char *old_name,
                        const char *new_name);

#ifdef __cplusplus
}
#endif

#endif /* VCS_CORE_H */
