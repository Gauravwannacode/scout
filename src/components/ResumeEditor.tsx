import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button, cx } from "./ui";
import { isDesktop } from "../lib/sqliteRepo";

interface Fact {
  id: string;
  kind: string;
  title: string;
  subtitle: string | null;
  detail: string | null;
  tech: string | null;
  url: string | null;
  level: string | null;
  period: string | null;
  sortOrder: number;
  enabled: boolean;
}

interface Header {
  name: string;
  headline: string;
  email: string;
  phone: string;
  github: string;
  linkedin: string;
  location: string;
}

interface ResumeData {
  header: Header;
  facts: Fact[];
}

const KINDS = [
  { id: "project", label: "Projects" },
  { id: "skill", label: "Skills" },
  { id: "education", label: "Education" },
  { id: "award", label: "Coursework" },
];

/**
 * The fact base editor.
 *
 * Everything a tailored resume can say lives here, so this is the one place
 * where a claim can be introduced or corrected. Switching a fact off keeps it
 * on disk but removes it from consideration entirely — safer than deleting
 * something that turns out to matter for a different company.
 */
export default function ResumeEditor({ onClose }: { onClose: () => void }) {
  const [data, setData] = useState<ResumeData | null>(null);
  const [kind, setKind] = useState("project");
  const [status, setStatus] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);
  const [openId, setOpenId] = useState<string | null>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  useEffect(() => {
    if (!isDesktop()) return;
    void invoke<ResumeData>("get_resume")
      .then(setData)
      .catch((e) => setStatus(`Could not read: ${e}`));
  }, []);

  function patch(id: string, changes: Partial<Fact>) {
    setData((d) =>
      d
        ? { ...d, facts: d.facts.map((f) => (f.id === id ? { ...f, ...changes } : f)) }
        : d,
    );
    setDirty(true);
  }

  function patchHeader(changes: Partial<Header>) {
    setData((d) => (d ? { ...d, header: { ...d.header, ...changes } } : d));
    setDirty(true);
  }

  function remove(id: string) {
    setData((d) => (d ? { ...d, facts: d.facts.filter((f) => f.id !== id) } : d));
    setDirty(true);
  }

  function add() {
    if (!data) return;
    const existing = data.facts.filter((f) => f.kind === kind);
    const fact: Fact = {
      // Time-based so a new fact cannot collide with a seeded id.
      id: `${kind}-${Date.now().toString(36)}`,
      kind,
      title: "",
      subtitle: null,
      detail: null,
      tech: null,
      url: null,
      level: kind === "skill" ? "Comfortable" : null,
      period: null,
      sortOrder: (existing.at(-1)?.sortOrder ?? 0) + 10,
      enabled: true,
    };
    setData({ ...data, facts: [...data.facts, fact] });
    setOpenId(fact.id);
    setDirty(true);
  }

  async function save() {
    if (!data) return;
    try {
      await invoke("save_resume", { header: data.header, facts: data.facts });
      setStatus("Saved.");
      setDirty(false);
    } catch (e) {
      setStatus(`Could not save: ${e}`);
    }
  }

  async function reset() {
    try {
      const fresh = await invoke<ResumeData>("reset_resume");
      setData(fresh);
      setStatus("Restored the original fact base.");
      setDirty(false);
    } catch (e) {
      setStatus(`Could not reset: ${e}`);
    }
  }

  const shown = (data?.facts ?? [])
    .filter((f) => f.kind === kind)
    .sort((a, b) => a.sortOrder - b.sortOrder);

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto bg-void/80 p-6 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="mt-8 mb-8 flex w-full max-w-[720px] flex-col gap-4 rounded-[18px] border border-line-2 bg-panel p-6"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-baseline gap-3">
          <h2 className="font-serif text-[24px] leading-none tracking-[-0.012em]">Your resume</h2>
          <span className="font-mono text-[10px] tracking-[0.1em] text-faint uppercase">
            everything a tailored resume can say
          </span>
          <button
            type="button"
            onClick={onClose}
            className="ml-auto cursor-pointer font-mono text-[10px] tracking-[0.1em] text-faint uppercase hover:text-cream"
          >
            Close
          </button>
        </div>

        {!isDesktop() ? (
          <p className="text-[13px] text-muted">The resume editor needs the desktop app.</p>
        ) : !data ? (
          <p className="font-mono text-[11px] text-faint">Reading…</p>
        ) : (
          <>
            <div className="grid grid-cols-2 gap-2.5">
              <Text label="Name" value={data.header.name} onChange={(v) => patchHeader({ name: v })} />
              <Text label="Location" value={data.header.location} onChange={(v) => patchHeader({ location: v })} />
              <Text label="Email" value={data.header.email} onChange={(v) => patchHeader({ email: v })} />
              <Text label="Phone" value={data.header.phone} onChange={(v) => patchHeader({ phone: v })} />
              <Text label="GitHub" value={data.header.github} onChange={(v) => patchHeader({ github: v })} />
              <Text label="LinkedIn" value={data.header.linkedin} onChange={(v) => patchHeader({ linkedin: v })} />
            </div>
            <Text
              label="Default headline"
              value={data.header.headline}
              onChange={(v) => patchHeader({ headline: v })}
              hint="Used when tailoring does not write its own."
            />

            <div className="flex flex-wrap gap-2 border-t border-line pt-4">
              {KINDS.map((k) => {
                const n = data.facts.filter((f) => f.kind === k.id).length;
                return (
                  <button
                    key={k.id}
                    type="button"
                    onClick={() => setKind(k.id)}
                    className={cx(
                      "cursor-pointer rounded-full border px-3.5 py-1.5 font-mono text-[10px] tracking-[0.1em] uppercase transition-colors",
                      kind === k.id
                        ? "border-clay bg-clay text-[#1a1210]"
                        : "border-line-2 text-muted hover:border-cream hover:text-cream",
                    )}
                  >
                    {k.label} · {n}
                  </button>
                );
              })}
            </div>

            <div className="flex flex-col gap-2">
              {shown.map((f) => (
                <FactRow
                  key={f.id}
                  fact={f}
                  open={openId === f.id}
                  onToggleOpen={() => setOpenId(openId === f.id ? null : f.id)}
                  onPatch={(c) => patch(f.id, c)}
                  onRemove={() => remove(f.id)}
                />
              ))}
              {shown.length === 0 && (
                <p className="font-mono text-[11px] text-faint">Nothing here yet.</p>
              )}
            </div>

            <div className="flex flex-wrap items-center gap-2 border-t border-line pt-4">
              <Button variant="primary" onClick={() => void save()} disabled={!dirty}>
                {dirty ? "Save changes" : "Saved"}
              </Button>
              <Button onClick={add}>Add {kind}</Button>
              <Button
                onClick={() => {
                  if (confirm("Restore the original fact base? Your edits will be lost.")) {
                    void reset();
                  }
                }}
              >
                Reset
              </Button>
              {status && (
                <span className="font-mono text-[10.5px] text-muted">{status}</span>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
}

function FactRow({
  fact,
  open,
  onToggleOpen,
  onPatch,
  onRemove,
}: {
  fact: Fact;
  open: boolean;
  onToggleOpen: () => void;
  onPatch: (c: Partial<Fact>) => void;
  onRemove: () => void;
}) {
  const isSkill = fact.kind === "skill";

  return (
    <div
      className={cx(
        "flex flex-col gap-2.5 rounded-[12px] border px-3.5 py-3",
        fact.enabled ? "border-line bg-panel-2" : "border-line bg-panel opacity-55",
      )}
    >
      <div className="flex items-center gap-2.5">
        <button
          type="button"
          role="switch"
          aria-checked={fact.enabled}
          aria-label={`Include ${fact.title || "this entry"}`}
          onClick={() => onPatch({ enabled: !fact.enabled })}
          title={fact.enabled ? "Included" : "Hidden from every resume"}
          className={cx(
            "relative h-[18px] w-[32px] flex-none cursor-pointer rounded-full border transition-colors",
            fact.enabled ? "border-clay bg-clay/15" : "border-line-2 bg-panel-3",
          )}
        >
          <span
            className={cx(
              "absolute top-[2px] block h-[12px] w-[12px] rounded-full transition-all",
              fact.enabled ? "left-[16px] bg-clay" : "left-[2px] bg-faint",
            )}
          />
        </button>

        <input
          value={fact.title}
          onChange={(e) => onPatch({ title: e.target.value })}
          placeholder="Title"
          className="flex-1 bg-transparent text-[13.5px] font-semibold text-cream outline-none"
        />

        {isSkill && (
          <input
            value={fact.level ?? ""}
            onChange={(e) => onPatch({ level: e.target.value })}
            placeholder="Level"
            className="w-[150px] bg-transparent text-right font-mono text-[10.5px] text-muted outline-none"
          />
        )}

        <button
          type="button"
          onClick={onToggleOpen}
          className="cursor-pointer font-mono text-[10px] tracking-[0.1em] text-faint uppercase hover:text-cream"
        >
          {open ? "Less" : "Edit"}
        </button>
      </div>

      {open && (
        <div className="flex flex-col gap-2.5 border-t border-line pt-2.5">
          <div className="grid grid-cols-2 gap-2.5">
            <Text label={isSkill ? "Group" : "Org"} value={fact.subtitle ?? ""} onChange={(v) => onPatch({ subtitle: v })} />
            <Text label="Period" value={fact.period ?? ""} onChange={(v) => onPatch({ period: v })} />
          </div>
          {!isSkill && (
            <>
              <label className="flex flex-col gap-1.5">
                <span className="font-mono text-[9.5px] tracking-[0.17em] text-faint uppercase">
                  What it does
                </span>
                <textarea
                  value={fact.detail ?? ""}
                  onChange={(e) => onPatch({ detail: e.target.value })}
                  rows={4}
                  className="w-full resize-y rounded-[10px] border border-line bg-panel px-3 py-2 text-[12.5px] leading-relaxed text-cream outline-none focus:border-line-2"
                />
              </label>
              <div className="grid grid-cols-2 gap-2.5">
                <Text label="Stack" value={fact.tech ?? ""} onChange={(v) => onPatch({ tech: v })} />
                <Text label="Link" value={fact.url ?? ""} onChange={(v) => onPatch({ url: v })} />
              </div>
            </>
          )}
          <div className="flex items-center gap-2.5">
            <Text
              label="Order"
              value={String(fact.sortOrder)}
              onChange={(v) => onPatch({ sortOrder: Number(v) || 0 })}
              hint="Lower shows first."
            />
            <button
              type="button"
              onClick={onRemove}
              className="mt-4 cursor-pointer self-start font-mono text-[10px] tracking-[0.1em] text-faint uppercase hover:text-clay-hot"
            >
              Delete
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

function Text({
  label,
  value,
  onChange,
  hint,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  hint?: string;
}) {
  return (
    <label className="flex flex-1 flex-col gap-1.5">
      <span className="font-mono text-[9.5px] tracking-[0.17em] text-faint uppercase">{label}</span>
      <input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        spellCheck={false}
        className="w-full rounded-[10px] border border-line bg-panel-2 px-3 py-2 text-[12.5px] text-cream outline-none focus:border-line-2"
      />
      {hint && <span className="text-[11px] text-muted">{hint}</span>}
    </label>
  );
}
