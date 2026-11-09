const CELL_WIDTH = 6;
const CELL_HEIGHT = 12;

/**
 * Portrait drawn as ASCII art (see site/scripts/ascii_portraits.py), so no photo is ever shipped.
 * Put it inside a `group` link: the characters turn Zap yellow on hover and keyboard focus.
 */
export function AsciiPortrait({ art, name }: { art: string[]; name: string }) {
  const cols = art[0]?.length ?? 0;

  return (
    <svg
      viewBox={`0 0 ${cols * CELL_WIDTH} ${art.length * CELL_HEIGHT}`}
      role="img"
      aria-label={name}
      className="block w-full fill-zinc-200 font-mono text-[10px] transition-[fill] duration-300 group-hover:fill-accent group-focus-visible:fill-accent"
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
  );
}
