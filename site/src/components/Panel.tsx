import type { ReactNode } from "react";

/** Surface card with a mono header strip; the building block of every section. */
export function Panel({
  label,
  meta,
  live = false,
  className = "",
  children,
}: {
  label: string;
  meta?: string;
  live?: boolean;
  className?: string;
  children: ReactNode;
}) {
  return (
    <div
      className={`flex h-full flex-col rounded-card border border-line bg-surface shadow-[inset_0_1px_0_0_rgba(255,255,255,0.04),0_20px_40px_-24px_rgba(0,0,0,0.9)] transition-[translate,border-color] duration-500 ease-out-soft hover:-translate-y-1 hover:border-zinc-600 ${className}`}
    >
      <div className="flex items-center gap-2 border-b border-line px-4 py-3">
        {live && <span className="live-dot size-1.5 rounded-full bg-accent" />}
        <span className="label text-muted">{label}</span>
        {meta && <span className="label ml-auto text-zinc-500">{meta}</span>}
      </div>
      <div className="flex-1">{children}</div>
    </div>
  );
}
