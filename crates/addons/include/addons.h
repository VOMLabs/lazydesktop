/*
 * addons.h — C ABI for the LazyDesktop addon system.
 *
 * Rust (lazydesktop-addons) is the trust boundary: package parsing, manifest
 * validation, ignore rules, path security, and resource access are enforced
 * here. C++ only ever consumes validated bytes and JSON descriptors.
 *
 * House style: mirrors vcs_core.h / ai_core.h. See ADDON_SPEC §11 for the
 * full ABI contract.
 */
#ifndef LAZYDESKTOP_ADDONS_H
#define LAZYDESKTOP_ADDONS_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define LDA_ABI_VERSION 1

/* Result codes. Values are stable; the JSON payload carries the detail. */
typedef enum lda_result {
    LDA_OK = 0,
    LDA_ERR_INVALID_ARGUMENT,  /* null/type errors from the C caller        */
    LDA_ERR_INVALID_ADDON,     /* package-level validity failure            */
    LDA_ERR_INVALID_MANIFEST,  /* config.toml schema/validation failure     */
    LDA_ERR_ARCHIVE,           /* zip/lzd structure, CRC, limits            */
    LDA_ERR_PATH_SECURITY,     /* traversal/symlink/containment violation   */
    LDA_ERR_PROVIDER,          /* provider-level failure (conflict, r/o)    */
    LDA_ERR_LUA_RUNTIME,       /* produced by the C++ host, shared taxonomy */
    LDA_ERR_FFI,               /* internal panic caught at the boundary     */
    LDA_ERR_INTERNAL,          /* unexpected internal error (io, alloc)     */
} lda_result;

typedef struct lda_registry lda_registry; /* opaque handle */

/* Logging callback (§15): called on the calling thread, must not re-enter
 * the lda API, must not block, must not use the Rust allocator. */
typedef void (*lda_log_fn)(int level, const char *msg, void *userdata);

/* --- Version / memory ------------------------------------------------ */

/* Writes the host API version the crate was built against. */
void lda_api_version(uint32_t *major, uint32_t *minor);

/* Returns the ABI version string (""LDA_ABI_VERSION""). Never freed. */
const char *lda_abi_version(void);

/* Free strings returned by the lda API. Never call free() on them. */
void lda_free_string(char *s);

/* Free byte buffers returned through out-params. Never call free(). */
void lda_free_bytes(uint8_t *p);

/* Install a log sink (pass NULL cb to disable). */
void lda_set_log_sink(lda_log_fn cb, void *userdata);

/* --- Registry lifecycle ---------------------------------------------- */

/* Creates a registry. Never returns NULL on success. */
lda_registry *lda_registry_create(void);

/* Destroys a registry and nulls *r. Double-destroy of a nulled handle is
 * a no-op; destroying a live handle twice is UB. */
lda_result lda_registry_destroy(lda_registry **r);

lda_result lda_registry_set_app_version(lda_registry *r, const char *version);
lda_result lda_registry_add_root(lda_registry *r, const char *path, int user_priority);
lda_result lda_registry_load(lda_registry *r);
lda_result lda_registry_refresh(lda_registry *r);

/* --- Listing & lookup ------------------------------------------------ */

/* out_json receives {"addons": [...]}. Free with lda_free_string. */
lda_result lda_registry_list(lda_registry *r, char **out_json);

/* out_json receives {"addon": {...}} or an error object on failure. */
lda_result lda_registry_get(lda_registry *r, const char *id, char **out_json);

/* --- Install / uninstall --------------------------------------------- */

/* source_path may be a directory, .zip, or .lzd. out_json gets the
 * installed descriptor (or an error object). */
lda_result lda_registry_install(lda_registry *r, const char *source_path, char **out_json);
lda_result lda_registry_uninstall(lda_registry *r, const char *id);

/* --- Resource access (validated bytes only) -------------------------- */

/* Tests whether an asset is readable (normalized, unignored, contained). */
lda_result lda_addon_has_asset(lda_registry *r, const char *id,
                               const char *rel_path, int *out_has);

/* Reads an asset as bytes; caller frees with lda_free_bytes. */
lda_result lda_addon_read_asset(lda_registry *r, const char *id,
                                const char *rel_path,
                                uint8_t **out_bytes, size_t *out_len);

/* Reads the addon's entry script as bytes (never as a host path). */
lda_result lda_addon_entry_script(lda_registry *r, const char *id,
                                  uint8_t **out_bytes, size_t *out_len);

/* Returns the per-addon storage directory (created by the host). */
lda_result lda_addon_storage_dir(lda_registry *r, const char *id, char **out_path);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* LAZYDESKTOP_ADDONS_H */
