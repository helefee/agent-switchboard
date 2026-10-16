//! The native rename journal for rollout sessions not indexed in the native registry.
use std::collections::HashMap;
use std::ops::ControlFlow;
use std::path::Path;

pub(super) fn load(root: &Path) -> (HashMap<String, String>, Vec<String>) {
    let path = root.join("session_index.jsonl");
    let mut titles = HashMap::new();
    match path.try_exists() {
        Ok(false) => return (titles, Vec::new()),
        Err(error) => return (titles, vec![format!("会话名称索引不可读: {error}")]),
        Ok(true) => {}
    }
    let scan = super::records::visit_records(&path, 0, |_, _, value| {
        let id = value.get("id").and_then(|value| value.as_str()).map(str::trim);
        let name = value.get("thread_name").and_then(|value| value.as_str()).map(str::trim);
        if let (Some(id), Some(name)) = (id, name) {
            if super::parser::valid_session_id(id) {
                if name.is_empty() { titles.remove(id); }
                else { titles.insert(id.to_string(), name.to_string()); }
            }
        }
        Ok(ControlFlow::Continue(()))
    });
    let issues = match scan { Ok(issues) => issues, Err(error) => vec![error] };
    (titles, issues)
}
