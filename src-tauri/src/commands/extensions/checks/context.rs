use std::path::PathBuf;
use asb_core::contracts::AppKind;
use asb_core::extensions::contracts::{ExtensionDefinition, ExtensionTarget, McpDefinition};
use crate::commands::error::CommandError;
use crate::commands::extensions::support::store_error;
use crate::extensions::{checks::CheckRequest, discovery::DiscoveredPaths, store::ExtensionStore};

pub(super) fn request_for(
    store: &ExtensionStore, definition: &ExtensionDefinition, mcp: &McpDefinition,
    target: ExtensionTarget, expected_revision: u64,
) -> Result<CheckRequest, CommandError> {
    if definition.revision != expected_revision {
        return Err(CommandError::keyed("extension-conflict", "errors.extops.checkRevisionChanged",
            "扩展定义已变化；刷新后再检测"));
    }
    if !mcp.supports_client(target.client()) || matches!(target,
        ExtensionTarget::ProjectPrivate { client: AppKind::Codex, .. }) {
        return Err(CommandError::keyed("extension-invalid", "errors.extops.checkTargetUnsupported",
            "该 MCP 不支持所选客户端或作用域"));
    }
    let base = if let Some(project_id) = target.project_id() {
        let project = store.list_projects().map_err(store_error)?.into_iter()
            .find(|project| project.id == project_id).ok_or_else(|| CommandError::keyed(
                "extension-not-found", "errors.extops.projectNotFound", "所选项目不存在或已取消注册"))?;
        PathBuf::from(project.root)
    } else {
        DiscoveredPaths::from_env().map_err(|error| CommandError::new("app-state-unavailable", error))?.home
    };
    let working_dir = match (mcp, target.client()) {
        (McpDefinition::Stdio { codex_options: Some(options), .. }, AppKind::Codex) =>
            options.cwd.as_ref().map(|cwd| base.join(cwd)).unwrap_or(base),
        _ => base,
    };
    if !working_dir.is_dir() {
        return Err(CommandError::keyed("extension-invalid", "errors.extops.checkWorkingDirectoryInvalid",
            "检测工作目录不存在或无法访问"));
    }
    Ok(CheckRequest {
        definition_id: definition.id.clone(), definition_revision: definition.revision,
        target, working_dir,
    })
}

#[cfg(test)]
mod tests;
