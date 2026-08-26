import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";
import { isDesktop } from "../lib/sqliteRepo";
import { Bubble, Button, Chip, Section, PageTitle, cx } from "../components/ui";
import type { Store } from "../lib/store";
import {
  DAY_LABELS,
  describeDays,
  fmtClock,
  fmtDuration,
  nextOccurrence,
  relativeTo,
  useNow,
} from "../lib/time";
import type { TimerApi } from "../lib/time";

const FOCUS_PRESETS = [15, 25, 45, 60];

export default function ClockPage({ store, timer }: { store: Store; timer: TimerApi }) {
  const now = useNow(1000);

  // The overlay shows this timer, it does not run one. Two independent
  // countdowns would drift apart within seconds, so the single source of
  // truth stays here and the overlay is pushed each change.
  useEffect(() => {
    if (!isDesktop()) return;
    void emitTo("mini", "scout://timer-tick", {
      displayMs: timer.displayMs,
      running: timer.running,
      label: timer.mode === "focus" ? `${timer.focusMinutes} min focus` : "Stopwatch",
    }).catch(() => {});
  }, [timer.displayMs, timer.running, timer.mode, timer.focusMinutes]);

  const { time, ampm } = fmtClock(now);
  const enabledCount = store.alarms.filter((a) => a.enabled).length;

  return (
    <div>
      <PageTitle
        title="Clock"
        sub={`${enabledCount} ${enabledCount === 1 ? "alarm" : "alarms"} on`}
      />

      <Section label="Now">
        <div className="flex flex-col items-center gap-1 rounded-[18px] border border-line bg-panel-2 px-6 py-9">
          <div className="font-serif text-[clamp(60px,12vw,104px)] leading-none tracking-[-0.025em] tabular-nums">
            {time}
            <span className="ml-2.5 text-[0.3em] tracking-[0.06em] text-faint">{ampm}</span>
          </div>
          <div className="mt-2.5 font-mono text-[11px] tracking-[0.13em] text-muted uppercase">
            {now.toLocaleDateString(undefined, {
              weekday: "long",
              day: "numeric",
              month: "long",
            })}
          </div>
        </div>
      </Section>

      <Section label="Session">
        <TimerCard timer={timer} />
      </Section>

      <Section label="Alarms">
        <AlarmList store={store} now={now} />
      </Section>
    </div>
  );
}

function TimerCard({ timer }: { timer: TimerApi }) {
  return (
    <div className="flex flex-col gap-4 rounded-[18px] border border-line bg-panel-2 p-5">
      <div className="flex gap-1.5 rounded-full border border-line bg-void p-1">
        {(["focus", "stopwatch"] as const).map((m) => (
          <button
            key={m}
            onClick={() => timer.setMode(m)}
            aria-pressed={timer.mode === m}
            className={cx(
              "flex-1 cursor-pointer rounded-full px-1 py-2 text-center text-[12.5px] font-semibold capitalize transition-colors",
              timer.mode === m ? "bg-panel-3 text-cream" : "text-muted hover:text-cream",
            )}
          >
            {m}
          </button>
        ))}
      </div>

      <div className="flex flex-col items-center gap-1">
        <div
          className={cx(
            "font-mono text-[clamp(38px,7vw,54px)] font-bold tracking-[-0.02em] tabular-nums transition-colors",
            timer.running ? "text-clay" : "text-cream",
          )}
        >
          {fmtDuration(timer.displayMs)}
        </div>
        <div className="font-mono text-[10px] tracking-[0.16em] text-faint uppercase">
          {timer.finished
            ? "Session complete"
            : timer.mode === "focus"
              ? `${timer.focusMinutes} minute focus`
              : "Stopwatch"}
        </div>
      </div>

      <div className="h-[3px] overflow-hidden rounded-full border border-line bg-void">
        <div
          className="h-full bg-clay transition-[width] duration-300 ease-linear"
          style={{ width: `${timer.progress * 100}%` }}
        />
      </div>

      {timer.mode === "focus" && (
        <div className="flex justify-center gap-1.5">
          {FOCUS_PRESETS.map((m) => (
            <button
              key={m}
              onClick={() => {
                timer.reset();
                timer.setFocusMinutes(m);
              }}
              className={cx(
                "cursor-pointer rounded-full border px-3 py-1 font-mono text-[10px] transition-colors",
                timer.focusMinutes === m
                  ? "border-clay text-clay"
                  : "border-line text-faint hover:border-line-2 hover:text-muted",
              )}
            >
              {m}m
            </button>
          ))}
        </div>
      )}

      <div className="flex gap-2">
        <Button
          variant="primary"
          className="flex-1 py-2.5 text-center"
          onClick={() => (timer.running ? timer.pause() : timer.start())}
        >
          {timer.running ? "Pause" : "Start"}
        </Button>
        <Button className="flex-1 py-2.5 text-center" onClick={timer.reset}>
          Reset
        </Button>
        {isDesktop() && (
          <Button
            className="flex-1 py-2.5 text-center"
            title="Park the timer on top of whatever you are working in"
            onClick={() => void invoke("open_mini")}
          >
            Minimise
          </Button>
        )}
      </div>
    </div>
  );
}

