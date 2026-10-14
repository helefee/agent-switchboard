import { invoke } from "./client";
import type { AppKind } from "./shared";
import type { LocalizedMessage } from "./switching";

export interface StorageUsage {
  dataBytes: number;
  backupBytes: number;
  historyBytes: number;
  cacheBytes: number;
  cacheBudgetBytes: number;
  cacheMaintenance: boolean;
  cacheClearing: boolean;
}

interface ArchiveDigest { records: number; revision: string }
export interface HistoryArchivePlan {
  cutoff: string;
  configuration: Record<AppKind, ArchiveDigest>;
  codex: ArchiveDigest;
  probes: ArchiveDigest;
}

export interface HistoryArchiveOutcome { path: string; removed: number; warnings: LocalizedMessage[] }
export interface StorageReclaimOutcome { removed: number; warning: string | null }

export const getStorageUsage = (): Promise<StorageUsage> => invoke("storage_usage");
export const clearWebviewCache = (): Promise<void> => invoke("clear_webview_cache");
export const previewHistoryArchive = (cutoff: string): Promise<HistoryArchivePlan> => invoke("preview_history_archive", { cutoff });
export const archiveHistory = (plan: HistoryArchivePlan): Promise<HistoryArchiveOutcome | null> => invoke("archive_history", { plan });
