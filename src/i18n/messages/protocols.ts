import type { MessageEntry } from "../types";

/** Domain dictionary for upstream protocol labels (zh-CN / en-US pairs).
 * Keys are prefixed with "protocol." and must never collide with another
 * domain file. Chinese leads and the wire-protocol name follows in
 * parentheses so the label still matches provider documentation. */
export const messages = {
  "protocol.responses": ["OpenAI 响应（Responses）", "OpenAI Responses"],
  "protocol.chatCompletions": ["对话补全（Chat Completions）", "Chat Completions"],
  "protocol.anthropicMessages": ["Anthropic 消息（Messages）", "Anthropic Messages"],
  "protocol.geminiGenerateContent": ["Gemini 原生（Native）", "Gemini Native"],
} as const satisfies Record<string, MessageEntry>;
