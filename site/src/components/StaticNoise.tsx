import { useEffect, useRef } from "react";

const FRAMES = 4; // pre-rendered noise frames, cycled so each tick is just a blit
const FPS = 12;
// One noise dot per physical pixel for the finest grain, capped so the frames stay a few MB.
const MAX_PIXELS = 1_500_000;

/**
 * TV-static "snow": grey noise flickering at a low frame rate. Frames are generated once per size
 * and cycled; pauses offscreen and holds a single still frame for reduced motion.
 */
export function StaticNoise({ className = "" }: { className?: string }) {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = ref.current;
    const ctx = canvas?.getContext("2d");
    if (!canvas || !ctx) return;

    const still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    let frames: ImageData[] = [];
    let index = 0;
    let timer = 0;
    let visible = true;

    const build = () => {
      const area = canvas.clientWidth * canvas.clientHeight;
      const density = Math.min(window.devicePixelRatio || 1, Math.sqrt(MAX_PIXELS / Math.max(1, area)));
      const w = Math.max(1, Math.ceil(canvas.clientWidth * density));
      const h = Math.max(1, Math.ceil(canvas.clientHeight * density));
      canvas.width = w;
      canvas.height = h;
      frames = Array.from({ length: still ? 1 : FRAMES }, () => {
        const image = ctx.createImageData(w, h);
        const d = image.data;
        for (let i = 0; i < d.length; i += 4) {
          // Skewed towards dark so the snow sits in the background; a few bright specks pop.
          const r = Math.random();
          const v = r > 0.988 ? 140 + Math.random() * 70 : r * r * 80;
          d[i] = d[i + 1] = d[i + 2] = v;
          d[i + 3] = 255;
        }
        return image;
      });
      ctx.putImageData(frames[0], 0, 0);
    };

    build();
    const ro = new ResizeObserver(build);
    ro.observe(canvas);
    const io = new IntersectionObserver(([e]) => (visible = e.isIntersecting));
    io.observe(canvas);

    if (!still) {
      timer = window.setInterval(() => {
        if (!visible || !frames.length) return;
        index = (index + 1) % frames.length;
        ctx.putImageData(frames[index], 0, 0);
      }, 1000 / FPS);
    }

    return () => {
      window.clearInterval(timer);
      ro.disconnect();
      io.disconnect();
    };
  }, []);

  return <canvas ref={ref} aria-hidden className={`pointer-events-none ${className}`} />;
}
