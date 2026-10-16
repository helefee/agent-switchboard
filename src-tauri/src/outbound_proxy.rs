//! Outbound proxy policy for application-owned HTTP clients.
//!
//! One owner of the egress contract: none (always direct), system (follow the
//! environment, minus a self-loop guard) or manual (an explicit http/https/
//! socks5/socks5h URL, credentials included). Gateway upstreams, probes and
//! provider requests all build through [`apply`] so a settings change is
//! picked up on the next client build; the revision-cached client registry
//! makes that immediate for long-lived holders.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

mod runtime;
pub(crate) use runtime::{apply, apply_blocking, cached_blocking_client, cached_client, note_gateway_port};

static MUTATIONS: Mutex<()> = Mutex::new(());

const VERSION: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum OutboundProxyMode {
    #[default]
    None,
    System,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OutboundProxySettings {
    pub version: u64,
    pub mode: OutboundProxyMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub revision: String,
}
impl Default for OutboundProxySettings {
    fn default() -> Self {
        Self {
            version: VERSION,
            mode: OutboundProxyMode::None,
            url: None,
            revision: uuid::Uuid::nil().to_string(),
        }
    }
}

/// One invalid or failed outbound-proxy settings operation. The `Display`
/// text is the scrubbed diagnostic detail; `parts` carries the renderer
/// translation coordinates.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum OutboundProxyError {
    #[error("无法读取出站代理设置：{0}")]
    Read(String),
    #[error("出站代理设置文件无效")]
    Unreadable,
    #[error("无法写入出站代理设置：{0}")]
    Write(String),
    #[error("出站代理设置已改变，请重新读取后再保存")]
    RevisionConflict,
    #[error("出站代理设置版本不受支持")]
    UnsupportedVersion,
    #[error("出站代理设置修订无效")]
    InvalidRevision,
    #[error("直连模式不能携带代理地址")]
    DirectUrlForbidden,
    #[error("系统代理模式不能携带代理地址")]
    SystemUrlForbidden,
    #[error("手动代理需要代理地址")]
    ManualUrlRequired,
    #[error("代理地址过长或含有控制字符")]
    UrlTooLong,
    #[error("代理协议必须是 http、https、socks5 或 socks5h：{0}")]
    UrlScheme(String),
    #[error("代理地址无效：{0}")]
    UrlInvalid(String),
}

impl OutboundProxyError {
    pub(crate) fn parts(&self) -> (&'static str, serde_json::Value) {
        match self {
            Self::Read(detail) | Self::Write(detail) => {
                ("errors.outboundProxy.unavailable", serde_json::json!({ "detail": detail }))
            }
            Self::Unreadable => ("errors.outboundProxy.unreadable", serde_json::json!({})),
            Self::RevisionConflict => ("errors.outboundProxy.revisionConflict", serde_json::json!({})),
            Self::UnsupportedVersion => ("errors.outboundProxy.versionUnsupported", serde_json::json!({})),
            Self::InvalidRevision => ("errors.outboundProxy.revisionInvalid", serde_json::json!({})),
            Self::DirectUrlForbidden => ("errors.outboundProxy.directUrlForbidden", serde_json::json!({})),
            Self::SystemUrlForbidden => ("errors.outboundProxy.systemUrlForbidden", serde_json::json!({})),
            Self::ManualUrlRequired => ("errors.outboundProxy.manualUrlRequired", serde_json::json!({})),
            Self::UrlTooLong => ("errors.outboundProxy.urlTooLong", serde_json::json!({})),
            Self::UrlScheme(scheme) => ("errors.outboundProxy.urlScheme", serde_json::json!({ "scheme": scheme })),
            Self::UrlInvalid(detail) => ("errors.outboundProxy.urlInvalid", serde_json::json!({ "detail": detail })),
        }
    }

    /// Read/write failures mean the settings surface is unavailable, not that
    /// the submitted values were invalid.
    pub(crate) fn is_io(&self) -> bool {
        matches!(self, Self::Read(_) | Self::Unreadable | Self::Write(_))
    }
}

