import { useId, type ReactNode } from "react";
import type { CloseBehavior, RuntimeLogLevel, StartupPage } from "../api/client";

/* ---------------------------------------------------------------------------
 * Visual preference previews. Every preview draws on one shared 96×64 canvas
 * grammar so the settings page reads as a single illustration system: the
 * app's own window anatomy, its two theme palettes, and its semantic colors
 * (action blue / warning orange / danger red). Literal color values, not
 * theme tokens — each thumbnail must depict its own subject regardless of
 * the theme the settings page currently renders in.
 * ------------------------------------------------------------------------- */

/** A radio group whose options are self-drawn SVG previews instead of text
 * segments. The a11y contract matches the shared segment control exactly —
 * a hidden native radio inside each card label — so keyboard and
 * screen-reader behavior never diverges. */
export function AppPreviewSetting<T extends string | number>({ label, detail, value, options, busy, onChange }: {
  label: string;
  detail?: string;
  value: T;
  options: readonly { value: T; label: string; preview: ReactNode }[];
  busy: boolean;
  onChange: (value: T) => void;
}) {
  return (
    <div className="asb-app-setting-row asb-app-setting-row-preview">
      <div className="asb-app-setting-copy">
        <span className="asb-checkbox-label">{label}</span>
        {detail && <span className="asb-app-setting-detail">{detail}</span>}
      </div>
      <div className="asb-preview-cards" role="radiogroup" aria-label={label}>
        {options.map((option) => (
          <label key={option.value} className="asb-preview-card">
            <input
              type="radio"
              name={label}
              checked={value === option.value}
              disabled={busy}
              onChange={() => onChange(option.value)}
            />
            <span className="asb-preview-card-art" aria-hidden="true">{option.preview}</span>
            <span className="asb-preview-card-name">{option.label}</span>
          </label>
        ))}
      </div>
    </div>
  );
}

interface WindowPalette {
  window: string;
  rail: string;
  stroke: string;
  text: string;
  textSoft: string;
  textSofter: string;
  railDot: string;
  accent: string;
  danger: string;
  warning: string;
  info: string;
}

/* The values mirror the app's light/dark token pairs; the dark console set
   doubles as the log-level art surface (logs read on a dark surface in both
   themes). */
const LIGHT: WindowPalette = {
  window: "#ffffff",
  rail: "#eceef1",
  stroke: "rgba(20, 22, 27, 0.18)",
  text: "#14161b",
  textSoft: "rgba(20, 22, 27, 0.55)",
  textSofter: "rgba(20, 22, 27, 0.3)",
  railDot: "rgba(20, 22, 27, 0.32)",
  accent: "#0a84ff",
  danger: "#d70015",
  warning: "#b25000",
  info: "rgba(20, 22, 27, 0.45)",
};

const DARK: WindowPalette = {
  window: "#0d2133",
  rail: "#111e2b",
  stroke: "rgba(237, 240, 247, 0.2)",
  text: "#edf0f7",
  textSoft: "rgba(237, 240, 247, 0.55)",
  textSofter: "rgba(237, 240, 247, 0.28)",
  railDot: "rgba(237, 240, 247, 0.38)",
  accent: "#0a84ff",
  danger: "#ff453a",
  warning: "#ff9f0a",
  info: "rgba(237, 240, 247, 0.5)",
};

const CANVAS = { width: 96, height: 64 } as const;

/** Keep window surfaces fixed; content reserves space for the largest 125%
 * scale so every option shows complete controls inside the same frame. */
function WindowArt({ theme, split = false, zoom = 1 }: {
  theme: "light" | "dark";
  split?: boolean;
  zoom?: number;
}) {
  const id = useId();
  const windowClip = `${id}-window`;
  const leftClip = `${id}-left`;
  const rightClip = `${id}-right`;
  const art = (palette: WindowPalette) => {
    return (
      <>
        <rect x="1" y="1" width="94" height="62" rx="9" fill={palette.rail} stroke={palette.stroke} />
        <rect x="19" y="8" width="73" height="53" rx="5" fill={palette.window} stroke={palette.stroke} />
        <circle cx="10" cy="13" r={3 * zoom} fill={palette.accent} />
        <circle cx="10" cy="24" r={3 * zoom} fill={palette.railDot} />
        <circle cx="10" cy="35" r={3 * zoom} fill={palette.railDot} />
        <g transform={`translate(26 12) scale(${zoom})`}>
          <rect width="30" height="4" rx="2" fill={palette.text} />
          <rect y="10" width="48" height="3" rx="1.5" fill={palette.textSoft} />
          <rect y="17" width="40" height="3" rx="1.5" fill={palette.textSofter} />
          <rect y="24" width="44" height="3" rx="1.5" fill={palette.textSofter} />
          <rect y="29" width="24" height="8" rx="4" fill={palette.accent} />
          <rect x="28" y="29.5" width="18" height="7" rx="3.5" fill="none" stroke={palette.stroke} />
        </g>
      </>
    );
  };
  return (
    <svg viewBox="0 0 96 64" aria-hidden="true" focusable="false">
      <defs>
        <clipPath id={windowClip}><rect x="1" y="1" width="94" height="62" rx="9" /></clipPath>
        {split && <clipPath id={leftClip}><rect x="0" y="0" width="48" height="64" /></clipPath>}
        {split && <clipPath id={rightClip}><rect x="48" y="0" width="48" height="64" /></clipPath>}
      </defs>
      <g clipPath={`url(#${windowClip})`}>
        {split ? (
          <>
            <g clipPath={`url(#${leftClip})`}>{art(LIGHT)}</g>
            <g clipPath={`url(#${rightClip})`}>{art(DARK)}</g>
          </>
        ) : art(theme === "light" ? LIGHT : DARK)}
      </g>
    </svg>
  );
}

