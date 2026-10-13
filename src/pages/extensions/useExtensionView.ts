import { useState } from "react";
import type { ExtensionKind, ExtensionListItem, McpEditViewEnvelope } from "../../api/client";
import type { ExtensionSection } from "../../app/navigation";

export type ExtensionDialogView =
  | { type: "management" | "skillEditor"; definitionId: string }
  | { type: "mcpEditor"; envelope: McpEditViewEnvelope }
  | { type: "remove" | "export"; item: ExtensionListItem }
  | { type: "newMcp" | "newSkill" | "portableImport" | "project" | "history" };

export interface ExtensionNavigation {
  section: ExtensionSection;
  onSectionChange: (section: ExtensionSection) => void;
}

export function useExtensionView({ section, onSectionChange }: ExtensionNavigation) {
  const [search, setSearch] = useState("");
  const [sourceBrowser, setSourceBrowser] = useState(false);
  const [discoveryOpen, setDiscoveryOpen] = useState(false);
  const [dialog, setDialog] = useState<ExtensionDialogView | null>(null);
  const openManagement = (id: string, nextKind: ExtensionKind) => {
    if (!sourceBrowser && !discoveryOpen) onSectionChange(nextKind);
    setDialog({ type: "management", definitionId: id });
  };
  const openSkillEditor = (id: string) => {
    if (!sourceBrowser && !discoveryOpen) onSectionChange("skill");
    setDialog({ type: "skillEditor", definitionId: id });
  };
  const changeSection = (nextSection: ExtensionSection) => {
    onSectionChange(nextSection);
  };
  return {
    section,
    kind: section,
    changeSection,
    search,
    setSearch,
    sourceBrowser,
    setSourceBrowser,
    discoveryOpen,
    setDiscoveryOpen,
    dialog,
    setDialog,
    closeDialog: () => setDialog(null),
    openManagement,
    openSkillEditor,
    clearFilters: () => setSearch(""),
  };
}
