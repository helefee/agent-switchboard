import type { SessionMeta } from "../../api/client";
import type { MessageKey } from "../../i18n";
import { tr } from "../../i18n/current";
import { uiMessage } from "../../i18n/errors";

const CODEX_IDE_CONTEXT_PREFIX = "# Context from my IDE setup:";
const CODEX_REQUEST_MARKER = "my request for codex";

export const sessionKey = (session: Pick<SessionMeta, "app" | "sessionId">) =>
  `${session.app}:${session.sessionId}`;

export const sessionTitle = (session: SessionMeta) => session.alias ?? session.title;

const ROLE_MESSAGE_KEYS: Record<string, MessageKey | undefined> = {
  user: "sessions.role.user",
  assistant: "sessions.role.assistant",
  system: "sessions.role.system",
  tool: "sessions.role.tool",
  reasoning: "sessions.role.reasoning",
};

export function messageRole(role: string): string {
  const key = ROLE_MESSAGE_KEYS[role.toLowerCase()];
  return key === undefined ? role : tr(key);
}

export function previewLine(content: string): string {
  return content.split(/\s+/).join(" ").trim();
}

/* Codex records non-conversational payloads as user-role messages: AGENTS.md
   dumps, environment context, and the IDE context wrapper VS Code injects.
   The outline lists real prompts only, so these markers are protocol facts to
   match byte-for-byte, not display copy. */
function codexRequestHeadingPayload(line: string): string | null {
  if (!line.startsWith("#")) return null;
  const heading = line.replace(/^#+\s*/, "");
  if (!heading.toLowerCase().startsWith(CODEX_REQUEST_MARKER)) return null;
  const suffix = heading.slice(CODEX_REQUEST_MARKER.length).trimStart();
  if (!suffix) return "";
  if (!/^[:：\-—]/.test(suffix)) return null;
  return suffix.replace(/^[:：\-—\s]+/, "").trim();
}

/* The real prompt inside an IDE wrapper is the LAST request heading: earlier
   matches can be headings living inside quoted file content. */
function codexPromptFromIdeContext(content: string): string | null {
  const lines = content.replace(/\r\n/g, "\n").split("\n");
  let prompt: string | null = null;
  for (const [index, line] of lines.entries()) {
    const inline = codexRequestHeadingPayload(line.trim());
    if (inline === null) continue;
    if (inline) {
      prompt = inline;
      continue;
    }
    const following = lines.slice(index + 1).join("\n").trim();
    prompt = following || null;
  }
  return prompt;
}

/** Null hides injected payloads from the user-prompt outline. */
export function codexOutlinePreview(content: string): string | null {
  const trimmed = content.trim();
  if (
    trimmed.startsWith("# AGENTS.md instructions for ") ||
    trimmed.startsWith("<environment_context>")
  ) {
    return null;
  }
  if (trimmed.startsWith(CODEX_IDE_CONTEXT_PREFIX)) {
    return codexPromptFromIdeContext(trimmed);
  }
  return content;
}

let copying = false;

export async function copyText(source: string | (() => Promise<string>)): Promise<void> {
  if (copying) throw uiMessage("sessions.message.copyBusy");
  copying = true;
  try {
    if (typeof source === "string") await navigator.clipboard.writeText(source);
    else await navigator.clipboard.write([new ClipboardItem({
      "text/plain": source().then((text) => new Blob([text], { type: "text/plain" })),
    })]);
  } finally { copying = false; }
}
