import { useI18n } from "../i18n";
import { isToastMessage, type ToastContent } from "./use-toast";

/** Resolve descriptors at render time so existing messages follow language changes. */
export function ToastText({ content }: { content: ToastContent }) {
  const { t } = useI18n();
  // The queued key and params crossed a runtime queue that erases the
  // template contract; `toastMessage` checked them at the producer.
  if (isToastMessage(content)) return <>{t(content.key, content.params as never)}</>;
  return <>{content}</>;
}

export function ToastMessageList({ items }: { items: readonly ToastContent[] }) {
  return <>{items.map((content, index) => <div key={index}><ToastText content={content} /></div>)}</>;
}
