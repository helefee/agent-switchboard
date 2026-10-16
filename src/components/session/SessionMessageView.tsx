import { useCallback, useEffect, useId, useState } from "react";
import type { SessionContentPage, SessionMessagePreview } from "../../api/client";
import { useI18n } from "../../i18n";
import { useMessageState } from "../../i18n/use-message-state";
import { Time } from "../Time";
import { Button } from "../Button";
import { Collapsible } from "../Collapsible";
import { copyText, messageRole } from "./session-content";
import { toast, toastMessage } from "../use-toast";
import { SessionContent, SessionText } from "./SessionContent";

interface Props {
  message: SessionMessagePreview;
  readContent: (id: string, offset: number) => Promise<SessionContentPage>;
  readCopyText: (id: string) => Promise<string>;
  targeted: boolean;
  expanded: boolean;
  contentOffset: number;
  onContentOffsetChange: (id: string, offset: number) => void;
  onToggleExpanded?: (id: string) => void;
  onBookmark?: () => void;
  bookmarked?: boolean | null;
  bookmarkBusy?: boolean;
}

function MessageHeader({ message, readCopyText, onBookmark, bookmarked, bookmarkBusy }: Pick<Props,
  "message" | "readCopyText" | "onBookmark" | "bookmarked" | "bookmarkBusy">) {
  const { t } = useI18n();
  const [copying, setCopying] = useState(false);
  const [error, setError] = useMessageState();
  const isLong = message.contentLength > [...message.content].length || message.imageCount > 0;
  const copy = async () => {
    setCopying(true); setError(null);
    try {
      await copyText(isLong ? () => readCopyText(message.id) : message.content);
      toast({ kind: "success", title: toastMessage("sessions.message.copied") });
    } catch (caught) { setError(caught); }
    finally { setCopying(false); }
  };
  return <><header>
    <span>{messageRole(message.role)}</span>
    <span className="asb-session-message-time">{message.at ? <Time iso={message.at} /> : null}</span>
    {onBookmark && (bookmarked
      ? <span className="asb-session-message-saved">{t("sessions.bookmarks.saved")}</span>
      : <Button variant="unstyled" className="asb-session-message-bookmark" disabled={bookmarkBusy || bookmarked === null}
        onClick={onBookmark}>{t("sessions.bookmarks.save")}</Button>)}
    <Button variant="unstyled" className="asb-session-message-copy" disabled={copying} onClick={() => void copy()}>
      {t(isLong ? "sessions.message.copyFull" : "sessions.message.copy")}</Button>
  </header>{error && <p className="asb-warn-text" role="alert">{error}</p>}</>;
}

/** Renders one read-only transcript entry and owns its local copy feedback. */
export function SessionMessageView({
  message,
  readContent,
  readCopyText,
  targeted,
  expanded,
  contentOffset,
  onContentOffsetChange,
  onToggleExpanded,
  onBookmark,
  bookmarked,
  bookmarkBusy,
}: Props) {
  const { t } = useI18n();
  const isLong = message.contentLength > [...message.content].length || message.imageCount > 0;
  const read = useCallback((offset: number) => readContent(message.id, offset), [readContent, message.id]);
  // Preview and full reader render the same text, so only one may exist at a
  // time: the preview returns exactly when the collapse transition ends and
  // the shared disclosure unmounts the reader (Collapsible onClosed).
  const [previewShown, setPreviewShown] = useState(!expanded);
  const bodyId = useId();
  useEffect(() => {
    if (expanded) setPreviewShown(false);
  }, [expanded]);

  return (
    <article
      className={`asb-session-message is-${message.role.toLowerCase()}${targeted ? " is-target" : ""}`}
      data-message-id={message.id}
      tabIndex={targeted ? -1 : undefined}
    >
      <MessageHeader message={message} readCopyText={readCopyText}
        onBookmark={onBookmark} bookmarked={bookmarked} bookmarkBusy={bookmarkBusy} />
      {!isLong ? <SessionText content={message.content} />
        : previewShown ? <SessionText content={message.content} preview /> : null}
      {isLong && <Collapsible open={expanded} id={bodyId} onClosed={() => setPreviewShown(true)}>
        <SessionContent read={read} offset={contentOffset}
          onOffsetChange={(offset) => onContentOffsetChange(message.id, offset)} />
      </Collapsible>}
      {isLong && onToggleExpanded && (
        <Button
          variant="unstyled"
          className="asb-session-message-toggle"
          aria-expanded={expanded}
          aria-controls={expanded ? bodyId : undefined}
          onClick={() => onToggleExpanded(message.id)}
        >
          {expanded
            ? t("sessions.message.collapse")
            : message.imageCount > 0 ? t("sessions.message.expandImages", { count: message.imageCount })
            : t("sessions.message.expand", { size: Math.ceil(message.contentLength / 1000) })}
        </Button>
      )}
    </article>
  );
}
