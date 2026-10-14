use super::{organization, parser::visit_messages, resolve_session, SessionMessage, SessionMeta};
use asb_core::contracts::AppKind;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

struct ExportFile(PathBuf);

impl Drop for ExportFile {
    fn drop(&mut self) { let _ = fs::remove_file(&self.0); }
}

pub fn export_markdown(root: &Path, app: AppKind, session_id: &str, target: &Path) -> Result<(), String> {
    let mut source = resolve_session(app, session_id)?;
    if target.exists() && target.canonicalize().map_err(export_error)? == source.path.canonicalize().map_err(export_error)? {
        return Err("导出目标不能覆盖原会话记录".into());
    }
    organization::apply(&mut source.meta, &organization::load(root)?);
    let parent = target.parent().ok_or("导出位置无效")?;
    let temporary = ExportFile(parent.join(format!(".asb-export-{}.tmp", uuid::Uuid::new_v4())));
    let mut file = File::create_new(&temporary.0).map_err(export_error)?;
    {
        let mut writer = BufWriter::new(&mut file);
        write_header(&mut writer, &source.meta).map_err(export_error)?;
        visit_messages(&source.path, |message| {
            write_message(&mut writer, &message).map_err(export_error)?;
            Ok(ControlFlow::Continue(()))
        })?;
        writer.flush().map_err(export_error)?;
    }
    file.sync_all().map_err(export_error)?;
    drop(file);
    fs::rename(&temporary.0, target).map_err(export_error)
}

fn write_header(writer: &mut impl Write, meta: &SessionMeta) -> std::io::Result<()> {
    writeln!(writer, "# {}\n\n- 来源：{}\n- 会话 ID：{}",
        escape(meta.alias.as_deref().unwrap_or(&meta.title)), meta.app.label(), escape(&meta.session_id))?;
    for (label, value) in [
        ("项目", meta.project_dir.as_deref()),
        ("创建时间", meta.created_at.as_deref()),
        ("最后活动", meta.last_active_at.as_deref()),
    ] {
        if let Some(value) = value { writeln!(writer, "- {label}：{}", escape(value))?; }
    }
    Ok(())
}

fn write_message(writer: &mut impl Write, message: &SessionMessage) -> std::io::Result<()> {
    write!(writer, "\n---\n\n## {}", escape(&message.role))?;
    if let Some(at) = &message.at { write!(writer, " · {}", escape(at))?; }
    writeln!(writer, "\n\n{}", message.content)
}

fn export_error(error: std::io::Error) -> String { format!("导出文件写入失败：{error}") }

fn escape(text: &str) -> String {
    text.chars().flat_map(|character| {
        if "\\`*_{}[]<>()#+-.!|".contains(character) { vec!['\\', character] }
        else if character == '\n' || character == '\r' { vec![' '] }
        else { vec![character] }
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_complete_markdown_content_and_escapes_metadata() {
        let meta = SessionMeta { app: AppKind::Codex, session_id: "s-1".into(),
            title: "[title]".into(), summary: String::new(), project_dir: None,
            created_at: Some("2026-09-22T00:00:00Z".into()), last_active_at: None,
            resume_command: String::new(), alias: None, pinned: false, tags: Vec::new() };
        let content = "```rust\nlet x = 1;\n```\n\n完整正文";
        let mut output = Vec::new();
        write_header(&mut output, &meta).unwrap();
        write_message(&mut output, &SessionMessage { id: "id".into(),
            role: "assistant".into(), content: content.into(), at: None }).unwrap();
        let markdown = String::from_utf8(output).unwrap();
        assert!(markdown.starts_with("# \\[title\\]"));
        assert!(markdown.contains(content));
        assert!(markdown.contains("来源：Codex"));
        assert!(markdown.contains("创建时间：2026"));
    }
}
