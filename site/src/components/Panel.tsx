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
      className={`flex h-full flex-col rounded-card border border-line bg-surface transition-colors duration-300 hover:border-line-strong ${className}`}
    >
      <div className="flex flex-wrap items-center gap-2 border-b border-line px-4 py-3">
        {live && <span className="live-dot size-1.5 rounded-full bg-accent" />}
        <span className="label text-muted">{label}</span>
        {meta && <span className="label ml-auto text-muted">{meta}</span>}
      </div>
      <div className="flex-1">{children}</div>
    </div>
  );
}
