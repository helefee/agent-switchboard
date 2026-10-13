const PREVIEW_POLICY = [
  "default-src 'none'", "script-src 'unsafe-inline'", "style-src 'unsafe-inline'",
  "img-src data: blob:", "font-src data:", "media-src data: blob:",
  "connect-src 'none'", "frame-src 'none'", "object-src 'none'",
  "base-uri 'none'", "form-action 'none'",
].join("; ");

/** Preserve document-level CSS and scripts; the stored answer is never rewritten. */
export function createProbeHtmlPreview(answer: string): string | null {
  const blocks = [...answer.matchAll(/```html[\t ]*\r?\n([\s\S]*?)```/gi)];
  if (blocks.length > 1) return null;
  const markup = blocks.length === 1 ? blocks[0][1] : (
    answer.match(/(?:<!doctype\s+html\b[^>]*>\s*)?<html(?=[\s>])[\s\S]*<\/html\s*>/i)?.[0]
    ?? answer.trim()
  );
  if (!/^\s*(?:<!doctype\s+html\b|<[a-z][\w:-]*(?=[\s/>]))/i.test(markup)) return null;
  // Policy precedes all model markup, so a later policy can only restrict it further.
  return `<!doctype html><html><head><meta http-equiv="Content-Security-Policy" content="${PREVIEW_POLICY}"><meta charset="utf-8">${markup}`;
}
