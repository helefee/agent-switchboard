import type {
  BindingStatus,
  ClientCapabilityReport,
  ExtensionListItem,
  ProjectRegistration,
  SkillUpdateReport,
} from "../../api/client";
import type { McpConnectionCheck } from "../../pages/extensions/useMcpConnectionCheck";

export interface ExtensionManagementProps {
  item: ExtensionListItem;
  projects: ProjectRegistration[];
  capabilities: ClientCapabilityReport[];
  busy: boolean;
  check: McpConnectionCheck;
  installTargets: string[];
  updateReport: SkillUpdateReport | null;
  projectNames: ReadonlyMap<string, string>;
  onInstallTargetsChange: (values: string[]) => void;
  onInstall: () => void;
  onChangeBinding: (binding: BindingStatus, enable: boolean) => void;
  onRemoveBinding: (binding: BindingStatus) => void;
  onToggleLock: (binding: BindingStatus, locked: boolean) => void;
  onDelete: () => void;
  onCheckUpdates: () => void;
  onApplyUpdate: () => void;
  onDeployCurrent: () => void;
  onExportPortable?: () => void;
}
