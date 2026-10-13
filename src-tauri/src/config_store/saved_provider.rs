use super::{codex_providers, ConfigStore, StoreOperationError};
use asb_core::contracts::{AppKind, CodexProviderFile, ProviderProfile, ProviderRecord};

/// Preserves the native file contract and revision across read-only consumers.
pub(crate) enum SavedProvider {
    Codex { file: CodexProviderFile, revision: String },
    Client(ProviderRecord),
}

impl SavedProvider {
    pub(crate) fn profile(&self) -> ProviderProfile {
        match self {
            Self::Codex { file, .. } => file.client_projection().into_profile(AppKind::Codex),
            Self::Client(record) => record.profile.clone(),
        }
    }

    pub(crate) fn revision(&self) -> &str {
        match self {
            Self::Codex { revision, .. } => revision,
            Self::Client(record) => &record.file_hash,
        }
    }
}

impl ConfigStore {
    pub(crate) fn resolve_saved_provider(&self, id: &str) -> Result<SavedProvider, StoreOperationError> {
        let codex = codex_providers::load(self)?;
        if let Some(loaded) = codex.into_iter().find(|loaded| loaded.file.profile.id == id) {
            return Ok(SavedProvider::Codex { file: loaded.file, revision: loaded.hash });
        }
        self.find_provider_record(id).map(SavedProvider::Client)
    }
}
