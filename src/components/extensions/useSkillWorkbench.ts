import { useEffect, useRef, useState } from "react";
import type { SkillDependency, SkillEditorView } from "../../api/client";
import type { SkillWorkbenchProps } from "./SkillWorkbench";

function fileTexts(view: SkillEditorView) {
  return Object.fromEntries(view.files.filter((file) => file.text !== null)
    .map((file) => [file.relativePath, file.text as string]));
}

interface DocumentState {
  view: SkillEditorView;
  drafts: Record<string, string>;
  stale: boolean;
}

function reconcile(current: DocumentState | null, view: SkillEditorView): DocumentState {
  if (!current || JSON.stringify(current.drafts) === JSON.stringify(fileTexts(current.view)))
    return { view, drafts: fileTexts(view), stale: false };
  if (current.view.contentDigest === view.contentDigest) return { ...current, view };
  return { ...current, stale: true };
}

export function useSkillFiles(props: SkillWorkbenchProps) {
  const [document, setDocument] = useState<DocumentState | null>(null);
  const [selected, setSelected] = useState("SKILL.md");
  const [loading, setLoading] = useState(true);
  const generation = useRef(0);
  const { item, onLoadEditor } = props;
  useEffect(() => {
    const request = ++generation.current;
    setLoading(true);
    void onLoadEditor(item.id).then((view) => {
      if (view && request === generation.current) setDocument((current) => reconcile(current, view));
    }).finally(() => { if (request === generation.current) setLoading(false); });
    return () => { generation.current += 1; };
  }, [item.id, item.revision, onLoadEditor]);
  const reload = async () => {
    const request = ++generation.current;
    setLoading(true);
    try {
      const view = await onLoadEditor(item.id);
      if (view && request === generation.current) setDocument({ view, drafts: fileTexts(view), stale: false });
    } finally { if (request === generation.current) setLoading(false); }
  };
  const paths = [...new Set([...(document?.view.files.map((file) => file.relativePath) ?? []),
    ...Object.keys(document?.drafts ?? {})])].sort();
  const dirty = document !== null && JSON.stringify(document.drafts) !== JSON.stringify(fileTexts(document.view));
  const change = (path: string, text: string) => setDocument((current) => current && ({
    ...current, drafts: { ...current.drafts, [path]: text },
  }));
  const save = async () => {
    if (!document || document.stale || !document.view.editable) return;
    const saved = await props.onSaveFiles(item.id, {
      expectedRevision: document.view.revision, expectedDigest: document.view.contentDigest,
      files: Object.entries(document.drafts).map(([relativePath, text]) => ({ relativePath, text })),
    });
    if (saved) await reload();
  };
  const editablePaths = paths.filter((path) => document?.drafts[path] !== undefined);
  return { document, paths, dirty, loading, selected: editablePaths.includes(selected) ? selected : editablePaths[0] ?? null,
    select: setSelected, change, save, reload };
}

export function useSkillDependencies(dependencies: SkillDependency[]) {
  const [state, setState] = useState(() => ({ baseline: dependencies, draft: dependencies, stale: false }));
  useEffect(() => {
    setState((current) => {
      if (JSON.stringify(current.baseline) === JSON.stringify(dependencies)) return current;
      if (JSON.stringify(current.draft) !== JSON.stringify(current.baseline)) return { ...current, stale: true };
      return { baseline: dependencies, draft: dependencies, stale: false };
    });
  }, [dependencies]);
  const change = (draft: SkillDependency[]) => setState((current) => ({ ...current, draft }));
  const reset = (draft: SkillDependency[] = dependencies) => setState({ baseline: draft, draft, stale: false });
  return { ...state, change, reset, dirty: JSON.stringify(state.draft) !== JSON.stringify(state.baseline) };
}
