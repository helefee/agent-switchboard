use asb_core::extensions::contracts::{McpDefinition, SecretValue};
use std::collections::BTreeMap;

pub(super) struct CheckMaterial {
    pub env: BTreeMap<String, String>,
    pub headers: BTreeMap<String, String>,
    pub bearer: Option<String>,
}

pub(super) fn material_for(
    definition: &McpDefinition,
    resolve: &dyn Fn(&str) -> Option<String>,
) -> Result<CheckMaterial, String> {
    let resolve_value = |value: &SecretValue| -> Result<String, String> {
        match value {
            SecretValue::EnvRef { name } =>
                std::env::var(name).map_err(|_| format!("环境变量 {name} 无法解析")),
            SecretValue::Plain { value } => Ok(value.clone()),
            SecretValue::SecretRef { reference } =>
                resolve(reference).ok_or_else(|| format!("凭据引用 {reference} 无法解析")),
        }
    };
    let mut material = CheckMaterial {
        env: BTreeMap::new(), headers: BTreeMap::new(), bearer: None,
    };
    match definition {
        McpDefinition::Stdio { env, .. } => {
            for (name, value) in env {
                material.env.insert(name.clone(), resolve_value(value)?);
            }
        }
        McpDefinition::Http { headers, bearer, .. } => {
            for (name, value) in headers {
                material.headers.insert(name.clone(), resolve_value(value)?);
            }
            material.bearer = bearer.as_ref().map(resolve_value).transpose()?;
        }
        McpDefinition::ClaudeSse { .. } | McpDefinition::ClaudeWs { .. } => {}
    }
    Ok(material)
}
