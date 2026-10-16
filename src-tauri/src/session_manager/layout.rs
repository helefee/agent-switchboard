use std::path::{Path, PathBuf};

pub(super) fn directory(state_root: &Path) -> PathBuf { state_root.join("sessions") }

/// Retires the former double-state layout before any session store opens.
/// Moving the complete directory preserves SQLite journals together with their databases.
pub(crate) fn initialize_storage(state_root: &Path) -> Result<(), String> {
    let previous = state_root.join("state").join("sessions");
    let current = directory(state_root);
    if !previous.try_exists().map_err(|error| error.to_string())? { return Ok(()); }
    if current.try_exists().map_err(|error| error.to_string())? {
        return Err("两处会话整理目录同时存在，已保留两份数据；需要明确合并方向后再启用".into());
    }
    let root = state_root.canonicalize().map_err(|error| error.to_string())?;
    let original = previous.canonicalize().map_err(|error| error.to_string())?;
    if !original.starts_with(&root) || previous.is_symlink() || !previous.is_dir() {
        return Err("原会话整理目录超出应用状态范围或不是普通目录，未移动原件".into());
    }
    let target = root.join("sessions");
    crate::storage::publish::move_new(&original, &target)
        .map_err(|error| format!("会话整理目录迁移未完成；请检查新旧目录，材料未主动删除: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_the_complete_namespace_once_and_keeps_sqlite_journal_material() {
        let temp = tempfile::tempdir().unwrap();
        let previous = temp.path().join("state/sessions");
        std::fs::create_dir_all(&previous).unwrap();
        for name in ["collections.sqlite3", "collections.sqlite3-wal", "organization.sqlite3"] {
            std::fs::write(previous.join(name), name.as_bytes()).unwrap();
        }
        initialize_storage(temp.path()).unwrap();
        initialize_storage(temp.path()).unwrap();
        assert!(!previous.exists());
        for name in ["collections.sqlite3", "collections.sqlite3-wal", "organization.sqlite3"] {
            assert_eq!(std::fs::read(directory(temp.path()).join(name)).unwrap(), name.as_bytes());
        }
    }

    #[test]
    fn conflicting_namespaces_are_preserved_without_guessing_a_winner() {
        let temp = tempfile::tempdir().unwrap();
        let previous = temp.path().join("state/sessions");
        std::fs::create_dir_all(&previous).unwrap();
        std::fs::create_dir_all(directory(temp.path())).unwrap();
        std::fs::write(previous.join("original"), b"keep").unwrap();
        assert!(initialize_storage(temp.path()).is_err());
        assert_eq!(std::fs::read(previous.join("original")).unwrap(), b"keep");
    }
}
