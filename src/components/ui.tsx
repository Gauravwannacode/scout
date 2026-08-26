import type { ReactNode } from "react";

export function cx(...parts: (string | false | null | undefined)[]): string {
  return parts.filter(Boolean).join(" ");
}

export function Bubble({
  children,
  className,
  tone = "panel",
}: {
  children: ReactNode;
  className?: string;
  tone?: "panel" | "panel-2";
}) {
  return (
    <div
      className={cx(
        "rounded-[18px] border border-line p-5",
        tone === "panel" ? "bg-panel" : "bg-panel-2",
        className,
      )}
    >
      {children}
    </div>
  );
}

export type ChipTone = "clay" | "quiet" | "sage";

export function Chip({
  children,
  tone = "quiet",
  pulse = false,
}: {
  children: ReactNode;
  tone?: ChipTone;
  pulse?: boolean;
}) {
  const tones: Record<ChipTone, string> = {
    clay: "border-clay text-clay-hot bg-[var(--clay-wash)]",
    quiet: "border-line-2 text-muted",
    sage: "border-sage/50 text-sage bg-sage/10",
  };
  return (
    <span
      className={cx(
        "inline-flex flex-none items-center gap-1.5 rounded-full border px-3 py-1.5",
        "font-mono text-[9.5px] font-medium tracking-[0.15em] uppercase",
        tones[tone],
      )}
    >
      {pulse && (
        <i
          className="block h-[5px] w-[5px] rounded-full bg-current"
          style={{ animation: "blink 2.6s ease-in-out infinite" }}
        />
      )}
      {children}
    </span>
  );
}

/** The labelled divider that opens each section of a page. */
export function SectionHead({ children }: { children: ReactNode }) {
  return (
    <div className="mb-3 flex items-center gap-2.5">
      <span className="eyebrow">{children}</span>
      <hr className="m-0 h-px flex-1 border-0 bg-line" />
    </div>
  );
}

export function Section({ label, children }: { label: string; children: ReactNode }) {
  return (
    <section className="mt-6 first:mt-0">
      <SectionHead>{label}</SectionHead>
      {children}
    </section>
  );
}

export function PageTitle({ title, sub }: { title: string; sub?: ReactNode }) {
  return (
    <div className="mb-5 flex flex-wrap items-baseline gap-3">
      <h1 className="font-serif text-[26px] leading-none tracking-[-0.012em]">{title}</h1>
      {sub && <span className="ml-auto font-mono text-[11px] text-faint">{sub}</span>}
    </div>
  );
}

export function Button({
  children,
  onClick,
  variant = "ghost",
  className,
  type = "button",
  title,
  disabled = false,
  "aria-label": ariaLabel,
}: {
  children: ReactNode;
  onClick?: () => void;
  variant?: "ghost" | "primary";
  className?: string;
  type?: "button" | "submit";
  title?: string;
  disabled?: boolean;
  "aria-label"?: string;
}) {
  return (
    <button
      type={type}
      title={title}
      aria-label={ariaLabel}
      onClick={onClick}
      disabled={disabled}
      className={cx(
        "rounded-full border px-4 py-2 font-mono text-[10px] tracking-[0.1em] uppercase transition-colors",
        // A dead control should look dead rather than merely refusing to work.
        disabled
          ? "cursor-not-allowed border-line text-faint opacity-50"
          : variant === "primary"
            ? "cursor-pointer border-clay bg-clay text-[#1a1210] hover:border-clay-hot hover:bg-clay-hot"
            : "cursor-pointer border-line-2 text-muted hover:border-cream hover:text-cream",
        className,
      )}
    >
      {children}
    </button>
  );
}

/** Shown wherever real content will land once the pipeline is running. */
export function Empty({ children }: { children: ReactNode }) {
  return (
    <div className="rounded-[12px] border border-dashed border-line-2 px-5 py-8 text-center">
      <p className="text-[13.5px] leading-relaxed text-faint">{children}</p>
    </div>
  );
}
