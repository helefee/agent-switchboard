import type { ReactNode } from "react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { openUrl } from "@tauri-apps/plugin-opener";

function MarkdownLink({ href, children }: { href?: string; children?: ReactNode }) {
  const target = typeof href === "string" && /^https?:\/\//i.test(href) ? href : null;
  return target ? <a href={target} onClick={(event) => { event.preventDefault(); void openUrl(target); }}>{children}</a>
    : <span>{children}</span>;
}

/** The one GitHub-flavored markdown viewer; every changelog and transcript
 * surface renders through it. Raw HTML never renders (react-markdown default)
 * and http(s) links open in the system browser. Pair the output with the
 * `.asb-markdown` styles for element spacing. */
export function MarkdownContent({ text }: { text: string }) {
  return <Markdown remarkPlugins={[remarkGfm]} components={{ a: MarkdownLink }}>{text}</Markdown>;
}
