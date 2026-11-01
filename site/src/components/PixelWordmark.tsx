import { useEffect, useRef, useState } from "react";

// 5x7 bitmap glyphs; "#" is a lit pixel.
const GLYPHS: Record<string, string[]> = {
  Z: ["#####", "....#", "...#.", "..#..", ".#...", "#....", "#####"],
  A: ["..#..", ".#.#.", "#...#", "#...#", "#####", "#...#", "#...#"],
  P: ["####.", "#...#", "#...#", "####.", "#....", "#....", "#...."],
};
const GAP = 1; // empty columns between letters

/** The word drawn as pixels, which switch on in a scattered order the first time it scrolls into view. */
export function PixelWordmark({ text, className = "" }: { text: string; className?: string }) {
  const ref = useRef<SVGSVGElement>(null);
  const [on, setOn] = useState(false);

  const pixels: { x: number; y: number; delay: number }[] = [];
  let offset = 0;
  for (const letter of text) {
    const rows = GLYPHS[letter];
    if (!rows) continue;
    rows.forEach((row, y) =>
      [...row].forEach((c, x) => {
        // Deterministic pseudo-random delay so the scatter looks the same on every render.
        if (c === "#") pixels.push({ x: offset + x, y, delay: ((offset + x) * 37 + y * 53) % 17 });
      })
    );
    offset += rows[0].length + GAP;
  }
  const width = offset - GAP;

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      setOn(true);
      return;
    }
    const io = new IntersectionObserver(([e]) => {
      if (e.isIntersecting) {
        setOn(true);
        io.disconnect();
      }
    });
    io.observe(el);
    return () => io.disconnect();
  }, []);

  return (
    <svg ref={ref} viewBox={`0 0 ${width} 7`} shapeRendering="crispEdges" role="img" aria-label={text} className={className}>
      {pixels.map((p) => (
        <rect
          key={`${p.x}-${p.y}`}
          x={p.x}
          y={p.y}
          width={1.02}
          height={1.02}
          className="fill-zinc-100 transition-opacity duration-300"
          style={{ opacity: on ? 1 : 0, transitionDelay: `${p.delay * 40}ms` }}
        />
      ))}
    </svg>
  );
}
