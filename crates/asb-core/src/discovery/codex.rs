use std::path::{Component, Path, PathBuf};

use serde_json::Value;
use toml_edit::{DocumentMut, Item, TableLike};

use crate::contracts::{
    default_model_limits, CodexCatalogEntry, CodexEndpoint, CodexProviderDraft,
    CodexReasoningLevel, CodexUpstream, LocalizedMessage, ResponsesRequestMode,
    CODEX_REASONING_LADDER, DEFAULT_CODEX_CAPABILITIES,
};
use crate::discovery::report::{CodexImportAction, CodexImportProposal, CodexImportSource};

/// Parses the current Codex config and companion auth/catalog files. The
/// returned draft is backend-only; callers expose only the proposal.
pub fn import_source(
    _config_path: &str,
    config_text: &str,
    auth_text: Option<&str>,
    catalog_text: Option<&str>,
) -> Result<CodexImportSource, LocalizedMessage> {
    let document = config_text.parse::<DocumentMut>().map_err(|_| {
        LocalizedMessage::new(
            "importDiscovery.discovery.codexConfigInvalid",
            serde_json::Value::Null,
            "Codex 配置无法解析",
        )
    })?;
    let provider = item_string(document.get("model_provider")).unwrap_or_else(|| "openai".into());
    let model = item_string(document.get("model"));
    let route = selected_route(&document, &provider)?;

    if route.official {
        return Ok(CodexImportSource {
            proposal: CodexImportProposal {
                name: "Codex 官方登录".into(),
                provider_name: provider,
                model: None,
                upstream: None,
                catalog_model_count: 0,
                api_key_available: false,
                official: true,
                basis: LocalizedMessage::new(
                    "importDiscovery.basis.codexOfficial",
                    serde_json::Value::Null,
                    "当前 Codex 配置使用官方登录；导入只创建无凭据的官方入口",
                ),
            },
            action: CodexImportAction::Official,
        });
    }

    let endpoint = route.endpoint.ok_or_else(|| {
        LocalizedMessage::new(
            "importDiscovery.discovery.codexMissingEndpoint",
            serde_json::Value::Null,
            "当前 Codex 自定义路由缺少服务地址",
        )
    })?;
    if crate::adapter::codex::is_gateway_base_url(&endpoint) {
        return Err(LocalizedMessage::new(
            "importDiscovery.discovery.codexGatewayManaged",
            serde_json::Value::Null,
            "当前 Codex 配置由本应用协议网关管理，不重复导入",
        ));
    }
    let default_model = model.ok_or_else(|| {
        LocalizedMessage::new(
            "importDiscovery.discovery.codexMissingModel",
            serde_json::Value::Null,
            "当前 Codex 自定义路由缺少主模型",
        )
    })?;
    let parameters =
        crate::adapter::read_provider_parameters(crate::contracts::AppKind::Codex, config_text)
            .map_err(|error| {
                LocalizedMessage::new(
                    "importDiscovery.discovery.codexUnsafeImport",
                    serde_json::json!({ "error": error.to_string() }),
                    format!("当前 Codex 配置无法安全导入：{error}"),
                )
            })?;
    let api_key = route
        .explicit_key
        .or_else(|| auth_api_key(auth_text))
        .unwrap_or_default();
    let catalog = catalog_entries(&default_model, catalog_text)?;
    let capabilities = DEFAULT_CODEX_CAPABILITIES;
    let draft = CodexProviderDraft {
        name: route.provider_name.clone(),
        endpoint: CodexEndpoint(endpoint),
        api_key,
        authentication: Some(route.upstream.protocol().authentication_scheme()),
        connection: Default::default(),
        upstream: route.upstream,
        request_mode: ResponsesRequestMode::Standard,
        default_model: default_model.clone(),
        catalog,
        model_routes: Vec::new(),
        subagent_route: None,
        capabilities,
        parameters,
        notes: None,
        website_url: None,
        usage_query: None,
    };
    let mut validation_draft = draft.clone();
    if validation_draft.api_key.is_empty() {
        // Discovery must remain informative for OAuth-only live configs, but
        // the specialized store still rejects an empty credential at save.
        validation_draft.api_key = "__missing_codex_credential__".to_string();
    }
    validation_draft
        .into_file("00000000-0000-0000-0000-000000000001".into(), 1)
        .validate()
        .map_err(|error| {
            LocalizedMessage::new(
                "importDiscovery.discovery.codexUnsafeImport",
                serde_json::json!({ "error": error.to_string() }),
                format!("当前 Codex 配置无法安全导入：{error}"),
            )
        })?;
    let proposal = CodexImportProposal {
        name: route.provider_name.clone(),
        provider_name: route.provider_name,
        model: Some(default_model),
        upstream: Some(draft.upstream),
        catalog_model_count: draft.catalog.len(),
        api_key_available: !draft.api_key.is_empty(),
        official: false,
        basis: LocalizedMessage::new(
            "importDiscovery.basis.codexThirdParty",
            serde_json::Value::Null,
            "由当前 Codex config.toml、auth.json 与模型目录生成",
        ),
    };
    Ok(CodexImportSource {
        proposal,
        action: CodexImportAction::ThirdParty(draft),
    })
}

