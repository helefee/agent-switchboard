use crate::{execute, read_preview, restore_projected, FsIo, RestoreExpectation, SwitchError, SwitchRequest};
use asb_core::{contracts::*, ownership};

fn plan() -> SwitchPlan {
    let profile = ProviderProfile {
        id: "isolated-repair".into(), app: AppKind::Claude, route_mode: RouteMode::Custom,
        name: "Isolated repair".into(), model: Some("saved-model".into()),
        base_url: Some("https://provider.example".into()), connection: Default::default(),
        api_key: "isolated-key".into(), authentication: None,
        upstream_protocol: Some(UpstreamProtocol::AnthropicMessages), responses_options: None,
        max_output_tokens: ExplicitMaxOutputTokens::none(), model_options: None,
        parameters: ownership::default_provider_parameters(AppKind::Claude), claude_fragment: Default::default(),
        notes: None, website_url: None, display: None, usage_query: None,
        official_quota_refresh_interval_minutes: None,
    };
    SwitchPlan::direct(profile, ownership::default_client_settings(AppKind::Claude)).for_provider_repair()
}

#[test]
fn provider_scope_survives_preview_execution_backup_and_guarded_undo() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("settings.json");
    let backups = directory.path().join("backups");
    let original = r#"{"model":"old-model","language":"Japanese","unmanaged":{"flag":true}}"#;
    std::fs::write(&target, original).unwrap();
    let plan = plan();
    let preview = read_preview(&FsIo, &target, &plan, &backups.to_string_lossy()).unwrap();
    assert!(preview.preview.changes.iter().all(|change| change.key != "language"));
    let request = SwitchRequest { target: &target, plan: &plan, backup_dir: &backups,
        expected_hash: &preview.content_hash, expected_rendered_hash: &preview.rendered_hash };
    let outcome = execute(&FsIo, &request, |_| Ok(())).unwrap();
    let live: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&target).unwrap()).unwrap();
    assert_eq!(live["model"], "saved-model");
    assert_eq!(live["language"], "Japanese");
    assert_eq!(live["unmanaged"]["flag"], true);
    assert_eq!(std::fs::read_to_string(&outcome.backup.backup_path).unwrap(), original);
    let expected = RestoreExpectation { content_hash: outcome.final_hash, target_existed: true, auth: None };
    restore_projected(&FsIo, &outcome.backup, &target, None, Some(&expected), |_| Ok(())).unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), original);
}

#[test]
fn provider_repair_rejects_a_public_setting_edit_made_after_preview() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("settings.json");
    let backups = directory.path().join("backups");
    std::fs::write(&target, r#"{"model":"old-model","language":"Japanese"}"#).unwrap();
    let plan = plan();
    let preview = read_preview(&FsIo, &target, &plan, &backups.to_string_lossy()).unwrap();
    let external = r#"{"model":"old-model","language":"English"}"#;
    std::fs::write(&target, external).unwrap();
    let request = SwitchRequest { target: &target, plan: &plan, backup_dir: &backups,
        expected_hash: &preview.content_hash, expected_rendered_hash: &preview.rendered_hash };
    assert!(matches!(execute(&FsIo, &request, |_| panic!("stale repair cannot commit")),
        Err(SwitchError::ExternalChange { .. })));
    assert_eq!(std::fs::read_to_string(&target).unwrap(), external);
    assert!(!backups.exists() || std::fs::read_dir(&backups).unwrap().next().is_none());
}
