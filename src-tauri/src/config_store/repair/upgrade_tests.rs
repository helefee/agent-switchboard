use super::*;
use asb_core::contracts::ProviderProfile;
use serde_json::json;

fn provider(position: u64) -> ProviderFile {
    let profile: ProviderProfile = serde_json::from_value(json!({
        "id": Uuid::new_v4().to_string(), "app": "claude", "routeMode": "custom",
        "name": format!("Saved provider {position}"), "model": "saved-model",
        "baseUrl": "https://relay.example", "apiKey": "isolated-saved-key",
        "upstreamProtocol": "anthropicMessages", "maxOutputTokens": null,
        "parameters": default_provider_parameters(AppKind::Claude),
        "claudeFragment": {"includeCoAuthoredBy": false},
        "notes": "saved notes", "websiteUrl": "https://relay.example"
    })).unwrap();
    ProviderFile::from_profile(&profile, position)
}

fn write_provider(store: &ConfigStore, value: &Value) -> (PathBuf, String) {
    let path = store.providers_dir(AppKind::Claude)
        .join(format!("{}.json", value["id"].as_str().unwrap()));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let original = serde_json::to_string_pretty(value).unwrap();
    fs::write(&path, &original).unwrap();
    (path, original)
}

#[test]
fn retired_automatic_effort_is_repaired_without_losing_saved_providers() {
    let directory = tempfile::tempdir().unwrap();
    let store = ConfigStore::new(directory.path().join("state"));
    let mut originals = Vec::new();
    let mut expected = Vec::new();
    for position in [100, 200, 300, 400] {
        let file = provider(position);
        let mut old = serde_json::to_value(&file).unwrap();
        old["parameters"]["settings"].as_object_mut().unwrap()
            .insert("effortLevel".into(), json!({"mode": "automatic"}));
        old["parameters"]["settings"].as_object_mut().unwrap().remove("fastMode");
        originals.push(write_provider(&store, &old));
        expected.push(file);
    }
    assert!(store.list_providers().is_err());

    let report = store.repair().unwrap();
    let backup = PathBuf::from(report.backup_path.unwrap());
    assert_eq!(report.repaired_files.len(), 4);
    let records = store.list_providers().unwrap();
    assert_eq!(records.len(), 4);
    for ((record, file), (path, original)) in records.iter().zip(expected).zip(originals) {
        assert_eq!(record.position, file.position);
        assert!(record.profile == file.into_profile(AppKind::Claude));
        let relative = path.strip_prefix(store.configuration_dir()).unwrap();
        assert_eq!(fs::read_to_string(backup.join(relative)).unwrap(), original);
        let saved: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        assert!(saved["parameters"]["settings"].get("effortLevel").is_none());
    }
    let repeated = store.repair().unwrap();
    assert!(repeated.repaired_files.is_empty());
    assert!(repeated.backup_path.is_none());
}

#[test]
fn retired_effort_only_change_is_persisted_even_without_missing_settings() {
    let directory = tempfile::tempdir().unwrap();
    let store = ConfigStore::new(directory.path().join("state"));
    let mut old = serde_json::to_value(provider(100)).unwrap();
    old["parameters"]["settings"]["effortLevel"] = json!({"mode": "automatic"});
    write_provider(&store, &old);
    let report = store.repair().unwrap();
    assert_eq!(report.repaired_files.len(), 1);
    assert_eq!(store.list_providers().unwrap().len(), 1);
}

#[test]
fn explicit_retired_effort_rejects_repair_without_changing_any_file() {
    let directory = tempfile::tempdir().unwrap();
    let store = ConfigStore::new(directory.path().join("state"));
    let mut automatic = serde_json::to_value(provider(100)).unwrap();
    automatic["parameters"]["settings"]["effortLevel"] = json!({"mode": "automatic"});
    write_provider(&store, &automatic);
    let mut explicit = serde_json::to_value(provider(200)).unwrap();
    explicit["parameters"]["settings"]["effortLevel"] = json!({"mode": "explicit", "value": "high"});
    write_provider(&store, &explicit);
    let before = file_tree(&store.configuration_dir()).unwrap();
    assert!(store.repair().unwrap_err().contains("effortLevel"));
    assert_eq!(file_tree(&store.configuration_dir()).unwrap(), before);
    assert_eq!(fs::read_dir(store.state_root()).unwrap().count(), 1);
}

#[test]
fn arbitrary_unknown_automatic_keys_are_never_discarded() {
    let mut settings = serde_json::to_value(default_provider_parameters(AppKind::Claude)).unwrap();
    settings["settings"]["unknownSavedOption"] = json!({"mode": "automatic"});
    assert!(!fill_automatic(&mut settings, AppKind::Claude, true).unwrap());
    let decoded: SettingsValues = serde_json::from_value(settings).unwrap();
    assert!(decoded.validate_provider_parameters(AppKind::Claude).is_err());
}
