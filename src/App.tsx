import { useCallback, useEffect, useState } from "react";
import type { ReactNode } from "react";
import { cx } from "./components/ui";
import { useStore } from "./lib/store";
import { invoke } from "@tauri-apps/api/core";
import { useTimer } from "./lib/time";
import Settings from "./components/Settings";
import { isDesktop } from "./lib/sqliteRepo";
import HomePage from "./pages/Home";
import NewsPage from "./pages/News";
import ClockPage from "./pages/Clock";
import TodoPage from "./pages/Todo";

export type Page = "home" | "news" | "clock" | "todo";

const PAGES: { id: Page; label: string; icon: ReactNode }[] = [
  {
    id: "home",
    label: "Home",
    icon: <path d="M4 11l8-6 8 6v8a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1z" />,
  },
  {
    id: "news",
    label: "News",
    icon: (
      <>
        <path d="M4 5h16v14H4z" />
        <path d="M7 9h6M7 13h10" />
      </>
    ),
  },
  {
    id: "clock",
    label: "Clock",
    icon: (
      <>
        <circle cx="12" cy="13" r="8" />
        <path d="M12 9v4l3 2M9 2h6" />
      </>
    ),
  },
  {
    id: "todo",
    label: "To-do",
    icon: (
      <>
        <path d="M9 6h11M9 12h11M9 18h11" />
        <path d="M4 6l1 1 2-2M4 12l1 1 2-2M4 18l1 1 2-2" />
      </>
    ),
  },
];

export default function App() {
  const [page, setPage] = useState<Page>("home");
  const [online, setOnline] = useState(navigator.onLine);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const store = useStore();

  // A completed focus session is worth recording even if the app is closed
  // right after, so it is written the moment the timer resolves.
  const onFinish = useCallback(() => {
    const now = new Date();
    store.logSession({
      startedAt: new Date(now.getTime() - 25 * 60_000).toISOString(),
      endedAt: now.toISOString(),
      taskId: null,
      mode: "focus",
      completed: true,
    });
    // A finished session should be audible — he minimised the timer precisely
    // so he would not be watching it.
    if (isDesktop()) void invoke("play_chime").catch(() => {});
  }, [store]);

  const timer = useTimer(25, onFinish);

  useEffect(() => {
    const on = () => setOnline(true);
    const off = () => setOnline(false);
    window.addEventListener("online", on);
    window.addEventListener("offline", off);
    return () => {
      window.removeEventListener("online", on);
      window.removeEventListener("offline", off);
    };
  }, []);

  if (!store.ready) {
    return <div className="flex h-full items-center justify-center text-faint">Loading…</div>;
  }

  return (
    <div className="mx-auto flex h-full max-w-[1000px] flex-col px-5 py-4">
      <nav
        className="flex gap-1 overflow-x-auto rounded-full border border-line bg-panel-2 p-1.5"
        aria-label="Pages"
      >
        {PAGES.map((p) => (
          <button
            key={p.id}
            onClick={() => setPage(p.id)}
            aria-current={page === p.id ? "page" : undefined}
            className={cx(
              "flex flex-none cursor-pointer items-center gap-2 rounded-full px-4 py-2 text-[13.5px] font-semibold transition-colors",
              page === p.id ? "bg-panel-3 text-cream" : "text-muted hover:text-cream",
            )}
          >
            <svg
              viewBox="0 0 24 24"
              className={cx("h-[15px] w-[15px] flex-none", page === p.id && "text-clay")}
              fill="none"
              stroke="currentColor"
              strokeWidth={1.7}
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              {p.icon}
            </svg>
            {p.label}
          </button>
        ))}

        {/* The timer keeps running across pages, so it stays visible. */}
        {timer.running && (
          <span className="ml-auto flex flex-none items-center gap-2 self-center pr-3 font-mono text-[11px] text-clay tabular-nums">
            <i
              className="block h-[5px] w-[5px] rounded-full bg-clay"
              style={{ animation: "blink 2.6s ease-in-out infinite" }}
            />
            focus
          </span>
        )}
      </nav>

      <main className="min-h-0 flex-1 overflow-y-auto py-6">
        {page === "home" && <HomePage store={store} go={setPage} />}
        {page === "news" && <NewsPage store={store} />}
        {page === "clock" && <ClockPage store={store} timer={timer} />}
        {page === "todo" && <TodoPage store={store} />}
      </main>

      <footer className="flex items-center gap-2 border-t border-line pt-3 font-mono text-[10px] text-faint">
        <span>Scout</span>
        <button
          type="button"
          onClick={() => setSettingsOpen(true)}
          className="cursor-pointer font-mono text-[10px] tracking-[0.1em] text-faint uppercase hover:text-cream"
        >
          Settings
        </button>
        <span className="ml-auto flex items-center gap-1.5">
          <i
            className={cx(
              "block h-[5px] w-[5px] rounded-full",
              online ? "bg-sage" : "bg-faint",
            )}
          />
          {online ? "online" : "offline — everything but news still works"}
        </span>
      </footer>

      {settingsOpen && <Settings onClose={() => setSettingsOpen(false)} />}
    </div>
  );
}
