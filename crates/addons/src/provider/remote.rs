//! RemoteProvider — design-ready stub (ADDON_SPEC §10.3).
//!
//! In v1 every operation fails explicitly with `NotSupported` (never a silent
//! no-op). The trait seam is in place so a signed-registry implementation can
//! be added without UI changes.

use std::sync::Arc;

use crate::package::{AddonDescriptor, LoadedPackage, PackageSource};
use crate::provider::{AddonProvider, ProviderError};

#[derive(Debug, Clone, Copy)]
pub struct RemoteProvider;

impl RemoteProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RemoteProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AddonProvider for RemoteProvider {
    fn provider_id(&self) -> &str {
        "remote"
    }

    fn display_name(&self) -> String {
        "Remote".to_string()
    }

    fn load(&self) -> Result<Vec<AddonDescriptor>, ProviderError> {
        Err(ProviderError::NotSupported(
            "remote addon provider is not available in this version".to_string(),
        ))
    }

    fn install(&self, _source: &PackageSource) -> Result<AddonDescriptor, ProviderError> {
        Err(ProviderError::NotSupported(
            "remote addon provider is not available in this version".to_string(),
        ))
    }

    fn uninstall(&self, _id: &str) -> Result<(), ProviderError> {
        Err(ProviderError::NotSupported(
            "remote addon provider is not available in this version".to_string(),
        ))
    }

    fn open(&self, _id: &str) -> Result<Arc<LoadedPackage>, ProviderError> {
        Err(ProviderError::NotSupported(
            "remote addon provider is not available in this version".to_string(),
        ))
    }
}
