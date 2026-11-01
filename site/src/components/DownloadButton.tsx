import type { Release } from "../useLatestRelease";

export function DownloadButton({ release }: { release: Release }) {
  return (
    <a
      href={release.downloadUrl}
      className="group inline-flex items-center gap-3 rounded-control bg-accent px-5 py-3 text-sm font-medium text-accent-ink shadow-[0_0_0_1px_rgba(250,204,21,0.4),0_8px_30px_-8px_rgba(250,204,21,0.6)] transition-all duration-300 hover:-translate-y-0.5 hover:shadow-[0_0_0_1px_rgba(250,204,21,0.6),0_12px_40px_-8px_rgba(250,204,21,0.8)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
    >
      Download for Windows
      {release.version && <span className="label rounded-full bg-black/15 px-2 py-1 text-[10px]">v{release.version}</span>}
      <span aria-hidden className="transition-transform duration-300 group-hover:translate-y-0.5">
        ↓
      </span>
    </a>
  );
}
