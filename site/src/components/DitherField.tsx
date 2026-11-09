import { useEffect, useRef } from "react";

// 4x4 Bayer matrix, normalised to 0..1 thresholds.
const BAYER = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5].map((v) => (v + 0.5) / 16);
const CELL = 4; // screen pixels per dither pixel

/**
 * Ambient ordered-dither field behind the hero: a slow flowing wave, rendered at 1/4 resolution
 * and scaled up with crisp pixels. Pauses offscreen; draws a single still frame for reduced motion.
 */
export function DitherField({ className = "" }: { className?: string }) {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = ref.current;
    const ctx = canvas?.getContext("2d");
    if (!canvas || !ctx) return;

    const still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    let w = 0;
    let h = 0;
    let image = ctx.createImageData(1, 1);
    let frame = 0;
    let visible = true;

    const resize = () => {
      w = Math.max(1, Math.ceil(canvas.clientWidth / CELL));
      h = Math.max(1, Math.ceil(canvas.clientHeight / CELL));
      canvas.width = w;
      canvas.height = h;
      image = ctx.createImageData(w, h);
    };

    const draw = (t: number) => {
      const d = image.data;
      const time = t * 0.00018;
      for (let y = 0; y < h; y++) {
        const ny = y / h;
        for (let x = 0; x < w; x++) {
          const nx = x / w;
          // Two interfering waves, strongest towards the right where the demo panel sits.
          let v =
            0.5 +
            0.25 * Math.sin(nx * 7 + time * 3 + Math.sin(ny * 4 - time * 2) * 1.6) +
            0.25 * Math.sin(ny * 9 - time * 2.4 + nx * 3);
          v *= 0.25 + nx * 0.85;
          v *= 1 - Math.abs(ny - 0.55) * 1.1;
          const on = v > BAYER[(y & 3) * 4 + (x & 3)];
          const i = (y * w + x) * 4;
          // Faint grey dots that read on both light and dark backgrounds.
          d[i] = on ? 115 : 0;
          d[i + 1] = on ? 115 : 0;
          d[i + 2] = on ? 115 : 0;
          d[i + 3] = on ? 70 : 0;
        }
      }
      ctx.putImageData(image, 0, 0);
    };

    const loop = (t: number) => {
      if (visible) draw(t);
      frame = requestAnimationFrame(loop);
    };

    resize();
    const ro = new ResizeObserver(() => {
      resize();
      if (still) draw(0);
    });
    ro.observe(canvas);
    const io = new IntersectionObserver(([e]) => (visible = e.isIntersecting));
    io.observe(canvas);

    if (still) draw(0);
    else frame = requestAnimationFrame(loop);

    return () => {
      cancelAnimationFrame(frame);
      ro.disconnect();
      io.disconnect();
    };
  }, []);

  return <canvas ref={ref} aria-hidden className={`pointer-events-none [image-rendering:pixelated] ${className}`} />;
}
