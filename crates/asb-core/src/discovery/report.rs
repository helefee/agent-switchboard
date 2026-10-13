use crate::contracts::{
    AppKind, CodexProviderDraft, CodexUpstream, LocalizedMessage, ProviderDraft, RouteState,
};

/// Paths the caller wants inspected, as display strings.
pub struct DiscoveryPaths {
    pub codex: String,
    pub codex_auth: String,
    pub claude: String,
}

/// What we found for one file.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredFile {
    pub app: AppKind,
    pub path: String,
    pub exists: bool,
    pub state: DiscoveredState,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum DiscoveredState {
    Missing,
    /// The canned read failure is app-owned and localized; the raw io detail
    /// stays in the backend log.
    ReadError {
        message: LocalizedMessage,
    },
    /// The parser's own diagnostic is an external message and stays raw by
    /// the command-error contract (no catalog entry).
    ParseError {
        message: String,
        line: Option<usize>,
    },
    Ok {
        route: RouteState,
        /// Whether the file already contains app-managed keys.
        managed: bool,
        /// Readiness warnings (unsupported shapes, plaintext secrets…).
        warnings: Vec<LocalizedMessage>,
        /// Whether this exact configuration shape can be converted into an
        /// app-owned profile without dropping settings.
        importable: bool,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryReport {
    pub codex: DiscoveredFile,
    pub claude: DiscoveredFile,
    pub codex_import_proposals: Vec<CodexImportProposal>,
    pub claude_import_proposals: Vec<ClaudeImportProposal>,
}

/// Renderer-safe facts for importing the current third-party Codex route.
/// The live config is re-read by the import command; this proposal contains
/// no API key and no catalog endpoint, so cached discovery cannot become a
/// credential or network-target store.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexImportProposal {
    pub name: String,
    pub provider_name: String,
    pub model: Option<String>,
    pub upstream: Option<CodexUpstream>,
    pub catalog_model_count: usize,
    pub api_key_available: bool,
    pub official: bool,
    pub basis: LocalizedMessage,
}

/// Internal result of parsing the live Codex files. Only the safe proposal is
/// serializable; the draft remains inside the backend until it is persisted.
#[derive(Debug, Clone, PartialEq)]
pub enum CodexImportAction {
    Official,
    ThirdParty(CodexProviderDraft),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodexImportSource {
    pub proposal: CodexImportProposal,
    pub action: CodexImportAction,
}

/// A suggested Claude provider profile derived read-only from discovered
/// content. Codex has a separate complete profile contract.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeImportProposal {
    pub draft: ProviderDraft,
    pub basis: LocalizedMessage,
}

impl DiscoveryReport {
    /// The copy persisted in the app-owned discovery cache: identical display
    /// facts with credentials cleared. Import always re-derives the draft from
    /// the live files, so a cached proposal never needs the secret.
    pub fn cached_display(&self) -> DiscoveryReport {
        let mut copy = self.clone();
        for proposal in &mut copy.claude_import_proposals {
            proposal.draft.api_key = String::new();
        }
        copy
    }
}
