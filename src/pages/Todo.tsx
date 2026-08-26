import { useState } from "react";
import { Button, Chip, Empty, Section, PageTitle, cx } from "../components/ui";
import type { Store } from "../lib/store";
import { hostOf, openStory } from "../lib/open";
import { todaysTasks } from "../lib/store";
import type { Task } from "../types";
import { relativeTo, useNow } from "../lib/time";

export default function TodoPage({ store }: { store: Store }) {
  const now = useNow(30_000);
  const list = todaysTasks(store.tasks);
  const done = store.tasks.filter((t) => t.status === "done").length;

  const deadlines = store.tasks
    .filter((t) => t.dueAt && t.status === "open")
    .sort((a, b) => (a.dueAt ?? "").localeCompare(b.dueAt ?? ""));

  const focusedMs = store.sessions
    .filter((s) => s.completed && s.endedAt)
    .reduce((sum, s) => sum + (Date.parse(s.endedAt!) - Date.parse(s.startedAt)), 0);
  const focusedHours = Math.round((focusedMs / 3_600_000) * 10) / 10;

  return (
    <div>
      <PageTitle title="To-do" sub={`${done} of ${store.tasks.length} done`} />

      <Section label="Today">
        <TaskComposer onAdd={(t) => store.addTask(t)} />
        <div className="mt-2.5 flex flex-col gap-2">
          {list.length === 0 ? (
            <Empty>Nothing yet. Add the first thing you want to get done today.</Empty>
          ) : (
            list.map((t) => <TaskRow key={t.id} task={t} store={store} now={now} />)
          )}
        </div>
      </Section>

      <Section label="Deadlines">
        {deadlines.length === 0 ? (
          <Empty>
            No deadlines. Accepting an opening from News will drop one here automatically.
          </Empty>
        ) : (
          <div className="flex flex-col gap-2.5">
            {deadlines.map((t) => {
              const due = new Date(t.dueAt!);
              const soon = due.getTime() - now.getTime() < 3 * 86_400_000;
              return (
                <div
                  key={t.id}
                  className={cx(
                    "flex flex-col gap-2 rounded-[12px] border p-4",
                    soon ? "border-clay-deep bg-panel-2" : "border-line bg-panel-2",
                  )}
                >
                  <div className="flex flex-wrap items-center gap-2.5">
                    <Chip tone={soon ? "clay" : "quiet"}>{relativeTo(due, now)}</Chip>
                    <h4 className="text-[15px] font-semibold">{t.title}</h4>
                  </div>
                  <p className="font-mono text-[10.5px] text-faint">
                    {due.toLocaleDateString(undefined, {
                      weekday: "long",
                      day: "numeric",
                      month: "long",
                    })}
                  </p>
                </div>
              );
            })}
          </div>
        )}
      </Section>

      <Section label="This week">
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <Stat value={focusedHours > 0 ? `${focusedHours}h` : "—"} label={`Focused, across ${store.sessions.length} sessions`} />
          <Stat value={String(done)} label="Tasks finished" />
          <Stat value={String(store.tasks.filter((t) => t.itemId).length)} label="Openings accepted" />
        </div>
      </Section>
    </div>
  );
}

function Stat({ value, label }: { value: string; label: string }) {
  return (
    <div className="rounded-[12px] border border-line bg-panel-2 px-4 py-4">
      <b className="block font-serif text-[34px] leading-none tabular-nums">{value}</b>
      <span className="mt-1.5 block text-[12.5px] text-muted">{label}</span>
    </div>
  );
}

function TaskComposer({ onAdd }: { onAdd(title: string): void }) {
  const [text, setText] = useState("");
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        onAdd(text);
        setText("");
      }}
      className="flex gap-2"
    >
      <input
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder="Add something to do"
        className="flex-1 rounded-[12px] border border-line bg-panel-2 px-4 py-3 text-[14px] text-cream placeholder:text-faint"
      />
      <Button type="submit" variant="primary">
        Add
      </Button>
    </form>
  );
}

function TaskRow({ task, store, now }: { task: Task; store: Store; now: Date }) {
  const done = task.status === "done";
  // Present only for tasks created by accepting an opening, and only while
  // that item is still in the store.
  const source = task.itemId ? store.items.find((i) => i.id === task.itemId) : undefined;
  const due = task.dueAt ? new Date(task.dueAt) : null;

  return (
    <div
      className={cx(
        "group flex items-center gap-3 rounded-[12px] border px-3.5 py-3 transition-colors",
        done ? "border-line bg-panel" : "border-line bg-panel-2",
      )}
    >
      <button
        onClick={() => store.toggleTask(task.id)}
        role="checkbox"
        aria-checked={done}
        aria-label={task.title}
        className={cx(
          "h-[15px] w-[15px] flex-none cursor-pointer rounded-[5px] border-[1.5px] transition-colors",
          done ? "border-faint bg-faint" : "border-line-2 hover:border-clay",
        )}
      />
      <p
        className={cx(
          "mr-auto text-[13.5px] leading-tight",
          done && "text-faint line-through",
        )}
      >
        {task.title}
      </p>
      {source && (
        // An accepted opening should lead back to where it came from —
        // otherwise the task is a line of text with nowhere to go.
        <button
          onClick={() => void openStory(source.url)}
          title={`Opens ${hostOf(source.url)} in your browser`}
          className="flex-none cursor-pointer rounded-full border border-line-2 px-2.5 py-1 font-mono text-[9.5px] tracking-[0.1em] text-muted uppercase transition-colors hover:border-cream hover:text-cream"
        >
          Open
        </button>
      )}
      <button
        onClick={() => store.removeTask(task.id)}
        title="Delete task"
        className="cursor-pointer font-mono text-[10px] text-faint opacity-0 transition-opacity group-hover:opacity-100 hover:text-cream"
      >
        Delete
      </button>
      {due && (
        <span className="flex-none font-mono text-[10px] text-faint">{relativeTo(due, now)}</span>
      )}
    </div>
  );
}
