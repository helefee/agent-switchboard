import { useEffect, useRef, useState } from "react";
import { testUsageQuery, type DeclarativeUsageQuery, type UsageQuery, type UsageSummary } from "../../api/client";
import { normalizeUsageQuery } from "../../lib/usage-query";
import { uiMessage } from "../../i18n/errors";
import { useMessageState } from "../../i18n/use-message-state";
import type { UsageQueryWorkspaceProps } from "../UsageQueryWorkspace";

export function emptyDeclarative(interval: number): DeclarativeUsageQuery {
  return { kind: "declarative", url: "", remainingPath: null, usedPath: null, totalPath: null,
    unit: null, refreshIntervalMinutes: interval };
}

export function canRun(query: UsageQuery | null): boolean {
  return Boolean(query && (query.kind === "declarative" ? query.url.trim() : query.source.trim()));
}

/** Only the selected mode is persisted; the other mode retains its session draft. */
export function useUsageQueryDraft(value: UsageQuery | null, clearResult: () => void) {
  const [draft, setDraft] = useState(value);
  const inactiveDraft = useRef<UsageQuery | null>(null);
  const kind = draft?.kind ?? "declarative";
  const declarative = draft?.kind === "declarative" ? draft : emptyDeclarative(0);
  const interval = draft?.refreshIntervalMinutes ?? 0;
  const [intervalText, setIntervalText] = useState(String(interval));
  const selectKind = (next: "declarative" | "script") => {
    if (next === kind) return;
    clearResult();
    const saved = inactiveDraft.current;
    inactiveDraft.current = draft;
    setDraft(saved?.kind === next ? { ...saved, refreshIntervalMinutes: interval }
      : next === "declarative" ? emptyDeclarative(interval)
        : { kind: "script", source: "", refreshIntervalMinutes: interval });
  };
  const patchDeclarative = (fields: Partial<DeclarativeUsageQuery>) => {
    clearResult(); setDraft({ ...declarative, ...fields, kind: "declarative" });
  };
  const patchScript = (source: string) => {
    clearResult(); setDraft({ kind: "script", source, refreshIntervalMinutes: interval });
  };
  const commitInterval = () => {
    const text = intervalText.trim();
    if (!/^\d+$/.test(text) || Number(text) > 1440) { setIntervalText(String(interval)); return; }
    const minutes = Number(text);
    setIntervalText(String(minutes));
    if (minutes !== interval) setDraft({ ...(draft ?? declarative), refreshIntervalMinutes: minutes });
  };
  const dormantChanged = canRun(inactiveDraft.current)
    && JSON.stringify(inactiveDraft.current) !== JSON.stringify(value);
  const dirty = dormantChanged || JSON.stringify(draft) !== JSON.stringify(value);
  return { draft, kind, declarative, intervalText, setIntervalText, selectKind,
    patchDeclarative, patchScript, commitInterval, dirty };
}

export function useUsageQueryExecution(props: UsageQueryWorkspaceProps) {
  const [querying, setQuerying] = useState(false);
  const [saving, setSaving] = useState(false);
  const [summary, setSummary] = useState<UsageSummary | null>(null);
  const [error, setError] = useMessageState();
  const version = useRef(0);
  const running = useRef(false);
  const firstRun = useRef(true);
  const clearResult = () => { version.current += 1; setSummary(null); setError(null); };
  const run = async (draft: UsageQuery | null) => {
    if (!draft || props.busy || running.current) return;
    if (!props.upstreamProtocol) { setError(uiMessage("usage.query.missingProtocol")); return; }
    const request = ++version.current;
    running.current = true; setQuerying(true); setError(null);
    try {
      const result = await testUsageQuery(draft, props.apiKey, props.baseUrl,
        props.upstreamProtocol, props.authentication, props.connection);
      if (request === version.current) setSummary(result);
    } catch (caught) {
      if (request === version.current) { setSummary(null); setError(caught); }
    } finally {
      running.current = false;
      if (request === version.current) setQuerying(false);
    }
  };
  const save = async (draft: UsageQuery | null) => {
    if (props.busy || running.current) return;
    running.current = true; setSaving(true);
    try { await props.onSave(normalizeUsageQuery(draft)); }
    finally { running.current = false; setSaving(false); }
  };
  useEffect(() => {
    if (firstRun.current) { firstRun.current = false; if (canRun(props.value)) void run(props.value); }
    // A workspace reads once on entry; edits require an explicit query.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return { querying, saving, summary, error, run, save, clearResult };
}
