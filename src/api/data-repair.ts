import { invoke } from "./client";
import type { CommandError } from "./shared";

export interface ApplicationDataRepairReport {
  repairedFiles: string[];
  backupPaths: string[];
  issues: CommandError[];
}

/** The global repair action is offered only for these application-owned domains. */
export function canRepairApplicationData(error: CommandError): boolean {
  return error.code === "profile-store-unsupported" || error.code === "profile-store-repair-failed"
    || error.code === "outbound-proxy-unavailable" || error.code === "outbound-proxy-invalid";
}

export function repairApplicationData(): Promise<ApplicationDataRepairReport> {
  return invoke<ApplicationDataRepairReport>("repair_application_data");
}
