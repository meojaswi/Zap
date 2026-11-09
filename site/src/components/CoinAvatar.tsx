const CELL_WIDTH = 6;
const CELL_HEIGHT = 12;

/**
 * Portrait drawn as ASCII art on a dark coin (see site/scripts/ascii_portraits.py), so no photo is
 * ever shipped. The characters turn Zap yellow when `active`. Falls back to initials with no art.
 */
export function CoinAvatar({
  art,
  name,
  active = false,
  className = "",
}: {
  art: string[];
  name: string;
  active?: boolean;
  className?: string;
}) {
  const initials = name
    .split(/\s+/)
    .map((w) => w[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();
  const cols = art[0]?.length ?? 0;

  return (
    <div className={`overflow-hidden rounded-full bg-[#050506] shadow-[inset_0_0_0_1px_rgba(255,255,255,0.12)] ${className}`}>
      {art.length ? (
        <svg
          viewBox={`0 0 ${cols * CELL_WIDTH} ${art.length * CELL_HEIGHT}`}
          role="img"
          aria-label={name}
          className={`size-full font-mono text-[10px] transition-[fill] duration-500 ${active ? "fill-accent" : "fill-zinc-200"}`}
        >
          {art.map((line, row) => (
            // textLength keeps the columns aligned whichever monospace font the visitor gets.
            <text
              key={row}
              y={(row + 1) * CELL_HEIGHT - 2}
              textLength={line.length * CELL_WIDTH}
              lengthAdjust="spacingAndGlyphs"
              style={{ whiteSpace: "pre" }}
            >
              {line}
            </text>
          ))}
        </svg>
      ) : (
        <div className="grid size-full place-items-center font-mono text-2xl font-semibold text-zinc-300">{initials}</div>
      )}
    </div>
  );
}
