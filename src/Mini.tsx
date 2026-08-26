import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { fmtDuration } from "./lib/time";

/** Mirrors the Rust `MiniMode` enum. */
type MiniMode =
  | { kind: "timer" }
  | { kind: "alarm"; label: string; at: string };

/**
 * The always-on-top overlay.
 *
 * Two jobs in one small window: park a running stopwatch or focus session in
 * a corner of the screen while working elsewhere, and give a ringing alarm
 * something to dismiss even when the main window is hidden in the tray.
 *
 * The timer state is mirrored from the main window rather than owned here —
 * two independent countdowns would drift apart within seconds.
 */
export default function Mini() {
  const [mode, setMode] = useState<MiniMode>({ kind: "timer" });
  const [elapsed, setElapsed] = useState(0);
  const [running, setRunning] = useState(false);
  const [label, setLabel] = useState("Stopwatch");

  useEffect(() => {
    const stops = [
      listen<MiniMode>("scout://mini-mode", (e) => setMode(e.payload)),
      listen<{ displayMs: number; running: boolean; label: string }>(
        "scout://timer-tick",
        (e) => {
          setElapsed(e.payload.displayMs);
          setRunning(e.payload.running);
          setLabel(e.payload.label);
        },
      ),
    ];
    return () => {
      for (const s of stops) void s.then((un) => un());
    };
  }, []);

  const isAlarm = mode.kind === "alarm";

  return (
    <div
      // The window is borderless, so the whole surface is the drag handle —
      // except the buttons, which stop the event so a click still registers.
      data-tauri-drag-region
      className="flex h-screen w-screen cursor-grab flex-col justify-between overflow-hidden border px-3.5 py-2.5 select-none active:cursor-grabbing"
      style={{
        background: isAlarm
          ? "linear-gradient(135deg, rgba(217,119,87,.22), var(--color-panel))"
          : "var(--color-panel)",
        borderColor: isAlarm ? "var(--color-clay)" : "var(--color-line-2)",
      }}
    >
      <div data-tauri-drag-region className="flex items-baseline gap-2">
        <span
          className="font-mono text-[9px] tracking-[0.16em] uppercase"
          style={{ color: isAlarm ? "var(--color-clay-hot)" : "var(--color-faint)" }}
        >
          {isAlarm ? "Alarm" : running ? "Running" : "Paused"}
        </span>
        <span className="truncate font-mono text-[9px] tracking-[0.1em] text-faint uppercase">
          {isAlarm ? mode.label : label}
        </span>
      </div>

      <div
        data-tauri-drag-region
        className="font-mono text-[30px] leading-none font-bold tabular-nums"
        style={{ color: isAlarm || running ? "var(--color-clay)" : "var(--color-cream)" }}
      >
        {isAlarm ? mode.at : fmtDuration(elapsed)}
      </div>

      <div className="flex gap-1.5">
        {isAlarm ? (
          <>
            <MiniButton primary onClick={() => void invoke("dismiss_alarm")}>
              Stop
            </MiniButton>
            <MiniButton
              onClick={() => {
                // Silence it now; the scheduler will raise it again on its
                // own next occurrence. A true snooze needs a one-shot alarm,
                // which is a bigger change than this window should make.
                void invoke("stop_alarm_sound");
                void getCurrentWindow().hide();
              }}
            >
              Silence
            </MiniButton>
          </>
        ) : (
          <>
            <MiniButton
              onClick={() => {
                // Bring the full app back and put the overlay away.
                void invoke("show_main").catch(() => {});
                void getCurrentWindow().hide();
              }}
            >
              Back
            </MiniButton>
            <MiniButton onClick={() => void getCurrentWindow().hide()}>Close</MiniButton>
          </>
        )}
      </div>
    </div>
  );
}

function MiniButton({
  children,
  onClick,
  primary = false,
}: {
  children: React.ReactNode;
  onClick: () => void;
  primary?: boolean;
}) {
  return (
    <button
      type="button"
      // Without this the drag region swallows the click.
      onPointerDown={(e) => e.stopPropagation()}
      onClick={onClick}
      className={
        "flex-1 cursor-pointer rounded-full border px-2 py-1 font-mono text-[9px] tracking-[0.1em] uppercase transition-colors " +
        (primary
          ? "border-clay bg-clay text-[#1a1210] hover:border-clay-hot hover:bg-clay-hot"
          : "border-line-2 text-muted hover:border-cream hover:text-cream")
      }
    >
      {children}
    </button>
  );
}
