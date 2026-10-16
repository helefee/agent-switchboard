import { useCallback, useEffect, useRef, useState } from "react";
import { getSessionCopyText, getSessionMessageContent, type SessionMessagePreview, type SessionMeta } from "../../api/client";
import { useI18n } from "../../i18n";
import { localizedMessageText } from "../../i18n/errors";
import { Button } from "../Button";
import { Pagination } from "../Pagination";
import { ScrollArea } from "../ScrollArea";
import { SessionMessageView } from "./SessionMessageView";
import { codexOutlinePreview, previewLine } from "./session-content";
import type { SessionBookmarks } from "./use-session-bookmarks";
import type { SessionDetailState } from "./use-session-detail";
import { useReadingScroll } from "./use-reading-scroll";

interface Props {
  session: SessionMeta;
  detail: SessionDetailState;
  bookmarks: SessionBookmarks;
}

function outline(messages: SessionMessagePreview[], app: SessionMeta["app"]) {
  return messages.filter((message) => message.role === "user").flatMap((message) => {
    const content = app === "codex" ? codexOutlinePreview(message.content) : message.content;
    return content === null ? [] : [{ id: message.id, preview: previewLine(content) }];
  });
}

export function SessionTranscript({ session, detail, bookmarks }: Props) {
  const { t } = useI18n();
  const page = detail.page!;
  const messages = page.items;
  const target = detail.target;
  const { expanded, setExpanded } = detail;
  const [jump, setJump] = useState(0);
  const { container, onScroll } = useReadingScroll(detail.scrollTop);
  const mounted = useRef(true);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  const items = outline(messages, session.app);
  const readContent = useCallback((id: string, offset: number) =>
    getSessionMessageContent(session.app, session.sessionId, id, offset), [session.app, session.sessionId]);
  const readCopyText = useCallback((id: string) => getSessionCopyText({ kind: "session", app: session.app,
    sessionId: session.sessionId, messageId: id }), [session.app, session.sessionId]);
  useEffect(() => {
    if (!target || (jump === 0 && detail.scrollTop.current > 0)) return;
    const node = Array.from(container.current?.querySelectorAll<HTMLElement>("[data-message-id]") ?? [])
      .find((element) => element.dataset.messageId === target);
    node?.focus({ preventScroll: true });
    node?.scrollIntoView({ block: "center" });
  }, [target, jump, detail.scrollTop, container]);
  const toggle = (id: string) => {
    const wasExpanded = expanded.has(id);
    detail.setTarget(wasExpanded ? null : id);
    setExpanded((current) => {
      const next = new Set(current);
      if (current.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  };
  return <div className="asb-session-body">
    <div className="asb-session-conversation">
      <div className="asb-session-list-heading">
        <span>{t("sessions.transcript.heading")}</span><span className="asb-session-count">{page.total}</span>
      </div>
      {page.bookmarkIssue && <p className="asb-warn-text" role="alert">{localizedMessageText(page.bookmarkIssue, t)}</p>}
      <ScrollArea className="asb-session-transcript" viewportRef={container} aria-label={t("sessions.transcript.aria")}
        onScroll={onScroll}>
        {messages.length === 0 && <p>{t("sessions.transcript.empty")}</p>}
        {messages.map((message) => <SessionMessageView key={message.id} message={message}
          targeted={target === message.id} expanded={expanded.has(message.id)}
          contentOffset={detail.contentOffsets[message.id] ?? 0} onContentOffsetChange={detail.setContentOffset}
          readContent={readContent} readCopyText={readCopyText} onToggleExpanded={toggle} bookmarked={message.bookmarked}
          bookmarkBusy={bookmarks.busy}
          onBookmark={["user", "assistant"].includes(message.role) ? () => {
            void bookmarks.save(session, message.id).then((saved) => { if (saved && mounted.current) detail.markBookmarked(message.id); });
          } : undefined} />)}
      </ScrollArea>
      <Pagination total={page.total} page={Math.floor(page.offset / page.pageSize) + 1} pageSize={page.pageSize}
        label={t("sessions.transcript.pagination")} onPageChange={detail.changePage} />
    </div>
    {items.length > 2 && <nav className="asb-session-toc" aria-label={t("sessions.toc.aria")}>
      <div className="asb-session-toc-heading">{t("sessions.toc.heading")}</div>
      <ScrollArea className="asb-session-toc-items" aria-label={t("sessions.toc.aria")}>{items.map((item, index) =>
        <Button variant="unstyled" key={item.id} onClick={() => {
          detail.setTarget(item.id); setJump((value) => value + 1);
          setExpanded((current) => new Set([...current, item.id]));
        }}>
          <span className="asb-session-toc-index">{index + 1}</span>
          <span className="asb-session-toc-preview" title={item.preview}>{item.preview}</span>
        </Button>)}</ScrollArea>
    </nav>}
  </div>;
}
