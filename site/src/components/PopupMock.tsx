const ROWS = [
  ["`pemail", "personal email", "your.email@example.com"],
  ["`addr", "home address", "221B Example Street, …"],
  ["`thx", "thanks reply", "Thanks for reaching out! I'll …"],
];

/** Static picture of the Ctrl+Shift+Space search popup. */
export function PopupMock() {
  return (
    <div className="rounded-card border border-line bg-surface-2">
      <div className="flex items-center gap-3 border-b border-line px-4 py-3 font-mono text-sm">
        <span className="text-accent">⚡</span>
        <span className="text-white">em</span>
        <span className="caret -ml-2 inline-block h-4 w-px bg-white" />
      </div>
      <ul>
        {ROWS.map(([trigger, name, preview], i) => (
          <li
            key={trigger}
            className={`flex items-center gap-4 border-b border-line/60 px-4 py-3 last:border-0 ${i === 0 ? "bg-white/[0.04]" : ""}`}
          >
            <span className={`w-20 shrink-0 font-mono text-xs ${i === 0 ? "text-accent" : "text-zinc-400"}`}>{trigger}</span>
            <span className="min-w-0">
              <span className="block text-sm text-white">{name}</span>
              <span className="block truncate font-mono text-xs text-zinc-500">{preview}</span>
            </span>
          </li>
        ))}
      </ul>
      <div className="flex gap-4 border-t border-line px-4 py-2.5">
        {["↑↓ move", "enter paste", "esc close"].map((k) => (
          <span key={k} className="label text-[10px] text-zinc-500">
            {k}
          </span>
        ))}
      </div>
    </div>
  );
}