/// Resolves a catalog pointer only inside the Codex config directory. Existing
/// symlinks are canonicalized before the containment check.
pub fn catalog_path(
    config_path: &str,
    config_text: &str,
) -> Result<Option<PathBuf>, LocalizedMessage> {
    let document = config_text.parse::<DocumentMut>().map_err(|_| {
        LocalizedMessage::new(
            "importDiscovery.discovery.codexConfigInvalid",
            serde_json::Value::Null,
            "Codex 配置无法解析",
        )
    })?;
    let Some(pointer) = item_string(document.get("model_catalog_json")) else {
        return Ok(None);
    };
    let config_path = Path::new(config_path);
    let directory = config_path.parent().ok_or_else(|| {
        LocalizedMessage::new(
            "importDiscovery.discovery.codexConfigDirInvalid",
            serde_json::Value::Null,
            "Codex 配置目录无效",
        )
    })?;
    let pointer_path = Path::new(&pointer);
    if pointer_path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(LocalizedMessage::new(
            "importDiscovery.discovery.codexCatalogParentRef",
            serde_json::Value::Null,
            "Codex model_catalog_json 不允许包含上级目录",
        ));
    }
    let candidate = if pointer_path.is_absolute() {
        pointer_path.to_path_buf()
    } else {
        directory.join(pointer)
    };
    let root = directory.canonicalize().map_err(|_| {
        LocalizedMessage::new(
            "importDiscovery.discovery.codexConfigDirUnresolved",
            serde_json::Value::Null,
            "Codex 配置目录无法解析",
        )
    })?;
    let resolved = candidate.canonicalize().map_err(|_| {
        LocalizedMessage::new(
            "importDiscovery.discovery.codexCatalogUnreadable",
            serde_json::Value::Null,
            "Codex 模型目录不存在或无法读取",
        )
    })?;
    if !resolved.starts_with(&root) || !resolved.is_file() {
        return Err(LocalizedMessage::new(
            "importDiscovery.discovery.codexCatalogOutside",
            serde_json::Value::Null,
            "Codex model_catalog_json 指向配置目录外，已拒绝读取",
        ));
    }
    Ok(Some(resolved))
}

/// Builds the current proposal while allowing the caller to resolve the
/// catalog through its own read-only filesystem boundary.
pub fn proposal(
    config_path: &str,
    config_text: &str,
    auth_text: Option<&str>,
    read_catalog: impl FnOnce(&Path) -> Result<Option<String>, LocalizedMessage>,
) -> Result<Option<CodexImportSource>, LocalizedMessage> {
    let path = catalog_path(config_path, config_text)?;
    let catalog = path.as_deref().map(read_catalog).transpose()?.flatten();
    Ok(Some(import_source(
        config_path,
        config_text,
        auth_text,
        catalog.as_deref(),
    )?))
}

struct LiveRoute {
    provider_name: String,
    endpoint: Option<String>,
    upstream: CodexUpstream,
    explicit_key: Option<String>,
    official: bool,
}

