import assert from "node:assert/strict";
import { test } from "node:test";
import { createProbeHtmlPreview } from "../src/lib/probe-html.ts";

const html = '<!doctype html><html><head><style>@keyframes wheel{to{transform:rotate(1turn)}}</style></head><body><svg id="bike"></svg><script>requestAnimationFrame(()=>document.querySelector("svg"));</script></body></html>';

test("保留完整文档、外层样式和动画脚本，移除文档外的回答说明", () => {
  const answer = `生成结果：\n\`\`\`html\n${html}\n\`\`\`\n`;
  const preview = createProbeHtmlPreview(answer);
  assert.ok(preview?.endsWith(`${html}\n`));
  assert.ok(createProbeHtmlPreview(`以下是代码：\n${html}\n说明结束。`)?.endsWith(html));
});

test("HTML 片段和大小写代码块可预览，普通说明不误作页面", () => {
  const fragment = '<style>svg{width:100%}</style><svg><animate attributeName="x"/></svg>';
  assert.ok(createProbeHtmlPreview(`\`\`\`HTML\r\n${fragment}\r\n\`\`\``)?.includes(fragment));
  assert.ok(createProbeHtmlPreview(fragment)?.endsWith(fragment));
  assert.equal(createProbeHtmlPreview("已写入 index.html，请打开查看。"), null);
  assert.equal(createProbeHtmlPreview(""), null);
});

test("多份 HTML 不任意选取一份掩盖输出歧义", () => {
  assert.equal(createProbeHtmlPreview(`\`\`\`html\n${html}\n\`\`\`\n\`\`\`html\n${html}\n\`\`\``), null);
});

test("预览文档在模型内容前声明外部资源与连接限制", () => {
  const preview = createProbeHtmlPreview(html);
  assert.ok(preview);
  assert.ok(preview.indexOf('http-equiv="Content-Security-Policy"') < preview.indexOf("<script>"));
  assert.ok(preview.includes("default-src 'none'"));
  assert.ok(preview.includes("connect-src 'none'"));
  assert.ok(preview.includes("base-uri 'none'"));
});