/** Theme-mode thumbnail: light, system (split), or dark window. */
export function ThemeModePreview({ mode }: { mode: "light" | "system" | "dark" }) {
  if (mode === "system") return <WindowArt theme="light" split />;
  return <WindowArt theme={mode} />;
}

/** Interface-scale thumbnail: the same window with its controls drawn
 * at the zoom factor the option applies. */
export function InterfaceScalePreview({ zoom }: { zoom: number }) {
  return <WindowArt theme="light" zoom={zoom} />;
}

/** Close-behavior thumbnail. Hide-to-tray: the window ghosts away while its
 * accent mark settles into the tray pill at the corner; exit: the frame goes
 * dashed, a danger close mark replaces the title row and one flat line says
 * the process ended. */
export function CloseBehaviorPreview({ behavior }: { behavior: CloseBehavior }) {
  if (behavior === "exit") {
    return (
      <svg viewBox={`0 0 ${CANVAS.width} ${CANVAS.height}`} aria-hidden="true" focusable="false">
        <rect x="1" y="1" width="94" height="62" rx="9" fill={LIGHT.window} fillOpacity="0.35"
          stroke={LIGHT.stroke} strokeDasharray="4 3" />
        <path d="M76 7 L86 17 M86 7 L76 17" stroke={LIGHT.danger} strokeWidth="2.4"
          strokeLinecap="round" fill="none" />
        <rect x="28" y="33" width="40" height="3.5" rx="1.75" fill={LIGHT.textSofter} />
      </svg>
    );
  }
  return (
    <svg viewBox={`0 0 ${CANVAS.width} ${CANVAS.height}`} aria-hidden="true" focusable="false">
      <g opacity="0.4">
        <rect x="1" y="1" width="94" height="62" rx="9" fill={LIGHT.rail} stroke={LIGHT.stroke} />
        <rect x="19" y="8" width="73" height="53" rx="5" fill={LIGHT.window} stroke={LIGHT.stroke} />
        <circle cx="10" cy="13" r="3" fill={LIGHT.accent} />
        <rect x="26" y="10" width="34" height="5" rx="2.5" fill={LIGHT.text} />
        <rect x="26" y="22" width="58" height="3.5" rx="1.75" fill={LIGHT.textSoft} />
        <rect x="26" y="30" width="50" height="3.5" rx="1.75" fill={LIGHT.textSofter} />
        <rect x="26" y="48" width="28" height="10" rx="5" fill={LIGHT.accent} />
      </g>
      <path d="M57 42 Q66 44 69 50" stroke={LIGHT.accent} strokeWidth="1.6"
        strokeDasharray="3 2" fill="none" />
      <polygon points="71.5,52.5 67.6,51.6 70.2,48.4" fill={LIGHT.accent} />
      <rect x="63" y="50" width="31" height="12" rx="6" fill={LIGHT.window} stroke={LIGHT.stroke} />
      <circle cx="70" cy="56" r="1.8" fill={LIGHT.railDot} />
      <circle cx="75.5" cy="56" r="1.8" fill={LIGHT.railDot} />
      <rect x="81" y="52" width="8" height="8" rx="2.5" fill={LIGHT.accent} />
    </svg>
  );
}

/** Motion thumbnail: one accent dot on a shared track. The system card is
 * the app's one deliberate looping demo (see app-settings.css for the
 * freeze rules); the reduce card is the same dot at rest beside stop bars. */
export function MotionPreview({ mode }: { mode: "system" | "reduce" }) {
  const resting = mode === "reduce";
  return (
    <svg viewBox={`0 0 ${CANVAS.width} ${CANVAS.height}`} aria-hidden="true" focusable="false">
      <rect x="1" y="1" width="94" height="62" rx="9" fill={LIGHT.window} stroke={LIGHT.stroke} />
      <rect x="22" y="29" width="52" height="6" rx="3" fill="rgba(20, 22, 27, 0.12)" />
      <circle cx="25" cy="32" r="4.5" fill={LIGHT.accent} className={resting ? undefined : "asb-motion-demo-dot"} />
      {resting ? (
        <>
          <rect x="66.5" y="27.5" width="2.8" height="9" rx="1.4" fill={LIGHT.textSoft} />
          <rect x="71.5" y="27.5" width="2.8" height="9" rx="1.4" fill={LIGHT.textSoft} />
        </>
      ) : (
        <>
          <rect x="8" y="24" width="11" height="8.5" rx="1.5" fill="none" stroke={LIGHT.textSoft} strokeWidth="1.3" />
          <path d="M13.5 32.5 L13.5 35 M9.5 35.5 L17.5 35.5" stroke={LIGHT.textSoft} strokeWidth="1.3" fill="none" />
        </>
      )}
    </svg>
  );
}