impl OutboundProxySettings {
    pub(crate) fn validate(&self) -> Result<(), OutboundProxyError> {
        if self.version != VERSION {
            return Err(OutboundProxyError::UnsupportedVersion);
        }
        if uuid::Uuid::parse_str(&self.revision).is_err() {
            return Err(OutboundProxyError::InvalidRevision);
        }
        match self.mode {
            OutboundProxyMode::None => {
                if self.url.is_some() {
                    return Err(OutboundProxyError::DirectUrlForbidden);
                }
            }
            OutboundProxyMode::System => {
                if self.url.is_some() {
                    return Err(OutboundProxyError::SystemUrlForbidden);
                }
            }
            OutboundProxyMode::Manual => {
                let Some(url) = self.url.as_deref().map(str::trim).filter(|u| !u.is_empty()) else {
                    return Err(OutboundProxyError::ManualUrlRequired);
                };
                if url.len() > 2048 || url.chars().any(char::is_control) {
                    return Err(OutboundProxyError::UrlTooLong);
                }
                // This module owns the accepted scheme set; reqwest then owns
                // the rest of the grammar (host, port, credentials).
                const SCHEMES: [&str; 4] = ["http", "https", "socks5", "socks5h"];
                let scheme = url.split("://").next().unwrap_or_default();
                if !SCHEMES.contains(&scheme) {
                    return Err(OutboundProxyError::UrlScheme(scheme.to_string()));
                }
                reqwest::Proxy::all(url)
                    .map_err(|error| OutboundProxyError::UrlInvalid(error.to_string()))?;
            }
        }
        Ok(())
    }
}

pub(crate) fn read(state_root: &Path) -> Result<OutboundProxySettings, OutboundProxyError> {
    let path = settings_path(state_root);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(OutboundProxySettings::default());
        }
        Err(error) => return Err(OutboundProxyError::Read(error.to_string())),
    };
    let settings: OutboundProxySettings =
        serde_json::from_slice(&bytes).map_err(|_| OutboundProxyError::Unreadable)?;
    settings.validate()?;
    Ok(settings)
}

pub(crate) fn save(
    state_root: &Path,
    settings: OutboundProxySettings,
    expected_revision: &str,
) -> Result<OutboundProxySettings, OutboundProxyError> {
    settings.validate()?;
    let _guard = MUTATIONS.lock().map_err(|_| OutboundProxyError::Write("出口设置写入锁不可用".into()))?;
    let current = read(state_root)?;
    if current.revision != expected_revision {
        return Err(OutboundProxyError::RevisionConflict);
    }
    if current == settings && current.mode != OutboundProxyMode::System {
        return Ok(current);
    }
    let mut saved = settings;
    saved.revision = uuid::Uuid::new_v4().to_string();
    let path = settings_path(state_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| OutboundProxyError::Write(error.to_string()))?;
    }
    let temporary = path.with_extension("tmp");
    std::fs::write(
        &temporary,
        serde_json::to_vec_pretty(&saved).map_err(|_| OutboundProxyError::Write("编码失败".into()))?,
    )
    .map_err(|error| OutboundProxyError::Write(error.to_string()))?;
    std::fs::rename(&temporary, &path)
        .map_err(|error| OutboundProxyError::Write(error.to_string()))?;
    runtime::update(&saved);
    Ok(saved)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OutboundProxyRepairReport {
    pub settings: OutboundProxySettings,
    pub backup_path: Option<String>,
}

/// Rechecks before repair: valid settings survive, unreadable files are
/// reported, and invalid bytes are retained before returning to direct mode.
pub(crate) fn repair(state_root: &Path) -> Result<OutboundProxyRepairReport, OutboundProxyError> {
    let _guard = MUTATIONS.lock().map_err(|_| OutboundProxyError::Write("出口设置写入锁不可用".into()))?;
    match read(state_root) {
        Ok(settings) => {
            runtime::update(&settings);
            return Ok(OutboundProxyRepairReport { settings, backup_path: None });
        }
        Err(error) if error.is_io() && error != OutboundProxyError::Unreadable => return Err(error),
        Err(_) => {}
    }
    let backup = state_root.join(format!("outbound-proxy-repair-backup-{}.json", uuid::Uuid::new_v4()));
    std::fs::rename(settings_path(state_root), &backup)
        .map_err(|error| OutboundProxyError::Write(error.to_string()))?;
    let settings = OutboundProxySettings::default();
    runtime::update(&settings);
    Ok(OutboundProxyRepairReport { settings, backup_path: Some(backup.display().to_string()) })
}

/// The only startup projection of persisted egress settings.
pub(crate) fn load(state_root: &Path) -> Result<(), String> {
    let settings = read(state_root).map_err(|error| error.to_string())?;
    runtime::update(&settings);
    Ok(())
}

fn settings_path(state_root: &Path) -> PathBuf {
    state_root.join("outbound-proxy.json")
}