function AlarmList({ store, now }: { store: Store; now: Date }) {
  const [adding, setAdding] = useState(false);

  return (
    <div className="flex flex-col gap-2.5">
      {store.alarms.map((a) => {
        const next = nextOccurrence(a, now);
        return (
          <div
            key={a.id}
            className="group flex items-center gap-3.5 rounded-[12px] border border-line bg-panel-2 px-4 py-3"
          >
            <span
              className={cx(
                "font-mono text-[22px] font-medium tracking-[-0.01em] tabular-nums",
                a.enabled ? "text-cream" : "text-faint",
              )}
            >
              {a.at}
            </span>
            <span className="flex flex-col gap-px">
              <b
                className={cx(
                  "text-[13px] font-semibold",
                  a.enabled ? "text-cream" : "text-faint",
                )}
              >
                {a.label}
              </b>
              <small className="font-mono text-[10px] tracking-[0.06em] text-faint">
                {describeDays(a.days)}
                {a.enabled && next && ` · ${relativeTo(next, now)}`}
              </small>
            </span>

            <button
              onClick={() => store.removeAlarm(a.id)}
              title="Delete alarm"
              className="ml-auto cursor-pointer px-1 font-mono text-[10px] text-faint opacity-0 transition-opacity group-hover:opacity-100 hover:text-cream"
            >
              Delete
            </button>

            <button
              onClick={() => store.toggleAlarm(a.id)}
              role="switch"
              aria-checked={a.enabled}
              aria-label={`${a.label} alarm`}
              className={cx(
                "relative h-[21px] w-[38px] flex-none cursor-pointer rounded-full border transition-colors",
                a.enabled ? "border-clay bg-[var(--clay-wash)]" : "border-line-2 bg-panel-3",
              )}
            >
              <span
                className={cx(
                  "absolute top-[2px] block h-[15px] w-[15px] rounded-full transition-all",
                  a.enabled ? "left-[19px] bg-clay" : "left-[2px] bg-faint",
                )}
              />
            </button>
          </div>
        );
      })}

      {adding ? (
        <AlarmForm
          onCancel={() => setAdding(false)}
          onSave={async (a) => {
            await store.addAlarm(a);
            setAdding(false);
          }}
        />
      ) : (
        <Button className="self-start" onClick={() => setAdding(true)}>
          + Add alarm
        </Button>
      )}
    </div>
  );
}

function AlarmForm({
  onSave,
  onCancel,
}: {
  onSave(a: { at: string; label: string; days: number[]; enabled: boolean }): void;
  onCancel(): void;
}) {
  const [at, setAt] = useState("07:00");
  const [label, setLabel] = useState("");
  const [days, setDays] = useState<number[]>([1, 2, 3, 4, 5]);

  return (
    <Bubble tone="panel-2" className="flex flex-col gap-3.5">
      <div className="flex flex-wrap items-center gap-3">
        <input
          type="time"
          value={at}
          onChange={(e) => setAt(e.target.value)}
          className="rounded-lg border border-line bg-void px-3 py-2 font-mono text-[18px] text-cream tabular-nums"
        />
        <input
          type="text"
          value={label}
          placeholder="What is it for?"
          onChange={(e) => setLabel(e.target.value)}
          className="min-w-40 flex-1 rounded-lg border border-line bg-void px-3 py-2.5 text-[14px] text-cream placeholder:text-faint"
        />
      </div>

      <div className="flex flex-wrap gap-1.5">
        {DAY_LABELS.map((d, i) => (
          <button
            key={d}
            onClick={() => setDays((prev) => (prev.includes(i) ? prev.filter((x) => x !== i) : [...prev, i]))}
            aria-pressed={days.includes(i)}
            className={cx(
              "cursor-pointer rounded-full border px-3 py-1.5 font-mono text-[10px] transition-colors",
              days.includes(i)
                ? "border-clay text-clay"
                : "border-line text-faint hover:border-line-2 hover:text-muted",
            )}
          >
            {d}
          </button>
        ))}
      </div>

      <div className="flex gap-2">
        <Button
          variant="primary"
          onClick={() => onSave({ at, label: label.trim() || "Alarm", days, enabled: true })}
        >
          Save
        </Button>
        <Button onClick={onCancel}>Cancel</Button>
        {days.length === 0 && <Chip>Fires once</Chip>}
      </div>
    </Bubble>
  );
}
