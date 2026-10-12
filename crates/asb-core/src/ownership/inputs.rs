use super::spec::{SettingControl, SettingOwner};
use crate::contracts::AppKind;

pub(super) struct InputSpec {
    pub app: AppKind,
    pub key: &'static str,
    pub owner: SettingOwner,
    pub label: &'static str,
    pub group: &'static str,
    pub control: SettingControl,
}

pub(super) const INPUTS: &[InputSpec] = &[
    InputSpec {
        app: AppKind::Codex,
        key: "tui.notifications",
        owner: SettingOwner::Client,
        label: "ownership.label.tuiNotifications",
        group: "ownership.group.terminalUi",
        control: SettingControl::Notifications,
    },
    InputSpec {
        app: AppKind::Codex,
        key: "service_tier",
        owner: SettingOwner::Provider,
        label: "ownership.label.serviceTier",
        group: "ownership.group.modelBehavior",
        control: SettingControl::Text,
    },
    InputSpec {
        app: AppKind::Codex,
        key: "model_auto_compact_token_limit",
        owner: SettingOwner::Provider,
        label: "ownership.label.autoCompactTokenLimit",
        group: "ownership.group.modelBehavior",
        control: SettingControl::Integer { min: 1, max: 9_007_199_254_740_991 },
    },
    InputSpec {
        app: AppKind::Claude,
        key: "language",
        owner: SettingOwner::Client,
        label: "ownership.label.responseLanguage",
        group: "ownership.group.interfaceAndInteraction",
        control: SettingControl::Text,
    },
    InputSpec {
        app: AppKind::Claude,
        key: "autoMemoryDirectory",
        owner: SettingOwner::Client,
        label: "ownership.label.autoMemoryDirectory",
        group: "ownership.group.filesAndGit",
        control: SettingControl::Directory,
    },
    InputSpec {
        app: AppKind::Claude,
        key: "autoCompactWindow",
        owner: SettingOwner::Provider,
        label: "ownership.label.autoCompactWindow",
        group: "ownership.group.modelBehavior",
        control: SettingControl::Integer { min: 100_000, max: 1_000_000 },
    },
    InputSpec {
        app: AppKind::Claude,
        key: "env.API_TIMEOUT_MS",
        owner: SettingOwner::Provider,
        label: "ownership.label.apiTimeoutMs",
        group: "ownership.group.modelBehavior",
        control: SettingControl::StringInteger { min: 1, max: 2_147_483_647 },
    },
    InputSpec {
        app: AppKind::Claude,
        key: "env.CLAUDE_CODE_MAX_OUTPUT_TOKENS",
        owner: SettingOwner::Provider,
        label: "ownership.label.maxOutputTokens",
        group: "ownership.group.modelBehavior",
        control: SettingControl::StringInteger { min: 1, max: 9_007_199_254_740_991 },
    },
];