fn selected_route(document: &DocumentMut, provider: &str) -> Result<LiveRoute, LocalizedMessage> {
    if provider == "openai" {
        let Some(base_url) = item_string(document.get("openai_base_url")) else {
            return Ok(LiveRoute {
                provider_name: "Codex 官方登录".into(),
                endpoint: None,
                upstream: CodexUpstream::Responses,
                explicit_key: None,
                official: true,
            });
        };
        let key = item_string(document.get("experimental_bearer_token"));
        return Ok(LiveRoute {
            provider_name: provider.to_string(),
            endpoint: Some(normalize_endpoint(&base_url, CodexUpstream::Responses)?),
            upstream: CodexUpstream::Responses,
            explicit_key: key,
            official: false,
        });
    }
    let table = document
        .get("model_providers")
        .and_then(Item::as_table_like)
        .and_then(|providers| providers.get(provider))
        .and_then(Item::as_table_like)
        .ok_or_else(|| {
            LocalizedMessage::new(
                "importDiscovery.discovery.codexProviderTableMissing",
                serde_json::json!({ "provider": provider }),
                format!("已选 model_provider = {provider} 没有对应表"),
            )
        })?;
    let endpoint = required_table_string(table, "base_url")?;
    let wire_api = table_string(table, "wire_api").unwrap_or("responses");
    let upstream = match wire_api {
        "responses" => CodexUpstream::Responses,
        "chat" | "chat_completions" => CodexUpstream::ChatCompletions,
        "anthropic" | "anthropic_messages" => CodexUpstream::AnthropicMessages,
        _ => {
            return Err(LocalizedMessage::new(
                "importDiscovery.discovery.codexWireApiUnsupported",
                serde_json::Value::Null,
                "已选供应商表的 wire_api 不受支持",
            ))
        }
    };
    let key = table_string(table, "experimental_bearer_token").map(str::to_string);
    let provider_name = table_string(table, "name")
        .map(str::to_string)
        .unwrap_or_else(|| provider.to_string());
    Ok(LiveRoute {
        provider_name,
        endpoint: Some(normalize_endpoint(&endpoint, upstream)?),
        upstream,
        explicit_key: key,
        official: false,
    })
}

fn normalize_endpoint(source: &str, upstream: CodexUpstream) -> Result<String, LocalizedMessage> {
    let invalid = || {
        LocalizedMessage::new(
            "importDiscovery.discovery.codexEndpointInvalid",
            serde_json::Value::Null,
            "Codex 服务地址无效",
        )
    };
    let mut url = url::Url::parse(source).map_err(|_| invalid())?;
    if matches!(
        upstream,
        CodexUpstream::Responses | CodexUpstream::ChatCompletions
    ) && url.path().trim_matches('/').is_empty()
    {
        url.set_path("/v1");
    }
    crate::endpoint::validate_base_url(url.as_str(), upstream.protocol()).map_err(|_| invalid())?;
    Ok(url.to_string().trim_end_matches('/').to_string())
}

fn catalog_entries(
    default_model: &str,
    catalog_text: Option<&str>,
) -> Result<Vec<CodexCatalogEntry>, LocalizedMessage> {
    let mut entries = catalog_text
        .map(parse_catalog)
        .transpose()?
        .unwrap_or_default();
    if !entries.iter().any(|entry| entry.id == default_model) {
        entries.push(CodexCatalogEntry::default_entry(default_model));
    }
    if entries.is_empty() {
        return Err(LocalizedMessage::new(
            "importDiscovery.discovery.codexCatalogEmpty",
            serde_json::Value::Null,
            "Codex 模型目录为空",
        ));
    }
    Ok(entries)
}

fn parse_catalog(text: &str) -> Result<Vec<CodexCatalogEntry>, LocalizedMessage> {
    let root: Value = serde_json::from_str(text).map_err(|_| {
        LocalizedMessage::new(
            "importDiscovery.discovery.codexCatalogUnparsable",
            serde_json::Value::Null,
            "Codex 模型目录无法解析",
        )
    })?;
    let models = root.get("models").and_then(Value::as_array).ok_or_else(|| {
        LocalizedMessage::new(
            "importDiscovery.discovery.codexCatalogNoModels",
            serde_json::Value::Null,
            "Codex 模型目录缺少 models 数组",
        )
    })?;
    let mut entries = Vec::new();
    for value in models.iter() {
        let Some(object) = value.as_object() else {
            continue;
        };
        let Some(id) = string_value(object, &["slug", "model"]) else {
            continue;
        };
        if entries
            .iter()
            .any(|entry: &CodexCatalogEntry| entry.id == id)
        {
            continue;
        }
        let reasoning = bool_value(object, &["supports_reasoning", "reasoning"]).unwrap_or(true);
        let levels = reasoning_levels(object, reasoning);
        let default_reasoning_level = default_reasoning_level(object, &levels, reasoning);
        let (context_window, max_output_tokens) = (
            positive_value(object, &["context_window", "contextWindow"])
                .unwrap_or_else(|| default_model_limits(&id).0),
            positive_value(object, &["max_output_tokens", "maxOutputTokens"])
                .unwrap_or_else(|| default_model_limits(&id).1),
        );
        let images = modalities_include_image(object);
        entries.push(CodexCatalogEntry {
            id,
            context_window,
            max_output_tokens,
            function_tools: bool_value(
                object,
                &["supports_function_tools", "function_tools", "functionTools"],
            )
            .unwrap_or(true),
            custom_tools: bool_value(
                object,
                &["supports_custom_tools", "custom_tools", "customTools"],
            )
            .unwrap_or(true),
            tool_search: bool_value(
                object,
                &["supports_tool_search", "tool_search", "toolSearch"],
            )
            .unwrap_or(true),
            reasoning,
            default_reasoning_level,
            supported_reasoning_levels: levels,
            images,
            compact: bool_value(object, &["supports_compact", "compact"]).unwrap_or(true),
            display_name: string_value(object, &["display_name", "displayName"]),
            description: string_value(object, &["description"]),
            base_instructions: string_value(object, &["base_instructions", "baseInstructions"]),
            supports_parallel_tool_calls: bool_value(
                object,
                &["supports_parallel_tool_calls", "supportsParallelToolCalls"],
            ),
        });
    }
    Ok(entries)
}

