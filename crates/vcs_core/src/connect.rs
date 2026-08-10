//! SSH connection testing via the native russh client (no ssh CLI).
//!
//! Implements BatchMode semantics: no interactive password prompts, no
//! ssh-agent, no host-key prompts. Host-key verification is skipped for this
//! diagnostic probe (matching the app's `ssh -T -o StrictHostKeyChecking=accept-new`
//! behavior); production connections should verify against `~/.ssh/known_hosts`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use russh::client;
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg};

use crate::error::{VcsError, VcsResult};

/// Default SSH user when the host string carries no `user@` prefix.
const DEFAULT_SSH_USER: &str = "git";

/// Upper bound for the whole connect + authenticate probe.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Candidate default private keys, tried in order (BatchMode semantics).
const DEFAULT_KEY_NAMES: [&str; 2] = ["id_ed25519", "id_rsa"];

const NO_SSH_KEY_MESSAGE: &str =
    "No SSH key found in ~/.ssh (looked for id_ed25519, id_rsa).";

/// `client::Handler` for the one-shot probe.
///
/// Host-key verification is intentionally skipped (`check_server_key` always
/// returns `Ok(true)`): this is a diagnostic probe equivalent to the app's
/// previous `ssh -T -o StrictHostKeyChecking=accept-new` behavior and it must
/// never block on an interactive "yes/no" prompt. Production connections
/// should verify against known_hosts instead.
struct Client;

impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

/// Connect to `host` on `port` and attempt public-key authentication using the
/// first default key present in `~/.ssh` (id_ed25519, then id_rsa). Returns a
/// human-readable success/error message suitable for display in the Qt UI.
///
/// This is a synchronous wrapper: it builds a one-shot Tokio current-thread
/// runtime and blocks on the async probe inside a hard 10s timeout, so a hung
/// connection can never block the caller indefinitely.
pub fn test_ssh_connection(host: &str, port: u16) -> VcsResult<String> {
    let (user, host_name) = split_user_host(host);

    let key_path = default_key_paths()
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| VcsError::Other(NO_SSH_KEY_MESSAGE.to_string()))?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let outcome = rt.block_on(async {
        tokio::time::timeout(CONNECT_TIMEOUT, probe_connection(&user, &host_name, port, &key_path))
            .await
    });

    match outcome {
        Err(_elapsed) => Err(timeout_error()),
        Ok(Ok(true)) => Ok(format!("Successfully authenticated to {host_name} as {user}.")),
        Ok(Ok(false)) => Err(VcsError::Other(format!(
            "Authentication failed for {user}@{host_name}."
        ))),
        Ok(Err(err)) => Err(err),
    }
}

/// Connect to `host` and attempt public-key authentication, returning whether
/// the server accepted the key. Every failure is mapped to a user-facing
/// `VcsError` message here, keeping the sync wrapper free of russh types.
async fn probe_connection(user: &str, host: &str, port: u16, key_path: &Path) -> VcsResult<bool> {
    // `None` = the key is unencrypted. BatchMode forbids passphrase prompts.
    let key_pair = load_secret_key(key_path, None).map_err(|e| {
        VcsError::Other(format!(
            "Failed to load SSH key at {}: {e}",
            key_path.display()
        ))
    })?;

    let config = Arc::new(client::Config::default());
    let mut session = client::connect(config, (host, port), Client)
        .await
        .map_err(|e| VcsError::Other(format!("Could not connect to {host}:{port}: {e}")))?;

    // Pick rsa-sha2-256/512 when the server supports it (matters for RSA keys).
    let hash_alg = session
        .best_supported_rsa_hash()
        .await
        .map_err(|e| VcsError::Other(format!("SSH negotiation failed for {user}@{host}: {e}")))?
        .flatten();

    let auth_res = session
        .authenticate_publickey(
            user,
            PrivateKeyWithHashAlg::new(Arc::new(key_pair), hash_alg),
        )
        .await
        .map_err(|e| VcsError::Other(format!("SSH authentication error for {user}@{host}: {e}")))?;

    // Best-effort clean shutdown; the result is irrelevant for a one-shot probe.
    let _ = session
        .disconnect(russh::Disconnect::ByApplication, "", "English")
        .await;

    Ok(auth_res.success())
}

