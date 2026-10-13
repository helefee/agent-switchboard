use super::*;
use asb_core::{contracts::AppKind, extensions::contracts::{DesiredState, ExtensionTarget, SkillDefinition, SkillManifest, EXTENSIONS_SCHEMA_VERSION}};

#[test]
fn unpinning_an_older_digest_is_pending_even_if_a_previous_skip_recorded_the_latest_revision() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("deployed");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("SKILL.md"), "---\nname: fixture\ndescription: fixture\n---\nold\n").unwrap();
    let old_digest = content_digest(&walk_skill_dir(&target).unwrap());
    let mut binding = ExtensionBinding {
        schema_version: EXTENSIONS_SCHEMA_VERSION, id: "binding".into(), resource_id: "fixture".into(),
        target: ExtensionTarget::App { client: AppKind::Codex }, native_key: None, deploy_name: Some("fixture".into()),
        desired: DesiredState::Enabled, locked_digest: Some(old_digest.clone()), last_applied_revision: Some(2),
        updated_at: "2026-10-13T00:00:00Z".into(),
    };
    let baseline = ManagedBaselineFile::new(&binding.id, vec![ManagedBaseline::Directory {
        target_dir: target.to_string_lossy().into(), original_existed: false, original_files: None,
        original_digest: None, last_files: Vec::new(), last_digest: old_digest,
        backup_reference: None,
    }]);
    let definition = ExtensionDefinition {
        schema_version: EXTENSIONS_SCHEMA_VERSION, id: "fixture".into(), name: "fixture".into(), revision: 2,
        mcp_metadata: None, created_at: binding.updated_at.clone(), updated_at: binding.updated_at.clone(),
        payload: ExtensionPayload::Skill(SkillDefinition {
            content_digest: "a".repeat(64), manifest: SkillManifest {
                name: "fixture".into(), description: Some("fixture".into()), license: None,
                allowed_tools: None, unparsed_keys: Vec::new(),
            }, source: None, host_scoped: None, compatibility: Vec::new(), dependencies: Vec::new(),
        }),
    };
    assert_eq!(binding_file_state(&binding, &definition, Some(&baseline), &[]), FileState::InSync);
    binding.locked_digest = None;
    assert_eq!(binding_file_state(&binding, &definition, Some(&baseline), &[]), FileState::PendingApply);
    std::fs::write(target.join("SKILL.md"), "external edit").unwrap();
    assert_eq!(binding_file_state(&binding, &definition, Some(&baseline), &[]), FileState::ExternalChange);
}