fn reasoning_levels(
    object: &serde_json::Map<String, Value>,
    reasoning: bool,
) -> Vec<CodexReasoningLevel> {
    if !reasoning {
        return vec![CodexReasoningLevel::None];
    }
    let source = object
        .get("supported_reasoning_levels")
        .or_else(|| object.get("reasoningLevels"));
    let Some(Value::Array(items)) = source else {
        return CODEX_REASONING_LADDER.to_vec();
    };
    let mut levels = Vec::new();
    for item in items {
        let value = item.as_str().map(str::to_string).or_else(|| {
            item.get("effort")
                .and_then(Value::as_str)
                .map(str::to_string)
        });
        match value.and_then(|text| serde_json::from_value(Value::String(text)).ok()) {
            Some(level) if !levels.contains(&level) => levels.push(level),
            _ => {}
        }
    }
    if levels.is_empty() {
        CODEX_REASONING_LADDER.to_vec()
    } else {
        levels
    }
}

fn default_reasoning_level(
    object: &serde_json::Map<String, Value>,
    levels: &[CodexReasoningLevel],
    reasoning: bool,
) -> CodexReasoningLevel {
    if !reasoning {
        return CodexReasoningLevel::None;
    }
    let declared = object
        .get("default_reasoning_level")
        .or_else(|| object.get("defaultReasoningLevel"))
        .and_then(|value| value.as_str())
        .and_then(|text| serde_json::from_value(Value::String(text.to_string())).ok());
    declared
        .filter(|level| levels.contains(level))
        .or_else(|| {
            levels
                .iter()
                .copied()
                .find(|level| *level == CodexReasoningLevel::Medium)
        })
        .unwrap_or_else(|| levels[0])
}

fn modalities_include_image(object: &serde_json::Map<String, Value>) -> bool {
    object
        .get("input_modalities")
        .or_else(|| object.get("inputModalities"))
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some("image")))
}

fn auth_api_key(auth_text: Option<&str>) -> Option<String> {
    let Some(text) = auth_text else {
        return None;
    };
    let Ok(root) = serde_json::from_str::<Value>(text) else {
        return None;
    };
    root.get("OPENAI_API_KEY")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "PROXY_MANAGED")
        .map(str::to_string)
}

fn item_string(item: Option<&Item>) -> Option<String> {
    item.and_then(Item::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn table_string<'a>(table: &'a dyn TableLike, key: &str) -> Option<&'a str> {
    table
        .get(key)
        .and_then(Item::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn required_table_string(table: &dyn TableLike, key: &str) -> Result<String, LocalizedMessage> {
    table_string(table, key).map(str::to_string).ok_or_else(|| {
        LocalizedMessage::new(
            "importDiscovery.discovery.codexBaseUrlMissing",
            serde_json::Value::Null,
            "已选供应商表缺少 base_url",
        )
    })
}

fn string_value(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn positive_value(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| match object.get(*key) {
        Some(Value::Number(value)) => value.as_u64().filter(|value| *value > 0),
        Some(Value::String(value)) => value.trim().parse::<u64>().ok().filter(|value| *value > 0),
        _ => None,
    })
}

fn bool_value(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<bool> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(Value::as_bool))
}
