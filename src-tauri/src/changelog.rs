use tauri::{AppHandle, Manager};

/// Returns the bundled CHANGELOG.md body for one version: the lines between
/// its `## [version]` heading and the next `## [` heading, without the heading
/// itself, as raw markdown for the shared frontend viewer. CHANGELOG.md is the
/// single in-app source of update notes; markdown itself is the format
/// contract, so this command only owns the version-section boundaries. `None`
/// when the file or the version's section is missing.
#[tauri::command]
pub(crate) fn changelog_section(app: AppHandle, version: String) -> Option<String> {
    let path = app
        .path()
        .resolve("CHANGELOG.md", tauri::path::BaseDirectory::Resource)
        .ok()?;
    let text = std::fs::read_to_string(path).ok()?;
    version_section(&text, &version)
}

/// The closing bracket keeps `[0.3.3]` from satisfying a `[0.3.30]` request.
fn version_section(text: &str, version: &str) -> Option<String> {
    let heading = format!("## [{version}]");
    let mut lines = text.lines();
    loop {
        match lines.next() {
            Some(line) if line.starts_with(&heading) => break,
            Some(_) => continue,
            None => return None,
        }
    }
    let mut section = Vec::new();
    for line in lines {
        if line.starts_with("## [") {
            break;
        }
        section.push(line);
    }
    let body = section.join("\n").trim().to_string();
    if body.is_empty() { None } else { Some(body) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_section_cuts_at_the_next_version_heading() {
        let text = "# 更新日志\n\n## [0.3.2] - 2026-10-13\n\n### 修复\n\n- 旧条目。\n\n## [0.3.3] - 2026-10-14\n\n### 新功能\n\n- 新条目。\n";
        assert_eq!(
            version_section(text, "0.3.3").as_deref(),
            Some("### 新功能\n\n- 新条目。")
        );
    }

    #[test]
    fn version_lookup_rejects_missing_and_prefix_versions() {
        let text = "## [0.3.3] - 2026-10-14\n\n### 修复\n\n- 条目。\n";
        assert!(version_section(text, "0.3.4").is_none());
        // `[0.3.3]` must not satisfy a `[0.3.30]` request.
        assert!(version_section(text, "0.3.30").is_none());
        assert!(version_section(text, "0.3.3").is_some());
    }
}
