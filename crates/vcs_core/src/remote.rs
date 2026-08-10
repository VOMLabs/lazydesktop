//! Git remote management via gix-config (no git CLI shell-outs).
//!
//! Operates directly on the repository's `.git/config` file, preserving
//! whitespace and comments via gix-config's lossless round-trip.

use std::path::{Path, PathBuf};

use gix_config::{AsBStr, File};

use crate::error::{VcsError, VcsResult};

/// A single remote entry: name + primary (fetch) URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
    pub name: String,
    pub url: String,
}

/// Load `.git/config` for the repository at `repo_path`, returning the parsed
/// config together with the path it came from so callers can write it back.
fn load_config(repo_path: &Path) -> VcsResult<(File, PathBuf)> {
    let config_path = repo_path.join(".git").join("config");
    if !config_path.is_file() {
        return Err(VcsError::Invalid(format!(
            "not a git repository: {}",
            repo_path.display()
        )));
    }
    let content = std::fs::read_to_string(&config_path)?;
    let config = File::try_from(content.as_str())
        .map_err(|err| VcsError::Config(err.to_string()))?;
    Ok((config, config_path))
}

/// Persist `config` back to `config_path` losslessly (preserving whitespace
/// and comments via gix-config's round-trip serialization).
fn save_config(config: &File, config_path: &Path) -> VcsResult<()> {
    std::fs::write(config_path, config.to_bstring().as_slice())?;
    Ok(())
}

/// Reject remote names that cannot be represented as a git-config subsection.
fn validate_remote_name(name: &str) -> VcsResult<()> {
    if name.trim().is_empty() {
        return Err(VcsError::Invalid(
            "remote name must not be empty".to_string(),
        ));
    }
    Ok(())
}

/// Upsert `remote.<name>.<key>`, creating the section if necessary and
/// overwriting the last value of an existing key (mirrors `git config`).
fn set_remote_value(
    config: &mut File,
    name: &str,
    key: &str,
    value: impl AsBStr,
) -> VcsResult<()> {
    config
        .set_raw_value_by("remote", name, key, value)
        .map_err(|err| VcsError::Config(err.to_string()))?;
    Ok(())
}

/// Rewrite a fetch refspec's destination namespace to the new remote name,
/// e.g. `+refs/heads/*:refs/remotes/origin/*` → `...:refs/remotes/upstream/*`.
/// Any refspec not targeting `refs/remotes/<old_name>/` is left unchanged.
fn rekey_fetch_refspec(refspec: &str, old_name: &str, new_name: &str) -> String {
    let target = format!("refs/remotes/{old_name}/");
    if let Some(pos) = refspec.find(&target) {
        let mut out = refspec.to_string();
        out.replace_range(pos..pos + target.len(), &format!("refs/remotes/{new_name}/"));
        out
    } else {
        refspec.to_string()
    }
}

/// List all remotes configured in the repository at `repo_path`.
pub fn list_remotes(repo_path: &Path) -> VcsResult<Vec<Remote>> {
    let (config, _) = load_config(repo_path)?;

    let mut remotes = Vec::new();
    if let Some(sections) = config.sections_by_name("remote") {
        for section in sections {
            // Skip sections without a subsection name or without a url value.
            if let (Some(name), Some(url)) =
                (section.header().subsection_name(), section.value("url"))
            {
                remotes.push(Remote {
                    name: name.to_string(),
                    url: url.to_string(),
                });
            }
        }
    }
    remotes.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(remotes)
}

/// Add a remote named `name` pointing at `url`, installing the fetch refspec.
pub fn add_remote(repo_path: &Path, name: &str, url: &str) -> VcsResult<()> {
    validate_remote_name(name)?;

    let (mut config, config_path) = load_config(repo_path)?;
    // `git remote add` stores both the url and the default fetch refspec.
    set_remote_value(&mut config, name, "url", url)?;
    set_remote_value(
        &mut config,
        name,
        "fetch",
        format!("+refs/heads/*:refs/remotes/{name}/*"),
    )?;
    save_config(&config, &config_path)
}

/// Remove the remote named `name`.
pub fn remove_remote(repo_path: &Path, name: &str) -> VcsResult<()> {
    validate_remote_name(name)?;

    let (mut config, config_path) = load_config(repo_path)?;
    if config.remove_section("remote", name).is_none() {
        return Err(VcsError::Config(format!("remote not found: {name}")));
    }
    save_config(&config, &config_path)
}

/// Update the URL of the remote named `name`.
pub fn set_remote_url(repo_path: &Path, name: &str, url: &str) -> VcsResult<()> {
    validate_remote_name(name)?;

    let (mut config, config_path) = load_config(repo_path)?;
    if config.section("remote", name).is_err() {
        return Err(VcsError::Config(format!("remote not found: {name}")));
    }
    set_remote_value(&mut config, name, "url", url)?;
    save_config(&config, &config_path)
}

