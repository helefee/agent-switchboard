import type { ExtensionListItem, ExtensionTarget, ProjectRegistration } from "../../api/client";
import { EXTENSION_CLIENTS, itemSupportsClient } from "../../app/extensions/deployment-state";
import type { SelectOption } from "../Select";
import { targetLabel, targetValue } from "./labels";

export type ExtensionTargetOption = SelectOption & { target: ExtensionTarget };

export function extensionTargetOptions(item: ExtensionListItem, projects: ProjectRegistration[]): ExtensionTargetOption[] {
  const projectNames = new Map(projects.map((project) => [project.id, project.displayName]));
  const targets: ExtensionTarget[] = [];
  for (const client of EXTENSION_CLIENTS) {
    if (!itemSupportsClient(item, client)) continue;
    targets.push({ scope: "app", client });
    for (const project of projects) {
      targets.push({ scope: "projectShared", client, projectId: project.id });
      if (client === "claude") targets.push({ scope: "projectPrivate", client, projectId: project.id });
    }
  }
  return targets.map((target) => ({ target, value: targetValue(target), label: targetLabel(target, projectNames) }));
}
