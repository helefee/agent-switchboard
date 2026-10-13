use super::*;
use asb_core::contracts::{
    AppKind, CodexCatalogEntry, CodexEndpoint, CodexModelRoute, CodexProviderDraft,
    CodexRouteMode, CodexUpstream, ResponsesRequestMode, DEFAULT_CODEX_CAPABILITIES,
};

fn draft(upstream: CodexUpstream) -> CodexProviderDraft {
    CodexProviderDraft {
        name: "Saved request fixture".into(),
        endpoint: CodexEndpoint("https://provider.example/v1".into()),
        api_key: "isolated-test-key".into(), authentication: None, connection: Default::default(),
        upstream, request_mode: ResponsesRequestMode::Standard, default_model: "client-model".into(),
        catalog: vec![CodexCatalogEntry::default_entry("client-model")],
        model_routes: vec![CodexModelRoute { client_model: "client-model".into(), upstream_model: "upstream-model".into() }],
        subagent_route: None, capabilities: DEFAULT_CODEX_CAPABILITIES,
        parameters: asb_core::ownership::default_provider_parameters(AppKind::Codex),
        notes: None, website_url: None, usage_query: None,
    }
}

#[test]
fn saved_codex_direct_and_gateway_requests_use_the_native_store_and_upstream_model() {
    for (upstream, mode, suffix) in [
        (CodexUpstream::Responses, CodexRouteMode::Direct, "/responses"),
        (CodexUpstream::ChatCompletions, CodexRouteMode::Gateway, "/chat/completions"),
        (CodexUpstream::AnthropicMessages, CodexRouteMode::Gateway, "/messages"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let store = ConfigStore::new(directory.path().join("state"));
        let record = store.create_codex_provider(draft(upstream)).unwrap();
        assert_eq!(record.profile.route_mode, mode);
        let requests = ProviderRequests::default();
        let prepared = prepare(&requests, &store,
            ProviderRequestTarget::Saved { profile_id: record.profile.id.clone() }).unwrap();
        assert_eq!(prepared.endpoint, format!("https://provider.example/v1{suffix}"));
        assert_eq!(prepared.default_model.as_deref(), Some("upstream-model"));
        let source = requests.inspect(&prepared.request_id).unwrap().source;
        let connection = resolve_prepared(&store, source).unwrap();
        assert_eq!(connection.api_key, "isolated-test-key");
        assert_eq!(connection.upstream_protocol, upstream.protocol());
        assert_eq!(store.find_codex_provider_with_revision(&record.profile.id).unwrap().1, record.file_hash);
    }
}

#[test]
fn a_changed_saved_codex_profile_rejects_the_prepared_request() {
    let directory = tempfile::tempdir().unwrap();
    let store = ConfigStore::new(directory.path().join("state"));
    let record = store.create_codex_provider(draft(CodexUpstream::Responses)).unwrap();
    let requests = ProviderRequests::default();
    let prepared = prepare(&requests, &store,
        ProviderRequestTarget::Saved { profile_id: record.profile.id.clone() }).unwrap();
    let mut edited = record.clone().into_draft();
    edited.api_key = "different-isolated-test-key".into();
    store.update_codex_provider(&record.profile.id, edited, &record.file_hash).unwrap();
    let result = resolve_prepared(&store, requests.inspect(&prepared.request_id).unwrap().source);
    assert_eq!(result.err().expect("changed source rejected").code, "provider-request-profile-changed");
}

#[test]
fn saved_official_codex_keeps_its_own_contract_and_cannot_send_a_custom_request() {
    let directory = tempfile::tempdir().unwrap();
    let store = ConfigStore::new(directory.path().join("state"));
    let record = store.create_provider(asb_core::codex_official_draft(
        asb_core::ownership::default_provider_parameters(AppKind::Codex))).unwrap();
    assert!(matches!(store.resolve_saved_provider(&record.profile.id).unwrap(), SavedProvider::Client(_)));
    let result = prepare(&ProviderRequests::default(), &store,
        ProviderRequestTarget::Saved { profile_id: record.profile.id });
    assert_eq!(result.err().unwrap().code, "provider-request-custom-only");
}

#[test]
fn corrupt_codex_storage_is_not_reinterpreted_as_a_generic_missing_provider() {
    let directory = tempfile::tempdir().unwrap();
    let store = ConfigStore::new(directory.path().join("state"));
    let record = store.create_codex_provider(draft(CodexUpstream::Responses)).unwrap();
    let path = store.providers_dir(AppKind::Codex).join(format!("{}.json", record.profile.id));
    std::fs::write(path, "{}").unwrap();
    assert!(matches!(store.resolve_saved_provider(&record.profile.id),
        Err(StoreOperationError::Store(crate::config_store::ProfileStoreError::Unsupported))));
    assert!(prepare(&ProviderRequests::default(), &store,
        ProviderRequestTarget::Saved { profile_id: record.profile.id }).is_err());
}

fn xai_accounts(root: &std::path::Path, profile_id: &str, selected: &str, token: &str) {
    let now = chrono::Utc::now().timestamp_millis();
    let value = serde_json::json!({
        "version": 1, "defaultId": selected,
        "bindings": {(profile_id): selected},
        "accounts": [{"id":selected,"label":"Fixture","updatedAt":now,
            "tokens":{"accessToken":token,"refreshToken":"isolated-refresh", "expiresAtMs":now + 3_600_000}}]
    });
    std::fs::create_dir_all(root.join("xai")).unwrap();
    std::fs::write(root.join("xai/accounts.json"), value.to_string()).unwrap();
}

#[test]
fn saved_xai_request_uses_the_bound_account_and_rejects_changed_account_identity() {
    let directory = tempfile::tempdir().unwrap();
    let store = ConfigStore::new(directory.path().join("state"));
    let mut input = draft(CodexUpstream::Responses);
    input.connection.provider_type = Some("xai_oauth".into());
    input.api_key = asb_core::contracts::XAI_OAUTH_PLACEHOLDER.into();
    let record = store.create_codex_provider(input).unwrap();
    xai_accounts(store.state_root(), &record.profile.id, "first-account", "first-isolated-token");
    let requests = ProviderRequests::default();
    let prepared = prepare(&requests, &store,
        ProviderRequestTarget::Saved { profile_id: record.profile.id.clone() }).unwrap();
    assert!(prepared.endpoint.starts_with(asb_core::contracts::XAI_API_BASE_URL));
    let source = requests.inspect(&prepared.request_id).unwrap().source;
    assert_eq!(resolve_prepared(&store, source.clone()).unwrap().api_key, "first-isolated-token");
    xai_accounts(store.state_root(), &record.profile.id, "first-account", "refreshed-isolated-token");
    assert_eq!(resolve_prepared(&store, source.clone()).unwrap().api_key, "refreshed-isolated-token");
    xai_accounts(store.state_root(), &record.profile.id, "second-account", "second-isolated-token");
    assert_eq!(resolve_prepared(&store, source).err().unwrap().code, "provider-request-profile-changed");
}
