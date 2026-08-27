/* config.h — LazyDesktop configuration C ABI.
 *
 * Provides access to settings (INI), projects (YAML), and themes (YAML)
 * without depending on Qt. All returned strings must be freed with
 * `config_free_string`.
 *
 * Thread safety: all functions are re-entrant.
 */

#ifndef CONFIG_H
#define CONFIG_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ─── Settings ────────────────────────────────────────────── */

/*
 * Load settings from an INI file. Returns a JSON string mapping
 * dotted keys to values (e.g. `{"git/user.name": "Alice", ...}`).
 * Returns NULL on error. Caller frees with config_free_string.
 */
char *config_settings_load(const char *path);

/*
 * Get a single setting from a JSON settings string.
 * Returns the value as a heap-allocated string, or NULL if not found.
 * Caller frees with config_free_string.
 */
char *config_settings_get(const char *settings_json, const char *key);

/* ─── Projects ────────────────────────────────────────────── */

/*
 * Load projects from a YAML file. Returns a JSON array of path strings.
 * Returns NULL on error. Caller frees with config_free_string.
 */
char *config_projects_load(const char *path);

/*
 * Save projects to a YAML file from a JSON array of path strings.
 * Returns 0 on success, non-zero on error.
 */
int32_t config_projects_save(const char *path, const char *projects_json);

/*
 * Add a project path to the projects file.
 * Returns 0 on success, non-zero on error.
 */
int32_t config_projects_add(const char *path, const char *project_path);

/*
 * Remove a project path from the projects file.
 * Returns 0 on success, non-zero on error.
 */
int32_t config_projects_remove(const char *path, const char *project_path);

/*
 * Clear all projects from the projects file.
 * Returns 0 on success, non-zero on error.
 */
int32_t config_projects_clear(const char *path);

/* ─── Memory ──────────────────────────────────────────────── */

/*
 * Free a string previously returned by this API.
 */
void config_free_string(char *s);

#ifdef __cplusplus
}
#endif

#endif /* CONFIG_H */
