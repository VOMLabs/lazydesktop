//! SSH key management: list public keys, generate keypairs, compute
//! fingerprints, read public key text.
//!
//! Security invariant: private key material is written to disk only and is
//! never read back or returned by any function in this module.

use std::fs;
use std::path::{Path, PathBuf};

use rand::rngs::OsRng;
use ssh_key::{Algorithm, HashAlg, LineEnding, PrivateKey, PublicKey};

use crate::error::{VcsError, VcsResult};

/// Absolute path to the user's SSH directory (~/.ssh).
pub fn ssh_keys_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(".ssh")
}

/// Absolute paths of every `*.pub` file in the SSH directory, sorted by name.
///
/// Returns an empty list when the directory does not exist.
pub fn list_public_keys() -> VcsResult<Vec<PathBuf>> {
    list_public_keys_in(&ssh_keys_dir())
}

/// List regular `*.pub` files under `dir`, sorted by file name.
///
/// Pure with respect to its input: the only side effect is reading the
/// directory, and the result depends only on `dir`'s contents.
fn list_public_keys_in(dir: &Path) -> VcsResult<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut keys = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let is_pub_file =
            entry.file_type()?.is_file() && path.extension().and_then(|e| e.to_str()) == Some("pub");
        if is_pub_file {
            keys.push(path);
        }
    }
    keys.sort();
    Ok(keys)
}

/// Generate a new keypair (`ed25519` or `rsa`) at `path`, writing both the
/// private key and the `.pub` companion. `comment` becomes the key comment;
/// `passphrase` is optional and may be empty. Private key material is written
/// to disk and never returned.
pub fn generate_key(
    key_type: &str,
    path: &Path,
    comment: &str,
    passphrase: Option<&str>,
) -> VcsResult<()> {
    let algorithm = match key_type {
        "ed25519" => Algorithm::Ed25519,
        // 0.6.7's `Algorithm::Rsa` carries only the signature hash; keygen
        // always produces 4096-bit keys (ssh-key's DEFAULT_RSA_KEY_SIZE).
        "rsa" => Algorithm::Rsa {
            hash: Some(HashAlg::Sha256),
        },
        other => {
            return Err(VcsError::Invalid(format!(
                "unsupported key type '{other}' (expected 'ed25519' or 'rsa')"
            )));
        }
    };
    generate_key_in(algorithm, path, comment, passphrase)
}

/// Generate a keypair for `algorithm` at `path`; see [`generate_key`].
fn generate_key_in(
    algorithm: Algorithm,
    path: &Path,
    comment: &str,
    passphrase: Option<&str>,
) -> VcsResult<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let mut rng = OsRng;
    let mut key = PrivateKey::random(&mut rng, algorithm)?;
    key.set_comment(comment);

    if let Some(passphrase) = passphrase.filter(|p| !p.is_empty()) {
        // `write_openssh_file` never encrypts, so encrypt explicitly
        // (ssh-key `encryption` feature: AES-256-CTR + bcrypt-pbkdf).
        key = key.encrypt(&mut rng, passphrase)?;
    }

    // Writes the private key with Unix mode 0600; never returns its contents.
    key.write_openssh_file(path, LineEnding::LF)?;

    // `encrypt` rebuilds the public key from key data and drops the comment,
    // so re-apply it to the `.pub` companion (a no-op for unencrypted keys).
    let mut public_key = key.public_key().clone();
    public_key.set_comment(comment);
    public_key.write_openssh_file(&path.with_extension("pub"))?;

    Ok(())
}

/// Compute the SHA256 fingerprint of a public key file, formatted like
/// `ssh-keygen -lf` (`SHA256:...`).
pub fn fingerprint_public_key(path: &Path) -> VcsResult<String> {
    let text = fs::read_to_string(path)?;
    let public_key = PublicKey::from_openssh(text.trim())?;
    Ok(public_key.fingerprint(HashAlg::Sha256).to_string())
}

