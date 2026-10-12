use super::Extra;

pub const EFFORT_LEVELS: &[&str] = &["low", "medium", "high", "xhigh"];

pub(super) fn validate(extra: &Extra) -> Result<(), String> {
    if extra.contains_key("effortLevel") {
        return Err("effortLevel 顶层配置已移除；请使用 modelSettings.<model>.effortLevel，持久化强度仅支持 low、medium、high、xhigh".into());
    }
    let Some(settings) = extra.get("modelSettings") else { return Ok(()); };
    let settings = settings.as_object().ok_or("modelSettings 必须是以模型标识为键的对象")?;
    for (model, preferences) in settings {
        if model.trim().is_empty() || model != model.trim() || model.chars().any(char::is_control) {
            return Err("modelSettings 的模型标识必须是非空文本且不能带首尾空白".into());
        }
        let preferences = preferences.as_object()
            .ok_or_else(|| format!("modelSettings.{model} 必须是对象"))?;
        if let Some(effort) = preferences.get("effortLevel") {
            if !effort.as_str().is_some_and(|level| EFFORT_LEVELS.contains(&level)) {
                return Err(format!("modelSettings.{model}.effortLevel 仅支持 low、medium、high、xhigh；max 不能保存为模型默认值"));
            }
        }
    }
    Ok(())
}
