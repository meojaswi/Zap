import type { Release } from "../useLatestRelease";

export function DownloadButton({ release }: { release: Release }) {
  return (
    <a
      href={release.downloadUrl}
      className="group inline-flex items-center gap-3 rounded-card bg-accent px-5 py-3 text-sm font-semibold text-accent-ink transition-opacity duration-300 hover:opacity-85 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-fg"
    >
      Download for Windows
      {release.version && <span className="label rounded-card bg-black/15 px-2 py-1 text-[10px]">v{release.version}</span>}
      <span aria-hidden className="transition-transform duration-300 group-hover:translate-y-0.5">
        ↓
      </span>
    </a>
  );
}