/// Read a public key file and return its trimmed OpenSSH text.
pub fn read_public_key(path: &Path) -> VcsResult<String> {
    Ok(fs::read_to_string(path)?.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_keys_dir_points_at_dot_ssh() {
        assert_eq!(ssh_keys_dir().file_name().and_then(|n| n.to_str()), Some(".ssh"));
    }

    #[test]
    fn list_public_keys_in_lists_only_pub_files_sorted() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("id_ed25519.pub"), "ssh-ed25519 AAAA one\n").unwrap();
        fs::write(dir.path().join("id_rsa.pub"), "ssh-rsa AAAA two\n").unwrap();
        // Not public keys: a private key and an unrelated text file.
        fs::write(dir.path().join("id_ed25519"), "private key material\n").unwrap();
        fs::write(dir.path().join("known_hosts"), "host ssh-ed25519 AAAA\n").unwrap();

        let keys = list_public_keys_in(dir.path()).unwrap();
        let names: Vec<String> = keys
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["id_ed25519.pub", "id_rsa.pub"]);
        // The helper returns absolute paths when given an absolute directory.
        assert!(keys.iter().all(|p| p.is_absolute()));
    }

    #[test]
    fn list_public_keys_in_missing_dir_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        assert!(list_public_keys_in(&missing).unwrap().is_empty());
    }

    #[test]
    fn list_public_keys_default_dir_never_fails() {
        // The real ~/.ssh may or may not exist; the public API must not error
        // in either case.
        list_public_keys().unwrap();
    }

    #[test]
    fn generate_ed25519_key_writes_pub_and_fingerprint() {
        let dir = tempfile::tempdir().unwrap();
        let priv_path = dir.path().join("id_ed25519");
        let pub_path = priv_path.with_extension("pub");

        generate_key("ed25519", &priv_path, "user@example.com", None).unwrap();

        assert!(priv_path.exists());
        assert!(pub_path.exists());

        let pub_text = read_public_key(&pub_path).unwrap();
        assert!(pub_text.starts_with("ssh-ed25519 "), "got: {pub_text}");
        assert!(pub_text.ends_with("user@example.com"), "got: {pub_text}");

        let fingerprint = fingerprint_public_key(&pub_path).unwrap();
        assert!(fingerprint.starts_with("SHA256:"), "got: {fingerprint}");
        assert!(fingerprint.len() > "SHA256:".len() + 40);
    }

    #[test]
    fn generate_key_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let priv_path = dir.path().join("nested").join("deeper").join("id_ed25519");

        generate_key("ed25519", &priv_path, "user@example.com", None).unwrap();

        assert!(priv_path.exists());
        assert!(priv_path.with_extension("pub").exists());
    }

    #[test]
    fn generate_key_rejects_unknown_type() {
        let dir = tempfile::tempdir().unwrap();
        let err = generate_key("ecdsa", &dir.path().join("id_bad"), "c", None).unwrap_err();
        assert!(err.to_string().contains("unsupported key type"), "got: {err}");
    }

    #[test]
    fn generate_key_with_passphrase_encrypts_private_key() {
        let dir = tempfile::tempdir().unwrap();
        let priv_path = dir.path().join("id_ed25519");

        generate_key("ed25519", &priv_path, "user@example.com", Some("hunter2")).unwrap();

        let key = PrivateKey::from_openssh(fs::read_to_string(&priv_path).unwrap()).unwrap();
        assert!(key.is_encrypted());
        assert_eq!(key.cipher(), ssh_key::Cipher::Aes256Ctr);

        // The encrypted key round-trips: the comment is baked into the
        // encrypted blob and restored on decrypt.
        let decrypted = key.decrypt("hunter2").unwrap();
        assert_eq!(decrypted.comment(), "user@example.com");
    }

    #[test]
    fn generate_key_treats_empty_passphrase_as_none() {
        let dir = tempfile::tempdir().unwrap();
        let priv_path = dir.path().join("id_ed25519");

        generate_key("ed25519", &priv_path, "user@example.com", Some("")).unwrap();

        let key = PrivateKey::from_openssh(fs::read_to_string(&priv_path).unwrap()).unwrap();
        assert!(!key.is_encrypted(), "empty passphrase must not encrypt the key");
    }

    #[test]
    fn generate_rsa_key_parses() {
        let dir = tempfile::tempdir().unwrap();
        let priv_path = dir.path().join("id_rsa");
        let pub_path = priv_path.with_extension("pub");

        // 4096-bit RSA generation via the `rsa` crate is intentionally slow.
        generate_key("rsa", &priv_path, "user@example.com", None).unwrap();

        assert!(priv_path.exists());
        let pub_text = read_public_key(&pub_path).unwrap();
        assert!(pub_text.starts_with("ssh-rsa "), "got: {pub_text}");
        let fingerprint = fingerprint_public_key(&pub_path).unwrap();
        assert!(fingerprint.starts_with("SHA256:"), "got: {fingerprint}");
    }
}
