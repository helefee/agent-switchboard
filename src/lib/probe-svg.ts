const SVG_NAMESPACE = "http://www.w3.org/2000/svg";

/** The persisted answer stays untouched; only a complete SVG is used for display. */
export function extractProbeSvg(answer: string): string | null {
  const candidate = answer.match(/<svg(?=[\s/>])[\s\S]*<\/svg\s*>/i)?.[0];
  if (!candidate) return null;
  const document = new DOMParser().parseFromString(candidate, "image/svg+xml");
  if (document.getElementsByTagName("parsererror").length > 0
    || document.documentElement.localName !== "svg"
    || document.documentElement.namespaceURI !== SVG_NAMESPACE) return null;
  return candidate;
}
