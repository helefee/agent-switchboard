export type UsageAvailability =
  | { kind: "available" }
  | { kind: "exhausted" }
  | { kind: "unavailable"; message?: string }
  | { kind: "unknown" };

/** Backend-normalized reading shared by provider rows, details, and the tray. */
export interface UsageReading {
  planName?: string;
  remaining: number | null;
  used: number | null;
  total: number | null;
  unit: string | null;
  resetsAt?: string;
  availability: UsageAvailability;
  extra?: string;
}