/// Rename the remote `old_name` to `new_name`, migrating all
/// `remote.<old>.url` keys (fetch + push) to the new name. The fetch refspec
/// is re-keyed to the new name, matching `git remote rename` (which also
/// rewrites the refspec and renames remote-tracking refs).
pub fn rename_remote(repo_path: &Path, old_name: &str, new_name: &str) -> VcsResult<()> {
    validate_remote_name(old_name)?;
    validate_remote_name(new_name)?;

    let (mut config, config_path) = load_config(repo_path)?;

    // Snapshot every key/value of the old section before mutating the file.
    let section = config
        .section("remote", old_name)
        .map_err(|_| VcsError::Config(format!("remote not found: {old_name}")))?;
    let entries: Vec<_> = section
        .value_names()
        .map(|key| {
            let value = section.value(&key).unwrap_or_default();
            (key, value)
        })
        .collect();

    // Remove the old section and re-create it under the new name, preserving
    // every key (url, fetch, push, ...). The fetch refspec is re-keyed so the
    // remote-tracking namespace follows the new name.
    if config.remove_section("remote", old_name).is_none() {
        return Err(VcsError::Config(format!("remote not found: {old_name}")));
    }
    let mut new_section = config
        .new_section("remote", new_name)
        .map_err(|err| VcsError::Config(err.to_string()))?;
    for (key, value) in entries {
        // `push` takes a BString; rebuild the value so the fetch refspec can
        // be re-keyed while all other values pass through unchanged.
        let value = if key == "fetch" {
            rekey_fetch_refspec(&value.to_string(), old_name, new_name).into()
        } else {
            value
        };
        new_section
            .push(key, value)
            .map_err(|err| VcsError::Config(err.to_string()))?;
    }
    save_config(&config, &config_path)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    /// Create a throwaway repository with a minimal `.git/config`
    /// (no real `git init` needed since we operate on the file directly).
    fn repo_fixture() -> (TempDir, PathBuf) {
        let dir = TempDir::new().expect("create temp dir");
        let repo_path = dir.path().to_path_buf();
        let git_dir = dir.path().join(".git");
        fs::create_dir(&git_dir).expect("create .git dir");
        fs::write(
            git_dir.join("config"),
            "[core]\n\trepositoryformatversion = 0\n",
        )
        .expect("write .git/config");
        (dir, repo_path)
    }

    /// Read the raw `.git/config` file of the fixture.
    fn raw_config(repo_path: &Path) -> String {
        fs::read_to_string(repo_path.join(".git").join("config")).expect("read config")
    }

    #[test]
    fn list_remotes_is_empty_for_fresh_repo() {
        let (_dir, repo) = repo_fixture();
        assert_eq!(list_remotes(&repo).expect("list"), Vec::new());
    }

    #[test]
    fn add_then_list_round_trip() {
        let (_dir, repo) = repo_fixture();
        add_remote(&repo, "origin", "git@github.com:user/repo.git").expect("add");
        assert_eq!(
            list_remotes(&repo).expect("list"),
            vec![Remote {
                name: "origin".to_string(),
                url: "git@github.com:user/repo.git".to_string(),
            }]
        );
        // `git remote add` also installs the fetch refspec.
        let raw = raw_config(&repo);
        assert!(
            raw.contains("+refs/heads/*:refs/remotes/origin/*"),
            "fetch refspec missing from config:\n{raw}"
        );
    }

    #[test]
    fn add_second_remote_is_sorted() {
        let (_dir, repo) = repo_fixture();
        add_remote(&repo, "origin", "git@github.com:user/repo.git").expect("add origin");
        add_remote(&repo, "upstream", "git@github.com:user/upstream.git").expect("add upstream");
        let remotes = list_remotes(&repo).expect("list");
        assert_eq!(remotes[0].name, "origin");
        assert_eq!(remotes[1].name, "upstream");
    }

    #[test]
    fn set_remote_url_updates_existing_remote() {
        let (_dir, repo) = repo_fixture();
        add_remote(&repo, "origin", "git@github.com:user/repo.git").expect("add");
        set_remote_url(&repo, "origin", "https://example.com/repo.git").expect("set url");
        assert_eq!(
            list_remotes(&repo).expect("list"),
            vec![Remote {
                name: "origin".to_string(),
                url: "https://example.com/repo.git".to_string(),
            }]
        );
    }

    #[test]
    fn rename_remote_migrates_all_keys() {
        let (_dir, repo) = repo_fixture();
        add_remote(&repo, "origin", "git@github.com:user/repo.git").expect("add");
        rename_remote(&repo, "origin", "upstream").expect("rename");
        assert_eq!(
            list_remotes(&repo).expect("list"),
            vec![Remote {
                name: "upstream".to_string(),
                url: "git@github.com:user/repo.git".to_string(),
            }]
        );
        // The fetch refspec must be re-keyed to the new name as well.
        let raw = raw_config(&repo);
        assert!(
            raw.contains("+refs/heads/*:refs/remotes/upstream/*"),
            "migrated fetch refspec missing from config:\n{raw}"
        );
        assert!(!raw.contains("origin"), "old remote section still present:\n{raw}");
    }

    #[test]
    fn remove_remote_deletes_section() {
        let (_dir, repo) = repo_fixture();
        add_remote(&repo, "origin", "git@github.com:user/repo.git").expect("add");
        remove_remote(&repo, "origin").expect("remove");
        assert_eq!(list_remotes(&repo).expect("list"), Vec::new());
    }

    #[test]
    fn mutating_missing_remote_is_an_error() {
        let (_dir, repo) = repo_fixture();
        assert!(set_remote_url(&repo, "nope", "https://example.com/x.git").is_err());
        assert!(rename_remote(&repo, "nope", "upstream").is_err());
        assert!(remove_remote(&repo, "nope").is_err());
    }

    #[test]
    fn empty_remote_name_is_rejected() {
        let (_dir, repo) = repo_fixture();
        assert!(add_remote(&repo, "", "https://example.com/x.git").is_err());
        assert!(add_remote(&repo, "   ", "https://example.com/x.git").is_err());
        assert!(rename_remote(&repo, "origin", "").is_err());
    }

    #[test]
    fn missing_git_dir_is_an_error() {
        let dir = TempDir::new().expect("create temp dir");
        let repo = dir.path().to_path_buf(); // no `.git/config` inside
        let err = list_remotes(&repo).expect_err("list must fail");
        assert!(
            err.to_string().contains("not a git repository"),
            "unexpected error: {err}"
        );
    }
}
