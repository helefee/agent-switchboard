use super::*;

/// Model preferences belong to the provider route. Restore the previous
/// route's declared model leaves while retaining current non-model settings.
pub fn restore_profile_models(current: &str, before: &str) -> Result<String, String> {
    let current_root: Value = serde_json::from_str(current).map_err(|_| "Claude 配置不是有效 JSON")?;
    let before_root: Value = serde_json::from_str(before).map_err(|_| "Claude 备份不是有效 JSON")?;
    let mut desired = Value::Object(Map::new());
    for (root, restore_models) in [(&current_root, false), (&before_root, true)] {
        for path in declared(root, PROFILE_MANIFEST)? {
            let segments = pointer::decode(&path)?;
            if segments.first().is_some_and(|key| key == "modelSettings") != restore_models {
                continue;
            }
            if let Some(value) = root.pointer(&path) {
                pointer::set(&mut desired, &segments, value.clone())?;
            }
        }
    }
    apply_profile(current, desired.as_object().expect("object root"))
}
