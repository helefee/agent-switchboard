import assert from "node:assert/strict";
import { test } from "node:test";
import type { TrayUsage, UsageReading } from "../src/api/client.ts";
import { applyLanguagePreference } from "../src/i18n/current.ts";
import { trayUsageContent } from "../src/tray/tray-usage.ts";

applyLanguagePreference("zh-CN");

const reading: UsageReading = { planName: "CNY", remaining: 131.5, used: null, total: null,
  unit: "CNY", availability: { kind: "available" } };
const script = (readings: UsageReading[], error: string | null = null): TrayUsage => ({
  kind: "script", reading: { summary: { readings, at: "2026-10-16T00:00:00Z" }, error },
});

test("tray projects missing readings without fabricating values", () => {
  assert.deepEqual(trayUsageContent(null), { readings: [], alert: null, summary: "" });
});

test("tray text and summary share one formatted reading and remove only repeated units", () => {
  const content = trayUsageContent(script([reading]));
  assert.deepEqual(content.readings[0], {
    name: null, value: "余额 131.5 CNY", tone: undefined, summary: "余额 131.5 CNY",
  });
  assert.equal(content.summary, content.readings[0].summary);
  assert.equal(trayUsageContent(script([{ ...reading, planName: "套餐 A" }])).readings[0].name, "套餐 A");
});

test("tray retains plan identity when unavailable text has no numeric unit", () => {
  const content = trayUsageContent(script([{ ...reading,
    availability: { kind: "unavailable", message: "套餐已过期" } }]));
  assert.equal(content.readings[0].name, "CNY");
  assert.equal(content.readings[0].value, "套餐已过期");
  assert.equal(content.readings[0].tone, "danger");
});

test("tray preserves cached readings and query diagnostics in the same projection", () => {
  const content = trayUsageContent(script([reading], "upstream failed"));
  assert.equal(content.readings[0].value, "余额 131.5 CNY");
  assert.equal(content.alert?.title, "查询失败");
  assert.match(content.alert!.detail, /upstream failed/);
  assert.match(content.summary, /余额 131.5 CNY.*查询失败/);
});

test("tray official windows use the committed interface language", () => {
  const usage: TrayUsage = { kind: "official", reading: { status: "available", at: null,
    stale: false, lastReset: null, windows: [{ label: "5 小时", usedPercent: 28, resetsAt: null }] } };
  try {
    applyLanguagePreference("en-US");
    assert.equal(trayUsageContent(usage).readings[0].summary, "5 hours · remaining 72%");
    applyLanguagePreference("zh-CN");
    assert.equal(trayUsageContent(usage).readings[0].summary, "5 小时 · 剩余 72%");
  } finally { applyLanguagePreference("zh-CN"); }
});