/// Split a host string such as `git@github.com` or `github.com:2222` into
/// `(user, host)`. The part before the last `@` is the user (defaulting to
/// `git` when absent); a numeric `:port` suffix is stripped from the host so
/// the connect target matches the `port` argument.
fn split_user_host(host: &str) -> (String, String) {
    let (user, host_and_port) = match host.rsplit_once('@') {
        Some((user, rest)) => (user, rest),
        None => (DEFAULT_SSH_USER, host),
    };
    let user = if user.is_empty() {
        DEFAULT_SSH_USER.to_string()
    } else {
        user.to_string()
    };
    (user, strip_port(host_and_port).to_string())
}

/// Strip a trailing numeric `:port` from a host. Bare IPv6 literals (which
/// contain `::`) are left untouched; a bracketed `[addr]:port` form has both
/// the brackets and the port removed.
fn strip_port(host: &str) -> &str {
    if let Some(rest) = host.strip_prefix('[') {
        return match rest.split_once(']') {
            Some((addr, "")) => addr,
            Some((addr, suffix)) if suffix.starts_with(':') => addr,
            _ => host,
        };
    }
    if host.contains("::") {
        return host;
    }
    match host.rsplit_once(':') {
        Some((name, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => name,
        _ => host,
    }
}

/// Absolute paths of the default private keys in `~/.ssh`, in lookup order.
fn default_key_paths() -> Vec<PathBuf> {
    let ssh_dir = match std::env::var_os("HOME") {
        Some(home) => PathBuf::from(home).join(".ssh"),
        None => return Vec::new(),
    };
    DEFAULT_KEY_NAMES
        .iter()
        .map(|name| ssh_dir.join(name))
        .collect()
}

fn timeout_error() -> VcsError {
    VcsError::Other(format!(
        "Connection timed out after {} seconds.",
        CONNECT_TIMEOUT.as_secs()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_host_user_pairs() {
        assert_eq!(
            split_user_host("git@github.com"),
            ("git".to_string(), "github.com".to_string())
        );
        assert_eq!(
            split_user_host("github.com"),
            ("git".to_string(), "github.com".to_string())
        );
        // A numeric :port suffix is stripped for the connect target.
        assert_eq!(
            split_user_host("user@host:2222"),
            ("user".to_string(), "host".to_string())
        );
        assert_eq!(
            split_user_host("git@github.com:2222"),
            ("git".to_string(), "github.com".to_string())
        );
        // An empty user falls back to the default.
        assert_eq!(
            split_user_host("@github.com"),
            ("git".to_string(), "github.com".to_string())
        );
    }

    #[test]
    fn strip_port_handles_hostname_port_and_ipv6() {
        assert_eq!(strip_port("github.com"), "github.com");
        assert_eq!(strip_port("github.com:2222"), "github.com");
        assert_eq!(strip_port("host:"), "host:");
        // Bare IPv6 literals are not host:port forms.
        assert_eq!(strip_port("::1"), "::1");
        assert_eq!(strip_port("2001:db8::1"), "2001:db8::1");
        // Bracketed IPv6 with an explicit port.
        assert_eq!(strip_port("[::1]:2222"), "::1");
        assert_eq!(strip_port("[::1]"), "::1");
    }

    #[test]
    fn default_key_paths_are_ed25519_then_rsa() {
        let names: Vec<String> = default_key_paths()
            .iter()
            .map(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default()
            })
            .collect();
        assert_eq!(names, vec!["id_ed25519".to_string(), "id_rsa".to_string()]);
    }
}
