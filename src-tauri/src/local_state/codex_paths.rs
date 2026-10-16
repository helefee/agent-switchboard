//! Codex native paths share one root; resolving them never creates files.

use std::path::{Path, PathBuf};

pub(crate) fn root_in_home(home: &Path, override_root: Option<&Path>) -> PathBuf {
    override_root
        .filter(|path| !path.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| home.join(".codex"))
}

pub(crate) fn session_roots(root: &Path) -> [PathBuf; 2] {
    [root.join("sessions"), root.join("archived_sessions")]
}

pub(crate) fn sqlite_root(root: &Path) -> Result<PathBuf, String> {
    let text = match std::fs::read_to_string(root.join("config.toml")) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.to_string()),
    };
    let document = text.parse::<toml_edit::DocumentMut>().map_err(|error| {
        let line = error.span().map(|span| text.bytes().take(span.start).filter(|byte| *byte == b'\n').count() + 1);
        format!("Codex 配置 TOML 语法无效（行 {}），不能读取 sqlite_home；请检查 config.toml", line.unwrap_or(1))
    })?;
    let configured = document.get("sqlite_home").map(|value|
        value.as_str().ok_or("sqlite_home 必须是字符串")).transpose()?;
    let environment = std::env::var_os("CODEX_SQLITE_HOME").map(PathBuf::from);
    resolve_sqlite_root(root, configured, environment.as_deref())
}

fn resolve_sqlite_root(root: &Path, configured: Option<&str>, environment: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(value) = configured {
        if value == "~" { return super::user_home_dir(); }
        if let Some(rest) = value.strip_prefix("~/").or_else(|| value.strip_prefix("~\\")) {
            return Ok(super::user_home_dir()?.join(rest));
        }
        let path = PathBuf::from(value);
        return Ok(if path.is_absolute() { path } else { root.join(path) });
    }
    if let Some(path) = environment.filter(|path| !path.as_os_str().is_empty()) {
        let value = path.to_str().ok_or("CODEX_SQLITE_HOME 不是有效文本路径")?.trim();
        if !value.is_empty() {
            let path = if value == "~" { super::user_home_dir()? }
                else if let Some(rest) = value.strip_prefix("~/").or_else(|| value.strip_prefix("~\\")) { super::user_home_dir()?.join(rest) }
                else { PathBuf::from(value) };
            if !path.is_absolute() { return Err("CODEX_SQLITE_HOME 是相对路径，无法确定 Codex 的原始启动目录；请使用绝对路径以读取完整原生历史".into()); }
            return Ok(path);
        }
    }
    Ok(root.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_roots_never_include_the_default_home() {
        let temporary = tempfile::tempdir().unwrap();
        let home = temporary.path().join("home");
        let first = temporary.path().join("first");
        let second = temporary.path().join("second");
        for root in [&first, &second] {
            let resolved = root_in_home(&home, Some(root));
            assert_eq!(resolved, *root);
            assert_eq!(
                session_roots(&resolved),
                [root.join("sessions"), root.join("archived_sessions")]
            );
            assert!(!resolved.exists(), "path resolution is read-only");
            assert!(!resolved.starts_with(&home));
        }
    }

    #[test]
    fn empty_overrides_use_the_native_default() {
        let home = Path::new("fixture-home");
        assert_eq!(root_in_home(home, None), home.join(".codex"));
        assert_eq!(root_in_home(home, Some(Path::new(""))), home.join(".codex"));
    }

    #[test]
    fn sqlite_config_owns_precedence_and_relative_paths_use_its_directory() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("codex");
        let environment = temp.path().join("environment");
        assert_eq!(resolve_sqlite_root(&root, Some("history"), Some(&environment)).unwrap(), root.join("history"));
        assert_eq!(resolve_sqlite_root(&root, Some(""), Some(&environment)).unwrap(), root);
        assert_eq!(resolve_sqlite_root(&root, None, Some(&environment)).unwrap(), environment);
        assert!(resolve_sqlite_root(&root, None, Some(Path::new("relative"))).is_err());
    }
}
