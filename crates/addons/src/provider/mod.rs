//! Provider abstraction (ADDON_SPEC §10.1) and provider error type.

use std::sync::Arc;

use crate::error::AddonError;
use crate::package::{AddonDescriptor, PackageSource};

/// Errors raised by a provider, normalized into [`AddonError::provider`].
#[derive(Debug, Clone, thiserror::Error)]
pub enum ProviderError {
    #[error("{0}")]
    NotSupported(String),
    #[error("{0}")]
    ReadOnly(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    InvalidAddon(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("{0}")]
    Internal(String),
}

impl From<ProviderError> for AddonError {
    fn from(e: ProviderError) -> Self {
        AddonError::provider("local", e.to_string())
    }
}

impl ProviderError {
    pub fn from_addon(e: AddonError) -> Self {
        match e {
            AddonError::ProviderError { message, .. } => ProviderError::Internal(message),
            AddonError::InvalidAddon { message, .. } => ProviderError::InvalidAddon(message),
            other => ProviderError::Internal(other.to_string()),
        }
    }
}

/// A source of addon packages. Providers are swappable without UI changes:
/// the registry exposes only descriptors through the FFI.
pub trait AddonProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn display_name(&self) -> String;

    /// Scan all sources known to this provider; return validated descriptors
    /// (including `shadowed`/`error`/`conflict` states for the UI).
    fn load(&self) -> Result<Vec<AddonDescriptor>, ProviderError>;

    /// Install a package source; returns the installed descriptor.
    fn install(&self, source: &PackageSource) -> Result<AddonDescriptor, ProviderError>;

    /// Remove an installed addon. Read-only providers return
    /// [`ProviderError::ReadOnly`].
    fn uninstall(&self, id: &str) -> Result<(), ProviderError>;

    /// Open an installed addon for execution/asset access.
    fn open(&self, id: &str) -> Result<Arc<crate::package::LoadedPackage>, ProviderError>;

    /// Advisory flag; the UI uses it to hide destructive actions.
    fn is_read_only(&self) -> bool {
        false
    }
}

/// Build a `PackageSource` from a host-supplied path.
pub fn package_source_from_path(path: &std::path::Path) -> PackageSource {
    if path.is_dir() {
        PackageSource::Directory {
            path: path.to_path_buf(),
        }
    } else if path.extension().and_then(|e| e.to_str()) == Some("lzd") {
        PackageSource::Lzd {
            path: path.to_path_buf(),
        }
    } else {
        PackageSource::Zip {
            path: path.to_path_buf(),
        }
    }
}

pub mod local;
pub mod remote;

/// Convenience alias used by callers.
pub type DynProvider = Arc<dyn AddonProvider>;