/** Log-level thumbnail: a dark console whose lines are kept or dropped by
 * the chosen threshold — the ladder the runtime filter actually applies.
 * Line colors follow the runtime log pill semantics (debug = action blue,
 * info = neutral, warn = warning, error = danger). */
const LOG_LADDER = [
  { level: "debug", bar: 44 },
  { level: "info", bar: 36 },
  { level: "warn", bar: 40 },
  { level: "error", bar: 32 },
] as const satisfies ReadonlyArray<{ level: RuntimeLogLevel; bar: number }>;

const LOG_RANK: Record<RuntimeLogLevel, number> = {
  debug: 0, info: 1, warn: 2, error: 3, silent: 4,
};

export function LogLevelPreview({ level }: { level: RuntimeLogLevel }) {
  const rank = LOG_RANK[level];
  const palette = DARK;
  return (
    <svg viewBox={`0 0 ${CANVAS.width} ${CANVAS.height}`} aria-hidden="true" focusable="false">
      <rect x="1" y="1" width="94" height="62" rx="9" fill={palette.window} stroke={palette.stroke} />
      {rank < 4 ? LOG_LADDER.map((line, index) => {
        if (LOG_RANK[line.level] < rank) return null;
        const y = 12 + index * 13;
        const color = line.level === "debug" ? palette.accent
          : line.level === "warn" ? palette.warning
          : line.level === "error" ? palette.danger
          : palette.info;
        return (
          <g key={line.level}>
            <rect x="10" y={y} width="18" height="7" rx="3.5" fill={color} />
            <rect x="34" y={y + 1.5} width={line.bar} height="4" rx="2" fill="rgba(237, 240, 247, 0.35)" />
          </g>
        );
      }) : (
        <rect x="28" y="30" width="40" height="4" rx="2" fill="rgba(237, 240, 247, 0.22)" />
      )}
    </svg>
  );
}

/** Startup-page thumbnail: the app window booting with the icon rail's first
 * slot active (always providers) versus a history slot active beside a
 * clock — the page the last session ended on. */
export function StartupPagePreview({ page }: { page: StartupPage }) {
  const palette = LIGHT;
  const railSlots = [13, 24, 35, 46];
  const activeSlot = page === "providers" ? 0 : 2;
  return (
    <svg viewBox={`0 0 ${CANVAS.width} ${CANVAS.height}`} aria-hidden="true" focusable="false">
      <rect x="1" y="1" width="94" height="62" rx="9" fill={palette.rail} stroke={palette.stroke} />
      <rect x="19" y="8" width="73" height="53" rx="5" fill={palette.window} stroke={palette.stroke} />
      {railSlots.map((cy, index) => (
        <circle key={cy} cx="10" cy={cy} r={index === activeSlot ? 3.5 : 2.5}
          fill={index === activeSlot ? palette.accent : palette.railDot} />
      ))}
      {page === "providers" ? (
        <>
          <rect x="26" y="12" width="58" height="12" rx="5.5" fill={palette.window} stroke={palette.stroke} />
          <circle cx="33" cy="18" r="2.2" fill={palette.accent} />
          <rect x="39" y="16.5" width="30" height="3" rx="1.5" fill={palette.textSoft} />
          <rect x="26" y="28" width="58" height="12" rx="5.5" fill={palette.window} stroke={palette.stroke} />
          <circle cx="33" cy="34" r="2.2" fill={palette.railDot} />
          <rect x="39" y="32.5" width="24" height="3" rx="1.5" fill={palette.textSofter} />
          <rect x="26" y="46" width="26" height="10" rx="5" fill={palette.accent} />
        </>
      ) : (
        <>
          <circle cx="33" cy="19" r="6" fill="none" stroke={palette.textSoft} strokeWidth="1.5" />
          <path d="M33 19 L33 14.8 M33 19 L36.2 20.6" stroke={palette.text} strokeWidth="1.5"
            strokeLinecap="round" fill="none" />
          <rect x="44" y="15" width="36" height="3.5" rx="1.75" fill={palette.textSofter} />
          <rect x="44" y="22" width="30" height="3.5" rx="1.75" fill={palette.textSofter} />
          <rect x="26" y="34" width="58" height="10" rx="5" fill={palette.window} stroke={palette.stroke} />
          <rect x="32" y="37.5" width="26" height="3" rx="1.5" fill={palette.textSoft} />
          <rect x="26" y="48" width="26" height="9" rx="4.5" fill={palette.accent} />
        </>
      )}
    </svg>
  );
}
